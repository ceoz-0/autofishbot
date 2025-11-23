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
        let writer_handle = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = write.send(msg).await {
                    error!("Failed to send message: {}", e);
                    break;
                }
            }
        });

        let heartbeat_interval_ms = self.heartbeat_interval; // Initial guess, will be updated
        let heartbeat_tx = tx.clone();

        // We need shared state for sequence number to include in heartbeat
        let sequence = Arc::new(Mutex::new(self.sequence));
        let seq_clone = sequence.clone();

        // The heartbeat interval is dynamic, set by Hello event.
        // So we can't start the fixed interval loop yet technically.
        // But usually we receive Hello first thing.

        // Let's use a loop with select! to handle incoming messages and heartbeats.
        // But since we split the stream, we can't easily put them back together in one select without channels.
        // Actually we can just loop on read and handle events. One event will be Hello, which sets up heartbeat.

        // Re-implementation: Don't split yet?
        // Or just spawn the reader and have it send events to a channel that the main loop processes?
        // Let's go with a simpler single-loop approach if possible, or the standard split.

        // I'll use a channel for incoming gateway payloads
        // And the main loop will handle logic.

        let (incoming_tx, mut incoming_rx) = tokio::sync::mpsc::channel::<GatewayPayload>(100);

        // Reader task
        let reader_handle = tokio::spawn(async move {
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
                                     // interval is immutable, so we need to reconstruct it or use a mutable wrapper if we want to change it?
                                     // Actually `interval` from tokio can be reset? No.
                                     // We have to construct a new one or use sleep.
                                     heartbeat_timer = interval(Duration::from_millis(hello.heartbeat_interval));
                                     heartbeat_timer.reset(); // Reset to start now + interval?
                                 }
                             }
                        },
                        11 => { // Heartbeat ACK
                            debug!("Heartbeat ACK");
                        },
                        0 => { // Dispatch
                            if let Err(_) = self.event_sender.send(payload).await {
                                break;
                            }
                        },
                        7 => { // Reconnect
                             info!("Received Reconnect op");
                             // Should implement reconnect logic
                             break;
                        },
                        9 => { // Invalid Session
                             warn!("Invalid Session");
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
        // We need to send this via the websocket.
        // Since I split logic in `run`, I can't easily send from here unless I use the channel.
        // But `run` hasn't started the loops yet when I called `identify` in my thought process.
        // Wait, `run` consumes `self`.

        // Let's refactor `run` to do everything.
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
}

// Redoing run to be more cohesive
impl Gateway {
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
