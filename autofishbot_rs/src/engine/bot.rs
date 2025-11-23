use crate::config::Config;
use crate::discord::client::DiscordClient;
use crate::engine::captcha::Captcha;
use crate::engine::scheduler::Scheduler;
use crate::engine::cooldown::CooldownManager;
use crate::engine::explorer::Explorer;
use crate::engine::database::Database;
use crate::engine::optimizer::{Optimizer, ActionType, Recommendation};
use crate::engine::game_data::{RodType, BoatType, Biome, FISH_DATA, ROD_DATA, BOAT_DATA};
use crate::engine::parser;
use log::{info, warn};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{Duration, Instant};
use serde_json::Value;

use crate::tui::app::App;

pub struct Bot {
    config: Config,
    client: Arc<DiscordClient>,
    scheduler: Arc<Mutex<Scheduler>>,
    captcha: Arc<Mutex<Captcha>>,
    app_state: Arc<Mutex<App>>,
    state: BotState,
    fish_command: Option<Value>,
    shop_command: Option<Value>,
    biome_command: Option<Value>,
    sell_command: Option<Value>,
    coinflip_command: Option<Value>,
    pub cooldown_manager: Arc<Mutex<CooldownManager>>,
    explorer: Arc<Mutex<Explorer>>,
    optimizer: Arc<Mutex<Optimizer>>,
    database: Arc<Database>,
    last_action: Option<(ActionType, Instant)>,
    pending_recommendation: Option<Recommendation>,
}

