use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;

use autofishbot_rs::config::Config;
use autofishbot_rs::tui::app::App;
use autofishbot_rs::discord::gateway::Gateway;
use autofishbot_rs::discord::types::GatewayPayload;
use autofishbot_rs::discord::client::DiscordClient;
use autofishbot_rs::engine::bot::Bot;
use autofishbot_rs::engine::database::Database;
use autofishbot_rs::engine::parser;

#[tokio::main]
async fn main() -> Result<()> {
    // Load config
    let config_path = "config.toml";
    let config = if std::path::Path::new(config_path).exists() {
        Config::load(config_path)?
    } else {
        let cfg = Config::default();
        cfg.save(config_path)?;
        cfg
    };

     env_logger::init();

    println!("Loaded config.");
    println!("Starting headless bot...");

    // Setup App State (mocked TUI)
    let app = Arc::new(Mutex::new(App::new(config.clone())));

    // Setup Database
    let db = Arc::new(Database::new("autofishbot.db").await?);

    // Enable running by default for headless
    {
        let mut app_guard = app.lock().await;
        app_guard.is_running = true;
    }

    // Discord Client
    let client = Arc::new(DiscordClient::new(config.clone())?);

    // Gateway event channel
    let (gateway_tx, mut gateway_rx) = tokio::sync::mpsc::channel::<GatewayPayload>(100);

    // Gateway
    let mut gateway = Gateway::new(config.clone(), gateway_tx);
    let _gateway_handle = tokio::spawn(async move {
        println!("Starting Gateway connection...");
        loop {
            if let Err(e) = gateway.run().await {
                eprintln!("Gateway error: {}", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            } else {
                // Reconnect immediately on clean exit (reconnect opcode)
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    });

    // Bot Engine
    let mut bot = Bot::new(config.clone(), client.clone(), app.clone(), db.clone()).await;
    let bot_cooldown_manager = bot.cooldown_manager.clone(); // Share cooldown manager

    let _bot_handle = tokio::spawn(async move {
        println!("Starting Bot engine...");
        bot.run().await;
    });

    // Event Processor & Logger
    let app_clone = app.clone();
    let db_clone = db.clone();

    let _event_processor = tokio::spawn(async move {
        while let Some(payload) = gateway_rx.recv().await {
             let mut app = app_clone.lock().await;

             // Handle specific events like MESSAGE_CREATE
             if let Some(t) = payload.t {
                 // println!("Event received: {}", t);
                 app.add_log(format!("Event: {}", t));
                 if t == "MESSAGE_CREATE" || t == "MESSAGE_UPDATE" {
                      if let Some(d) = payload.d {
                           // Try to parse full message object
                           if let Ok(msg) = serde_json::from_value::<autofishbot_rs::discord::types::Message>(d.clone()) {
                               app.last_message_object = Some(msg);
                           }

                           // Check Author
                           let is_vf = if let Some(author) = d.get("author") {
                                author.get("id").and_then(|id| id.as_str()).map(|s| s == "574652751745777665").unwrap_or(false)
                           } else { false };

                           if let Some(content) = d.get("content").and_then(|v| v.as_str()) {
                               app.last_message = content.to_string();
                               // println!("Message Content: {}", content);

                               // Update Profile Parsing
                               let embeds = d.get("embeds").and_then(|v| v.as_array());
                               if let Some(embeds_arr) = embeds {
                                   if let Some(first_embed) = embeds_arr.first() {
                                       if let Some(title) = first_embed.get("title").and_then(|v| v.as_str()) {
                                            println!("Embed Title: {}", title);
                                            if let Some(desc) = first_embed.get("description").and_then(|v| v.as_str()) {
                                                 // app.profile.update_from_message(desc, Some(title));

                                                 if is_vf {
                                                     if title.contains("You caught") {
                                                         if let Some(catch) = parser::parse_catch_embed(desc) {
                                                             println!("Parsed Catch: {:?}", catch);
                                                              // Report success to cooldown manager
                                                              {
                                                                  let mut cm = bot_cooldown_manager.lock().await;
                                                                  cm.report_success();
                                                              }

                                                             // Default biome for now or from state
                                                             let current_biome = app.profile.current_biome.clone().unwrap_or("Unknown".to_string());
                                                             for (fish, count) in catch.fish {
                                                                 if let Err(e) = db_clone.log_catch(&fish, count, catch.xp, &current_biome).await {
                                                                     eprintln!("DB Error: {}", e);
                                                                 }
                                                             }
                                                         }
                                                     } else if title.contains("Inventory") || title.contains("Virtual Farmer") { // "Virtual Farmer" is profile?
                                                         let stats = parser::parse_profile_embed(desc);
                                                         println!("Parsed Stats: {:?}", stats);
                                                         if let (Some(lvl), Some(bal), Some(bio)) = (stats.level, stats.balance, stats.biome) {
                                                             if let Err(e) = db_clone.log_snapshot(lvl, 0.0, bal, &bio).await {
                                                                  eprintln!("DB Error: {}", e);
                                                             }
                                                             // Update app state too
                                                             app.profile.current_biome = Some(bio);
                                                         }
                                                     }
                                                 }
                                            }
                                       } else if let Some(desc) = first_embed.get("description").and_then(|v| v.as_str()) {
                                            // Some embeds might not have a title but have a description (e.g., Cooldown warnings)
                                            // println!("Embed Description (No Title): {}", desc);

                                            if is_vf {
                                                if let Some(cd_event) = parser::parse_cooldown_embed(desc) {
                                                    println!("Parsed Cooldown: {:?}", cd_event);

                                                    // Log to DB
                                                    if let Err(e) = db_clone.log_cooldown(cd_event.wait_time, cd_event.total_cooldown).await {
                                                        eprintln!("DB Error: {}", e);
                                                    }

                                                    // Update Cooldown Manager
                                                    {
                                                        let mut cm = bot_cooldown_manager.lock().await;
                                                        cm.report_cooldown_hit(cd_event.wait_time as f64, cd_event.total_cooldown as f64);
                                                    }
                                                }
                                            }
                                       }
                                   }
                               }
                           }
                      }
                 }
             }
        }
    });

    // Keep alive for testing
    let minutes = 30;
    println!("Running for {} minutes...", minutes);
    tokio::time::sleep(Duration::from_secs(60 * minutes)).await;
    println!("Test complete.");

    // Verify Database
    println!("--- Database Verification ---");
    let catches: i32 = sqlx::query_scalar("SELECT count(*) FROM catch_history")
        .fetch_one(&db.pool)
        .await?;
    println!("Catches logged: {}", catches);

    let snapshots: i32 = sqlx::query_scalar("SELECT count(*) FROM player_snapshots")
        .fetch_one(&db.pool)
        .await?;
    println!("Snapshots logged: {}", snapshots);

    let fish: i32 = sqlx::query_scalar("SELECT count(*) FROM fish")
        .fetch_one(&db.pool)
        .await?;
    println!("Unique fish known: {}", fish);

    // Check gathered data
    let shop_items: i32 = sqlx::query_scalar("SELECT count(*) FROM shop_items")
        .fetch_one(&db.pool)
        .await?;
    println!("Shop items gathered: {}", shop_items);

    let entities: i32 = sqlx::query_scalar("SELECT count(*) FROM game_entities")
        .fetch_one(&db.pool)
        .await?;
    println!("Game entities gathered: {}", entities);

    Ok(())
}
