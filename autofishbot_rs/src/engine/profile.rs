use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Profile {
    pub balance: String,
    pub level: String,
    pub rod: String,
    pub biome: String,
    pub current_biome: Option<String>, // Added for persistent tracking
    pub pet: String,
    pub bait: String,
    pub inventory_value: String,
    pub exotic_fish: ExoticFish,
    pub inventory: Vec<(String, String)>,
    pub charms: Charms,
    pub buffs: Buffs,
    pub quests: Vec<Quest>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExoticFish {
    pub gold: i32,
    pub emerald: i32,
    pub lava: i32,
    pub diamond: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Charms {
    pub marketing: String,
    pub endurance: String,
    pub haste: String,
    pub quantity: String,
    pub worker: String,
    pub treasure: String,
    pub quality: String,
    pub experience: String,
    pub found: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Buffs {
    pub sell_price: String,
    pub fish_catch: String,
    pub fish_quality: String,
    pub treasure_chance: String,
    pub treasure_quality: String,
    pub xp_multiplier: String,
    pub fishing_cooldown: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Quest {
    pub category: String,
    pub objective: String,
    pub progress: String,
    pub is_completed: bool,
}

impl Profile {
    pub fn update_from_message(&mut self, content: &str, title: Option<&str>) {
        if let Some(t) = title {
            if t.contains("Profile") {
                self.parse_profile(content);
            } else if t.contains("Charms") {
                self.parse_charms(content);
            } else if t.contains("Active Buffs") {
                self.parse_buffs(content);
            } else if t.contains("Quests") {
                self.parse_quests(content);
            }
        }
    }

    fn remove_markdown(data: &str) -> String {
        data.replace('*', "").replace('+', "").replace('_', "")
    }

    fn parse_profile(&mut self, content: &str) {
        self.inventory.clear();
        let clean_content = Self::remove_markdown(content);

        for line in clean_content.lines() {
            let line = line.trim();
            if line.is_empty() { continue; }

            if line.starts_with("Balance:") {
                self.balance = line.replace("Balance:", "").trim().to_string();
            } else if line.contains("XP to next level") {
                // Simplified parsing for now
                self.level = line.split(',').next().unwrap_or("").trim().to_string();
            } else if line.contains("Rod") {
                 // Handle emoji removal roughly
                 if let Some(idx) = line.find('>') {
                     self.rod = line[idx+1..].trim().to_string();
                 }
            } else if line.starts_with("Current biome:") {
                 if let Some(idx) = line.find('>') {
                     self.biome = line[idx+1..].trim().to_string();
                 }
            } else if line.starts_with("Pet:") {
                 if let Some(idx) = line.find('>') {
                     self.pet = line[idx+1..].trim().to_string();
                 }
            } else if line.starts_with("Bait:") {
                 if let Some(idx) = line.find('>') {
                     self.bait = line[idx+1..].trim().to_string();
                 }
            } else if line.contains("Gold Fish") {
                 if let Some(end) = line.find('<') {
                      if let Ok(val) = line[..end].replace(',', "").trim().parse() {
                          self.exotic_fish.gold = val;
                      }
                 }
            } else if line.contains("Emerald Fish") {
                 if let Some(end) = line.find('<') {
                      if let Ok(val) = line[..end].replace(',', "").trim().parse() {
                          self.exotic_fish.emerald = val;
                      }
                 }
            } else if line.contains("Lava Fish") {
                 if let Some(end) = line.find('<') {
                      if let Ok(val) = line[..end].replace(',', "").trim().parse() {
                          self.exotic_fish.lava = val;
                      }
                 }
            } else if line.contains("Diamond Fish") {
                 if let Some(end) = line.find('<') {
                      if let Ok(val) = line[..end].replace(',', "").trim().parse() {
                          self.exotic_fish.diamond = val;
                      }
                 }
            } else if line.starts_with("Fish Value:") {
                self.inventory_value = line.replace("Fish Value:", "").trim().to_string();
            } else {
                // Inventory items usually start with a number
                if line.chars().next().map_or(false, |c| c.is_numeric()) {
                    if let Some(first_space) = line.find('<') {
                         let amount = line[..first_space].trim().to_string();
                         let name = if let Some(last_space) = line.rfind('>') {
                             line[last_space+1..].trim().to_string()
                         } else {
                             line.to_string()
                         };
                         self.inventory.push((amount, name));
                    }
                }
            }
        }
    }

    fn parse_charms(&mut self, content: &str) {
        let clean_content = Self::remove_markdown(content);
         for line in clean_content.lines() {
            if line.is_empty() { continue; }
            let parts: Vec<&str> = line.split('/').collect();
            if parts.len() < 2 { continue; }
            let value = parts[0].trim();

            if line.contains("Marketing") { self.charms.marketing = value.to_string(); }
            else if line.contains("Endurance") { self.charms.endurance = value.to_string(); }
            else if line.contains("Haste") { self.charms.haste = value.to_string(); }
            else if line.contains("Quantity") { self.charms.quantity = value.to_string(); }
            else if line.contains("Worker") { self.charms.worker = value.to_string(); }
            else if line.contains("Treasure") { self.charms.treasure = value.to_string(); }
            else if line.contains("Quality") { self.charms.quality = value.to_string(); }
            else if line.contains("Experience") { self.charms.experience = value.to_string(); }
            else if line.contains("Total charms found") { self.charms.found = value.to_string(); }
         }
    }

    fn parse_buffs(&mut self, content: &str) {
         let clean_content = Self::remove_markdown(content);
         for line in clean_content.lines() {
            if line.is_empty() { continue; }
            if let Some(idx) = line.find(':') {
                 let value = line[idx+1..].trim().to_string();
                 if line.starts_with("Sell") { self.buffs.sell_price = value; }
                 else if line.contains("catch") { self.buffs.fish_catch = value; }
                 else if line.contains("Fish quality") { self.buffs.fish_quality = value; }
                 else if line.contains("chance") { self.buffs.treasure_chance = value; }
                 else if line.contains("Treasure quality") { self.buffs.treasure_quality = value; }
                 else if line.contains("XP") { self.buffs.xp_multiplier = value; }
                 else if line.contains("cooldown") { self.buffs.fishing_cooldown = value; }
            }
         }
    }

    fn parse_quests(&mut self, content: &str) {
        self.quests.clear();
        let clean_content = Self::remove_markdown(content);
        for line in clean_content.lines() {
            if line.is_empty() || line.contains("Quests have multiple tiers") || line.contains("Quests reset") { continue; }

            if line.starts_with("Daily") {
                let parts: Vec<&str> = line.split(" - ").collect();
                if parts.len() >= 3 {
                    let category = parts[0].to_string();
                    let objective = parts[1].to_string();
                    let progress = parts[2].to_string();
                    self.quests.push(Quest {
                        category,
                        objective,
                        progress,
                        is_completed: false,
                    });
                } else if parts.len() == 2 {
                     // Completed
                    let category = parts[0].to_string();
                    let objective = parts[1].replace(" COMPLETED", "");
                    self.quests.push(Quest {
                        category,
                        objective,
                        progress: "Completed".to_string(),
                        is_completed: true,
                    });
                }
            }
        }
    }
}
