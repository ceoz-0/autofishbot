use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use anyhow::Result;
use std::path::Path;
use tokio::fs;

pub struct Database {
    pub pool: Pool<Sqlite>,
}

impl Database {
    pub async fn new(db_path: &str) -> Result<Self> {
        // Create file if not exists
        if !Path::new(db_path).exists() {
            fs::File::create(db_path).await?;
        }

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&format!("sqlite://{}", db_path))
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
                biome TEXT
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

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
}
