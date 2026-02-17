//! Seed Data - Initial game templates for LMDB
//!
//! Populates the template store with starter content for all game systems.
//! This provides a baseline for testing and development.

use super::lmdb_templates::LmdbTemplateStore;
use crate::proto::tower::economy::*;
use crate::proto::tower::entities::*;
use crate::proto::tower::game::{SemanticTags as ProtoSemanticTags, TagPair};
use crate::proto::tower::quests::*;
use tracing::info;

/// Seed all template databases with initial data
pub fn seed_all(store: &LmdbTemplateStore) -> Result<(), Box<dyn std::error::Error>> {
    let mut total = 0;
    total += seed_monsters(store)?;
    total += seed_items(store)?;
    total += seed_abilities(store)?;
    total += seed_recipes(store)?;
    total += seed_loot_tables(store)?;
    total += seed_quests(store)?;
    total += seed_factions(store)?;

    info!("Seeded {} total templates", total);
    Ok(())
}

/// Seed monster templates
fn seed_monsters(store: &LmdbTemplateStore) -> Result<usize, Box<dyn std::error::Error>> {
    let monsters = vec![
        // === Plains (Floors 1-100) ===
        MonsterTemplate {
            id: "goblin_scout".into(),
            name: "Goblin Scout".into(),
            monster_type: MonsterType::Normal as i32,
            tier: 1,
            base_health: 80.0,
            base_damage: 12.0,
            base_defense: 3.0,
            base_speed: 3.5,
            ai_behavior: AiBehavior::Aggressive as i32,
            aggro_range: 10.0,
            leash_range: 30.0,
            loot_table_id: "loot_goblin".into(),
            gold_min: 5,
            gold_max: 15,
            model_id: "mdl_goblin_scout".into(),
            scale: 0.9,
            semantic_tags: Some(tags(&[("plains", 0.7), ("melee", 0.8), ("weak", 0.6)])),
            ..Default::default()
        },
        MonsterTemplate {
            id: "wild_boar".into(),
            name: "Wild Boar".into(),
            monster_type: MonsterType::Normal as i32,
            tier: 1,
            base_health: 120.0,
            base_damage: 18.0,
            base_defense: 8.0,
            base_speed: 2.5,
            ai_behavior: AiBehavior::Defensive as i32,
            aggro_range: 8.0,
            leash_range: 25.0,
            loot_table_id: "loot_beast".into(),
            gold_min: 3,
            gold_max: 10,
            model_id: "mdl_wild_boar".into(),
            scale: 1.2,
            semantic_tags: Some(tags(&[("plains", 0.8), ("beast", 0.9), ("heavy", 0.5)])),
            ..Default::default()
        },
        MonsterTemplate {
            id: "plains_golem".into(),
            name: "Plains Golem".into(),
            monster_type: MonsterType::Elite as i32,
            tier: 2,
            base_health: 500.0,
            base_damage: 35.0,
            base_defense: 20.0,
            base_speed: 1.5,
            ai_behavior: AiBehavior::Patrol as i32,
            aggro_range: 12.0,
            leash_range: 40.0,
            loot_table_id: "loot_golem".into(),
            gold_min: 30,
            gold_max: 80,
            model_id: "mdl_plains_golem".into(),
            scale: 2.5,
            semantic_tags: Some(tags(&[("earth", 0.9), ("heavy", 0.9), ("stone", 0.7)])),
            ..Default::default()
        },
        // === Forest (Floors 101-200) ===
        MonsterTemplate {
            id: "forest_spider".into(),
            name: "Forest Spider".into(),
            monster_type: MonsterType::Normal as i32,
            tier: 2,
            base_health: 100.0,
            base_damage: 20.0,
            base_defense: 5.0,
            base_speed: 4.0,
            ai_behavior: AiBehavior::Aggressive as i32,
            aggro_range: 6.0,
            leash_range: 20.0,
            loot_table_id: "loot_spider".into(),
            gold_min: 8,
            gold_max: 25,
            model_id: "mdl_forest_spider".into(),
            scale: 1.0,
            semantic_tags: Some(tags(&[("forest", 0.8), ("poison", 0.7), ("stealth", 0.6)])),
            ..Default::default()
        },
        MonsterTemplate {
            id: "treant_guardian".into(),
            name: "Treant Guardian".into(),
            monster_type: MonsterType::Elite as i32,
            tier: 3,
            base_health: 800.0,
            base_damage: 40.0,
            base_defense: 25.0,
            base_speed: 1.0,
            ai_behavior: AiBehavior::Defensive as i32,
            aggro_range: 15.0,
            leash_range: 20.0,
            loot_table_id: "loot_treant".into(),
            gold_min: 50,
            gold_max: 120,
            model_id: "mdl_treant".into(),
            scale: 3.0,
            semantic_tags: Some(tags(&[("forest", 0.9), ("nature", 0.9), ("defense", 0.8)])),
            ..Default::default()
        },
        // === Desert (Floors 201-300) ===
        MonsterTemplate {
            id: "sand_scorpion".into(),
            name: "Sand Scorpion".into(),
            monster_type: MonsterType::Normal as i32,
            tier: 3,
            base_health: 150.0,
            base_damage: 30.0,
            base_defense: 12.0,
            base_speed: 3.0,
            ai_behavior: AiBehavior::Aggressive as i32,
            aggro_range: 8.0,
            leash_range: 25.0,
            loot_table_id: "loot_scorpion".into(),
            gold_min: 15,
            gold_max: 40,
            model_id: "mdl_sand_scorpion".into(),
            scale: 1.5,
            semantic_tags: Some(tags(&[("desert", 0.8), ("poison", 0.6), ("piercing", 0.7)])),
            ..Default::default()
        },
        // === Mountains (Floors 301-500) ===
        MonsterTemplate {
            id: "stone_drake".into(),
            name: "Stone Drake".into(),
            monster_type: MonsterType::Elite as i32,
            tier: 4,
            base_health: 1200.0,
            base_damage: 55.0,
            base_defense: 35.0,
            base_speed: 2.5,
            ai_behavior: AiBehavior::Aggressive as i32,
            aggro_range: 20.0,
            leash_range: 50.0,
            loot_table_id: "loot_drake".into(),
            gold_min: 80,
            gold_max: 200,
            model_id: "mdl_stone_drake".into(),
            scale: 4.0,
            semantic_tags: Some(tags(&[("mountain", 0.9), ("earth", 0.8), ("fire", 0.5)])),
            ..Default::default()
        },
        // === Volcano (Floors 701-900) ===
        MonsterTemplate {
            id: "magma_elemental".into(),
            name: "Magma Elemental".into(),
            monster_type: MonsterType::Normal as i32,
            tier: 5,
            base_health: 400.0,
            base_damage: 60.0,
            base_defense: 20.0,
            base_speed: 2.0,
            ai_behavior: AiBehavior::Aggressive as i32,
            aggro_range: 12.0,
            leash_range: 35.0,
            loot_table_id: "loot_elemental".into(),
            gold_min: 60,
            gold_max: 150,
            model_id: "mdl_magma_elemental".into(),
            scale: 2.0,
            semantic_tags: Some(tags(&[
                ("fire", 0.95),
                ("volcano", 0.8),
                ("aggressive", 0.9),
            ])),
            ..Default::default()
        },
        // === Void (Floors 901+) ===
        MonsterTemplate {
            id: "void_wraith".into(),
            name: "Void Wraith".into(),
            monster_type: MonsterType::Elite as i32,
            tier: 6,
            base_health: 2000.0,
            base_damage: 80.0,
            base_defense: 30.0,
            base_speed: 5.0,
            ai_behavior: AiBehavior::Aggressive as i32,
            aggro_range: 25.0,
            leash_range: 100.0,
            loot_table_id: "loot_void".into(),
            gold_min: 200,
            gold_max: 500,
            model_id: "mdl_void_wraith".into(),
            scale: 2.5,
            semantic_tags: Some(tags(&[("void", 0.95), ("corruption", 0.9), ("chaos", 0.8)])),
            ..Default::default()
        },
        // === World Boss ===
        MonsterTemplate {
            id: "tower_guardian_alpha".into(),
            name: "Tower Guardian Alpha".into(),
            monster_type: MonsterType::WorldBoss as i32,
            tier: 10,
            base_health: 50000.0,
            base_damage: 200.0,
            base_defense: 100.0,
            base_speed: 3.0,
            ai_behavior: AiBehavior::Boss as i32,
            aggro_range: 50.0,
            leash_range: 200.0,
            loot_table_id: "loot_world_boss".into(),
            gold_min: 5000,
            gold_max: 15000,
            model_id: "mdl_tower_guardian".into(),
            scale: 8.0,
            semantic_tags: Some(tags(&[
                ("corruption", 0.5),
                ("extreme", 1.0),
                ("endgame", 1.0),
            ])),
            ..Default::default()
        },
    ];

    let count = monsters.len();
    for m in &monsters {
        store.put_monster(m)?;
    }
    info!("Seeded {} monster templates", count);
    Ok(count)
}

