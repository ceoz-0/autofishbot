use regex::Regex;
use lazy_static::lazy_static;
use log::{info, warn};

lazy_static! {
    // Example: "3 Salmon" or "1 Golden Fish"
    static ref CATCH_PATTERN: Regex = Regex::new(r"(\d+)\s+<:[^>]+>\s+([\w\s]+)").unwrap();
    // Example: "+173 XP" or "+37,129 XP"
    static ref XP_PATTERN: Regex = Regex::new(r"\+([\d,]+)\s+XP").unwrap();
    // Example: "Balance: **$3,548**"
    static ref BALANCE_PATTERN: Regex = Regex::new(r"Balance: \*\*\$([\d,]+)\*\*").unwrap();
    // Example: "Level 21"
    static ref LEVEL_PATTERN: Regex = Regex::new(r"Level (\d+)").unwrap();
    // Example: "Current Biome: <:...:...> **Flatland**"
    static ref BIOME_PATTERN: Regex = Regex::new(r"Current Biome: .* \*\*([\w\s]+)\*\*").unwrap();
    // Example: "You must wait **2.5**s" or "**0.1**s"
    static ref COOLDOWN_WAIT_PATTERN: Regex = Regex::new(r"You must wait \*\*([\d\.]+)\*\*s").unwrap();
    // Example: "Current cooldown: **3.5** seconds"
    static ref COOLDOWN_TOTAL_PATTERN: Regex = Regex::new(r"Current cooldown: \*\*([\d\.]+)\*\* seconds").unwrap();
}

#[derive(Debug)]
pub struct CatchEvent {
    pub fish: Vec<(String, i32)>, // Name, Count
    pub xp: f32,
}

#[derive(Debug)]
pub struct PlayerStats {
    pub balance: Option<f32>,
    pub level: Option<i32>,
    pub biome: Option<String>,
}

#[derive(Debug)]
pub struct CooldownEvent {
    pub wait_time: f32,
    pub total_cooldown: f32,
}

pub fn parse_cooldown_embed(description: &str) -> Option<CooldownEvent> {
    let mut wait = 0.0;
    let mut total = 0.0;

    if let Some(caps) = COOLDOWN_WAIT_PATTERN.captures(description) {
        if let Some(val) = caps.get(1) {
            if let Ok(v) = val.as_str().parse::<f32>() {
                wait = v;
            }
        }
    }

    if let Some(caps) = COOLDOWN_TOTAL_PATTERN.captures(description) {
        if let Some(val) = caps.get(1) {
            if let Ok(v) = val.as_str().parse::<f32>() {
                total = v;
            }
        }
    }

    if wait > 0.0 || total > 0.0 {
        Some(CooldownEvent { wait_time: wait, total_cooldown: total })
    } else {
        None
    }
}

pub fn parse_catch_embed(description: &str) -> Option<CatchEvent> {
    let mut fish_list = Vec::new();
    let mut xp = 0.0;

    // Parse lines
    for line in description.lines() {
        if let Some(caps) = CATCH_PATTERN.captures(line) {
            if let (Some(count_str), Some(name_str)) = (caps.get(1), caps.get(2)) {
                if let Ok(count) = count_str.as_str().parse::<i32>() {
                    let name = name_str.as_str().trim().to_string();
                    fish_list.push((name, count));
                }
            }
        }
        if let Some(caps) = XP_PATTERN.captures(line) {
            if let Some(xp_str) = caps.get(1) {
                let clean_xp = xp_str.as_str().replace(",", "");
                if let Ok(val) = clean_xp.parse::<f32>() {
                    xp = val;
                }
            }
        }
    }

    if !fish_list.is_empty() || xp > 0.0 {
        Some(CatchEvent { fish: fish_list, xp })
    } else {
        None
    }
}

pub fn parse_profile_embed(description: &str) -> PlayerStats {
    let mut balance = None;
    let mut level = None;
    let mut biome = None;

    if let Some(caps) = BALANCE_PATTERN.captures(description) {
        if let Some(val) = caps.get(1) {
             let clean = val.as_str().replace(",", "");
             if let Ok(v) = clean.parse::<f32>() {
                 balance = Some(v);
             }
        }
    }

    if let Some(caps) = LEVEL_PATTERN.captures(description) {
        if let Some(val) = caps.get(1) {
             if let Ok(v) = val.as_str().parse::<i32>() {
                 level = Some(v);
             }
        }
    }

    if let Some(caps) = BIOME_PATTERN.captures(description) {
        if let Some(val) = caps.get(1) {
             biome = Some(val.as_str().to_string());
        }
    }

    PlayerStats { balance, level, biome }
}
