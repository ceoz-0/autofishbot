use sqlx::{sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteJournalMode}, Pool, Sqlite, Row};
use anyhow::Result;
use std::path::Path;
use tokio::fs;
use std::str::FromStr;
use log::info;
use crate::engine::game_data::Biome;
use crate::engine::optimizer::BiomeStats;
use std::collections::HashMap;

pub struct Database {
    pub pool: Pool<Sqlite>,
}

impl Database {
    pub async fn new(db_path: &str) -> Result<Self> {
        // Create file if not exists
        if !Path::new(db_path).exists() {
            info!("Creating database file: {}", db_path);
            fs::File::create(db_path).await?;
        }

        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path))?
            .journal_mode(SqliteJournalMode::Delete) // Use DELETE journal mode to avoid WAL lock issues
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        let db = Self { pool };
        db.migrate().await?;
        Ok(db)
    }

    async fn migrate(&self) -> Result<()> {
        // Fish Table: Stores discovered fish
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS fish (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT UNIQUE NOT NULL,
                rarity TEXT,
                base_value REAL,
                biome TEXT,
                sell_value REAL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Ensure sell_value column exists (manual migration for existing dbs)
        let _ = sqlx::query("ALTER TABLE fish ADD COLUMN sell_value REAL").execute(&self.pool).await;

        // Ensure stats column exists for shop_items
        let _ = sqlx::query("ALTER TABLE shop_items ADD COLUMN stats TEXT").execute(&self.pool).await;

        // Catch History: Logs every fishing result
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS catch_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                fish_name TEXT,
                quantity INTEGER,
                xp REAL,
                biome TEXT,
                money_gained REAL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Player Snapshots: Logs player stats over time
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS player_snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                level INTEGER,
                xp REAL,
                balance REAL,
                current_biome TEXT
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Cooldown Events: Track when we hit a cooldown
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS cooldown_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                wait_time REAL,
                total_cooldown REAL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        // --- NEW TABLES FOR DATA GATHERING ---

        // Shop Items: Catalogs items found in shops
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS shop_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                shop_type TEXT NOT NULL,
                price REAL,
                currency TEXT,
                description TEXT,
                stock INTEGER,
                last_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(name, shop_type)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Game Entities: Generic storage for anything else (Buffs, Quests, etc found in lists)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS game_entities (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                entity_type TEXT NOT NULL, -- "Buff", "Quest", "Badge"
                name TEXT NOT NULL,
                details TEXT, -- JSON or raw text
                last_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(entity_type, name)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

         // Command Registry: Tracks commands we've found and executed
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS command_registry (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                full_command_name TEXT UNIQUE NOT NULL, -- e.g. "shop buy"
                description TEXT,
                params TEXT,
                command_structure TEXT, -- JSON of full command definition
                last_executed DATETIME
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Migration to add command_structure if missing
        let _ = sqlx::query("ALTER TABLE command_registry ADD COLUMN command_structure TEXT").execute(&self.pool).await;

        // Biome Stats: Persistent learning for Optimizer
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS biome_stats (
                biome_name TEXT PRIMARY KEY,
                total_gold INTEGER,
                total_xp INTEGER,
                total_catches INTEGER
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn log_catch(&self, fish_name: &str, quantity: i32, xp: f32, biome: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO catch_history (fish_name, quantity, xp, biome)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(fish_name)
        .bind(quantity)
        .bind(xp)
        .bind(biome)
        .execute(&self.pool)
        .await?;

        // Also ensure fish exists
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO fish (name, biome)
            VALUES (?, ?)
            "#,
        )
        .bind(fish_name)
        .bind(biome)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn log_snapshot(&self, level: i32, xp: f32, balance: f32, biome: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO player_snapshots (level, xp, balance, current_biome)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(level)
        .bind(xp)
        .bind(balance)
        .bind(biome)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn log_cooldown(&self, wait_time: f32, total_cooldown: f32) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO cooldown_events (wait_time, total_cooldown)
            VALUES (?, ?)
            "#,
        )
        .bind(wait_time)
        .bind(total_cooldown)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn upsert_shop_item(&self, name: &str, shop_type: &str, price: f32, currency: &str, description: &str, stock: Option<i32>, stats: Option<&str>) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO shop_items (name, shop_type, price, currency, description, stock, stats, last_seen)
            VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(name, shop_type) DO UPDATE SET
            price = excluded.price,
            currency = excluded.currency,
            description = excluded.description,
            stock = excluded.stock,
            stats = excluded.stats,
            last_seen = CURRENT_TIMESTAMP;
            "#,
        )
        .bind(name)
        .bind(shop_type)
        .bind(price)
        .bind(currency)
        .bind(description)
        .bind(stock)
        .bind(stats)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn upsert_game_entity(&self, entity_type: &str, name: &str, details: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO game_entities (entity_type, name, details, last_seen)
            VALUES (?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(entity_type, name) DO UPDATE SET
            details = excluded.details,
            last_seen = CURRENT_TIMESTAMP;
            "#,
        )
        .bind(entity_type)
        .bind(name)
        .bind(details)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn register_command(&self, name: &str, description: &str, params: &str, structure: &str) -> Result<()> {
         sqlx::query(
            r#"
            INSERT INTO command_registry (full_command_name, description, params, command_structure)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(full_command_name) DO UPDATE SET
            description = excluded.description,
            params = excluded.params,
            command_structure = excluded.command_structure;
            "#,
        )
        .bind(name)
        .bind(description)
        .bind(params)
        .bind(structure)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_command_executed(&self, name: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE command_registry SET last_executed = CURRENT_TIMESTAMP WHERE full_command_name = ?
            "#,
        )
        .bind(name)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn save_biome_stats(&self, biome: Biome, stats: &BiomeStats) -> Result<()> {
        let biome_name = format!("{:?}", biome);
        sqlx::query(
            r#"
            INSERT INTO biome_stats (biome_name, total_gold, total_xp, total_catches)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(biome_name) DO UPDATE SET
            total_gold = excluded.total_gold,
            total_xp = excluded.total_xp,
            total_catches = excluded.total_catches;
            "#
        )
        .bind(biome_name)
        .bind(stats.total_gold as i64)
        .bind(stats.total_xp as i64)
        .bind(stats.total_catches as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn load_biome_stats(&self) -> Result<HashMap<Biome, BiomeStats>> {
        let rows = sqlx::query("SELECT biome_name, total_gold, total_xp, total_catches FROM biome_stats")
            .fetch_all(&self.pool)
            .await?;

        let mut map = HashMap::new();
        for row in rows {
            let name: String = row.get("biome_name");
            let biome = match name.as_str() {
                "River" => Biome::River,
                "Volcanic" => Biome::Volcanic,
                "Ocean" => Biome::Ocean,
                "Sky" => Biome::Sky,
                "Space" => Biome::Space,
                "Alien" => Biome::Alien,
                _ => continue,
            };

            let total_catches: i64 = row.get("total_catches");
            let total_gold: i64 = row.get("total_gold");
            let total_xp: i64 = row.get("total_xp");

            let mut stats = BiomeStats {
                total_catches: total_catches as u64,
                total_gold: total_gold as u64,
                total_xp: total_xp as u64,
                avg_gold_per_fish: 0.0,
                avg_xp_per_fish: 0.0,
            };

            if stats.total_catches > 0 {
                stats.avg_gold_per_fish = stats.total_gold as f64 / stats.total_catches as f64;
                stats.avg_xp_per_fish = stats.total_xp as f64 / stats.total_catches as f64;
            }
            map.insert(biome, stats);
        }
        Ok(map)
    }
}
