use std::sync::Arc;
use log::{info, error, warn};
use std::time::Duration;
use crate::discord::client::DiscordClient;
use crate::engine::database::Database;
use crate::discord::types::{Message};
use crate::engine::parser::{self};
use serde_json::Value;

pub struct Explorer {
    client: Arc<DiscordClient>,
    db: Arc<Database>, // Shared via Arc, not Mutex because Database methods take &self
    guild_id: String,
    channel_id: String,
    known_commands: Vec<Value>, // Changed to Value to hold raw JSON
    target_commands: Vec<String>,
    current_command_index: usize,
    state: ExplorerState,
    discovery_attempts: u32,

    // Submenu navigation tracking
    submenu_custom_id: Option<String>,
    submenu_options: Vec<parser::SelectMenuOption>,
    current_submenu_index: usize,
    current_message_id: String,
}

#[derive(Debug, PartialEq)]
enum ExplorerState {
    Idle,
    DiscoveringCommands,
    ExecutingCommand,
    WaitingForResponse,
    NavigatingSubmenu,
    WaitingForSubmenuResponse,
    #[allow(dead_code)] NavigatingPagination,
    Cooldown,
}

impl Explorer {
    pub fn new(client: Arc<DiscordClient>, db: Arc<Database>, guild_id: String, channel_id: String) -> Self {
        Self {
            client,
            db,
            guild_id,
            channel_id,
            known_commands: Vec::new(),
            // Priority list of commands to explore
            target_commands: vec![
                "shop".to_string(),
                "fishdex".to_string(),
                "buffs".to_string(),
                "boosters".to_string(),
                "prestige shop".to_string(),
                "clan shop".to_string(),
                "daily".to_string(), // Just to log it
                "quests".to_string(), // If it exists
            ],
            current_command_index: 0,
            state: ExplorerState::Idle,
            discovery_attempts: 0,
            submenu_custom_id: None,
            submenu_options: Vec::new(),
            current_submenu_index: 0,
            current_message_id: String::new(),
        }
    }

    pub async fn start(&mut self) {
        info!("Starting Explorer Mode...");
        self.state = ExplorerState::DiscoveringCommands;
        self.discovery_attempts = 0;
    }

