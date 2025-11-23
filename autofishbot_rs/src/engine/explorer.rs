use std::sync::Arc;
use log::{info, error, warn};
use std::time::Duration;
use crate::discord::client::DiscordClient;
use crate::engine::database::Database;
use crate::discord::types::{ApplicationCommand, Message};
use crate::engine::parser::{self};

pub struct Explorer {
    client: Arc<DiscordClient>,
    db: Arc<Database>, // Shared via Arc, not Mutex because Database methods take &self
    guild_id: String,
    channel_id: String,
    known_commands: Vec<ApplicationCommand>,
    target_commands: Vec<String>,
    current_command_index: usize,
    state: ExplorerState,
    discovery_attempts: u32,
}

#[derive(Debug, PartialEq)]
enum ExplorerState {
    Idle,
    DiscoveringCommands,
    ExecutingCommand,
    WaitingForResponse,
    #[allow(dead_code)] NavigatingPagination,
    #[allow(dead_code)]
    NavigatingSubmenu,
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

                        // Register all commands in DB
                        for cmd in &self.known_commands {
                             let params = serde_json::to_string(&cmd.options).unwrap_or_default();
                             let _ = self.db.register_command(&cmd.name, &cmd.description, &params).await;
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
                            // Exponential backoff: 2^attempts * 2 seconds (e.g., 2, 4, 8, 16...)
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

                // Handle subcommands (e.g. "prestige shop")
                let parts: Vec<&str> = cmd_name.split_whitespace().collect();
                let main_name = parts[0];
                let sub_name = if parts.len() > 1 { Some(parts[1]) } else { None };

                if let Some(cmd) = self.known_commands.iter().find(|c| c.name == main_name) {
                     // Prepare options if subcommand
                    let options = if let Some(sub) = sub_name {
                        // Find subcommand option type
                         if let Some(opts) = &cmd.options {
                             if let Some(sub_opt) = opts.iter().find(|o| o.name == sub) {
                                  // Construct payload for subcommand
                                  Some(vec![serde_json::json!({
                                      "name": sub,
                                      "type": sub_opt.r#type,
                                      // If the subcommand itself has required options, we might fail here.
                                      // For this task, we assume no arguments needed for shops.
                                  })])
                             } else {
                                 warn!("Subcommand {} not found for {}", sub, main_name);
                                 None
                             }
                         } else {
                             None
                         }
                    } else {
                        // If no explicit subcommand requested, check if we NEED one
                        if let Some(opts) = &cmd.options {
                            // If there are options and they are SUB_COMMAND (1) or SUB_COMMAND_GROUP (2), we must pick one.
                            if let Some(first_sub) = opts.iter().find(|o| o.r#type == 1 || o.r#type == 2) {
                                info!("Auto-selecting first subcommand: {} for {}", first_sub.name, main_name);
                                Some(vec![serde_json::json!({
                                    "name": first_sub.name,
                                    "type": first_sub.r#type,
                                    // Recursive subcommands not handled deep enough here, assuming 1 level
                                })])
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };

                    if let Err(e) = self.client.send_command(&self.guild_id, &self.channel_id, cmd, options).await {
                        error!("Failed to execute {}: {}", cmd_name, e);
                    } else {
                        let _ = self.db.mark_command_executed(cmd_name).await;
                        self.state = ExplorerState::WaitingForResponse;
                    }
                } else {
                    warn!("Command {} not found in guild (or fallback list incomplete).", main_name);
                    self.advance_command();
                }
            },
            ExplorerState::WaitingForResponse => {
                // Check if last message matches what we expect
                // For simplicity, we just wait a bit and parse whatever appears.
                // In a real robust system, we'd check interaction IDs.
                tokio::time::sleep(Duration::from_secs(3)).await;

                if let Some(msg) = last_message {
                    self.parse_and_save(msg).await;

                    // Check for pagination or submenus
                    if self.has_pagination(msg) {
                         self.handle_pagination(msg).await;
                         // state remains WaitingForResponse (conceptually), but we need to wait for update
                         // actually handle_pagination clicks the button.
                         // We should wait again.
                    } else {
                        self.advance_command();
                    }
                } else {
                    // No message? Maybe lag.
                    self.advance_command();
                }
            },
            ExplorerState::NavigatingPagination => {
                 // Logic to handle multiple pages
            },
            ExplorerState::Cooldown => {
                tokio::time::sleep(Duration::from_secs(3600)).await;
                self.state = ExplorerState::DiscoveringCommands;
            },
            _ => {}
        }
    }