/// Seed item templates
fn seed_items(store: &LmdbTemplateStore) -> Result<usize, Box<dyn std::error::Error>> {
    let items = vec![
        // === Weapons ===
        ItemTemplate {
            id: "iron_sword".into(),
            name: "Iron Sword".into(),
            description: "A reliable starter blade.".into(),
            item_type: ItemType::Sword as i32,
            rarity: Rarity::Common as i32,
            tier: 1,
            base_damage: 20.0,
            base_speed: 1.0,
            vendor_value: 50,
            max_stack: 1,
            required_mastery_domain: "SwordMastery".into(),
            semantic_tags: Some(tags(&[("melee", 0.9), ("slashing", 0.8), ("iron", 0.7)])),
            ..Default::default()
        },
        ItemTemplate {
            id: "flame_katana".into(),
            name: "Flame Katana".into(),
            description: "A katana imbued with eternal fire.".into(),
            item_type: ItemType::Sword as i32,
            rarity: Rarity::Rare as i32,
            tier: 4,
            base_damage: 65.0,
            base_speed: 1.2,
            vendor_value: 800,
            max_stack: 1,
            required_mastery_domain: "SwordMastery".into(),
            socket_count: 2,
            semantic_tags: Some(tags(&[("melee", 0.9), ("slashing", 0.8), ("fire", 0.85)])),
            ..Default::default()
        },
        ItemTemplate {
            id: "hunter_bow".into(),
            name: "Hunter's Bow".into(),
            description: "A dependable bow for any marksman.".into(),
            item_type: ItemType::Bow as i32,
            rarity: Rarity::Common as i32,
            tier: 1,
            base_damage: 18.0,
            base_speed: 0.8,
            vendor_value: 45,
            max_stack: 1,
            required_mastery_domain: "BowMastery".into(),
            semantic_tags: Some(tags(&[
                ("ranged", 0.9),
                ("piercing", 0.8),
                ("precision", 0.6),
            ])),
            ..Default::default()
        },
        ItemTemplate {
            id: "crystal_staff".into(),
            name: "Crystal Staff".into(),
            description: "Channels elemental energy with clarity.".into(),
            item_type: ItemType::Staff as i32,
            rarity: Rarity::Uncommon as i32,
            tier: 2,
            base_damage: 30.0,
            base_speed: 0.7,
            vendor_value: 200,
            max_stack: 1,
            required_mastery_domain: "StaffMastery".into(),
            socket_count: 1,
            semantic_tags: Some(tags(&[
                ("magic", 0.9),
                ("elemental", 0.7),
                ("crystal", 0.6),
            ])),
            ..Default::default()
        },
        // === Armor ===
        ItemTemplate {
            id: "leather_chest".into(),
            name: "Leather Chest Armor".into(),
            description: "Light protection for agile fighters.".into(),
            item_type: ItemType::Chest as i32,
            rarity: Rarity::Common as i32,
            tier: 1,
            base_defense: 10.0,
            vendor_value: 40,
            max_stack: 1,
            semantic_tags: Some(tags(&[("defense", 0.6), ("light", 0.8), ("leather", 0.9)])),
            ..Default::default()
        },
        ItemTemplate {
            id: "iron_plate".into(),
            name: "Iron Plate Armor".into(),
            description: "Heavy plate for frontline warriors.".into(),
            item_type: ItemType::Chest as i32,
            rarity: Rarity::Uncommon as i32,
            tier: 2,
            base_defense: 25.0,
            vendor_value: 150,
            max_stack: 1,
            semantic_tags: Some(tags(&[("defense", 0.9), ("heavy", 0.8), ("iron", 0.7)])),
            ..Default::default()
        },
        // === Consumables ===
        ItemTemplate {
            id: "health_potion_small".into(),
            name: "Small Health Potion".into(),
            description: "Restores 50 health.".into(),
            item_type: ItemType::Potion as i32,
            rarity: Rarity::Common as i32,
            tier: 1,
            vendor_value: 10,
            max_stack: 99,
            semantic_tags: Some(tags(&[("healing", 0.9), ("consumable", 0.8)])),
            ..Default::default()
        },
        ItemTemplate {
            id: "health_potion_large".into(),
            name: "Large Health Potion".into(),
            description: "Restores 200 health.".into(),
            item_type: ItemType::Potion as i32,
            rarity: Rarity::Uncommon as i32,
            tier: 3,
            vendor_value: 50,
            max_stack: 50,
            semantic_tags: Some(tags(&[("healing", 0.95), ("consumable", 0.8)])),
            ..Default::default()
        },
        // === Materials ===
        ItemTemplate {
            id: "iron_ore".into(),
            name: "Iron Ore".into(),
            description: "Raw iron, ready for smelting.".into(),
            item_type: ItemType::Ore as i32,
            rarity: Rarity::Common as i32,
            tier: 1,
            vendor_value: 5,
            max_stack: 999,
            semantic_tags: Some(tags(&[("mining", 0.9), ("iron", 0.9), ("material", 0.8)])),
            ..Default::default()
        },
        ItemTemplate {
            id: "herb_lifeleaf".into(),
            name: "Lifeleaf".into(),
            description: "A common herb with restorative properties.".into(),
            item_type: ItemType::Herb as i32,
            rarity: Rarity::Common as i32,
            tier: 1,
            vendor_value: 3,
            max_stack: 999,
            semantic_tags: Some(tags(&[
                ("herbalism", 0.9),
                ("healing", 0.7),
                ("nature", 0.6),
            ])),
            ..Default::default()
        },
        ItemTemplate {
            id: "fire_crystal".into(),
            name: "Fire Crystal".into(),
            description: "Crystallized flame essence from volcanic regions.".into(),
            item_type: ItemType::Essence as i32,
            rarity: Rarity::Rare as i32,
            tier: 4,
            vendor_value: 100,
            max_stack: 99,
            semantic_tags: Some(tags(&[("fire", 0.95), ("crystal", 0.8), ("material", 0.7)])),
            ..Default::default()
        },
        ItemTemplate {
            id: "void_shard".into(),
            name: "Void Shard".into(),
            description: "A fragment of corrupted reality from the Void.".into(),
            item_type: ItemType::Essence as i32,
            rarity: Rarity::Epic as i32,
            tier: 6,
            vendor_value: 500,
            max_stack: 50,
            semantic_tags: Some(tags(&[
                ("void", 0.95),
                ("corruption", 0.8),
                ("material", 0.7),
            ])),
            ..Default::default()
        },
        // === Gems ===
        ItemTemplate {
            id: "ruby_t1".into(),
            name: "Rough Ruby".into(),
            description: "A rough ruby gem. Provides minor strength.".into(),
            item_type: ItemType::Gem as i32,
            rarity: Rarity::Uncommon as i32,
            tier: 1,
            vendor_value: 30,
            max_stack: 50,
            semantic_tags: Some(tags(&[("fire", 0.6), ("strength", 0.7), ("gem", 0.9)])),
            ..Default::default()
        },
    ];

    let count = items.len();
    for i in &items {
        store.put_item(i)?;
    }
    info!("Seeded {} item templates", count);
    Ok(count)
}

