//! Bevy Storage Plugin
//!
//! Integrates the StorageManager into Bevy ECS as a Resource,
//! providing all game systems access to repositories.

use bevy::prelude::*;
use std::sync::Arc;
use tracing::info;

use super::repository::StorageManager;

/// Bevy Resource wrapping the StorageManager
///
/// Accessible from any Bevy system via `Res<StorageResource>`.
///
/// ## Usage
/// ```rust,ignore
/// fn my_system(storage: Res<StorageResource>) {
///     let monsters = storage.manager.monsters.get_all().await;
/// }
/// ```
#[derive(Resource)]
pub struct StorageResource {
    pub manager: Arc<StorageManager>,
}

/// Configuration for the storage plugin
#[derive(Resource, Clone)]
pub struct StorageConfig {
    pub lmdb_path: String,
    pub lmdb_max_size: usize,
    pub postgres_url: String,
    pub pg_max_connections: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            lmdb_path: "data/templates".to_string(),
            lmdb_max_size: 500_000_000, // 500MB
            postgres_url: "postgres://postgres:postgres@localhost:5432/tower_game".to_string(),
            pg_max_connections: 10,
        }
    }
}

/// Bevy plugin that initializes the storage layer
///
/// Insert `StorageConfig` resource before adding this plugin,
/// or it will use defaults.
///
/// ## Example
/// ```rust,ignore
/// App::new()
///     .insert_resource(StorageConfig {
///         lmdb_path: "data/templates".to_string(),
///         postgres_url: "postgres://localhost/tower".to_string(),
///         ..Default::default()
///     })
///     .add_plugins(StoragePlugin)
///     .run();
/// ```
pub struct StoragePlugin;

impl Plugin for StoragePlugin {
    fn build(&self, app: &mut App) {
        // Ensure StorageConfig exists (use defaults if not provided)
        if !app.world().contains_resource::<StorageConfig>() {
            app.insert_resource(StorageConfig::default());
        }

        app.add_systems(Startup, init_storage_system);
    }
}

/// Startup system that initializes the storage layer
///
/// Spawns a blocking task to connect to PostgreSQL and initialize LMDB,
/// then inserts the StorageResource into the Bevy world.
fn init_storage_system(mut commands: Commands, config: Res<StorageConfig>) {
    let config = config.clone();

    info!("Initializing storage layer...");

    // Initialize LMDB synchronously (it's fast)
    let lmdb = match super::lmdb_templates::LmdbTemplateStore::new(
        &config.lmdb_path,
        config.lmdb_max_size,
    ) {
        Ok(store) => {
            info!("LMDB template store opened at: {}", config.lmdb_path);
            Arc::new(store)
        }
        Err(e) => {
            tracing::error!("Failed to open LMDB store: {}", e);
            return;
        }
    };

    // Seed template data
    if let Err(e) = super::seed_data::seed_all(&lmdb) {
        tracing::error!("Failed to seed LMDB data: {}", e);
        return;
    }
    info!("LMDB template data seeded");

    // Build LMDB repository adapters
    use super::lmdb_repo_adapter::*;
    let monsters = Box::new(LmdbMonsterRepo::new(lmdb.clone()));
    let items = Box::new(LmdbItemRepo::new(lmdb.clone()));
    let abilities = Box::new(LmdbAbilityRepo::new(lmdb.clone()));
    let recipes = Box::new(LmdbRecipeRepo::new(lmdb.clone()));
    let quests = Box::new(LmdbQuestRepo::new(lmdb.clone()));
    let factions = Box::new(LmdbFactionRepo::new(lmdb.clone()));
    let loot_tables = Box::new(LmdbLootTableRepo::new(lmdb.clone()));
    let item_sets = Box::new(LmdbItemSetRepo::new(lmdb.clone()));
    let gems = Box::new(LmdbGemRepo::new(lmdb));

    // PostgreSQL initialization is async - store config for deferred init
    // For now, use placeholder player repos that will be replaced
    // when PostgreSQL connects
    commands.insert_resource(PendingPostgresInit {
        url: config.postgres_url.clone(),
        max_connections: config.pg_max_connections,
    });

    // Store LMDB repos immediately (they're ready)
    commands.insert_resource(LmdbRepos {
        monsters,
        items,
        abilities,
        recipes,
        quests,
        factions,
        loot_tables,
        item_sets,
        gems,
    });

    info!("LMDB repositories ready, PostgreSQL initialization pending");
}

/// Pending PostgreSQL initialization
#[derive(Resource)]
#[allow(dead_code)]
struct PendingPostgresInit {
    url: String,
    max_connections: u32,
}

/// LMDB repositories (available immediately at startup)
#[derive(Resource)]
pub struct LmdbRepos {
    pub monsters: Box<dyn super::repository::MonsterTemplateRepo>,
    pub items: Box<dyn super::repository::ItemTemplateRepo>,
    pub abilities: Box<dyn super::repository::AbilityTemplateRepo>,
    pub recipes: Box<dyn super::repository::RecipeRepo>,
    pub quests: Box<dyn super::repository::QuestTemplateRepo>,
    pub factions: Box<dyn super::repository::FactionTemplateRepo>,
    pub loot_tables: Box<dyn super::repository::LootTableRepo>,
    pub item_sets: Box<dyn super::repository::ItemSetRepo>,
    pub gems: Box<dyn super::repository::GemTemplateRepo>,
}
