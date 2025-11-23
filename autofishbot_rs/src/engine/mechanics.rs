use crate::engine::game_data::*;

// --- Constants & Configuration ---

/// Hardcoded asymptotic limit for worker speed (~10.52s)
pub const WORKER_EFFICIENCY_LIMIT_SECONDS: f64 = 10.52;

// Estimated values for heuristic optimization (Arbitrary units of "Utility" or Gold equivalent)
const VAL_GOLD_FISH: f64 = 500.0;
const VAL_EMERALD_FISH: f64 = 1000.0;
const VAL_LAVA_FISH: f64 = 2000.0;
const VAL_DIAMOND_FISH: f64 = 5000.0;
const VAL_CHARM: f64 = 10_000.0;
const VAL_SUPER_CRATE: f64 = 50_000.0;
const VAL_MONEY_UNIT: f64 = 1.0; // Base value of $1

pub struct GameState {
    pub money: u64,
    pub level: u32,
    pub boost_level: u32,  // "Boost Booster" level
    pub frenzy_level: u32, // "Fishing Frenzy" level
    pub current_biome: Biome,
    pub current_rod: RodType,
    pub owned_boats: Vec<BoatType>,
}

#[derive(Debug, Clone)]
pub enum Action {
    BuyUpgrade(String),
    UnlockBiome(Biome),
    BuyRod(RodType),
    BuyBoat(BoatType),
    SaveMoney,
}

// --- Mathematical Implementations ---

/// Calculates the exact 'Fish Boost' gain based on the User's LaTeX formula.
///
/// Formula: Gain = 0.05 + (0.01 * BoostLvl) + (E[0,3] * FrenzyLvl) + (E[1,4] + BoostLvl) * RodMult * BiomeMult
///
/// We interpret the random ranges [0,3] and [1,4] as their Expected Values:
/// E[0,3] = 1.5
/// E[1,4] = 2.5
pub fn calculate_fish_boost_gain(
    boost_lvl: u32,
    frenzy_lvl: u32,
    rod: &Rod,
    biome: Biome,
) -> f64 {
    // Term 1: 0.05
    let t1 = 0.05;

    // Term 2: 0.01 * BoostLvl
    let t2 = 0.01 * boost_lvl as f64;

    // Term 3: E[0,3] * FrenzyLvl (Flat bonus)
    let e_0_3 = 1.5;
    let t3 = e_0_3 * frenzy_lvl as f64;

    // Term 4: (E[1,4] + BoostLvl) * RodMult * BiomeMult
    let e_1_4 = 2.5;
    let rod_mult = *rod.biome_multipliers.get(&biome).unwrap_or(&1.0);
    // Assuming BiomeMult is 1.0 based on context or captured in RodMult (Supporter Rod logic)
    let biome_mult = 1.0;
    let t4 = (e_1_4 + boost_lvl as f64) * rod_mult * biome_mult;

    // Final Formula:
    t1 + t2 + t3 + t4
}

/// Calculates the cooldown topology.
/// Formula: T_total = min(5.0, T_base + T_biome_penalty - T_boat_reduction)
pub fn calculate_cooldown(
    biome: Biome,
    owned_boats: &[BoatType],
) -> f64 {
    let biome_stats = &BIOME_DATA[&biome];
    let base = biome_stats.base_cooldown; // 3.0
    let penalty = biome_stats.cooldown_penalty;

    // "Each boat decreases your cooldown by 0.25s"
    let reduction: f64 = owned_boats.iter().map(|b| BOAT_DATA[b].cooldown_reduction).sum();

    let raw_time = base + penalty - reduction;

    // "Caps at 5s".
    let t_total = raw_time.min(5.0);

    t_total.max(0.0)
}

