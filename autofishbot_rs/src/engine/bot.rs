use crate::config::Config;
use crate::discord::client::DiscordClient;
use crate::engine::captcha::Captcha;
use crate::engine::scheduler::Scheduler;
use crate::engine::cooldown::CooldownManager;
use crate::engine::explorer::Explorer;
use crate::engine::database::Database;
use log::{info, warn};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use serde_json::Value;

use crate::tui::app::App;
// use crate::discord::types::ApplicationCommand; // Removed as we use Value now

pub struct Bot {
    config: Config,
    client: Arc<DiscordClient>,
    scheduler: Arc<Mutex<Scheduler>>,
    captcha: Arc<Mutex<Captcha>>,
    app_state: Arc<Mutex<App>>,
    state: BotState,
    fish_command: Option<Value>, // Changed to Value
    pub cooldown_manager: Arc<Mutex<CooldownManager>>,
    explorer: Arc<Mutex<Explorer>>,
    #[allow(dead_code)]
    database: Arc<Database>,
}

#[derive(Debug, PartialEq)]
enum BotState {
    Idle,
    Fishing,
    Captcha,
    #[allow(dead_code)] Break,
    Exploration, // New State
}

impl Bot {
    pub async fn new(config: Config, client: Arc<DiscordClient>, app_state: Arc<Mutex<App>>, database: Arc<Database>) -> Self {
        let scheduler = Arc::new(Mutex::new(Scheduler::new(config.clone())));
        let captcha = Arc::new(Mutex::new(Captcha::new(config.clone())));
        let cooldown_manager = Arc::new(Mutex::new(CooldownManager::new(config.system.user_cooldown)));

        // Initialize Explorer
        let guild_id = config.system.guild_id.to_string();
        let channel_id = config.system.channel_id.to_string();
        let explorer = Arc::new(Mutex::new(Explorer::new(client.clone(), database.clone(), guild_id, channel_id)));

        Self {
            config,
            client,
            scheduler,
            captcha,
            app_state,
            state: BotState::Idle,
            fish_command: None,
            cooldown_manager,
            explorer,
            database,
        }
    }

    pub async fn run(&mut self) {
        // Startup delay to prevent rate limit spikes
        info!("Bot warming up... waiting 5 seconds.");
        tokio::time::sleep(Duration::from_secs(5)).await;

        loop {
            // Check if bot is running from TUI state
            let is_running = {
                let app = self.app_state.lock().await;
                app.is_running
            };

            if !is_running {
                self.state = BotState::Idle;
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }

            // Simple Logic: If running, default to Exploration for this test
            // In real app, we might toggle between Fishing and Exploration
            if self.state == BotState::Idle {
                self.state = BotState::Exploration;
                {
                    let mut explorer = self.explorer.lock().await;
                    explorer.start().await;
                }
            }

            // Check Captcha
            let captcha_detected = {
                self.captcha.lock().await.detected
            };

            if captcha_detected {
                self.state = BotState::Captcha;
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }

            match self.state {
                BotState::Fishing => {
                    // Perform fishing action
                    info!("Fishing...");
                    {
                        let mut app = self.app_state.lock().await;
                        app.stats.fish_caught += 1; // Optimistic update
                    }

                    // Fish command
                    let guild_id = self.config.system.guild_id.to_string();

                    if self.fish_command.is_none() {
                        match self.client.get_command(&guild_id, "fish").await {
                            Ok(Some(cmd)) => {
                                info!("Found fish command: {:?}", cmd);
                                self.fish_command = Some(cmd);
                            },
                            Ok(None) => {
                                log::error!("Could not find 'fish' command in guild");
                            },
                            Err(e) => {
                                log::error!("Failed to fetch commands: {}", e);
                            }
                        }
                    }

                    if let Some(cmd) = &self.fish_command {
                         if let Err(e) = self.client.send_command(&guild_id, &self.config.system.channel_id.to_string(), cmd, None).await {
                            log::error!("Failed to send fish command: {}", e);
                        }
                    }

                    // Sleep random amount using Dynamic Cooldown Manager
                    let sleep_duration = {
                        let manager = self.cooldown_manager.lock().await;
                        manager.get_sleep_time()
                    };

                    info!("Sleeping for {:.2}s", sleep_duration.as_secs_f64());
                    tokio::time::sleep(sleep_duration).await;
                },
                BotState::Exploration => {
                    // FIX: We need to access the last full message.
                    let last_msg_obj = {
                         let app = self.app_state.lock().await;
                         app.last_message_object.clone()
                    };

                    {
                        let mut explorer = self.explorer.lock().await;
                        explorer.tick(last_msg_obj.as_ref()).await;
                    }

                    tokio::time::sleep(Duration::from_secs(1)).await;
                },
                BotState::Captcha => {
                    warn!("Waiting for captcha solution...");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                },
                BotState::Break => {
                    info!("Taking a break...");
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    self.state = BotState::Fishing;
                },
                BotState::Idle => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }

            // Run Scheduler
            {
                let mut sched = self.scheduler.lock().await;
                sched.process(&self.client).await;
            }
        }
    }
}