/// Seed ability templates
fn seed_abilities(store: &LmdbTemplateStore) -> Result<usize, Box<dyn std::error::Error>> {
    let abilities = vec![
        AbilityTemplate {
            id: "slash_basic".into(),
            name: "Slash".into(),
            description: "A basic horizontal slash.".into(),
            ability_type: AbilityType::Active as i32,
            required_mastery_domain: "SwordMastery".into(),
            required_mastery_tier: MasteryTier::Novice as i32,
            cooldown_ms: 500,
            kinetic_cost: 10.0,
            target_type: TargetType::Cone as i32,
            range: 2.5,
            effects: vec![AbilityEffect {
                effect_type: "damage:physical".into(),
                value: 15.0,
                ..Default::default()
            }],
            animation_id: "anim_slash".into(),
            semantic_tags: Some(tags(&[("melee", 0.9), ("slashing", 0.8)])),
            ..Default::default()
        },
        AbilityTemplate {
            id: "heavy_cleave".into(),
            name: "Heavy Cleave".into(),
            description: "A powerful overhead strike with knockback.".into(),
            ability_type: AbilityType::Active as i32,
            required_mastery_domain: "AxeMastery".into(),
            required_mastery_tier: MasteryTier::Apprentice as i32,
            cooldown_ms: 2000,
            kinetic_cost: 25.0,
            target_type: TargetType::Cone as i32,
            range: 3.0,
            effects: vec![AbilityEffect {
                effect_type: "damage:physical".into(),
                value: 45.0,
                ..Default::default()
            }],
            animation_id: "anim_heavy_cleave".into(),
            semantic_tags: Some(tags(&[("melee", 0.9), ("heavy", 0.8), ("knockback", 0.7)])),
            ..Default::default()
        },
        AbilityTemplate {
            id: "precise_shot".into(),
            name: "Precise Shot".into(),
            description: "A carefully aimed arrow with bonus critical chance.".into(),
            ability_type: AbilityType::Active as i32,
            required_mastery_domain: "BowMastery".into(),
            required_mastery_tier: MasteryTier::Novice as i32,
            cooldown_ms: 1500,
            kinetic_cost: 15.0,
            target_type: TargetType::Single as i32,
            range: 30.0,
            effects: vec![AbilityEffect {
                effect_type: "damage:physical".into(),
                value: 25.0,
                ..Default::default()
            }],
            animation_id: "anim_precise_shot".into(),
            semantic_tags: Some(tags(&[
                ("ranged", 0.9),
                ("piercing", 0.8),
                ("precision", 0.9),
            ])),
            ..Default::default()
        },
        AbilityTemplate {
            id: "fireball".into(),
            name: "Fireball".into(),
            description: "Hurl a ball of fire that explodes on impact.".into(),
            ability_type: AbilityType::Active as i32,
            required_mastery_domain: "StaffMastery".into(),
            required_mastery_tier: MasteryTier::Apprentice as i32,
            cooldown_ms: 3000,
            thermal_cost: 30.0,
            target_type: TargetType::Aoe as i32,
            range: 20.0,
            aoe_radius: 5.0,
            effects: vec![AbilityEffect {
                effect_type: "damage:fire".into(),
                value: 60.0,
                ..Default::default()
            }],
            animation_id: "anim_fireball".into(),
            semantic_tags: Some(tags(&[("fire", 0.95), ("magic", 0.8), ("explosive", 0.7)])),
            ..Default::default()
        },
        AbilityTemplate {
            id: "parry_stance".into(),
            name: "Parry Stance".into(),
            description: "Enter a defensive stance, next incoming attack is parried.".into(),
            ability_type: AbilityType::Active as i32,
            required_mastery_domain: "ParryMastery".into(),
            required_mastery_tier: MasteryTier::Novice as i32,
            cooldown_ms: 4000,
            kinetic_cost: 20.0,
            target_type: TargetType::Self_ as i32,
            effects: vec![AbilityEffect {
                effect_type: "buff:parry".into(),
                value: 1.0,
                duration_ms: 2000,
                ..Default::default()
            }],
            animation_id: "anim_parry_stance".into(),
            semantic_tags: Some(tags(&[("defense", 0.9), ("parry", 0.95), ("timing", 0.8)])),
            ..Default::default()
        },
        AbilityTemplate {
            id: "dodge_roll".into(),
            name: "Dodge Roll".into(),
            description: "Roll to evade attacks with i-frames.".into(),
            ability_type: AbilityType::Active as i32,
            required_mastery_domain: "DodgeMastery".into(),
            required_mastery_tier: MasteryTier::Novice as i32,
            cooldown_ms: 1000,
            kinetic_cost: 15.0,
            target_type: TargetType::Self_ as i32,
            effects: vec![AbilityEffect {
                effect_type: "buff:invincible".into(),
                value: 1.0,
                duration_ms: 300,
                ..Default::default()
            }],
            animation_id: "anim_dodge_roll".into(),
            semantic_tags: Some(tags(&[
                ("defense", 0.7),
                ("dodge", 0.95),
                ("movement", 0.8),
            ])),
            ..Default::default()
        },
        // Passive ability
        AbilityTemplate {
            id: "sword_mastery_passive".into(),
            name: "Sword Proficiency".into(),
            description: "Increases sword damage by 5% per mastery tier.".into(),
            ability_type: AbilityType::Passive as i32,
            required_mastery_domain: "SwordMastery".into(),
            required_mastery_tier: MasteryTier::Apprentice as i32,
            effects: vec![AbilityEffect {
                effect_type: "stat:damage".into(),
                value: 0.05,
                scaling_stat: StatType::Damage as i32,
                scaling_ratio: 0.05,
                ..Default::default()
            }],
            semantic_tags: Some(tags(&[("melee", 0.7), ("slashing", 0.6), ("passive", 0.9)])),
            ..Default::default()
        },
    ];

    let count = abilities.len();
    for a in &abilities {
        store.put_ability(a)?;
    }
    info!("Seeded {} ability templates", count);
    Ok(count)
}