    pub async fn tick(&mut self, last_message: Option<&Message>) {
        match self.state {
            ExplorerState::Idle => {
                // Do nothing
            },
            ExplorerState::DiscoveringCommands => {
                info!("Discovering commands (Attempt {})...", self.discovery_attempts + 1);
                match self.client.get_commands(&self.guild_id).await {
                    Ok(cmds) => {
                        self.known_commands = cmds;
                        info!("Discovered {} commands.", self.known_commands.len());

                        // Register all commands in DB with full raw structure
                        for cmd in &self.known_commands {
                             let name = cmd["name"].as_str().unwrap_or("unknown");
                             let desc = cmd["description"].as_str().unwrap_or("");
                             let params = serde_json::to_string(&cmd["options"]).unwrap_or_default();
                             let structure = serde_json::to_string_pretty(&cmd).unwrap_or_default();

                             let _ = self.db.register_command(name, desc, &params, &structure).await;
                        }

                        self.state = ExplorerState::ExecutingCommand;
                        self.discovery_attempts = 0;
                    },
                    Err(e) => {
                        error!("Failed to discover commands: {}", e);
                        self.discovery_attempts += 1;

                        if self.discovery_attempts >= 5 {
                            warn!("Max discovery attempts reached. Loading fallback commands.");
                            self.load_fallback_commands();
                            self.state = ExplorerState::ExecutingCommand;
                            self.discovery_attempts = 0;
                        } else {
                            // Exponential backoff: 2^attempts * 2 seconds
                            let backoff = 2u64.pow(self.discovery_attempts) * 2;
                            warn!("Retrying discovery in {} seconds...", backoff);
                            tokio::time::sleep(Duration::from_secs(backoff)).await;
                        }
                    }
                }
            },
            ExplorerState::ExecutingCommand => {
                if self.current_command_index >= self.target_commands.len() {
                    info!("Exploration cycle complete. Restarting in 1 hour.");
                    self.current_command_index = 0;
                    self.state = ExplorerState::Cooldown;
                    return;
                }

                let cmd_name = &self.target_commands[self.current_command_index];
                info!("Exploring command: {}", cmd_name);

                // Handle subcommands
                let parts: Vec<&str> = cmd_name.split_whitespace().collect();
                let main_name = parts[0];

                // Find command in known_commands (Vec<Value>)
                if let Some(cmd) = self.known_commands.iter().find(|c| c["name"] == main_name) {
                     // Prepare options (deep structure)
                    let options = self.build_command_options(&parts[1..], cmd["options"].as_array());

                    // Pass the whole cmd Value
                    if let Err(e) = self.client.send_command(&self.guild_id, &self.channel_id, cmd, options).await {
                        error!("Failed to execute {}: {}", cmd_name, e);
                    } else {
                        let _ = self.db.mark_command_executed(cmd_name).await;
                        self.state = ExplorerState::WaitingForResponse;
                    }
                } else {
                    warn!("Command {} not found in guild.", main_name);
                    self.advance_command();
                }
            },
            ExplorerState::WaitingForResponse => {
                tokio::time::sleep(Duration::from_secs(3)).await;

                if let Some(msg) = last_message {
                    self.parse_and_save(msg).await;

                    // Check for submenu first
                    if let Some((custom_id, options)) = parser::parse_select_menu_options(msg) {
                        info!("Found submenu with {} options.", options.len());
                        self.submenu_custom_id = Some(custom_id);
                        self.submenu_options = options;
                        self.current_submenu_index = 0;
                        self.current_message_id = msg.id.clone();
                        self.state = ExplorerState::NavigatingSubmenu;
                    } else if self.has_pagination(msg) {
                         self.handle_pagination(msg).await;
                    } else {
                        self.advance_command();
                    }
                } else {
                    self.advance_command();
                }
            },
            ExplorerState::NavigatingSubmenu => {
                if self.current_submenu_index >= self.submenu_options.len() {
                    info!("Finished submenu exploration.");
                    self.submenu_custom_id = None;
                    self.submenu_options.clear();
                    self.advance_command();
                    return;
                }

                let option = &self.submenu_options[self.current_submenu_index];
                info!("Selecting submenu option: {}", option.label);

                if let Some(custom_id) = &self.submenu_custom_id {
                    let values = vec![option.value.clone()];
                    if let Err(e) = self.client.interact_component(&self.guild_id, &self.channel_id, &self.current_message_id, custom_id, Some(3), Some(values)).await {
                        error!("Failed to select option: {}", e);
                        // Skip if failed
                        self.current_submenu_index += 1;
                    } else {
                        self.state = ExplorerState::WaitingForSubmenuResponse;
                    }
                } else {
                    self.advance_command();
                }
            },
            ExplorerState::WaitingForSubmenuResponse => {
                tokio::time::sleep(Duration::from_secs(4)).await;

                match self.client.get_message(&self.channel_id, &self.current_message_id).await {
                    Ok(msg) => {
                        self.parse_and_save(&msg).await;
                        self.current_message_id = msg.id.clone();
                    },
                    Err(e) => {
                        error!("Failed to fetch updated message in submenu: {}", e);
                    }
                }

                self.current_submenu_index += 1;
                self.state = ExplorerState::NavigatingSubmenu;
            },
            ExplorerState::NavigatingPagination => {},
            ExplorerState::Cooldown => {
                tokio::time::sleep(Duration::from_secs(3600)).await;
                self.state = ExplorerState::DiscoveringCommands;
            },
        }
    }

