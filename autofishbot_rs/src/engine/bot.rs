use crate::config::Config;
use crate::discord::client::DiscordClient;
use crate::engine::captcha::Captcha;
use crate::engine::scheduler::Scheduler;
use crate::engine::cooldown::CooldownManager;
use log::{info, warn};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use rand::Rng;

use crate::tui::app::App;

use crate::discord::types::ApplicationCommand;

pub struct Bot {
    config: Config,
    client: Arc<DiscordClient>,
    scheduler: Arc<Mutex<Scheduler>>,
    captcha: Arc<Mutex<Captcha>>,
    app_state: Arc<Mutex<App>>,
    state: BotState,
    fish_command: Option<ApplicationCommand>,
    pub cooldown_manager: Arc<Mutex<CooldownManager>>,
}

#[derive(Debug, PartialEq)]
enum BotState {
    Idle,
    Fishing,
    Captcha,
    Break,
}

impl Bot {
    pub fn new(config: Config, client: Arc<DiscordClient>, app_state: Arc<Mutex<App>>) -> Self {
        let scheduler = Arc::new(Mutex::new(Scheduler::new(config.clone())));
        let captcha = Arc::new(Mutex::new(Captcha::new(config.clone())));
        let cooldown_manager = Arc::new(Mutex::new(CooldownManager::new(config.system.user_cooldown)));

        Self {
            config,
            client,
            scheduler,
            captcha,
            app_state,
            state: BotState::Idle,
            fish_command: None,
            cooldown_manager,
        }
    }

    pub async fn run(&mut self) {
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

            if self.state == BotState::Idle {
                self.state = BotState::Fishing;
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
                    // Check cooldowns
                    // We need to coordinate with scheduler for other tasks

                    // Prioritize clicking "Play Again" button if available
                    // We need access to the last message state or similar.
                    // Ideally, we should have a shared state for "Last Actionable Component"

                    // For now, let's just fish.

                    // Send 'fish' command
                    info!("Fishing...");
                    {
                        let mut app = self.app_state.lock().await;
                        app.stats.fish_caught += 1; // Optimistic update
                    }

                    // Fish command
                    let guild_id = "1273750160022835301"; // Hardcoded for test

                    if self.fish_command.is_none() {
                        match self.client.get_command(guild_id, "fish").await {
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
                         if let Err(e) = self.client.send_command(guild_id, &self.config.system.channel_id.to_string(), cmd, None).await {
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
                BotState::Captcha => {
                    // Handled by event listener triggering solver
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