/// Seed crafting recipes
fn seed_recipes(store: &LmdbTemplateStore) -> Result<usize, Box<dyn std::error::Error>> {
    let recipes = vec![
        CraftingRecipe {
            id: "recipe_iron_sword".into(),
            name: "Iron Sword".into(),
            profession: "SmithingMastery".into(),
            required_tier: MasteryTier::Novice as i32,
            ingredients: vec![RecipeIngredient {
                item_template_id: "iron_ore".into(),
                quantity: 5,
            }],
            result_item_id: "iron_sword".into(),
            result_quantity: 1,
            craft_time_ms: 5000,
            base_success_rate: 1.0,
            learned_by_default: true,
            ..Default::default()
        },
        CraftingRecipe {
            id: "recipe_iron_plate".into(),
            name: "Iron Plate Armor".into(),
            profession: "SmithingMastery".into(),
            required_tier: MasteryTier::Apprentice as i32,
            ingredients: vec![RecipeIngredient {
                item_template_id: "iron_ore".into(),
                quantity: 10,
            }],
            result_item_id: "iron_plate".into(),
            result_quantity: 1,
            craft_time_ms: 10000,
            base_success_rate: 0.9,
            learned_by_default: true,
            ..Default::default()
        },
        CraftingRecipe {
            id: "recipe_health_potion_small".into(),
            name: "Small Health Potion".into(),
            profession: "AlchemyMastery".into(),
            required_tier: MasteryTier::Novice as i32,
            ingredients: vec![RecipeIngredient {
                item_template_id: "herb_lifeleaf".into(),
                quantity: 3,
            }],
            result_item_id: "health_potion_small".into(),
            result_quantity: 3,
            craft_time_ms: 3000,
            base_success_rate: 1.0,
            station: CraftingStation::AlchemyLab as i32,
            learned_by_default: true,
            ..Default::default()
        },
    ];

    let count = recipes.len();
    for r in &recipes {
        store.put_recipe(r)?;
    }
    info!("Seeded {} crafting recipes", count);
    Ok(count)
}

