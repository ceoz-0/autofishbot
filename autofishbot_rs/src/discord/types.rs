use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub bot: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Member {
    pub user: User,
    // add other fields if necessary
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub id: String,
    pub channel_id: String,
    pub author: User,
    pub content: String,
    pub timestamp: String,
    pub embeds: Vec<Embed>,
    pub components: Option<Vec<Component>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Embed {
    pub title: Option<String>,
    pub description: Option<String>,
    pub fields: Option<Vec<EmbedField>>,
    pub footer: Option<EmbedFooter>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    pub inline: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmbedFooter {
    pub text: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Component {
    pub r#type: u8,
    pub components: Option<Vec<Component>>,
    pub custom_id: Option<String>,
    pub label: Option<String>,
    pub style: Option<u8>,
    pub emoji: Option<Emoji>,
    pub options: Option<Vec<SelectOption>>,
    pub placeholder: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SelectOption {
    pub label: String,
    pub value: String,
    pub description: Option<String>,
    pub emoji: Option<Emoji>,
    pub default: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Emoji {
    pub name: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GatewayPayload {
    pub op: u8,
    pub d: Option<serde_json::Value>,
    pub s: Option<u64>,
    pub t: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IdentifyPayload {
    pub token: String,
    pub properties: IdentifyProperties,
    pub compress: Option<bool>,
    pub large_threshold: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct IdentifyProperties {
    pub os: String,
    pub browser: String,
    pub device: String,
}

#[derive(Debug, Deserialize)]
pub struct HelloPayload {
    pub heartbeat_interval: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApplicationCommand {
    pub id: String,
    pub application_id: String,
    pub version: String,
    pub default_permission: Option<bool>,
    pub default_member_permissions: Option<String>,
    pub r#type: Option<u8>,
    pub name: String,
    pub description: String,
    pub guild_id: Option<String>,
    pub options: Option<Vec<ApplicationCommandOption>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApplicationCommandOption {
    pub r#type: u8,
    pub name: String,
    pub description: String,
    pub required: Option<bool>,
    pub choices: Option<Vec<ApplicationCommandOptionChoice>>,
    pub options: Option<Vec<ApplicationCommandOption>>, // Nested options for subcommands
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApplicationCommandOptionChoice {
    pub name: String,
    pub value: serde_json::Value,
}
