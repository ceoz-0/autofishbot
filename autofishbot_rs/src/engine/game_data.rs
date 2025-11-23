use lazy_static::lazy_static;
use std::collections::HashMap;

// --- Enums ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Biome {
    River,
    Volcanic,
    Ocean,
    Sky,
    Space,
    Alien,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RodType {
    Plastic,
    Improved,
    Steel,
    Fiberglass,
    Heavy,
    Alloy,
    Lava,
    Magma,
    Oceanium,
    Golden,
    Superium,
    Infinity,
    Floating,
    Sky,
    Meteor,
    Space,
    Alien,
    Supporter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoatType {
    Rowboat,
    FishingBoat,
    Speedboat,
    Pontoon,
    Sailboat,
    Yacht,
    LuxuryYacht,
    CruiseShip,
    GoldBoat,
    SkyCruiser,
    Satellite,
    SpaceShuttle,
    Cruiser,
    AlienRaft,
    AlienSubmarine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BaitType {
    Worms,
    Leeches,
    Magnet,
    WiseBait,
    Fish,
    ArtifactMagnet,
    MagicBait,
    SupportBait,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TreasureQuality {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
    Artifact,
    VoteUnder100,
    VoteOver100,
    Super,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FishRarity {
    Common,
    Special,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UpgradeCurrency {
    Money,
    LavaFish,
    DiamondFish,
    GoldFish,
    EmeraldFish,
    AzureFish,
    Hooks,
}

// --- Structs ---

#[derive(Debug, Clone)]
pub struct Fish {
    pub name: &'static str,
    pub price: u64,
    pub xp: u64,
    pub biomes: Vec<Biome>,
}

#[derive(Debug, Clone)]
pub struct Rod {
    pub name: &'static str,
    pub price: u64,
    /// Expected value of fish caught per cast
    pub expected_fish: f64,
    pub treasure_chance: f64,
    pub treasure_quality_bonus: f64,
    pub biome_multipliers: HashMap<Biome, f64>,
}

#[derive(Debug, Clone)]
pub struct Boat {
    pub name: &'static str,
    pub price: u64,
    /// Text says "Each boat decreases your cooldown by 0.25s".
    pub cooldown_reduction: f64,
}

#[derive(Debug, Clone)]
pub struct BiomeStats {
    pub name: &'static str,
    pub cooldown_penalty: f64,
    pub catch_rate: f64,
    pub base_cooldown: f64, // Inferred as 3.0 for River
}

#[derive(Debug, Clone)]
pub struct Bait {
    pub name: &'static str,
    pub price: u64,
    pub fish_catch_bonus: f64, // %
    pub fish_quality_bonus: f64, // %
    pub treasure_chance_bonus: f64, // %
    pub treasure_quality_bonus: f64, // %
    /// Normalized Multiplier (1.0 = Base, 0.9 = -10%, 2.5 = +150%)
    pub xp_multiplier: f64,
    pub extra_fish_per_cast: f64,
}

#[derive(Debug, Clone)]
pub struct TreasureDrop {
    pub quality: TreasureQuality,
    pub multiplier: f64,
    pub chance_gold_fish: f64,
    pub chance_emerald_fish: f64,
    pub chance_lava_fish: f64,
    pub chance_diamond_fish: f64,
    pub chance_charm: f64,
    pub chance_money_xp: f64,
    pub chance_super_crate: f64,
    /// Expected number of charms if charms are dropped (Default 1.0, Artifact 2.0)
    pub expected_charms_count: f64,
}

#[derive(Debug, Clone)]
pub struct Upgrade {
    pub name: &'static str,
    pub max_level: u32,
    pub max_cost: u64,
    pub currency: UpgradeCurrency,
    pub description: &'static str,
}

#[derive(Debug, Clone)]
pub struct Pet {
    pub name: &'static str,
    pub catch_bonus: f64,
    pub xp_bonus: f64,
}

// --- Data Loading ---

lazy_static! {
    pub static ref FISH_DATA: HashMap<&'static str, Fish> = {
        let mut m = HashMap::new();
        // River
        m.insert("Raw Fish", Fish { name: "Raw Fish", price: 1, xp: 1, biomes: vec![Biome::River] });
        m.insert("Raw Salmon", Fish { name: "Raw Salmon", price: 3, xp: 2, biomes: vec![Biome::River, Biome::Volcanic, Biome::Space] });
        m.insert("Cod", Fish { name: "Cod", price: 10, xp: 5, biomes: vec![Biome::River, Biome::Volcanic] });
        m.insert("Tropical Fish", Fish { name: "Tropical Fish", price: 50, xp: 10, biomes: vec![Biome::River, Biome::Volcanic, Biome::Ocean] });
        m.insert("Pufferfish", Fish { name: "Pufferfish", price: 150, xp: 25, biomes: vec![Biome::River, Biome::Ocean] });

        // Volcanic
        m.insert("Fiery Pufferfish", Fish { name: "Fiery Pufferfish", price: 250, xp: 50, biomes: vec![Biome::Volcanic] });
        m.insert("Hot Cod", Fish { name: "Hot Cod", price: 500, xp: 100, biomes: vec![Biome::Volcanic] });

        // Ocean
        m.insert("Squid", Fish { name: "Squid", price: 1200, xp: 175, biomes: vec![Biome::Ocean, Biome::Sky] });
        m.insert("Turtle", Fish { name: "Turtle", price: 4000, xp: 400, biomes: vec![Biome::Ocean] });
        m.insert("Dolphin", Fish { name: "Dolphin", price: 20000, xp: 800, biomes: vec![Biome::Ocean] });

        // Sky
        m.insert("Guardian", Fish { name: "Guardian", price: 29000, xp: 1100, biomes: vec![Biome::Sky] });
        m.insert("Emerald Squid", Fish { name: "Emerald Squid", price: 42000, xp: 1900, biomes: vec![Biome::Sky, Biome::Alien] });
        m.insert("Rainbow Fish", Fish { name: "Rainbow Fish", price: 125000, xp: 4800, biomes: vec![Biome::Sky, Biome::Space, Biome::Alien] });

        // Space
        m.insert("Space Fish", Fish { name: "Space Fish", price: 200000, xp: 8000, biomes: vec![Biome::Space, Biome::Alien] });
        m.insert("Galactic Crab", Fish { name: "Galactic Crab", price: 600000, xp: 15000, biomes: vec![Biome::Space] });

        // Alien
        m.insert("Shark", Fish { name: "Shark", price: 2000000, xp: 35000, biomes: vec![Biome::Alien] });
        m.insert("Alien Fish", Fish { name: "Alien Fish", price: 5000000, xp: 65000, biomes: vec![Biome::Alien] });

        m
    };

    pub static ref ROD_DATA: HashMap<RodType, Rod> = {
        let mut m = HashMap::new();
        let mk_rod = |name, price, min, max, treasure_chance, quality_bonus| Rod {
            name, price, expected_fish: (min as f64 + max as f64) / 2.0, treasure_chance, treasure_quality_bonus: quality_bonus, biome_multipliers: HashMap::new()
        };

        m.insert(RodType::Plastic, mk_rod("Plastic Rod", 0, 4, 10, 0.05, 0.0));
        m.insert(RodType::Improved, mk_rod("Improved Rod", 500, 5, 10, 0.05, 0.0));
        m.insert(RodType::Steel, mk_rod("Steel Rod", 8_000, 5, 8, 0.05, 0.0));
        m.insert(RodType::Fiberglass, mk_rod("Fiberglass Rod", 50_000, 7, 10, 0.05, 0.0));
        m.insert(RodType::Heavy, mk_rod("Heavy Rod", 100_000, 6, 9, 0.085, 0.05));
        m.insert(RodType::Alloy, mk_rod("Alloy Rod", 250_000, 4, 13, 0.05, 0.0));
        m.insert(RodType::Lava, mk_rod("Lava Rod", 1_000_000, 7, 11, 0.05, 0.0));
        m.insert(RodType::Magma, mk_rod("Magma Rod", 10_000_000, 10, 13, 0.05, 0.0));
        m.insert(RodType::Oceanium, mk_rod("Oceanium Rod", 75_000_000, 11, 14, 0.05, 0.0));
        m.insert(RodType::Golden, mk_rod("Golden Rod", 120_000_000, 4, 6, 0.13, 0.0));
        m.insert(RodType::Superium, mk_rod("Superium Rod", 250_000_000, 8, 18, 0.055, 0.0));
        m.insert(RodType::Infinity, mk_rod("Infinity Rod", 1_000_000_000, 15, 18, 0.06, 0.0));
        m.insert(RodType::Floating, mk_rod("Floating Rod", 50_000_000_000, 15, 30, 0.065, 0.0));
        m.insert(RodType::Sky, mk_rod("Sky Rod", 250_000_000_000, 30, 34, 0.067, 0.0));
        m.insert(RodType::Meteor, mk_rod("Meteor Rod", 500_000_000_000, 20, 24, 0.15, 0.30));
        m.insert(RodType::Space, mk_rod("Space Rod", 1_000_000_000_000, 33, 37, 0.068, 0.0));
        m.insert(RodType::Alien, mk_rod("Alien Rod", 5_000_000_000_000, 37, 42, 0.07, 0.10));

        let mut supporter = mk_rod("Supporter Rod", 0, 7, 10, 0.065, 0.10);
        supporter.biome_multipliers.insert(Biome::River, 1.0);
        supporter.biome_multipliers.insert(Biome::Volcanic, 1.2);
        supporter.biome_multipliers.insert(Biome::Ocean, 1.4);
        supporter.biome_multipliers.insert(Biome::Sky, 2.0);
        supporter.biome_multipliers.insert(Biome::Space, 3.2);
        supporter.biome_multipliers.insert(Biome::Alien, 4.0);
        m.insert(RodType::Supporter, supporter);

        m
    };

    pub static ref BOAT_DATA: HashMap<BoatType, Boat> = {
        let mut m = HashMap::new();
        let mk_boat = |name, price| Boat { name, price, cooldown_reduction: 0.25 };
        m.insert(BoatType::Rowboat, mk_boat("Rowboat", 5_000));
        m.insert(BoatType::FishingBoat, mk_boat("Fishing Boat", 25_000));
        m.insert(BoatType::Speedboat, mk_boat("Speedboat", 100_000));
        m.insert(BoatType::Pontoon, mk_boat("Pontoon", 250_000));
        m.insert(BoatType::Sailboat, mk_boat("Sailboat", 1_000_000));
        m.insert(BoatType::Yacht, mk_boat("Yacht", 20_000_000));
        m.insert(BoatType::LuxuryYacht, mk_boat("Luxury Yacht", 100_000_000));
        m.insert(BoatType::CruiseShip, mk_boat("Cruise Ship", 500_000_000));
        m.insert(BoatType::GoldBoat, mk_boat("Gold Boat", 2_500_000_000));
        m.insert(BoatType::SkyCruiser, mk_boat("Sky Cruiser", 10_000_000_000));
        m.insert(BoatType::Satellite, mk_boat("Satellite", 50_000_000_000));
        m.insert(BoatType::SpaceShuttle, mk_boat("Space Shuttle", 250_000_000_000));
        m.insert(BoatType::Cruiser, mk_boat("Cruiser", 1_000_000_000_000));
        m.insert(BoatType::AlienRaft, mk_boat("Alien Raft", 2_500_000_000_000));
        m.insert(BoatType::AlienSubmarine, mk_boat("Alien Submarine", 5_000_000_000_000));
        m
    };

    pub static ref BIOME_DATA: HashMap<Biome, BiomeStats> = {
        let mut m = HashMap::new();
        m.insert(Biome::River, BiomeStats { name: "River", cooldown_penalty: 0.0, catch_rate: 1.0, base_cooldown: 3.0 });
        m.insert(Biome::Volcanic, BiomeStats { name: "Volcanic", cooldown_penalty: 0.5, catch_rate: 0.60, base_cooldown: 3.0 });
        m.insert(Biome::Ocean, BiomeStats { name: "Ocean", cooldown_penalty: 1.0, catch_rate: 0.30, base_cooldown: 3.0 });
        m.insert(Biome::Sky, BiomeStats { name: "Sky", cooldown_penalty: 2.0, catch_rate: 0.12, base_cooldown: 3.0 });
        m.insert(Biome::Space, BiomeStats { name: "Space", cooldown_penalty: 3.0, catch_rate: 0.065, base_cooldown: 3.0 });
        m.insert(Biome::Alien, BiomeStats { name: "Alien", cooldown_penalty: 4.0, catch_rate: 0.03, base_cooldown: 3.0 });
        m
    };

    pub static ref BAIT_DATA: HashMap<BaitType, Bait> = {
        let mut m = HashMap::new();
        let default = Bait {
            name: "", price: 0,
            fish_catch_bonus: 0.0, fish_quality_bonus: 0.0,
            treasure_chance_bonus: 0.0, treasure_quality_bonus: 0.0,
            xp_multiplier: 1.0, extra_fish_per_cast: 0.0
        };

        // Worms: XP -10% -> 0.9
        m.insert(BaitType::Worms, Bait { name: "Worms", price: 4, xp_multiplier: 0.9, extra_fish_per_cast: 2.0, ..default });
        // Leeches: XP -20% -> 0.8
        m.insert(BaitType::Leeches, Bait { name: "Leeches", price: 25, fish_catch_bonus: 0.20, fish_quality_bonus: 0.20, xp_multiplier: 0.8, extra_fish_per_cast: 3.0, ..default });
        // Magnet: XP +20% -> 1.2
        m.insert(BaitType::Magnet, Bait { name: "Magnet", price: 25, fish_catch_bonus: -0.10, fish_quality_bonus: -0.10, treasure_chance_bonus: 0.50, xp_multiplier: 1.2, ..default });
        // Wise Bait: XP +150% -> 2.5 (1.0 + 1.5)
        m.insert(BaitType::WiseBait, Bait { name: "Wise Bait", price: 35, xp_multiplier: 2.5, ..default });
        // Fish: XP -30% -> 0.7
        m.insert(BaitType::Fish, Bait { name: "Fish", price: 70, fish_catch_bonus: 0.50, fish_quality_bonus: 1.00, xp_multiplier: 0.7, extra_fish_per_cast: 1.0, ..default });
        // Artifact Magnet: XP +30% -> 1.3
        m.insert(BaitType::ArtifactMagnet, Bait { name: "Artifact Magnet", price: 75, fish_catch_bonus: -0.30, fish_quality_bonus: -0.30, treasure_chance_bonus: 0.40, treasure_quality_bonus: 0.50, xp_multiplier: 1.3, ..default });
        // Magic Bait: XP -20% -> 0.8
        m.insert(BaitType::MagicBait, Bait { name: "Magic Bait", price: 250, fish_catch_bonus: 1.00, fish_quality_bonus: 0.50, treasure_chance_bonus: 0.15, treasure_quality_bonus: 0.15, xp_multiplier: 0.8, extra_fish_per_cast: 2.0, ..default });
        // Support Bait: Pet XP +35% (Not strictly global XP multiplier, but stored as 1.0 for global logic per instruction "Sanity Check")
        m.insert(BaitType::SupportBait, Bait { name: "Support Bait", price: 500, ..default });
        m
    };

    pub static ref TREASURE_DATA: HashMap<TreasureQuality, TreasureDrop> = {
        let mut m = HashMap::new();
        let default_drop = TreasureDrop {
            quality: TreasureQuality::Common, multiplier: 1.0,
            chance_gold_fish: 0.0, chance_emerald_fish: 0.0, chance_lava_fish: 0.0, chance_diamond_fish: 0.0,
            chance_charm: 0.0, chance_money_xp: 0.0, chance_super_crate: 0.0, expected_charms_count: 1.0
        };

        m.insert(TreasureQuality::Common, TreasureDrop { quality: TreasureQuality::Common, multiplier: 1.0, chance_gold_fish: 0.15, chance_emerald_fish: 0.02, chance_lava_fish: 0.01, chance_money_xp: 0.82, ..default_drop });
        m.insert(TreasureQuality::Uncommon, TreasureDrop { quality: TreasureQuality::Uncommon, multiplier: 1.0, chance_gold_fish: 0.25, chance_emerald_fish: 0.05, chance_lava_fish: 0.019, chance_diamond_fish: 0.01, chance_money_xp: 0.671, ..default_drop });
        m.insert(TreasureQuality::Rare, TreasureDrop { quality: TreasureQuality::Rare, multiplier: 1.4, chance_gold_fish: 0.38, chance_emerald_fish: 0.15, chance_lava_fish: 0.065, chance_diamond_fish: 0.06, chance_charm: 0.02, chance_money_xp: 0.325, ..default_drop });
        m.insert(TreasureQuality::Epic, TreasureDrop { quality: TreasureQuality::Epic, multiplier: 1.7, chance_gold_fish: 0.25, chance_emerald_fish: 0.25, chance_lava_fish: 0.10, chance_diamond_fish: 0.15, chance_charm: 0.25, ..default_drop });
        m.insert(TreasureQuality::Legendary, TreasureDrop { quality: TreasureQuality::Legendary, multiplier: 2.2, chance_gold_fish: 0.15, chance_emerald_fish: 0.10, chance_lava_fish: 0.15, chance_diamond_fish: 0.20, chance_charm: 0.40, ..default_drop });
        // Artifact: 75% Charm, 1-3 Charms (Expected: 2.0)
        m.insert(TreasureQuality::Artifact, TreasureDrop { quality: TreasureQuality::Artifact, multiplier: 3.0, chance_gold_fish: 0.05, chance_emerald_fish: 0.05, chance_lava_fish: 0.05, chance_diamond_fish: 0.10, chance_charm: 0.75, expected_charms_count: 2.0, ..default_drop });

        m.insert(TreasureQuality::VoteUnder100, TreasureDrop { quality: TreasureQuality::VoteUnder100, multiplier: 5.0, chance_gold_fish: 0.10, chance_emerald_fish: 0.40, chance_charm: 0.50, ..default_drop });
        m.insert(TreasureQuality::VoteOver100, TreasureDrop { quality: TreasureQuality::VoteOver100, multiplier: 5.0, chance_diamond_fish: 0.50, chance_charm: 0.50, ..default_drop });
        // Super: 50% Charm, 45% Super Crate, 5% Money
        m.insert(TreasureQuality::Super, TreasureDrop { quality: TreasureQuality::Super, multiplier: 7.5, chance_charm: 0.50, chance_money_xp: 0.05, chance_super_crate: 0.45, ..default_drop });

        m
    };

    pub static ref UPGRADE_DATA: HashMap<&'static str, Upgrade> = {
        let mut m = HashMap::new();
        // Normal
        m.insert("Better Fish", Upgrade { name: "Better Fish", max_level: 21, max_cost: 14_902_000, currency: UpgradeCurrency::Money, description: "Increases Fish Quality by 5%." });
        m.insert("Salesman", Upgrade { name: "Salesman", max_level: 18, max_cost: 4_058_000, currency: UpgradeCurrency::Money, description: "Increases sell price by 5% per upgrade." });
        m.insert("Bait Efficiency", Upgrade { name: "Bait Efficiency", max_level: 9, max_cost: 220_500, currency: UpgradeCurrency::Money, description: "Lowers the chance of consuming bait by 5%." });
        m.insert("More Chests", Upgrade { name: "More Chests", max_level: 11, max_cost: 6_087_000, currency: UpgradeCurrency::Money, description: "Increases Treasure Chance by 5%." });
        m.insert("Worker Motivation", Upgrade { name: "Worker Motivation", max_level: 12, max_cost: 777_310_000, currency: UpgradeCurrency::Money, description: "Increases Fish Catch from workers by 10%." });
        m.insert("Artifact Specialist", Upgrade { name: "Artifact Specialist", max_level: 7, max_cost: 521_500, currency: UpgradeCurrency::Money, description: "Improves Treasure rewards by 10% excluding charms." });
        m.insert("Experienced", Upgrade { name: "Experienced", max_level: 5, max_cost: 555_550_000, currency: UpgradeCurrency::Money, description: "Increases XP Gain by 10%." });
        m.insert("Better Chests", Upgrade { name: "Better Chests", max_level: 5, max_cost: 131_100_000, currency: UpgradeCurrency::Money, description: "Increases Treasure Quality by 10%." });
        m.insert("Better Dailies", Upgrade { name: "Better Dailies", max_level: 10, max_cost: 2_990_000, currency: UpgradeCurrency::Money, description: "Increases Daily Rewards items by 10%." });

        // Special
        m.insert("Fish Ovens", Upgrade { name: "Fish Ovens", max_level: 20, max_cost: 3360, currency: UpgradeCurrency::LavaFish, description: "Increases Sell Price by 5%." });
        m.insert("Bait Lover", Upgrade { name: "Bait Lover", max_level: 4, max_cost: 385, currency: UpgradeCurrency::DiamondFish, description: "Increases effectiveness of bait by 15%." });
        m.insert("Aquatic Expert", Upgrade { name: "Aquatic Expert", max_level: 4, max_cost: 385, currency: UpgradeCurrency::DiamondFish, description: "Increases Fish Catch by 5%." });
        m.insert("Worker Extender", Upgrade { name: "Worker Extender", max_level: 4, max_cost: 385, currency: UpgradeCurrency::DiamondFish, description: "Increases the length of Worker Boosts by 10%." });
        m.insert("Ultimate Salesman", Upgrade { name: "Ultimate Salesman", max_level: 4, max_cost: 385, currency: UpgradeCurrency::DiamondFish, description: "Increases Sell Price by 15%." });
        m.insert("Highly Experienced", Upgrade { name: "Highly Experienced", max_level: 4, max_cost: 395, currency: UpgradeCurrency::DiamondFish, description: "Increases XP Gain by 15%." });
        m.insert("Boost Booster", Upgrade { name: "Boost Booster", max_level: 4, max_cost: 385, currency: UpgradeCurrency::DiamondFish, description: "Increases Treasure Quality, Fish Quality, and Worker speed." });
        m.insert("Statistician", Upgrade { name: "Statistician", max_level: 10, max_cost: 4000, currency: UpgradeCurrency::GoldFish, description: "Increases all Multipliers by 2%." });
        m.insert("Duplicator", Upgrade { name: "Duplicator", max_level: 10, max_cost: 4400, currency: UpgradeCurrency::EmeraldFish, description: "Increases chance to duplicate fish by 2%." });
        m.insert("Charmer", Upgrade { name: "Charmer", max_level: 10, max_cost: 3200, currency: UpgradeCurrency::LavaFish, description: "Increases Charms found by 2.5%." });

        // Prestige (All max 26, cost 1 Azure Fish per tier? Assuming linear cost 1 per tier implies Max Cost = Max Level)
        m.insert("International Ties", Upgrade { name: "International Ties", max_level: 26, max_cost: 26, currency: UpgradeCurrency::AzureFish, description: "Increases all multipliers by 10%." });
        m.insert("Business Education", Upgrade { name: "Business Education", max_level: 26, max_cost: 26, currency: UpgradeCurrency::AzureFish, description: "Increases Sell Price by 40%." });
        m.insert("Fish Whisperer", Upgrade { name: "Fish Whisperer", max_level: 26, max_cost: 26, currency: UpgradeCurrency::AzureFish, description: "Increases Fish Catch by 25%." });
        m.insert("Ancient One", Upgrade { name: "Ancient One", max_level: 26, max_cost: 26, currency: UpgradeCurrency::AzureFish, description: "Increases XP Gain by 35%." });
        m.insert("Virtual Fisher", Upgrade { name: "Virtual Fisher", max_level: 26, max_cost: 26, currency: UpgradeCurrency::AzureFish, description: "Increases Fish Quality, Sell Price, and XP Gain." });

        // League
        m.insert("Pet Helper", Upgrade { name: "Pet Helper", max_level: 5, max_cost: 1050, currency: UpgradeCurrency::Hooks, description: "Increases max Pet Level." });
        m.insert("Bait Helper", Upgrade { name: "Bait Helper", max_level: 5, max_cost: 1050, currency: UpgradeCurrency::Hooks, description: "Increases effectiveness of Bait." });
        m.insert("Super Crates", Upgrade { name: "Super Crates", max_level: 5, max_cost: 1050, currency: UpgradeCurrency::Hooks, description: "Unlocks Super Crates." });
        m.insert("Worker Crates", Upgrade { name: "Worker Crates", max_level: 5, max_cost: 1050, currency: UpgradeCurrency::Hooks, description: "Unlocks Worker Crates." });
        m.insert("Fishing Frenzy", Upgrade { name: "Fishing Frenzy", max_level: 5, max_cost: 1050, currency: UpgradeCurrency::Hooks, description: "Increases Worker fish speed." });
        m.insert("Duplicator 2.0", Upgrade { name: "Duplicator 2.0", max_level: 10, max_cost: 1185, currency: UpgradeCurrency::Hooks, description: "Increases Duplication chance." });

        m
    };

    pub static ref PET_DATA: HashMap<&'static str, Pet> = {
        let mut m = HashMap::new();
        m.insert("Dolphin", Pet { name: "Dolphin", catch_bonus: 0.05, xp_bonus: 0.10 });
        m.insert("Puffer", Pet { name: "Puffer", catch_bonus: 0.10, xp_bonus: 0.0 });
        m.insert("Shark", Pet { name: "Shark", catch_bonus: 0.15, xp_bonus: 0.05 });
        m
    };
}
