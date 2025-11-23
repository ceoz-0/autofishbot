use anyhow::{Result, anyhow};
use reqwest::{Client, Proxy};
use serde_json::{json, Value};
use crate::config::Config;
use crate::discord::types::ApplicationCommand;
use log::{error, info, debug};
use std::time::Duration;

pub struct DiscordClient {
    client: Client,
    config: Config,
    token: String,
    application_id: String,
}

impl DiscordClient {
    pub fn new(config: Config) -> Result<Self> {
        let mut client_builder = Client::builder()
            .timeout(Duration::from_secs(15));

        if let Some(ua) = &config.network.user_agent {
            client_builder = client_builder.user_agent(ua);
        } else {
            client_builder = client_builder.user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/103.0.0.0 Safari/537.36");
        }

        if let Some(proxy_ip) = &config.network.proxy_ip {
             if let Some(proxy_port) = config.network.proxy_port {
                let proxy_url = format!("http://{}:{}", proxy_ip, proxy_port);
                let mut proxy = Proxy::all(&proxy_url)?;
                if let (Some(user), Some(pass)) = (&config.network.proxy_auth_user, &config.network.proxy_auth_password) {
                    proxy = proxy.basic_auth(user, pass);
                }
                client_builder = client_builder.proxy(proxy);
             }
        }

        let client = client_builder.build()?;
        // Hardcoded Application ID from original code
        let application_id = "574652751745777665".to_string();

        Ok(Self {
            client,
            token: config.system.user_token.clone(),
            config,
            application_id,
        })
    }

    pub async fn get_commands(&self, guild_id: &str) -> Result<Vec<ApplicationCommand>> {
        let url = format!("https://discord.com/api/v9/guilds/{}/application-command-index", guild_id);
        let res = self.client.get(&url)
            .header("Authorization", &self.token)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await?;
            error!("Failed to get commands: {} - {}", status, text);
            return Err(anyhow!("Failed to get commands: {}", status));
        }

        let body: Value = res.json().await?;
        // Filter commands for the target application
        let commands_array = body.get("application_commands")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("Invalid response structure for commands"))?;

        let mut commands = Vec::new();
        for cmd_val in commands_array {
            if let Some(app_id) = cmd_val.get("application_id").and_then(|v| v.as_str()) {
                 if app_id == self.application_id {
                     let cmd: ApplicationCommand = serde_json::from_value(cmd_val.clone())?;
                     commands.push(cmd);
                 }
            }
        }
        Ok(commands)
    }

    pub async fn send_command(&self, guild_id: &str, channel_id: &str, command: &ApplicationCommand, options: Option<Vec<Value>>) -> Result<()> {
        let url = "https://discord.com/api/v9/interactions";

        let nonce = chrono::Utc::now().timestamp_millis() * 1000; // Simple nonce

        let payload = json!({
            "type": 2,
            "application_id": self.application_id,
            "guild_id": guild_id,
            "channel_id": channel_id,
            "session_id": "random_session_id_placeholder", // In real client we might need the session id from gateway
            "data": {
                "version": command.version,
                "id": command.id,
                "name": command.name,
                "type": command.r#type,
                "options": options.unwrap_or_default(),
                "application_command": command,
                "attachments": []
            },
            "nonce": nonce.to_string()
        });

        // Note: The original code uses a random session_id generated locally.
        // "session.join(choice(ascii_letters + digits) for _ in range(32))"
        // So we can generate one here if needed or pass it in.

        // Wait, we need a valid session ID? The python code says:
        // "Generates user session" locally. So it doesn't seem to be the gateway session id?
        // Let's double check.

        let res = self.client.post(url)
            .header("Authorization", &self.token)
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
             let status = res.status();
             let text = res.text().await?;
             // Handle 429
             if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                 // Parse retry_after
                 // For now just log
                 error!("Rate limited: {}", text);
             }
             return Err(anyhow!("Failed to send command: {} - {}", status, text));
        }

        Ok(())
    }

    pub async fn interact_component(&self, guild_id: &str, channel_id: &str, message_id: &str, custom_id: &str) -> Result<()> {
        let url = "https://discord.com/api/v9/interactions";
        let nonce = chrono::Utc::now().timestamp_millis() * 1000;

         let payload = json!({
            "type": 3,
            "nonce": nonce.to_string(),
            "guild_id": guild_id,
            "channel_id": channel_id,
            "message_flags": 0,
            "message_id": message_id,
            "application_id": self.application_id,
            "data": {
                "component_type": 2,
                "custom_id": custom_id
            },
            "session_id": "random_session_id_placeholder"
        });

        let res = self.client.post(url)
            .header("Authorization", &self.token)
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
             let status = res.status();
             let text = res.text().await?;
             return Err(anyhow!("Failed to interact component: {} - {}", status, text));
        }
        Ok(())
    }
}
