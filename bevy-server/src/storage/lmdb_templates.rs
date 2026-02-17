//! LMDB Template Store - Persistent storage for static game data
//!
//! Stores all game templates (monsters, items, abilities, etc.) in LMDB
//! for fast read access. Templates are loaded once and rarely change.
//!
//! ## Performance
//! - Read: ~300-400µs (memory-mapped I/O)
//! - Write: ~500-600µs (ACID transaction)
//! - Memory: Zero-copy reads from disk

use heed::{Database, Env, EnvOpenOptions};
use prost::Message;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

/// LMDB-backed template store for game data
pub struct LmdbTemplateStore {
    env: Arc<Env>,
    /// Database for monster templates
    pub monsters: Database<heed::types::Str, heed::types::Bytes>,
    /// Database for item templates
    pub items: Database<heed::types::Str, heed::types::Bytes>,
    /// Database for ability templates
    pub abilities: Database<heed::types::Str, heed::types::Bytes>,
    /// Database for crafting recipes
    pub recipes: Database<heed::types::Str, heed::types::Bytes>,
    /// Database for quest templates
    pub quests: Database<heed::types::Str, heed::types::Bytes>,
    /// Database for faction templates
    pub factions: Database<heed::types::Str, heed::types::Bytes>,
    /// Database for loot tables
    pub loot_tables: Database<heed::types::Str, heed::types::Bytes>,
    /// Database for item sets
    pub item_sets: Database<heed::types::Str, heed::types::Bytes>,
    /// Database for gem/rune templates
    pub gems: Database<heed::types::Str, heed::types::Bytes>,
    /// Database for NPC templates
    pub npcs: Database<heed::types::Str, heed::types::Bytes>,
    /// Database for achievement templates
    pub achievements: Database<heed::types::Str, heed::types::Bytes>,
    /// Database for season passes
    pub seasons: Database<heed::types::Str, heed::types::Bytes>,
}

