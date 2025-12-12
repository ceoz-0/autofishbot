use crate::config::Config;
use crate::discord::types::{GatewayPayload, HelloPayload};
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use log::{info, error, debug, warn};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
use tokio::time::interval;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;
use url::Url;

// Gateway constants
const GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=9&encoding=json";

pub struct Gateway {
    config: Config,
    ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    heartbeat_interval: u64,
    sequence: Option<u64>,
    session_id: Arc<RwLock<Option<String>>>,
    event_sender: tokio::sync::mpsc::Sender<GatewayPayload>,
    running: bool,
}

impl Gateway {
    pub fn new(config: Config, event_sender: tokio::sync::mpsc::Sender<GatewayPayload>, session_id: Arc<RwLock<Option<String>>>) -> Self {
        Self {
            config,
            ws_stream: None,
            heartbeat_interval: 41250, // Default
            sequence: None,
            session_id,
            event_sender,
            running: false,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        info!("Connecting to gateway...");
        let url = Url::parse(GATEWAY_URL)?;
        let (ws_stream, _) = connect_async(url.as_str()).await?;
        info!("Connected to gateway!");
        self.ws_stream = Some(ws_stream);
        self.running = true;
        Ok(())
    }

    // Helper to create identify payload
    fn get_identify_payload(&self) -> String {
        let payload = json!({
            "op": 2,
            "d": {
                "token": self.config.system.user_token,
                "properties": {
                    "os": "linux",
                    "browser": "autofishbot_rs",
                    "device": "autofishbot_rs"
                }
            }
        });
        payload.to_string()
    }

    pub async fn run_loop(mut self) -> Result<()> {
         if self.ws_stream.is_none() {
            self.connect().await?;
        }

        let (mut write, mut read) = self.ws_stream.take().unwrap().split();

        // Send Identify
        let identify_msg = self.get_identify_payload();
        write.send(Message::Text(identify_msg)).await?;

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Message>(10);

        let _writer_handle = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(_) = write.send(msg).await {
                    break;
                }
            }
        });

        let (incoming_tx, mut incoming_rx) = tokio::sync::mpsc::channel::<GatewayPayload>(100);

         let _reader_handle = tokio::spawn(async move {
            while let Some(message) = read.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        if let Ok(payload) = serde_json::from_str::<GatewayPayload>(&text) {
                             let _ = incoming_tx.send(payload).await;
                        }
                    },
                     Ok(Message::Close(_)) => break,
                     _ => {}
                }
            }
        });

        let mut heartbeat_interval = Duration::from_millis(41250);
        let mut next_heartbeat = tokio::time::Instant::now() + heartbeat_interval;

        let mut seq_num: Option<u64> = None;

        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(next_heartbeat) => {
                    // Send heartbeat
                    let heartbeat = json!({
                        "op": 1,
                        "d": seq_num
                    });
                    if let Err(_) = tx.send(Message::Text(heartbeat.to_string())).await {
                        break;
                    }
                    next_heartbeat = tokio::time::Instant::now() + heartbeat_interval;
                }
                Some(payload) = incoming_rx.recv() => {
                    if let Some(s) = payload.s {
                        seq_num = Some(s);
                    }

                    match payload.op {
                        10 => { // Hello
                            if let Some(d) = payload.d {
                                if let Some(interval) = d.get("heartbeat_interval").and_then(|v| v.as_u64()) {
                                    heartbeat_interval = Duration::from_millis(interval);
                                    next_heartbeat = tokio::time::Instant::now() + Duration::from_millis((interval as f64 * rand::random::<f64>()) as u64); // Jitter first heartbeat
                                }
                            }
                        },
                         0 => { // Dispatch
                            // Capture session_id from READY event
                            if payload.t.as_deref() == Some("READY") {
                                if let Some(d) = &payload.d {
                                    if let Some(sid) = d.get("session_id").and_then(|v| v.as_str()) {
                                        let mut writer = self.session_id.write().await;
                                        *writer = Some(sid.to_string());
                                        info!("Captured Session ID from Gateway: {}", sid);
                                    }
                                }
                            }

                            if let Err(_) = self.event_sender.send(payload).await {
                                break;
                            }
                        },
                        _ => {}
                    }
                }
                else => break,
            }
        }
        Ok(())
    }
}