    fn build_command_options(&self, path: &[&str], schema_options: Option<&Vec<Value>>) -> Option<Vec<Value>> {
        let opts = match schema_options {
            Some(o) => o,
            None => return Some(Vec::new()),
        };

        if let Some(&target) = path.first() {
            // Explicit path navigation
            if let Some(option_def) = opts.iter().find(|o| o["name"] == target) {
                 let sub_options = self.build_command_options(&path[1..], option_def["options"].as_array());

                 if let Some(sub_opts_vec) = sub_options {
                     Some(vec![serde_json::json!({
                         "name": target,
                         "type": option_def["type"],
                         "options": sub_opts_vec
                     })])
                 } else {
                     None
                 }
            } else {
                warn!("Subcommand/Option '{}' not found.", target);
                None
            }
        } else {
            // End of explicit path. Check if we need to auto-select a child.
            // Look for type 1 (SUB_COMMAND) or 2 (SUB_COMMAND_GROUP)
            if let Some(first_sub) = opts.iter().find(|o| {
                let t = o["type"].as_u64().unwrap_or(0);
                t == 1 || t == 2
            }) {
                let name = first_sub["name"].as_str().unwrap_or("unknown");
                info!("Auto-selecting subcommand: {}", name);

                let sub_options = self.build_command_options(&[], first_sub["options"].as_array());
                let sub_opts_vec = sub_options.unwrap_or_default();

                Some(vec![serde_json::json!({
                    "name": name,
                    "type": first_sub["type"],
                    "options": sub_opts_vec
                })])
            } else {
                // No subcommands to select, we are at leaf (or only have parameters which we ignore for now)
                Some(Vec::new())
            }
        }
    }

    fn load_fallback_commands(&mut self) {
        // Fallback IDs are dummies
        let app_id = "574652751745777665".to_string();

        let make_cmd = |name: &str, desc: &str, options: Option<Vec<Value>>| -> Value {
             serde_json::json!({
                "id": "0",
                "application_id": app_id,
                "version": "1",
                "default_permission": true,
                "type": 1,
                "name": name,
                "description": desc,
                "options": options.unwrap_or_default()
             })
        };

        // Shop fallback
        let shop_opts = vec![
            serde_json::json!({
                "type": 1,
                "name": "view",
                "description": "View the shop",
            })
        ];

        self.known_commands = vec![
            make_cmd("shop", "Open shop", Some(shop_opts)),
            make_cmd("fishdex", "View fishdex", None),
            make_cmd("buffs", "View buffs", None),
            make_cmd("boosters", "View boosters", None),
            make_cmd("daily", "Daily reward", None),
            make_cmd("profile", "View profile", None),
            make_cmd("quests", "View quests", None),
             make_cmd("prestige", "Prestige commands", Some(vec![
                  serde_json::json!({
                    "type": 1,
                    "name": "shop",
                    "description": "Prestige shop",
                })
             ])),
        ];
    }

    fn advance_command(&mut self) {
        self.current_command_index += 1;
        self.state = ExplorerState::ExecutingCommand;
    }

    async fn parse_and_save(&self, msg: &Message) {
        let embeds = &msg.embeds;
        if !embeds.is_empty() {
            for embed in embeds {
                let title = embed.title.clone().unwrap_or_default();
                let desc = embed.description.clone().unwrap_or_default();

                let items = parser::parse_shop_embed(&title, &desc, embed.fields.as_ref());
                if !items.is_empty() {
                    info!("Found {} items in {}", items.len(), title);
                    for item in items {
                        let _ = self.db.upsert_shop_item(&item.name, &title, item.price, &item.currency, &item.description, item.stock, item.stats.as_deref()).await;
                    }
                } else {
                    let entities = parser::parse_generic_list(&title, &desc);
                     if !entities.is_empty() {
                         info!("Found {} generic entities in {}", entities.len(), title);
                         for entity in entities {
                             let _ = self.db.upsert_game_entity(&entity.entity_type, &entity.name, &entity.details).await;
                         }
                     } else {
                         let _ = self.db.upsert_game_entity("RawEmbed", &title, &desc).await;
                     }
                }
            }
        }
    }

    fn has_pagination(&self, msg: &Message) -> bool {
        if let Some(components) = &msg.components {
            for row in components {
                if let Some(comps) = &row.components {
                    for comp in comps {
                        if let Some(label) = &comp.label {
                            if label.contains("Next") || label.contains(">") {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    async fn handle_pagination(&self, msg: &Message) {
        if let Some(components) = &msg.components {
            for row in components {
                if let Some(comps) = &row.components {
                    for comp in comps {
                        if let Some(custom_id) = &comp.custom_id {
                             if let Some(label) = &comp.label {
                                if label.contains("Next") || label.contains(">") {
                                    info!("Clicking Next Page...");
                                    let _ = self.client.interact_component(&self.guild_id, &self.channel_id, &msg.id, custom_id, Some(2), None).await;
                                    tokio::time::sleep(Duration::from_secs(2)).await;
                                    return;
                                }
                             }
                        }
                    }
                }
            }
        }
    }
}