/// Error type for LMDB template operations
#[derive(Debug, thiserror::Error)]
pub enum TemplateStoreError {
    #[error("LMDB error: {0}")]
    Heed(#[from] heed::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Template not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl LmdbTemplateStore {
    /// Open or create the template store
    ///
    /// # Arguments
    /// * `path` - Directory for LMDB data files
    /// * `max_size` - Maximum database size in bytes (recommended: 500MB for production)
    pub fn new<P: AsRef<Path>>(path: P, max_size: usize) -> Result<Self, TemplateStoreError> {
        std::fs::create_dir_all(&path)?;

        let env = unsafe {
            EnvOpenOptions::new()
                .map_size(max_size)
                .max_dbs(12) // 12 databases for all template types
                .open(path)?
        };

        let mut wtxn = env.write_txn()?;

        let monsters = env.create_database::<heed::types::Str, heed::types::Bytes>(&mut wtxn, Some("monsters"))?;
        let items = env.create_database::<heed::types::Str, heed::types::Bytes>(&mut wtxn, Some("items"))?;
        let abilities = env.create_database::<heed::types::Str, heed::types::Bytes>(&mut wtxn, Some("abilities"))?;
        let recipes = env.create_database::<heed::types::Str, heed::types::Bytes>(&mut wtxn, Some("recipes"))?;
        let quests = env.create_database::<heed::types::Str, heed::types::Bytes>(&mut wtxn, Some("quests"))?;
        let factions = env.create_database::<heed::types::Str, heed::types::Bytes>(&mut wtxn, Some("factions"))?;
        let loot_tables = env.create_database::<heed::types::Str, heed::types::Bytes>(&mut wtxn, Some("loot_tables"))?;
        let item_sets = env.create_database::<heed::types::Str, heed::types::Bytes>(&mut wtxn, Some("item_sets"))?;
        let gems = env.create_database::<heed::types::Str, heed::types::Bytes>(&mut wtxn, Some("gems"))?;
        let npcs = env.create_database::<heed::types::Str, heed::types::Bytes>(&mut wtxn, Some("npcs"))?;
        let achievements = env.create_database::<heed::types::Str, heed::types::Bytes>(&mut wtxn, Some("achievements"))?;
        let seasons = env.create_database::<heed::types::Str, heed::types::Bytes>(&mut wtxn, Some("seasons"))?;

        wtxn.commit()?;

        info!("LMDB template store initialized with 12 databases ({}MB)", max_size / (1024 * 1024));

        Ok(Self {
            env: Arc::new(env),
            monsters,
            items,
            abilities,
            recipes,
            quests,
            factions,
            loot_tables,
            item_sets,
            gems,
            npcs,
            achievements,
            seasons,
        })
    }

    // ========================================================================
    // Generic CRUD operations
    // ========================================================================

    /// Store a Protobuf message in the specified database
    pub fn put<M: Message>(&self, db: Database<heed::types::Str, heed::types::Bytes>, key: &str, value: &M) -> Result<(), TemplateStoreError> {
        let bytes = value.encode_to_vec();
        let mut wtxn = self.env.write_txn()?;
        db.put(&mut wtxn, key, &bytes)?;
        wtxn.commit()?;
        debug!("Stored template: {}", key);
        Ok(())
    }

    /// Get a Protobuf message from the specified database
    pub fn get<M: Message + Default>(&self, db: Database<heed::types::Str, heed::types::Bytes>, key: &str) -> Result<Option<M>, TemplateStoreError> {
        let rtxn = self.env.read_txn()?;
        match db.get(&rtxn, key)? {
            Some(bytes) => {
                let msg = M::decode(bytes).map_err(|e| TemplateStoreError::Serialization(e.to_string()))?;
                Ok(Some(msg))
            }
            None => Ok(None),
        }
    }

    /// Get all entries from a database
    pub fn get_all<M: Message + Default>(&self, db: Database<heed::types::Str, heed::types::Bytes>) -> Result<Vec<M>, TemplateStoreError> {
        let rtxn = self.env.read_txn()?;
        let mut results = Vec::new();
        let iter = db.iter(&rtxn)?;
        for item in iter {
            let (_, bytes) = item?;
            let msg = M::decode(bytes).map_err(|e| TemplateStoreError::Serialization(e.to_string()))?;
            results.push(msg);
        }
        Ok(results)
    }

    /// Delete a template by key
    pub fn delete(&self, db: Database<heed::types::Str, heed::types::Bytes>, key: &str) -> Result<bool, TemplateStoreError> {
        let mut wtxn = self.env.write_txn()?;
        let deleted = db.delete(&mut wtxn, key)?;
        wtxn.commit()?;
        Ok(deleted)
    }

    /// Count entries in a database
    pub fn count(&self, db: Database<heed::types::Str, heed::types::Bytes>) -> Result<usize, TemplateStoreError> {
        let rtxn = self.env.read_txn()?;
        let count = db.len(&rtxn)? as usize;
        Ok(count)
    }

    // ========================================================================
    // Bulk operations
    // ========================================================================

    /// Bulk insert templates (single transaction for performance)
    pub fn bulk_put<M: Message>(&self, db: Database<heed::types::Str, heed::types::Bytes>, items: &[(&str, &M)]) -> Result<usize, TemplateStoreError> {
        let mut wtxn = self.env.write_txn()?;
        let mut count = 0;
        for (key, value) in items {
            let bytes = value.encode_to_vec();
            db.put(&mut wtxn, key, &bytes)?;
            count += 1;
        }
        wtxn.commit()?;
        info!("Bulk inserted {} templates", count);
        Ok(count)
    }

    /// Clear all data in a database
    pub fn clear(&self, db: Database<heed::types::Str, heed::types::Bytes>) -> Result<(), TemplateStoreError> {
        let mut wtxn = self.env.write_txn()?;
        db.clear(&mut wtxn)?;
        wtxn.commit()?;
        Ok(())
    }

    // ========================================================================
    // Convenience methods for each template type
    // ========================================================================

    /// Store a monster template
    pub fn put_monster(&self, template: &crate::proto::tower::entities::MonsterTemplate) -> Result<(), TemplateStoreError> {
        self.put(self.monsters, &template.id, template)
    }

    /// Get a monster template by ID
    pub fn get_monster(&self, id: &str) -> Result<Option<crate::proto::tower::entities::MonsterTemplate>, TemplateStoreError> {
        self.get(self.monsters, id)
    }

    /// Store an item template
    pub fn put_item(&self, template: &crate::proto::tower::entities::ItemTemplate) -> Result<(), TemplateStoreError> {
        self.put(self.items, &template.id, template)
    }

    /// Get an item template by ID
    pub fn get_item(&self, id: &str) -> Result<Option<crate::proto::tower::entities::ItemTemplate>, TemplateStoreError> {
        self.get(self.items, id)
    }

    /// Store an ability template
    pub fn put_ability(&self, template: &crate::proto::tower::entities::AbilityTemplate) -> Result<(), TemplateStoreError> {
        self.put(self.abilities, &template.id, template)
    }

    /// Get an ability template by ID
    pub fn get_ability(&self, id: &str) -> Result<Option<crate::proto::tower::entities::AbilityTemplate>, TemplateStoreError> {
        self.get(self.abilities, id)
    }

    /// Store a crafting recipe
    pub fn put_recipe(&self, recipe: &crate::proto::tower::economy::CraftingRecipe) -> Result<(), TemplateStoreError> {
        self.put(self.recipes, &recipe.id, recipe)
    }

    /// Get a crafting recipe by ID
    pub fn get_recipe(&self, id: &str) -> Result<Option<crate::proto::tower::economy::CraftingRecipe>, TemplateStoreError> {
        self.get(self.recipes, id)
    }

    /// Store a quest template
    pub fn put_quest(&self, template: &crate::proto::tower::quests::QuestTemplate) -> Result<(), TemplateStoreError> {
        self.put(self.quests, &template.id, template)
    }

    /// Get a quest template by ID
    pub fn get_quest(&self, id: &str) -> Result<Option<crate::proto::tower::quests::QuestTemplate>, TemplateStoreError> {
        self.get(self.quests, id)
    }

    /// Store a loot table
    pub fn put_loot_table(&self, table: &crate::proto::tower::entities::LootTable) -> Result<(), TemplateStoreError> {
        self.put(self.loot_tables, &table.id, table)
    }

    /// Get a loot table by ID
    pub fn get_loot_table(&self, id: &str) -> Result<Option<crate::proto::tower::entities::LootTable>, TemplateStoreError> {
        self.get(self.loot_tables, id)
    }

    /// Get store statistics
    pub fn stats(&self) -> Result<TemplateStoreStats, TemplateStoreError> {
        Ok(TemplateStoreStats {
            monsters: self.count(self.monsters)?,
            items: self.count(self.items)?,
            abilities: self.count(self.abilities)?,
            recipes: self.count(self.recipes)?,
            quests: self.count(self.quests)?,
            factions: self.count(self.factions)?,
            loot_tables: self.count(self.loot_tables)?,
            item_sets: self.count(self.item_sets)?,
            gems: self.count(self.gems)?,
            npcs: self.count(self.npcs)?,
            achievements: self.count(self.achievements)?,
            seasons: self.count(self.seasons)?,
        })
    }
}

/// Statistics for the template store
#[derive(Debug, Clone)]
pub struct TemplateStoreStats {
    pub monsters: usize,
    pub items: usize,
    pub abilities: usize,
    pub recipes: usize,
    pub quests: usize,
    pub factions: usize,
    pub loot_tables: usize,
    pub item_sets: usize,
    pub gems: usize,
    pub npcs: usize,
    pub achievements: usize,
    pub seasons: usize,
}

impl TemplateStoreStats {
    /// Total number of templates across all databases
    pub fn total(&self) -> usize {
        self.monsters + self.items + self.abilities + self.recipes
            + self.quests + self.factions + self.loot_tables + self.item_sets
            + self.gems + self.npcs + self.achievements + self.seasons
    }

