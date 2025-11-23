use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::Result;
use std::fs;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub system: SystemConfig,
    pub captcha: CaptchaConfig,
    pub network: NetworkConfig,
    pub automation: AutomationConfig,
    pub menu: MenuConfig,
    pub cosmetic: CosmeticConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SystemConfig {
    pub user_token: String,
    pub user_cooldown: f64,
    pub channel_id: u64,
    pub debug: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CaptchaConfig {
    pub ocr_api_key: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NetworkConfig {
    pub user_agent: Option<String>,
    pub proxy_ip: Option<String>,
    pub proxy_port: Option<u16>,
    pub proxy_auth_user: Option<String>,
    pub proxy_auth_password: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AutomationConfig {
    pub boosts_length: u64,
    pub more_fish: bool,
    pub more_treasures: bool,
    pub fish_on_exit: bool,
    pub auto_daily: bool,
    pub auto_buy_baits: bool,
    pub auto_sell: bool,
    pub auto_update_inventory: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MenuConfig {
    pub compact_mode: bool,
    pub refresh_rate: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CosmeticConfig {
    pub pet: Option<String>,
    pub bait: Option<String>,
    pub biome: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            system: SystemConfig {
                user_token: "".to_string(),
                user_cooldown: 3.5,
                channel_id: 0,
                debug: false,
            },
            captcha: CaptchaConfig {
                ocr_api_key: "".to_string(),
            },
            network: NetworkConfig {
                user_agent: None,
                proxy_ip: None,
                proxy_port: None,
                proxy_auth_user: None,
                proxy_auth_password: None,
            },
            automation: AutomationConfig {
                boosts_length: 5,
                more_fish: true,
                more_treasures: false,
                fish_on_exit: true,
                auto_daily: true,
                auto_buy_baits: false,
                auto_sell: true,
                auto_update_inventory: false,
            },
            menu: MenuConfig {
                compact_mode: false,
                refresh_rate: 0.3,
            },
            cosmetic: CosmeticConfig {
                pet: Some("dolphin".to_string()),
                bait: Some("fish".to_string()),
                biome: Some("ocean".to_string()),
            },
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())?;
        let mut config: Config = toml::from_str(&content)?;

        // Trim token
        config.system.user_token = config.system.user_token.trim().to_string();

        Ok(config)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}