/// Seed loot tables
fn seed_loot_tables(store: &LmdbTemplateStore) -> Result<usize, Box<dyn std::error::Error>> {
    let tables = vec![
        LootTable {
            id: "loot_goblin".into(),
            entries: vec![
                LootEntry {
                    item_template_id: "iron_ore".into(),
                    drop_chance: 0.5,
                    min_quantity: 1,
                    max_quantity: 3,
                    ..Default::default()
                },
                LootEntry {
                    item_template_id: "health_potion_small".into(),
                    drop_chance: 0.2,
                    min_quantity: 1,
                    max_quantity: 1,
                    ..Default::default()
                },
            ],
        },
        LootTable {
            id: "loot_beast".into(),
            entries: vec![
                LootEntry {
                    item_template_id: "herb_lifeleaf".into(),
                    drop_chance: 0.6,
                    min_quantity: 1,
                    max_quantity: 3,
                    ..Default::default()
                },
                LootEntry {
                    item_template_id: "health_potion_small".into(),
                    drop_chance: 0.15,
                    min_quantity: 1,
                    max_quantity: 1,
                    ..Default::default()
                },
            ],
        },
        LootTable {
            id: "loot_golem".into(),
            entries: vec![LootEntry {
                item_template_id: "iron_ore".into(),
                drop_chance: 0.7,
                min_quantity: 3,
                max_quantity: 8,
                ..Default::default()
            }],
        },
        LootTable {
            id: "loot_spider".into(),
            entries: vec![LootEntry {
                item_template_id: "herb_lifeleaf".into(),
                drop_chance: 0.4,
                min_quantity: 1,
                max_quantity: 2,
                ..Default::default()
            }],
        },
        LootTable {
            id: "loot_treant".into(),
            entries: vec![LootEntry {
                item_template_id: "herb_lifeleaf".into(),
                drop_chance: 0.6,
                min_quantity: 2,
                max_quantity: 5,
                ..Default::default()
            }],
        },
        LootTable {
            id: "loot_scorpion".into(),
            entries: vec![LootEntry {
                item_template_id: "iron_ore".into(),
                drop_chance: 0.4,
                min_quantity: 1,
                max_quantity: 3,
                ..Default::default()
            }],
        },
        LootTable {
            id: "loot_drake".into(),
            entries: vec![LootEntry {
                item_template_id: "fire_crystal".into(),
                drop_chance: 0.4,
                min_quantity: 1,
                max_quantity: 3,
                ..Default::default()
            }],
        },
        LootTable {
            id: "loot_elemental".into(),
            entries: vec![LootEntry {
                item_template_id: "fire_crystal".into(),
                drop_chance: 0.3,
                min_quantity: 1,
                max_quantity: 2,
                ..Default::default()
            }],
        },
        LootTable {
            id: "loot_void".into(),
            entries: vec![LootEntry {
                item_template_id: "void_shard".into(),
                drop_chance: 0.25,
                min_quantity: 1,
                max_quantity: 1,
                ..Default::default()
            }],
        },
        LootTable {
            id: "loot_world_boss".into(),
            entries: vec![
                LootEntry {
                    item_template_id: "void_shard".into(),
                    drop_chance: 1.0,
                    min_quantity: 3,
                    max_quantity: 10,
                    ..Default::default()
                },
                LootEntry {
                    item_template_id: "fire_crystal".into(),
                    drop_chance: 0.5,
                    min_quantity: 2,
                    max_quantity: 5,
                    ..Default::default()
                },
            ],
        },
    ];

    let count = tables.len();
    for t in &tables {
        store.put_loot_table(t)?;
    }
    info!("Seeded {} loot tables", count);
    Ok(count)
}