    /// Human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "Template Store ({} total):\n\
             - Monsters:     {}\n\
             - Items:        {}\n\
             - Abilities:    {}\n\
             - Recipes:      {}\n\
             - Quests:       {}\n\
             - Factions:     {}\n\
             - Loot Tables:  {}\n\
             - Item Sets:    {}\n\
             - Gems:         {}\n\
             - NPCs:         {}\n\
             - Achievements: {}\n\
             - Seasons:      {}",
            self.total(),
            self.monsters, self.items, self.abilities, self.recipes,
            self.quests, self.factions, self.loot_tables, self.item_sets,
            self.gems, self.npcs, self.achievements, self.seasons,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_store_creation() {
        let temp_dir = std::env::temp_dir().join(format!("tower_tmpl_test_{}", std::process::id()));
        let store = LmdbTemplateStore::new(&temp_dir, 10 * 1024 * 1024).unwrap();

        let stats = store.stats().unwrap();
        assert_eq!(stats.total(), 0);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_monster_template_crud() {
        let temp_dir = std::env::temp_dir().join(format!("tower_monster_test_{}", std::process::id()));
        let store = LmdbTemplateStore::new(&temp_dir, 10 * 1024 * 1024).unwrap();

        // Create monster template
        let goblin = crate::proto::tower::entities::MonsterTemplate {
            id: "goblin_scout".to_string(),
            name: "Goblin Scout".to_string(),
            monster_type: crate::proto::tower::entities::MonsterType::Normal as i32,
            tier: 1,
            base_health: 100.0,
            base_damage: 15.0,
            base_defense: 5.0,
            base_speed: 3.0,
            ai_behavior: crate::proto::tower::entities::AiBehavior::Aggressive as i32,
            aggro_range: 10.0,
            leash_range: 30.0,
            loot_table_id: "goblin_loot".to_string(),
            gold_min: 5,
            gold_max: 15,
            model_id: "mdl_goblin_scout".to_string(),
            scale: 1.0,
            ..Default::default()
        };

        // Put
        store.put_monster(&goblin).unwrap();

        // Get
        let retrieved = store.get_monster("goblin_scout").unwrap().unwrap();
        assert_eq!(retrieved.name, "Goblin Scout");
        assert_eq!(retrieved.base_health, 100.0);

        // Count
        assert_eq!(store.count(store.monsters).unwrap(), 1);

        // Delete
        store.delete(store.monsters, "goblin_scout").unwrap();
        assert!(store.get_monster("goblin_scout").unwrap().is_none());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_item_template_crud() {
        let temp_dir = std::env::temp_dir().join(format!("tower_item_test_{}", std::process::id()));
        let store = LmdbTemplateStore::new(&temp_dir, 10 * 1024 * 1024).unwrap();

        let sword = crate::proto::tower::entities::ItemTemplate {
            id: "iron_sword".to_string(),
            name: "Iron Sword".to_string(),
            description: "A basic iron sword".to_string(),
            item_type: crate::proto::tower::entities::ItemType::Sword as i32,
            rarity: crate::proto::tower::entities::Rarity::Common as i32,
            tier: 1,
            base_damage: 25.0,
            vendor_value: 50,
            max_stack: 1,
            ..Default::default()
        };

        store.put_item(&sword).unwrap();

        let retrieved = store.get_item("iron_sword").unwrap().unwrap();
        assert_eq!(retrieved.name, "Iron Sword");
        assert_eq!(retrieved.base_damage, 25.0);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_bulk_operations() {
        let temp_dir = std::env::temp_dir().join(format!("tower_bulk_test_{}", std::process::id()));
        let store = LmdbTemplateStore::new(&temp_dir, 10 * 1024 * 1024).unwrap();

        // Create 100 item templates
        let mut items = Vec::new();
        for i in 0..100 {
            items.push(crate::proto::tower::entities::ItemTemplate {
                id: format!("item_{:03}", i),
                name: format!("Item #{}", i),
                tier: (i % 10 + 1) as u32,
                ..Default::default()
            });
        }

        let refs: Vec<(&str, &crate::proto::tower::entities::ItemTemplate)> =
            items.iter().map(|item| (item.id.as_str(), item)).collect();
        let count = store.bulk_put(store.items, &refs).unwrap();
        assert_eq!(count, 100);

        // Verify count
        assert_eq!(store.count(store.items).unwrap(), 100);

        // Verify retrieval
        let item_50 = store.get_item("item_050").unwrap().unwrap();
        assert_eq!(item_50.name, "Item #50");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_store_stats() {
        let temp_dir = std::env::temp_dir().join(format!("tower_stats_test_{}", std::process::id()));
        let store = LmdbTemplateStore::new(&temp_dir, 10 * 1024 * 1024).unwrap();

        // Add some templates
        let monster = crate::proto::tower::entities::MonsterTemplate {
            id: "test_mob".to_string(),
            name: "Test Mob".to_string(),
            ..Default::default()
        };
        store.put_monster(&monster).unwrap();

        let item = crate::proto::tower::entities::ItemTemplate {
            id: "test_item".to_string(),
            name: "Test Item".to_string(),
            ..Default::default()
        };
        store.put_item(&item).unwrap();

        let stats = store.stats().unwrap();
        assert_eq!(stats.monsters, 1);
        assert_eq!(stats.items, 1);
        assert_eq!(stats.total(), 2);

        println!("\n{}", stats.summary());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
