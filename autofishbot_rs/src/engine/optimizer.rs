use std::collections::HashMap;
use crate::engine::game_data::{Rod, Boat, Biome, ROD_DATA, BOAT_DATA, BIOME_DATA};

#[derive(Debug, Default, Clone)]
pub struct BiomeStats {
    pub total_catches: u64,
    pub total_gold: u64,
    pub total_xp: u64,
    pub avg_gold_per_fish: f64,
    pub avg_xp_per_fish: f64,
}

impl BiomeStats {
    pub fn update(&mut self, gold: u64, xp: u64, count: u64) {
        self.total_catches += count;
        self.total_gold += gold;
        self.total_xp += xp;
        if self.total_catches > 0 {
            self.avg_gold_per_fish = self.total_gold as f64 / self.total_catches as f64;
            self.avg_xp_per_fish = self.total_xp as f64 / self.total_catches as f64;
        }
    }
}

pub struct Optimizer {
    pub biome_knowledge: HashMap<Biome, BiomeStats>,
}

#[derive(Debug, Clone)]
pub struct Recommendation {
    pub action: String,
    pub target_name: String,
    pub cost: u64,
    pub roi_seconds: f64,
}

impl Optimizer {
    pub fn new() -> Self { Self { biome_knowledge: HashMap::new() } }

    pub fn calculate_metrics(&self, rod: &Rod, boat: &Boat, biome: Biome) -> f64 {
        // Use learned stats if available, else roughly 15.0 gold/fish base
        let stats = self.biome_knowledge.get(&biome);
        let avg_val = stats.map(|s| s.avg_gold_per_fish).unwrap_or(15.0);
        let avg_val = if avg_val == 0.0 { 15.0 } else { avg_val };

        // Safe lookup for biome data
        let biome_data = BIOME_DATA.get(&biome).unwrap_or_else(|| {
             BIOME_DATA.get(&Biome::River).expect("River biome data missing from static map")
        });

        // Formula: GPS = (Avg_Gold_Per_Cast * Catch_Rate) / (Base_Cooldown + Penalty - Reduction)
        // Avg_Gold_Per_Cast = rod.expected_fish * avg_val

        let base_cd = biome_data.base_cooldown;
        let total_cd = (base_cd + biome_data.cooldown_penalty - boat.cooldown_reduction).max(2.0);

        let catch_rate = biome_data.catch_rate;
        let fish_per_cast = rod.expected_fish * catch_rate;

        (fish_per_cast * avg_val) / total_cd
    }

    pub fn solve_next_move(&self, current_rod: &Rod, current_boat: &Boat, current_biome: Biome) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();
        let current_gps = self.calculate_metrics(current_rod, current_boat, current_biome);

        // Evaluate Rods
        for rod in ROD_DATA.values() {
             if rod.price > current_rod.price {
                let new_gps = self.calculate_metrics(rod, current_boat, current_biome);
                if new_gps > current_gps {
                    let cost = rod.price;
                    let gain = new_gps - current_gps;
                    // ROI: Seconds to recover cost
                    let roi = if gain > 0.0 { cost as f64 / gain } else { f64::INFINITY };

                    recommendations.push(Recommendation {
                        action: "Buy Rod".to_string(),
                        target_name: rod.name.to_string(),
                        cost,
                        roi_seconds: roi,
                    });
                }
            }
        }

        // Evaluate Boats
        for boat in BOAT_DATA.values() {
             if boat.price > current_boat.price {
                let new_gps = self.calculate_metrics(current_rod, boat, current_biome);
                if new_gps > current_gps {
                    let cost = boat.price;
                    let gain = new_gps - current_gps;
                    let roi = if gain > 0.0 { cost as f64 / gain } else { f64::INFINITY };

                    recommendations.push(Recommendation {
                        action: "Buy Boat".to_string(),
                        target_name: boat.name.to_string(),
                        cost,
                        roi_seconds: roi,
                    });
                }
            }
        }

        // Sort by ROI (lowest seconds is best)
        recommendations.sort_by(|a, b| a.roi_seconds.partial_cmp(&b.roi_seconds).unwrap_or(std::cmp::Ordering::Equal));

        recommendations
    }
}