/// Seed quest templates
fn seed_quests(store: &LmdbTemplateStore) -> Result<usize, Box<dyn std::error::Error>> {
    let quests_data = vec![
        QuestTemplate {
            id: "quest_first_blood".into(),
            name: "First Blood".into(),
            description: "Defeat your first monster on the plains.".into(),
            quest_type: QuestType::MainStory as i32,
            required_floor_min: 1,
            is_repeatable: false,
            objectives: vec![QuestObjective {
                objective_type: ObjectiveType::KillMonster as i32,
                target: "".into(),
                required_count: 1,
                description: "Kill any monster".into(),
            }],
            rewards: Some(QuestRewards {
                gold: 50,
                items: vec![QuestRewardItem {
                    item_template_id: "health_potion_small".into(),
                    quantity: 3,
                    ..Default::default()
                }],
                ..Default::default()
            }),
            ..Default::default()
        },
        QuestTemplate {
            id: "quest_goblin_menace".into(),
            name: "Goblin Menace".into(),
            description: "Clear the goblin scouts from the plains.".into(),
            quest_type: QuestType::MainStory as i32,
            required_floor_min: 1,
            prerequisite_quest_id: "quest_first_blood".into(),
            is_repeatable: false,
            objectives: vec![QuestObjective {
                objective_type: ObjectiveType::KillMonster as i32,
                target: "goblin_scout".into(),
                required_count: 5,
                description: "Defeat Goblin Scouts".into(),
            }],
            rewards: Some(QuestRewards {
                gold: 200,
                items: vec![QuestRewardItem {
                    item_template_id: "iron_sword".into(),
                    quantity: 1,
                    ..Default::default()
                }],
                ..Default::default()
            }),
            ..Default::default()
        },
        QuestTemplate {
            id: "quest_daily_hunt".into(),
            name: "Daily Hunt".into(),
            description: "Hunt monsters for the Tower Guard.".into(),
            quest_type: QuestType::Daily as i32,
            required_floor_min: 1,
            is_repeatable: true,
            reset_interval: "daily".into(),
            objectives: vec![QuestObjective {
                objective_type: ObjectiveType::KillMonster as i32,
                target: "".into(),
                required_count: 10,
                description: "Defeat any monsters".into(),
            }],
            rewards: Some(QuestRewards {
                gold: 100,
                reputation_amount: 50,
                ..Default::default()
            }),
            ..Default::default()
        },
    ];

    let count = quests_data.len();
    for q in &quests_data {
        store.put_quest(q)?;
    }
    info!("Seeded {} quest templates", count);
    Ok(count)
}