    fn load_fallback_commands(&mut self) {
        // Fallback IDs are dummies, but structure mimics real commands to allow logic to proceed.
        // We assume typical VF structure.
        // This allows the explorer to try executing them even if discovery failed.
        let app_id = "574652751745777665".to_string();

        let make_cmd = |name: &str, desc: &str, options: Option<Vec<crate::discord::types::ApplicationCommandOption>>| -> ApplicationCommand {
             ApplicationCommand {
                id: "0".to_string(), // Unknown
                application_id: app_id.clone(),
                version: "1".to_string(),
                default_permission: Some(true),
                default_member_permissions: None,
                r#type: Some(1),
                name: name.to_string(),
                description: desc.to_string(),
                guild_id: None,
                options,
             }
        };

        // Shop typically has subcommands: view, buy, etc. But if we just send "shop", maybe it defaults?
        // Or we need to guess "view". Let's guess "view".
        let shop_opts = vec![
            crate::discord::types::ApplicationCommandOption {
                r#type: 1, // Subcommand
                name: "view".to_string(),
                description: "View the shop".to_string(),
                required: None,
                choices: None,
                options: None,
            }
        ];

        self.known_commands = vec![
            make_cmd("shop", "Open shop", Some(shop_opts)),
            make_cmd("fishdex", "View fishdex", None),
            make_cmd("buffs", "View buffs", None),
            make_cmd("boosters", "View boosters", None),
            make_cmd("daily", "Daily reward", None),
            make_cmd("profile", "View profile", None),
            make_cmd("quests", "View quests", None),
            // Prestige shop usually is a subcommand of prestige? or /prestige shop?
            // "prestige" command with "shop" subcommand
             make_cmd("prestige", "Prestige commands", Some(vec![
                  crate::discord::types::ApplicationCommandOption {
                    r#type: 1,
                    name: "shop".to_string(),
                    description: "Prestige shop".to_string(),
                    required: None,
                    choices: None,
                    options: None,
                }
             ])),
        ];
    }

    fn advance_command(&mut self) {
        self.current_command_index += 1;
        self.state = ExplorerState::ExecutingCommand;
        // Add a small delay between commands
        // We can't use tokio::sleep here easily if we are inside tick which might be synchronous or we want to return.
        // But since tick is async, we can.
    }

    async fn parse_and_save(&self, msg: &Message) {
        // Log generic entity first
        // Message.embeds is Vec<Embed>, it is not Option.
        let embeds = &msg.embeds;
        if !embeds.is_empty() {
            for embed in embeds {
                let title = embed.title.clone().unwrap_or_default();
                let desc = embed.description.clone().unwrap_or_default();

                // Try Parse Shop
                let items = parser::parse_shop_embed(&title, &desc, embed.fields.as_ref());
                if !items.is_empty() {
                    info!("Found {} items in {}", items.len(), title);
                    for item in items {
                        let _ = self.db.upsert_shop_item(&item.name, &title, item.price, &item.currency, &item.description, item.stock).await;
                    }
                } else {
                    // Fallback: Generic Entity Log
                    let entities = parser::parse_generic_list(&title, &desc);
                     if !entities.is_empty() {
                         info!("Found {} generic entities in {}", entities.len(), title);
                         for entity in entities {
                             let _ = self.db.upsert_game_entity(&entity.entity_type, &entity.name, &entity.details).await;
                         }
                     } else {
                         // Even if parser failed, log the raw text as a "RawEmbed" entity so we don't lose data
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
                        // Check for "Next" button (usually label "Next" or emoji arrow)
                        if let Some(label) = &comp.label {
                            if label.contains("Next") || label.contains(">") {
                                return true; // AND ensure it's not disabled? We don't have disabled field in types yet properly mapped maybe
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
                                    let _ = self.client.interact_component(&self.guild_id, &self.channel_id, &msg.id, custom_id).await;
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
