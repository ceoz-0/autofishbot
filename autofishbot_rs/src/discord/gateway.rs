use crate::config::Config;
use crate::discord::types::{GatewayPayload, HelloPayload};
use anyhow::{Result, anyhow};
use futures_util::{SinkExt, StreamExt};
use log::{info, error, debug, warn};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
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
    session_id: Option<String>,
    event_sender: tokio::sync::mpsc::Sender<GatewayPayload>,
    running: bool,
}

impl Gateway {
    pub fn new(config: Config, event_sender: tokio::sync::mpsc::Sender<GatewayPayload>) -> Self {
        Self {
            config,
            ws_stream: None,
            heartbeat_interval: 41250, // Default
            sequence: None,
            session_id: None,
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

    pub async fn run(&mut self) -> Result<()> {
        if self.ws_stream.is_none() {
            self.connect().await?;
        }

        // Identification
        self.identify().await?;

        let (mut write, mut read) = self.ws_stream.take().unwrap().split();

        // We need to handle heartbeats in a separate task or select loop.
        // But we need access to the writer.

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Message>(10);

        // Writer task
        let _writer_handle = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = write.send(msg).await {
                    error!("Failed to send message: {}", e);
                    break;
                }
            }
        });

        // We need shared state for sequence number to include in heartbeat
        let sequence = Arc::new(Mutex::new(self.sequence));

        // I'll use a channel for incoming gateway payloads
        // And the main loop will handle logic.

        let (incoming_tx, mut incoming_rx) = tokio::sync::mpsc::channel::<GatewayPayload>(100);

        // Reader task
        let _reader_handle = tokio::spawn(async move {
            while let Some(message) = read.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        match serde_json::from_str::<GatewayPayload>(&text) {
                            Ok(payload) => {
                                if let Err(_) = incoming_tx.send(payload).await {
                                    break;
                                }
                            },
                            Err(e) => error!("Failed to parse payload: {}", e),
                        }
                    },
                    Ok(Message::Close(_)) => {
                        info!("Gateway closed connection");
                        break;
                    },
                    Err(e) => {
                        error!("Websocket error: {}", e);
                        break;
                    },
                    _ => {}
                }
            }
        });

        // Main loop
        let mut heartbeat_timer = interval(Duration::from_millis(self.heartbeat_interval));

        loop {
            tokio::select! {
                _ = heartbeat_timer.tick() => {
                    let seq = *sequence.lock().await;
                    let heartbeat = json!({
                        "op": 1,
                        "d": seq
                    });
                    if let Err(e) = tx.send(Message::Text(heartbeat.to_string())).await {
                        error!("Failed to send heartbeat: {}", e);
                        break;
                    }
                }
                Some(payload) = incoming_rx.recv() => {
                    // Update sequence
                    if let Some(s) = payload.s {
                        let mut seq = sequence.lock().await;
                        *seq = Some(s);
                        self.sequence = Some(s);
                    }

                    match payload.op {
                        10 => { // Hello
                             if let Some(d) = payload.d {
                                 if let Ok(hello) = serde_json::from_value::<HelloPayload>(d) {
                                     info!("Received Hello. Heartbeat interval: {}", hello.heartbeat_interval);
                                     // Update heartbeat timer
                                     heartbeat_timer = interval(Duration::from_millis(hello.heartbeat_interval));
                                     heartbeat_timer.reset();
                                 }
                             }
                        },
                        11 => { // Heartbeat ACK
                            debug!("Heartbeat ACK");
                        },
                        0 => { // Dispatch
                            // Intercept READY to capture session_id
                            if let Some(ref t) = payload.t {
                                if t == "READY" {
                                    if let Some(d) = &payload.d {
                                        if let Some(sid) = d.get("session_id").and_then(|v| v.as_str()) {
                                            self.session_id = Some(sid.to_string());
                                            info!("Session ID acquired: {}", sid);
                                        }
                                    }
                                }
                            }

                            if let Err(_) = self.event_sender.send(payload).await {
                                break;
                            }
                        },
                        7 => { // Reconnect
                             info!("Received Reconnect op. Closing connection to reconnect.");
                             // Break the loop to trigger reconnect in the outer loop (in main.rs)
                             break;
                        },
                        9 => { // Invalid Session
                             warn!("Invalid Session. Clearing session state.");
                             self.session_id = None;
                             self.sequence = None;
                             {
                                 let mut seq = sequence.lock().await;
                                 *seq = None;
                             }
                             break;
                        }
                        _ => {
                            debug!("Received op {}", payload.op);
                        }
                    }
                }
                else => break, // Channels closed
            }
        }

        // Cleanup
        self.running = false;
        Ok(())
    }

    async fn identify(&mut self) -> Result<()> {
        let msg = if self.session_id.is_some() && self.sequence.is_some() {
            info!("Resuming session...");
            self.get_resume_payload()
        } else {
            info!("Identifying...");
            self.get_identify_payload()
        };

        self.ws_stream.as_mut()
            .ok_or(anyhow!("No stream available"))?
            .send(Message::Text(msg)).await?;

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

    fn get_resume_payload(&self) -> String {
        let payload = json!({
            "op": 6,
            "d": {
                "token": self.config.system.user_token,
                "session_id": self.session_id,
                "seq": self.sequence
            }
        });
        payload.to_string()
    }
}
