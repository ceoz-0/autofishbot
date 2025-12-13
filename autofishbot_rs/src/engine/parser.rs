use regex::Regex;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

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

    // Shop Item Pattern: "**Item Name** - $500", "**Item Name**: $500", "**Item Name** - **$500**"
    // Refined to handle colon separators and bold prices
    static ref SHOP_ITEM_PATTERN: Regex = Regex::new(r"\*\*([^\*]+)\*\*\s*(?:-|:|–)\s*(?:\*\*)?\$([\d,]+)(?:\*\*)?").unwrap();
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ShopItem {
    pub name: String,
    pub price: f32,
    pub currency: String,
    pub description: String,
    pub stock: Option<i32>,
    pub stats: Option<String>,
}

#[derive(Debug)]
pub struct SelectMenuOption {
    pub label: String,
    pub value: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameEntity {
    pub entity_type: String,
    pub name: String,
    pub details: String,
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

pub fn parse_shop_embed(title: &str, description: &str, fields: Option<&Vec<crate::discord::types::EmbedField>>) -> Vec<ShopItem> {
    let mut items = Vec::new();
    let shop_currency = if title.contains("Magma") { "Magma" } else { "Money" };

    // Method 1: Check fields (Common for many bots)
    if let Some(fields_vec) = fields {
        for field in fields_vec {
            // Assume format: Name -> "Price: $X\nDesc: ..."
            let name = field.name.replace("*", "").trim().to_string();
            let mut price = 0.0;
            let mut desc = String::new();

            for line in field.value.lines() {
                if line.to_lowercase().contains("price") || line.contains("$") {
                     // Extract number
                     let num_str: String = line.chars().filter(|c| c.is_digit(10) || *c == '.').collect();
                     if let Ok(p) = num_str.parse::<f32>() {
                         price = p;
                     }
                } else {
                    desc.push_str(line);
                    desc.push('\n');
                }
            }

            if price > 0.0 {
                let desc_str = desc.trim().to_string();
                // Extract stats from description (simple heuristic: lines starting with +)
                let stats: Vec<String> = desc_str.lines()
                    .filter(|l| l.trim().starts_with('+'))
                    .map(|l| l.trim().to_string())
                    .collect();
                let stats_str = if stats.is_empty() { None } else { Some(stats.join(", ")) };

                items.push(ShopItem {
                    name,
                    price,
                    currency: shop_currency.to_string(),
                    description: desc_str,
                    stock: None,
                    stats: stats_str,
                });
            }
        }
    }

    // Method 2: Check Description (List format)
    if items.is_empty() {
        for line in description.lines() {
            // Very basic heuristic parser
             if let Some(caps) = SHOP_ITEM_PATTERN.captures(line) {
                 if let (Some(name_cap), Some(price_cap)) = (caps.get(1), caps.get(2)) {
                      let clean_price = price_cap.as_str().replace(",", "");
                      if let Ok(price) = clean_price.parse::<f32>() {
                           items.push(ShopItem {
                               name: name_cap.as_str().trim().to_string(),
                               price,
                               currency: shop_currency.to_string(),
                               description: line.to_string(), // Store full line as desc for now
                               stock: None,
                               stats: None,
                           });
                      }
                 }
             }
        }
    }

    items
}

pub fn parse_select_menu_options(msg: &crate::discord::types::Message) -> Option<(String, Vec<SelectMenuOption>)> {
    if let Some(rows) = &msg.components {
        for row in rows {
             if let Some(comps) = &row.components {
                 for comp in comps {
                     if comp.r#type == 3 { // Select Menu
                          if let Some(opts) = &comp.options {
                               let parsed_opts = opts.iter().map(|o| SelectMenuOption {
                                   label: o.label.clone(),
                                   value: o.value.clone(),
                                   description: o.description.clone()
                               }).collect();
                               return Some((comp.custom_id.clone().unwrap_or_default(), parsed_opts));
                          }
                     }
                 }
             }
        }
    }
    None
}

pub fn parse_generic_list(title: &str, description: &str) -> Vec<GameEntity> {
    let mut entities = Vec::new();
    let type_name = title.split_whitespace().last().unwrap_or("Unknown").to_string();

    for line in description.lines() {
        if line.trim().is_empty() { continue; }
        // Store every non-empty line as an entity for now
        entities.push(GameEntity {
            entity_type: type_name.clone(),
            name: line.chars().take(50).collect(), // First 50 chars as name?
            details: line.to_string()
        });
    }
    entities
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shop_embed_variations() {
        let title = "Fish Shop";

        // Variation 1: Existing assumption
        let desc1 = "**Plastic Rod** - $500\n**Steel Rod** - $1,500";
        let items1 = parse_shop_embed(title, desc1, None);
        assert_eq!(items1.len(), 2, "Failed Variation 1");
        assert_eq!(items1[0].name, "Plastic Rod");
        assert_eq!(items1[0].price, 500.0);
        assert_eq!(items1[1].name, "Steel Rod");
        assert_eq!(items1[1].price, 1500.0);

        // Variation 2: With Emojis and no bold on price
        let desc2 = "<:rod:123> **Plastic Rod** - $500\n<:rod:123> **Steel Rod** - $1,500";
        let items2 = parse_shop_embed(title, desc2, None);
        // This fails with current regex if it strictly expects start of line or specific format
        // Current regex: r"\*\*([^\*]+)\*\*\s+-\s+\$([\d,]+)"
        // It doesn't anchor to start, so it might pass if format is " - $".

        // Variation 3: Price in bold
        let desc3 = "**Plastic Rod** - **$500**";
        let items3 = parse_shop_embed(title, desc3, None);

        // Variation 4: Colon separator
        let desc4 = "**Plastic Rod**: $500";
        let items4 = parse_shop_embed(title, desc4, None);

        // Variation 6: ID prefix
        let desc6 = "1. **Plastic Rod** - $500";
        let items6 = parse_shop_embed(title, desc6, None);

        assert_eq!(items2.len(), 2, "Failed Variation 2");
        assert_eq!(items2[0].name, "Plastic Rod");
        assert_eq!(items2[0].price, 500.0);
        assert_eq!(items2[1].name, "Steel Rod");
        assert_eq!(items2[1].price, 1500.0);

        assert_eq!(items3.len(), 1, "Failed Variation 3");
        assert_eq!(items3[0].name, "Plastic Rod");
        assert_eq!(items3[0].price, 500.0);

        assert_eq!(items4.len(), 1, "Failed Variation 4");
        assert_eq!(items4[0].name, "Plastic Rod");
        assert_eq!(items4[0].price, 500.0);

        assert_eq!(items6.len(), 1, "Failed Variation 6");
        assert_eq!(items6[0].name, "Plastic Rod");
        assert_eq!(items6[0].price, 500.0);
    }
}
