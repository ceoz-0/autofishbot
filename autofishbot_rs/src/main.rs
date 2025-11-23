use std::io;
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

use autofishbot_rs::config::Config;
use autofishbot_rs::tui::app::App;
use autofishbot_rs::tui::ui;
use autofishbot_rs::tui::events;
use autofishbot_rs::discord::gateway::Gateway;
use autofishbot_rs::discord::types::GatewayPayload;
use autofishbot_rs::discord::client::DiscordClient;
use autofishbot_rs::engine::bot::Bot;

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

    // Setup TUI
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = Arc::new(Mutex::new(App::new(config.clone())));

    // Discord Client
    let client = Arc::new(DiscordClient::new(config.clone())?);

    // Gateway event channel
    let (gateway_tx, mut gateway_rx) = tokio::sync::mpsc::channel::<GatewayPayload>(100);

    // Gateway
    let gateway = Gateway::new(config.clone(), gateway_tx);
    let _gateway_handle = tokio::spawn(async move {
        if let Err(e) = gateway.run_loop().await {
            eprintln!("Gateway error: {}", e);
        }
    });

    // Bot Engine
    let mut bot = Bot::new(config.clone(), client.clone(), app.clone());
    let _bot_handle = tokio::spawn(async move {
        bot.run().await;
    });

    // Event Processor
    let app_clone = app.clone();
    let _event_processor = tokio::spawn(async move {
        while let Some(payload) = gateway_rx.recv().await {
             let mut app = app_clone.lock().await;

             // Handle specific events like MESSAGE_CREATE
             if let Some(t) = payload.t {
                 app.add_log(format!("Event: {}", t));
                 if t == "MESSAGE_CREATE" || t == "MESSAGE_UPDATE" {
                      if let Some(d) = payload.d {
                           if let Some(content) = d.get("content").and_then(|v| v.as_str()) {
                               app.last_message = content.to_string();

                               // Update Profile Parsing
                               let embeds = d.get("embeds").and_then(|v| v.as_array());
                               if let Some(embeds_arr) = embeds {
                                   if let Some(first_embed) = embeds_arr.first() {
                                       if let Some(title) = first_embed.get("title").and_then(|v| v.as_str()) {
                                            if let Some(desc) = first_embed.get("description").and_then(|v| v.as_str()) {
                                                 app.profile.update_from_message(desc, Some(title));
                                            }
                                       }
                                   }
                               }

                               // Also check content for untitled messages if needed
                           }
                      }
                 }
             }
        }
    });

    // Main Loop (TUI)
    let res = run_app(&mut terminal, app).await;

    // Cleanup
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: Arc<Mutex<App>>) -> Result<()> {
    loop {
        {
            let app_guard = app.lock().await;
            terminal.draw(|f| ui::draw(f, &app_guard))?;
            if app_guard.should_quit {
                return Ok(());
            }
        }

        // Handle input
        {
             let mut app_guard = app.lock().await;
             events::handle_events(&mut app_guard)?;
        }
    }
}