/// Calculates the expected monetary value (EV) of a treasure chest.
/// Accounts for conditional probabilities like Artifact charm counts.
pub fn get_treasure_ev(tier: TreasureQuality) -> f64 {
    let drop = &TREASURE_DATA[&tier];

    // Calculate EV of contents
    // EV = Sum(Probability * Value)

    let ev_contents =
        (drop.chance_gold_fish * VAL_GOLD_FISH) +
        (drop.chance_emerald_fish * VAL_EMERALD_FISH) +
        (drop.chance_lava_fish * VAL_LAVA_FISH) +
        (drop.chance_diamond_fish * VAL_DIAMOND_FISH) +
        (drop.chance_charm * VAL_CHARM * drop.expected_charms_count) + // Adjusted for expected count (e.g. 2.0 for Artifact)
        (drop.chance_super_crate * VAL_SUPER_CRATE) +
        (drop.chance_money_xp * VAL_MONEY_UNIT * 1000.0);

    // Apply the Treasure Quality Multiplier (e.g. 1.4x for Rare)
    ev_contents * drop.multiplier
}

// --- Optimization Heuristic ---

/// Skeleton function to calculate the next best action.
/// Compares marginal utility of upgrades vs biome unlocks.
pub fn calculate_next_best_action(state: &GameState) -> Action {
    // 1. Calculate current Income Rate (Gold/sec)
    let current_income = calculate_income_rate(state);

    // 2. Evaluate "Buy Generic Upgrade"
    // We iterate over all upgrades that cost Money ("Generic").
    // Note: In a real solver, we'd check current level and specific cost curve.
    // Here we use a simplified "Average Cost" based on Max Cost / Max Level to estimate heuristic.

    let mut best_upgrade_action = None;
    let mut max_upgrade_utility = 0.0;

    for (key, upgrade) in UPGRADE_DATA.iter() {
        if upgrade.currency == UpgradeCurrency::Money {
            // Estimate current cost (Simplified: MaxCost / MaxLevel for rough average)
            let avg_cost = upgrade.max_cost as f64 / upgrade.max_level as f64;

            // Estimate benefit (Simplified: Assume 5% income boost for Salesman-like, or small constant)
            // Real implementation would parse 'description' or have specific logic per upgrade type.
            let estimated_boost = if upgrade.name.contains("Salesman") {
                0.05
            } else {
                0.01
            };

            let new_income = current_income * (1.0 + estimated_boost);
            let marginal_utility = (new_income - current_income) / avg_cost;

            if marginal_utility > max_upgrade_utility {
                max_upgrade_utility = marginal_utility;
                best_upgrade_action = Some(Action::BuyUpgrade(key.to_string()));
            }
        }
    }

    // 3. Evaluate "Unlock Next Biome"
    let next_biome = match state.current_biome {
        Biome::River => Some(Biome::Volcanic),
        Biome::Volcanic => Some(Biome::Ocean),
        Biome::Ocean => Some(Biome::Sky),
        Biome::Sky => Some(Biome::Space),
        Biome::Space => Some(Biome::Alien),
        Biome::Alien => None,
    };

    let mut best_action = Action::SaveMoney;
    let max_utility = max_upgrade_utility;

    if let Some(action) = best_upgrade_action {
        best_action = action;
    }

    if let Some(_biome) = next_biome {
        // Placeholder for biome unlocking comparison
    }

    // Heuristic decision
    if max_utility <= 0.0 {
        // If no upgrade gives utility (or money low), defaults to Save
        best_action = Action::SaveMoney;
    }

    best_action
}

fn calculate_income_rate(state: &GameState) -> f64 {
    // Simple Model: Fish Price * (Fish/Cast) / Cooldown
    let rod = &ROD_DATA[&state.current_rod];
    let avg_fish = rod.expected_fish;

    // Average fish price in biome
    let biome_fish = get_fish_in_biome(state.current_biome);
    if biome_fish.is_empty() { return 0.0; }

    let avg_price: f64 = biome_fish.iter().map(|f| f.price as f64).sum::<f64>() / biome_fish.len() as f64;

    let cooldown = calculate_cooldown(state.current_biome, &state.owned_boats);

    if cooldown <= 0.001 { return 999_999.0; }

    (avg_price * avg_fish) / cooldown
}

fn get_fish_in_biome(biome: Biome) -> Vec<&'static Fish> {
    FISH_DATA.values().filter(|f| f.biomes.contains(&biome)).collect()
}