/// Seed faction templates
fn seed_factions(store: &LmdbTemplateStore) -> Result<usize, Box<dyn std::error::Error>> {
    let factions = vec![
        FactionTemplate {
            id: "tower_guard".into(),
            name: "Tower Guard".into(),
            description: "The protectors of the lower floors. They maintain order and safety."
                .into(),
            alignment: FactionAlignment::Order as i32,
            relations: vec![
                FactionRelation {
                    target_faction_id: "crimson_order".into(),
                    political: 60,
                    economic: 30,
                    military: 70,
                    cultural: 40,
                },
                FactionRelation {
                    target_faction_id: "merchant_guild".into(),
                    political: 40,
                    economic: 50,
                    military: 10,
                    cultural: 30,
                },
                FactionRelation {
                    target_faction_id: "void_seekers".into(),
                    political: -50,
                    economic: -20,
                    military: -40,
                    cultural: -30,
                },
            ],
            banner_icon: "icon_tower_guard".into(),
            color_primary: "#4488CC".into(),
            color_secondary: "#CCDDEE".into(),
            ..Default::default()
        },
        FactionTemplate {
            id: "void_seekers".into(),
            name: "Void Seekers".into(),
            description: "Researchers who study the corruption. Dangerous allies.".into(),
            alignment: FactionAlignment::Chaos as i32,
            relations: vec![
                FactionRelation {
                    target_faction_id: "tower_guard".into(),
                    political: -50,
                    economic: -20,
                    military: -40,
                    cultural: -30,
                },
                FactionRelation {
                    target_faction_id: "crimson_order".into(),
                    political: -70,
                    economic: -10,
                    military: -60,
                    cultural: 20,
                },
                FactionRelation {
                    target_faction_id: "merchant_guild".into(),
                    political: -10,
                    economic: 30,
                    military: -20,
                    cultural: 10,
                },
            ],
            banner_icon: "icon_void_seekers".into(),
            color_primary: "#8844AA".into(),
            color_secondary: "#CCAADD".into(),
            ..Default::default()
        },
        FactionTemplate {
            id: "merchant_guild".into(),
            name: "Merchant's Guild".into(),
            description: "The economic backbone of the Tower. Controls trade routes.".into(),
            alignment: FactionAlignment::Neutral as i32,
            relations: vec![
                FactionRelation {
                    target_faction_id: "tower_guard".into(),
                    political: 40,
                    economic: 50,
                    military: 10,
                    cultural: 30,
                },
                FactionRelation {
                    target_faction_id: "void_seekers".into(),
                    political: -10,
                    economic: 30,
                    military: -20,
                    cultural: 10,
                },
                FactionRelation {
                    target_faction_id: "crimson_order".into(),
                    political: 20,
                    economic: 60,
                    military: 0,
                    cultural: 20,
                },
            ],
            banner_icon: "icon_merchant_guild".into(),
            color_primary: "#CC8844".into(),
            color_secondary: "#EEDDAA".into(),
            ..Default::default()
        },
        FactionTemplate {
            id: "crimson_order".into(),
            name: "Crimson Order".into(),
            description: "Elite warriors sworn to push through the highest floors.".into(),
            alignment: FactionAlignment::Order as i32,
            relations: vec![
                FactionRelation {
                    target_faction_id: "tower_guard".into(),
                    political: 60,
                    economic: 30,
                    military: 70,
                    cultural: 40,
                },
                FactionRelation {
                    target_faction_id: "void_seekers".into(),
                    political: -70,
                    economic: -10,
                    military: -60,
                    cultural: 20,
                },
                FactionRelation {
                    target_faction_id: "merchant_guild".into(),
                    political: 20,
                    economic: 60,
                    military: 0,
                    cultural: 20,
                },
            ],
            banner_icon: "icon_crimson_order".into(),
            color_primary: "#CC4444".into(),
            color_secondary: "#EECCCC".into(),
            ..Default::default()
        },
    ];

    let count = factions.len();
    for f in &factions {
        store.put(store.factions, &f.id, f)?;
    }
    info!("Seeded {} faction templates", count);
    Ok(count)
}

/// Helper: create ProtoSemanticTags from a slice of (tag, weight) pairs
fn tags(pairs: &[(&str, f32)]) -> ProtoSemanticTags {
    ProtoSemanticTags {
        tags: pairs
            .iter()
            .map(|(tag, weight)| TagPair {
                tag: tag.to_string(),
                weight: *weight,
            })
            .collect(),
    }
}