#[derive(Debug, PartialEq)]
enum BotState {
    Idle,
    Fishing,
    Captcha,
    #[allow(dead_code)] Break,
    Exploration,
    Selling,
    Shopping,
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
            shop_command: None,
            biome_command: None,
            sell_command: None,
            coinflip_command: None,
            cooldown_manager,
            explorer,
            optimizer,
            database,
            last_action: None,
            pending_recommendation: None,
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
                    if let Some(msg) = &last_msg {
                         for embed in &msg.embeds {
                             if let Some(desc) = &embed.description {
                                 // Auto-Sell Check
                                 if desc.to_lowercase().contains("full") {
                                     info!("Inventory Full detected! Switching to Selling.");
                                     self.state = BotState::Selling;
                                     continue; // Break loop iteration to switch state immediately
                                 }

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
                                             if let Err(e) = self.database.save_biome_stats(&format!("{:?}", current_biome), stats).await {
                                                 warn!("Failed to save biome stats: {}", e);
                                             }
                                         }
                                         info!("Learned: {} gold, {} xp from {} fish in {:?}", total_gold, catch.xp, total_fish, current_biome);
                                     }
                                 }
                             }
                         }
                    }

                    // 2. Optimization / Recommendation / Autonomy
                    {
                        let rod_name = profile_data.rod.clone();
                        let balance_str = profile_data.balance.clone();

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

                            let current_gps = opt.calculate_metrics(rod, boat, current_biome, &profile_data);
                            let recs = opt.solve_next_move(rod, boat, current_biome, &profile_data, current_balance);

                            if let Some(best) = recs.first() {
                                 // Update Strategy Info
                                 {
                                     let mut app = self.app_state.lock().await;
                                     app.strategy.current_goal = format!("{} ({:?})", best.target_name, best.action);
                                     app.strategy.current_gps = format!("${:.2}/s", current_gps);
                                     app.strategy.progress = format!("{} / {} ({:.1}%)",
                                         current_balance, best.cost,
                                         if best.cost > 0 { (current_balance as f64 / best.cost as f64) * 100.0 } else { 100.0 }
                                     );
                                     app.strategy.est_time = format!("{:.1} mins", best.roi_seconds / 60.0);
                                 }

                                 info!("ROI Recommendation: {:?} {} ({:.2}s)", best.action, best.target_name, best.roi_seconds);

                                 let now = Instant::now();
                                 let is_repeat = if let Some((last_type, last_time)) = &self.last_action {
                                     // For Coinflip, we assume 'action' enum variant equality checks variants.
                                     // But Coinflip has data. PartialEq on enum compares data too.
                                     // So if amount is different, it's not repeat. Good.
                                     *last_type == best.action && now.duration_since(*last_time) < Duration::from_secs(15)
                                 } else {
                                     false
                                 };

                                 // Autonomy Check
                                 if !is_repeat {
                                     let guild_id = self.config.system.guild_id.to_string();
                                     let channel_id = self.config.system.channel_id.to_string();

                                     match &best.action {
                                         ActionType::BuyRod | ActionType::BuyBoat if current_balance >= best.cost => {
                                             info!("AUTONOMOUS ACTION: Transitioning to Shopping for {}", best.target_name);
                                             self.pending_recommendation = Some(best.clone());
                                             self.state = BotState::Shopping;
                                             continue; // Switch state
                                         },
                                         ActionType::Travel => {
                                             info!("AUTONOMOUS ACTION: Traveling to {}", best.target_name);
                                             if self.biome_command.is_none() {
                                                 self.biome_command = self.client.get_command(&guild_id, "biome").await.unwrap_or(None);
                                             }
                                             if let Some(cmd) = &self.biome_command {
                                                 let options = vec![
                                                     serde_json::json!({ "name": "biome", "value": best.target_name })
                                                 ];
                                                 let _ = self.client.send_command(&guild_id, &channel_id, cmd, Some(options)).await;
                                                 self.last_action = Some((ActionType::Travel, now));

                                                 {
                                                     let mut app = self.app_state.lock().await;
                                                     app.profile.biome = best.target_name.clone();
                                                 }
                                                 tokio::time::sleep(Duration::from_secs(3)).await;
                                             }
                                         },
                                         ActionType::Coinflip { amount, .. } if self.config.automation.danger_mode => {
                                             info!("AUTONOMOUS ACTION: Coinflip {} for {}", amount, best.target_name);
                                             if self.coinflip_command.is_none() {
                                                  self.coinflip_command = self.client.get_command(&guild_id, "coinflip").await.unwrap_or(None);
                                             }
                                             if let Some(cmd) = &self.coinflip_command {
                                                 // /coinflip [amount] heads
                                                 let options = vec![
                                                     serde_json::json!({ "name": "amount", "value": amount }),
                                                     serde_json::json!({ "name": "side", "value": "heads" })
                                                 ];
                                                 let _ = self.client.send_command(&guild_id, &channel_id, cmd, Some(options)).await;
                                                 self.last_action = Some((best.action.clone(), now));
                                                 tokio::time::sleep(Duration::from_secs(5)).await;
                                             }
                                         },
                                         _ => {}
                                     }
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
                                self.fish_command = Some(cmd);
                            },
                            Ok(None) => {},
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
                BotState::Selling => {
                    info!("Performing Auto-Sell...");
                    let guild_id = self.config.system.guild_id.to_string();
                    let channel_id = self.config.system.channel_id.to_string();

                    if self.sell_command.is_none() {
                         self.sell_command = self.client.get_command(&guild_id, "sell").await.unwrap_or(None);
                    }
                    if let Some(cmd) = &self.sell_command {
                         let _ = self.client.send_command(&guild_id, &channel_id, cmd, None).await;
                    }
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    self.state = BotState::Fishing;
                },
                BotState::Shopping => {
                     let guild_id = self.config.system.guild_id.to_string();
                     let channel_id = self.config.system.channel_id.to_string();
                     let now = Instant::now();

                     if let Some(rec) = &self.pending_recommendation {
                          info!("Shopping: Executing {:?}", rec.action);

                          if self.shop_command.is_none() {
                               self.shop_command = self.client.get_command(&guild_id, "shop").await.unwrap_or(None);
                          }

                          if let Some(cmd) = &self.shop_command {
                               match &rec.action {
                                   ActionType::BuyRod => {
                                       let options = vec![
                                           serde_json::json!({
                                               "name": "buy",
                                               "type": 1,
                                               "options": [
                                                   { "name": "rod", "value": rec.target_name }
                                               ]
                                           })
                                       ];
                                       let _ = self.client.send_command(&guild_id, &channel_id, cmd, Some(options)).await;
                                       self.last_action = Some((ActionType::BuyRod, now));
                                   },
                                   ActionType::BuyBoat => {
                                       let options = vec![
                                           serde_json::json!({
                                               "name": "buy",
                                               "type": 1,
                                               "options": [
                                                   { "name": "boat", "value": rec.target_name }
                                               ]
                                           })
                                       ];
                                       let _ = self.client.send_command(&guild_id, &channel_id, cmd, Some(options)).await;
                                       self.last_action = Some((ActionType::BuyBoat, now));
                                   },
                                   _ => {}
                               }
                          }
                     }
                     self.pending_recommendation = None;
                     tokio::time::sleep(Duration::from_secs(5)).await;
                     self.state = BotState::Fishing;
                },
                BotState::Exploration => {
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
