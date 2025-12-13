use crate::config::Config;
use crate::discord::client::DiscordClient;
use std::sync::Arc;
use log::{info, error, warn};
use serde_json::{json, Value};

pub struct Scheduler {
    config: Config,
    tasks: Vec<Task>,
}

struct Task {
    name: String,
    last_run: u64,
    interval: u64,
}

impl Scheduler {
    pub fn new(config: Config) -> Self {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let mut tasks = Vec::new();

        if config.automation.auto_daily {
             tasks.push(Task {
                name: "daily".to_string(),
                last_run: now,
                interval: 24 * 60 * 60, // 24 hours
            });
        }

        if config.automation.auto_sell {
             tasks.push(Task {
                name: "sell".to_string(),
                last_run: now,
                interval: 10 * 60, // 10 minutes
            });
        }

        // Clan Claim
        tasks.push(Task {
            name: "claim".to_string(),
            last_run: now,
            interval: 4 * 60 * 60, // 4 hours
        });

        // Boosts (Buy buffs)
        if config.automation.boosts_length > 0 {
             tasks.push(Task {
                name: "shop buy".to_string(),
                last_run: now,
                interval: config.automation.boosts_length * 60,
            });
        }

        Self {
            config,
            tasks,
        }
    }

    pub async fn process(&mut self, client: &Arc<DiscordClient>) {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        for task in &mut self.tasks {
             if task.last_run > now { task.last_run = now; } // Sanity check

            if now - task.last_run > task.interval {
                info!("Running scheduled task: {}", task.name);

                let guild_id = self.config.system.guild_id.to_string();
                let channel_id = self.config.system.channel_id.to_string();

                let parts: Vec<&str> = task.name.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }
                let cmd_name = parts[0];
                let sub_parts = &parts[1..];

                // Try fetching command first
                match client.get_command(&guild_id, cmd_name).await {
                    Ok(Some(cmd)) => {
                        let options = Self::build_command_options(&cmd, sub_parts);

                        if let Err(e) = client.send_command(&guild_id, &channel_id, &cmd, options).await {
                            error!("Task {} failed: {}", task.name, e);
                        } else {
                            task.last_run = now;
                        }
                    },
                    Ok(None) => {
                         // Fallback logic
                         warn!("Command {} not found via discovery, skipping task.", cmd_name);
                    },
                    Err(e) => {
                         error!("Scheduler error fetching command {}: {}", cmd_name, e);
                    }
                }
            }
        }
    }

    fn build_command_options(cmd_def: &Value, parts: &[&str]) -> Option<Vec<Value>> {
        if parts.is_empty() {
            return None;
        }

        let current_part = parts[0];
        let remaining_parts = &parts[1..];

        // Find the option definition in the command
        if let Some(options_array) = cmd_def.get("options").and_then(|v| v.as_array()) {
            if let Some(option_def) = options_array.iter().find(|o| o["name"] == current_part) {
                // Recursively build options for the next part
                let child_options = Self::build_command_options(option_def, remaining_parts);

                let mut option_payload = json!({
                    "name": current_part,
                    "type": option_def["type"]
                });

                if let Some(opts) = child_options {
                     option_payload["options"] = json!(opts);
                } else {
                     option_payload["options"] = json!([]);
                }

                return Some(vec![option_payload]);
            } else {
                warn!("Subcommand/Option '{}' not found in definition.", current_part);
                return None;
            }
        }

        None
    }
}
