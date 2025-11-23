use crate::config::Config;
use crate::discord::client::DiscordClient;
use crate::engine::captcha::Captcha;
use crate::engine::scheduler::Scheduler;
use log::{info, warn};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use rand::Rng;

use crate::tui::app::App;

pub struct Bot {
    config: Config,
    client: Arc<DiscordClient>,
    scheduler: Arc<Mutex<Scheduler>>,
    captcha: Arc<Mutex<Captcha>>,
    app_state: Arc<Mutex<App>>,
    state: BotState,
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

        Self {
            config,
            client,
            scheduler,
            captcha,
            app_state,
            state: BotState::Idle,
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
                     let _ = self.client.send_command(&self.config.system.channel_id.to_string(), &self.config.system.channel_id.to_string(),
                        &crate::discord::types::ApplicationCommand {
                            id: "0".to_string(),
                            application_id: "574652751745777665".to_string(),
                            version: "0".to_string(),
                            default_permission: None,
                            default_member_permissions: None,
                            r#type: 1,
                            name: "fish".to_string(),
                            description: "".to_string(),
                            guild_id: None
                        }, None).await;

                    // Sleep random amount
                    let sleep_time = self.config.system.user_cooldown + rand::thread_rng().gen_range(0.5..2.0);
                    tokio::time::sleep(Duration::from_secs_f64(sleep_time)).await;
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
