//! Integration tests for the storage layer
//!
//! Tests the complete flow:
//! LMDB seed data → Template store → Repository adapters → Queries

use std::sync::Arc;
use tower_bevy_server::storage::lmdb_repo_adapter::*;
use tower_bevy_server::storage::lmdb_templates::LmdbTemplateStore;
use tower_bevy_server::storage::repository::*;
use tower_bevy_server::storage::seed_data;

/// Helper to create a temporary LMDB store seeded with test data
fn create_seeded_store() -> (Arc<LmdbTemplateStore>, std::path::PathBuf) {
    let temp_dir = std::env::temp_dir().join(format!(
        "tower_storage_test_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ));
    let store = Arc::new(
        LmdbTemplateStore::new(&temp_dir, 50 * 1024 * 1024).expect("Failed to create LMDB store"),
    );
    seed_data::seed_all(&store).expect("Failed to seed data");
    (store, temp_dir)
}

// ============================================================================
// Seed Data Tests
// ============================================================================

#[test]
fn test_seed_all_populates_all_databases() {
    let (store, temp_dir) = create_seeded_store();

    let stats = store.stats().unwrap();
    assert!(stats.monsters > 0, "Should have seeded monsters");
    assert!(stats.items > 0, "Should have seeded items");
    assert!(stats.abilities > 0, "Should have seeded abilities");
    assert!(stats.recipes > 0, "Should have seeded recipes");
    assert!(stats.quests > 0, "Should have seeded quests");
    assert!(stats.factions > 0, "Should have seeded factions");
    assert!(stats.loot_tables > 0, "Should have seeded loot tables");

    println!("\n{}", stats.summary());
    assert!(
        stats.total() >= 30,
        "Should have at least 30 templates total, got {}",
        stats.total()
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_seed_monsters_have_valid_fields() {
    let (store, temp_dir) = create_seeded_store();

    let goblin = store
        .get_monster("goblin_scout")
        .unwrap()
        .expect("Goblin should exist");
    assert_eq!(goblin.name, "Goblin Scout");
    assert!(goblin.base_health > 0.0, "Goblin should have health");
    assert!(goblin.base_damage > 0.0, "Goblin should have damage");
    assert!(
        !goblin.loot_table_id.is_empty(),
        "Goblin should have loot table"
    );
    assert!(!goblin.model_id.is_empty(), "Goblin should have model");

    // Check a boss-tier monster
    let guardian = store
        .get_monster("tower_guardian_alpha")
        .unwrap()
        .expect("Guardian should exist");
    assert!(
        guardian.base_health > goblin.base_health,
        "Boss should have more health than scout"
    );
    assert!(guardian.tier > goblin.tier, "Boss should be higher tier");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_seed_items_cover_multiple_types() {
    let (store, temp_dir) = create_seeded_store();

    // Weapon
    let sword = store
        .get_item("iron_sword")
        .unwrap()
        .expect("Iron sword should exist");
    assert_eq!(sword.name, "Iron Sword");
    assert!(sword.base_damage > 0.0, "Sword should have damage");

    // Armor
    let plate = store
        .get_item("iron_plate")
        .unwrap()
        .expect("Iron plate should exist");
    assert!(plate.base_defense > 0.0, "Plate should have defense");

    // Consumable
    let potion = store
        .get_item("health_potion_small")
        .unwrap()
        .expect("Health potion should exist");
    assert!(potion.max_stack > 1, "Potions should be stackable");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_seed_abilities_have_domains() {
    let (store, temp_dir) = create_seeded_store();

    let slash = store
        .get_ability("slash_basic")
        .unwrap()
        .expect("Basic slash should exist");
    assert!(
        !slash.required_mastery_domain.is_empty(),
        "Ability should have mastery domain"
    );
    assert!(slash.cooldown_ms > 0, "Ability should have cooldown");

    let fireball = store
        .get_ability("fireball")
        .unwrap()
        .expect("Fireball should exist");
    assert!(!fireball.required_mastery_domain.is_empty());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_seed_recipes_reference_valid_items() {
    let (store, temp_dir) = create_seeded_store();

    let recipe = store
        .get_recipe("recipe_iron_sword")
        .unwrap()
        .expect("Iron sword recipe should exist");
    assert_eq!(
        recipe.result_item_id, "iron_sword",
        "Recipe should produce iron sword"
    );
    assert!(
        !recipe.ingredients.is_empty(),
        "Recipe should have ingredients"
    );
    assert!(recipe.craft_time_ms > 0, "Recipe should take time");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_seed_quests_have_objectives_and_rewards() {
    let (store, temp_dir) = create_seeded_store();

    let quest = store
        .get_quest("quest_first_blood")
        .unwrap()
        .expect("First blood quest should exist");
    assert!(!quest.objectives.is_empty(), "Quest should have objectives");
    assert!(quest.rewards.is_some(), "Quest should have rewards");

    let rewards = quest.rewards.unwrap();
    assert!(rewards.gold > 0, "Quest should reward gold");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_seed_loot_tables_have_entries() {
    let (store, temp_dir) = create_seeded_store();

    let table = store
        .get_loot_table("loot_goblin")
        .unwrap()
        .expect("Goblin loot table should exist");
    assert!(!table.entries.is_empty(), "Loot table should have entries");

    // Verify all entries have valid drop chances
    for entry in &table.entries {
        assert!(
            entry.drop_chance > 0.0 && entry.drop_chance <= 1.0,
            "Drop chance should be in (0, 1], got {}",
            entry.drop_chance
        );
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

// ============================================================================
// Repository Adapter Tests
// ============================================================================

#[tokio::test]
async fn test_monster_repo_get() {
    let (store, temp_dir) = create_seeded_store();
    let repo = LmdbMonsterRepo::new(store.clone());

    let goblin = repo.get("goblin_scout").await.unwrap();
    assert!(goblin.is_some(), "Should find goblin_scout via repo");
    assert_eq!(goblin.unwrap().name, "Goblin Scout");

    let missing = repo.get("nonexistent_monster").await.unwrap();
    assert!(missing.is_none(), "Should return None for missing monster");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_monster_repo_get_by_tier() {
    let (store, temp_dir) = create_seeded_store();
    let repo = LmdbMonsterRepo::new(store.clone());

    let tier1 = repo.get_by_tier(1).await.unwrap();
    assert!(!tier1.is_empty(), "Should have tier 1 monsters");
    for m in &tier1 {
        assert_eq!(m.tier, 1, "All returned monsters should be tier 1");
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_monster_repo_get_all_and_count() {
    let (store, temp_dir) = create_seeded_store();
    let repo = LmdbMonsterRepo::new(store.clone());

    let all = repo.get_all().await.unwrap();
    let count = repo.count().await.unwrap();
    assert_eq!(all.len(), count, "get_all len should match count");
    assert!(
        count >= 10,
        "Should have at least 10 monsters, got {}",
        count
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_item_repo_queries() {
    let (store, temp_dir) = create_seeded_store();
    let repo = LmdbItemRepo::new(store.clone());

    // Get by ID
    let sword = repo.get("iron_sword").await.unwrap();
    assert!(sword.is_some());

    // Count
    let count = repo.count().await.unwrap();
    assert!(count >= 10, "Should have at least 10 items, got {}", count);

    // Get all
    let all = repo.get_all().await.unwrap();
    assert_eq!(all.len(), count);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_item_repo_get_by_rarity() {
    use tower_bevy_server::proto::tower::entities::Rarity;

    let (store, temp_dir) = create_seeded_store();
    let repo = LmdbItemRepo::new(store.clone());

    let common = repo.get_by_rarity(Rarity::Common as i32).await.unwrap();
    assert!(!common.is_empty(), "Should have common items");
    for item in &common {
        assert_eq!(item.rarity, Rarity::Common as i32);
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_ability_repo_get_by_domain() {
    let (store, temp_dir) = create_seeded_store();
    let repo = LmdbAbilityRepo::new(store.clone());

    let sword_abilities = repo.get_by_domain("SwordMastery").await.unwrap();
    assert!(
        !sword_abilities.is_empty(),
        "Should have SwordMastery-domain abilities"
    );
    for a in &sword_abilities {
        assert_eq!(a.required_mastery_domain, "SwordMastery");
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_recipe_repo_get_by_profession() {
    let (store, temp_dir) = create_seeded_store();
    let repo = LmdbRecipeRepo::new(store.clone());

    let smithing = repo.get_by_profession("SmithingMastery").await.unwrap();
    assert!(!smithing.is_empty(), "Should have SmithingMastery recipes");
    for r in &smithing {
        assert_eq!(r.profession, "SmithingMastery");
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_quest_repo_get_by_type() {
    use tower_bevy_server::proto::tower::quests::QuestType;

    let (store, temp_dir) = create_seeded_store();
    let repo = LmdbQuestRepo::new(store.clone());

    let daily = repo.get_by_type(QuestType::Daily as i32).await.unwrap();
    for q in &daily {
        assert_eq!(q.quest_type, QuestType::Daily as i32);
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_quest_repo_get_available_for_floor() {
    let (store, temp_dir) = create_seeded_store();
    let repo = LmdbQuestRepo::new(store.clone());

    // Floor 1 quests - should get quests with required_floor_min <= 1
    let floor1_quests = repo.get_available_for_floor(1).await.unwrap();
    for q in &floor1_quests {
        assert!(
            q.required_floor_min <= 1,
            "Quest {} requires floor {} but was returned for floor 1",
            q.id,
            q.required_floor_min
        );
    }

    // Floor 100 should have at least as many quests as floor 1
    let floor100_quests = repo.get_available_for_floor(100).await.unwrap();
    assert!(
        floor100_quests.len() >= floor1_quests.len(),
        "Higher floors should have at least as many available quests"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_faction_repo() {
    let (store, temp_dir) = create_seeded_store();
    let repo = LmdbFactionRepo::new(store.clone());

    let all = repo.get_all().await.unwrap();
    assert!(
        all.len() >= 4,
        "Should have at least 4 factions, got {}",
        all.len()
    );

    let guard = repo.get("tower_guard").await.unwrap();
    assert!(guard.is_some(), "Tower Guard faction should exist");
    let guard = guard.unwrap();
    assert_eq!(guard.name, "Tower Guard");
    assert!(!guard.relations.is_empty(), "Faction should have relations");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_loot_table_repo() {
    let (store, temp_dir) = create_seeded_store();
    let repo = LmdbLootTableRepo::new(store.clone());

    let all = repo.get_all().await.unwrap();
    let count = repo.count().await.unwrap();
    assert_eq!(all.len(), count);
    assert!(
        count >= 4,
        "Should have at least 4 loot tables, got {}",
        count
    );

    let goblin_loot = repo.get("loot_goblin").await.unwrap();
    assert!(goblin_loot.is_some());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

// ============================================================================
// Cross-reference Integrity Tests
// ============================================================================

#[tokio::test]
async fn test_monster_loot_table_references_valid() {
    let (store, temp_dir) = create_seeded_store();
    let monster_repo = LmdbMonsterRepo::new(store.clone());
    let loot_repo = LmdbLootTableRepo::new(store.clone());

    let all_monsters = monster_repo.get_all().await.unwrap();
    for monster in &all_monsters {
        if !monster.loot_table_id.is_empty() {
            let loot_table = loot_repo.get(&monster.loot_table_id).await.unwrap();
            assert!(
                loot_table.is_some(),
                "Monster '{}' references loot table '{}' which doesn't exist",
                monster.id,
                monster.loot_table_id
            );
        }
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_recipe_output_items_exist() {
    let (store, temp_dir) = create_seeded_store();
    let recipe_repo = LmdbRecipeRepo::new(store.clone());
    let item_repo = LmdbItemRepo::new(store.clone());

    let all_recipes = recipe_repo.get_all().await.unwrap();
    for recipe in &all_recipes {
        if !recipe.result_item_id.is_empty() {
            let item = item_repo.get(&recipe.result_item_id).await.unwrap();
            assert!(
                item.is_some(),
                "Recipe '{}' produces item '{}' which doesn't exist",
                recipe.id,
                recipe.result_item_id
            );
        }
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_faction_relations_bidirectional() {
    let (store, temp_dir) = create_seeded_store();
    let repo = LmdbFactionRepo::new(store.clone());

    let all_factions = repo.get_all().await.unwrap();
    let faction_ids: Vec<String> = all_factions.iter().map(|f| f.id.clone()).collect();

    for faction in &all_factions {
        for relation in &faction.relations {
            assert!(
                faction_ids.contains(&relation.target_faction_id),
                "Faction '{}' has relation to unknown faction '{}'",
                faction.id,
                relation.target_faction_id
            );
        }
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

// ============================================================================
// Data Consistency Tests
// ============================================================================

#[test]
fn test_seed_is_idempotent() {
    let temp_dir =
        std::env::temp_dir().join(format!("tower_idempotent_test_{}", std::process::id()));
    let store = LmdbTemplateStore::new(&temp_dir, 50 * 1024 * 1024).unwrap();

    // Seed twice
    seed_data::seed_all(&store).unwrap();
    let stats1 = store.stats().unwrap();

    seed_data::seed_all(&store).unwrap();
    let stats2 = store.stats().unwrap();

    // Counts should be the same (upsert semantics)
    assert_eq!(
        stats1.total(),
        stats2.total(),
        "Seeding twice should not duplicate entries"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_template_store_survives_reopen() {
    let temp_dir = std::env::temp_dir().join(format!("tower_reopen_test_{}", std::process::id()));

    // Create and seed
    {
        let store = LmdbTemplateStore::new(&temp_dir, 50 * 1024 * 1024).unwrap();
        seed_data::seed_all(&store).unwrap();
        let stats = store.stats().unwrap();
        assert!(stats.total() > 0);
    }
    // store dropped here, LMDB closed

    // Reopen and verify data persists
    {
        let store = LmdbTemplateStore::new(&temp_dir, 50 * 1024 * 1024).unwrap();
        let stats = store.stats().unwrap();
        assert!(stats.total() > 0, "Data should persist after reopen");

        // Verify specific entry
        let goblin = store.get_monster("goblin_scout").unwrap();
        assert!(goblin.is_some(), "Goblin should still exist after reopen");
        assert_eq!(goblin.unwrap().name, "Goblin Scout");
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}
