use crate::config::Config;
use crate::discord::client::DiscordClient;
use crate::engine::captcha::Captcha;
use crate::engine::scheduler::Scheduler;
use crate::engine::cooldown::CooldownManager;
use crate::engine::explorer::Explorer;
use crate::engine::database::Database;
use crate::engine::optimizer::Optimizer;
use crate::engine::game_data::{RodType, BoatType, Biome, FISH_DATA, ROD_DATA, BOAT_DATA};
use crate::engine::parser;
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
    optimizer: Arc<Mutex<Optimizer>>,
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

        // Initialize Optimizer
        let mut optimizer = Optimizer::new();
        if let Ok(stats) = database.load_biome_stats().await {
            optimizer.biome_knowledge = stats;
        }
        let optimizer = Arc::new(Mutex::new(optimizer));

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
            optimizer,
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

            // Simple Logic: If running, default to Fishing
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
                    // 1. Analyze previous state / message
                    let (last_msg, profile_data) = {
                        let app = self.app_state.lock().await;
                        (app.last_message_object.clone(), app.profile.clone())
                    };

                    let current_biome = match profile_data.biome.as_str() {
                        "Volcanic" => Biome::Volcanic,
                        "Ocean" => Biome::Ocean,
                        "Sky" => Biome::Sky,
                        "Space" => Biome::Space,
                        "Alien" => Biome::Alien,
                        _ => Biome::River,
                    };

                    // Check if we caught something in the last message
                    if let Some(msg) = last_msg {
                         for embed in &msg.embeds {
                             if let Some(desc) = &embed.description {
                                 if let Some(catch) = parser::parse_catch_embed(desc) {
                                     // Calculate Gold
                                     let mut total_gold = 0;
                                     let mut total_fish = 0;
                                     for (fish_name, count) in &catch.fish {
                                         let price = FISH_DATA.get(fish_name.as_str()).map(|f| f.price).unwrap_or(0);
                                         total_gold += price * (*count as u64);
                                         total_fish += *count as u64;
                                     }

                                     // Update Optimizer
                                     if total_fish > 0 {
                                         let mut opt = self.optimizer.lock().await;
                                         let stats = opt.biome_knowledge.entry(current_biome).or_default();
                                         stats.update(total_gold, catch.xp as u64, total_fish);

                                         // Save periodically
                                         if stats.total_catches % 50 == 0 {
                                             let _ = self.database.save_biome_stats(&format!("{:?}", current_biome), stats).await;
                                         }
                                         info!("Learned: {} gold, {} xp from {} fish in {:?}", total_gold, catch.xp, total_fish, current_biome);
                                     }
                                 }
                             }
                         }
                    }

                    // 2. Optimization / Recommendation
                    {
                        let rod_name = profile_data.rod;
                        let balance_str = profile_data.balance;

                        // Parse balance: "$1,234,567" -> 1234567
                        let current_balance = balance_str
                            .replace('$', "")
                            .replace(',', "")
                            .trim()
                            .parse::<u64>()
                            .unwrap_or(0);

                        let current_rod = ROD_DATA.values().find(|r| r.name == rod_name)
                             .or_else(|| ROD_DATA.get(&RodType::Plastic));

                        let current_boat = BOAT_DATA.get(&BoatType::Rowboat); // Default to Rowboat as Profile doesn't track boat yet

                        if let (Some(rod), Some(boat)) = (current_rod, current_boat) {
                            let opt = self.optimizer.lock().await;
                            let recs = opt.solve_next_move(rod, boat, current_biome);

                            if let Some(best) = recs.first() {
                                 info!("ROI Recommendation: {} {} ({:.2}s)", best.action, best.target_name, best.roi_seconds);

                                 if current_balance > best.cost {
                                     info!("READY TO BUY: {} {} for {} (Balance: {})", best.action, best.target_name, best.cost, current_balance);
                                 }
                            }
                        } else {
                            warn!("Critical: Could not load Game Data for optimization.");
                        }
                    }

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
