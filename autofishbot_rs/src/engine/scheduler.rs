use crate::config::Config;
use crate::discord::client::DiscordClient;
use std::sync::Arc;
use log::{info, error, warn};

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
                interval: 10 * 60, // 10 minutes (example interval, wasn't in original but implied)
            });
        }

        // Clan Claim
        tasks.push(Task {
            name: "claim".to_string(),
            last_run: now,
            interval: 4 * 60 * 60, // 4 hours
        });

        // Boosts (Buy buffs) - simplified placeholder based on "boosts_length" logic
        // The original logic for buying boosts is complex, we just add the task scheduler slot here.
        if config.automation.boosts_length > 0 {
             tasks.push(Task {
                name: "shop buy".to_string(), // Placeholder for boost buying
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

                // If task name is composite like "shop buy", we need logic.
                // For simplicity/compatibility with "daily" / "claim" which are top level or simple:

                // Try fetching command first
                match client.get_command(&guild_id, &task.name).await {
                    Ok(Some(cmd)) => {
                        if let Err(e) = client.send_command(&guild_id, &channel_id, &cmd, None).await {
                            error!("Task {} failed: {}", task.name, e);
                        } else {
                            task.last_run = now;
                        }
                    },
                    Ok(None) => {
                         // Fallback logic
                         warn!("Command {} not found via discovery, skipping task.", task.name);
                    },
                    Err(e) => {
                         error!("Scheduler error fetching command {}: {}", task.name, e);
                    }
                }
            }
        }
    }
}
