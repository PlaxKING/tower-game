//! Storage Layer - Unified data access for Tower Game
//!
//! Implements the Repository pattern with multi-tier storage:
//! - **LMDB**: Static templates (monsters, items, abilities, recipes, quests, factions)
//! - **PostgreSQL**: Player data, economy, social, quest progress
//! - **LRU Cache**: Hot entities in memory
//!
//! ## Architecture
//! ```text
//! [Game Systems]
//!       ↓
//! [Repository Trait]
//!       ↓
//! ┌─────────────────┬──────────────┐
//! │ LmdbStore       │ PostgresStore│
//! │ (templates)     │ (player data)│
//! │ + RepoAdapters  │              │
//! └─────────────────┴──────────────┘
//! ```
//!
//! ## Usage
//! ```rust,ignore
//! // Initialize storage
//! let lmdb = LmdbTemplateStore::new("data/templates", 500_000_000)?;
//! let pg = PostgresStore::new("postgres://...", 10).await?;
//!
//! // Seed initial data
//! seed_data::seed_all(&lmdb)?;
//!
//! // Use directly
//! let monster = lmdb.get_monster("goblin_scout")?;
//! let player = pg.get_player(1).await?;
//! ```

pub mod lmdb_repo_adapter;
pub mod lmdb_templates;
pub mod migrations;
pub mod plugin;
pub mod postgres;
pub mod postgres_repo_adapter;
pub mod repository;
pub mod seed_data;

use std::sync::Arc;
use tracing::info;

use self::lmdb_repo_adapter::*;
use self::lmdb_templates::LmdbTemplateStore;
use self::postgres::PostgresStore;
use self::postgres_repo_adapter::*;
use self::repository::StorageManager;

/// Initialize the complete storage layer
///
/// Creates both LMDB (static templates) and PostgreSQL (player data) stores,
/// seeds initial template data, and returns a unified StorageManager.
pub async fn init_storage(
    lmdb_path: &str,
    lmdb_max_size: usize,
    postgres_url: &str,
    pg_max_connections: u32,
) -> Result<StorageManager, Box<dyn std::error::Error + Send + Sync>> {
    // Initialize LMDB for static templates
    let lmdb = Arc::new(LmdbTemplateStore::new(lmdb_path, lmdb_max_size)?);
    seed_data::seed_all(&lmdb)
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() })?;
    info!("LMDB template store initialized and seeded");

    // Initialize PostgreSQL for player data
    let pg = Arc::new(PostgresStore::new(postgres_url, pg_max_connections).await?);
    info!("PostgreSQL player store initialized");

    // Build unified StorageManager with all repository adapters
    let manager = StorageManager {
        // Template repos (LMDB)
        monsters: Box::new(LmdbMonsterRepo::new(lmdb.clone())),
        items: Box::new(LmdbItemRepo::new(lmdb.clone())),
        abilities: Box::new(LmdbAbilityRepo::new(lmdb.clone())),
        recipes: Box::new(LmdbRecipeRepo::new(lmdb.clone())),
        quests: Box::new(LmdbQuestRepo::new(lmdb.clone())),
        factions: Box::new(LmdbFactionRepo::new(lmdb.clone())),
        loot_tables: Box::new(LmdbLootTableRepo::new(lmdb.clone())),
        item_sets: Box::new(LmdbItemSetRepo::new(lmdb.clone())),
        gems: Box::new(LmdbGemRepo::new(lmdb)),

        // Player data repos (PostgreSQL)
        players: Box::new(PgPlayerRepo::new(pg.clone())),
        mastery: Box::new(PgMasteryRepo::new(pg.clone())),
        inventory: Box::new(PgInventoryRepo::new(pg.clone())),
        wallets: Box::new(PgWalletRepo::new(pg.clone())),
        guilds: Box::new(PgGuildRepo::new(pg.clone())),
        quest_progress: Box::new(PgQuestProgressRepo::new(pg.clone())),
        auctions: Box::new(PgAuctionRepo::new(pg.clone())),
        reputation: Box::new(PgReputationRepo::new(pg)),
    };

    info!("StorageManager initialized with 17 repositories (9 LMDB + 8 PostgreSQL)");
    Ok(manager)
}
