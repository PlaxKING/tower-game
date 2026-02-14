//! Loot generation system.
//!
//! Drops are determined by: monster semantic tags + floor biome + player luck.
//! Semantic similarity between monster and drop creates thematic consistency.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::economy::ItemRarity;
use crate::semantic::SemanticTags;

pub struct LootPlugin;

impl Plugin for LootPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<LootDropEvent>()
            .add_systems(Update, process_loot_drops);
    }
}

/// Loot drop event (fired when a monster dies or chest is opened)
#[derive(Event, Debug)]
pub struct LootDropEvent {
    pub position: Vec3,
    pub source_tags: SemanticTags,
    pub floor_level: u32,
    pub drop_hash: u64,
}

/// Loot categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LootCategory {
    CombatResource, // kinetic/thermal/semantic energy
    Material,       // crafting materials
    Consumable,     // potions, scrolls
    Equipment,      // weapons, armor (later phases)
    Currency,       // tower shards
    EchoFragment,   // rare currency from echoes
}

/// Generated loot item
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct LootItem {
    pub name: String,
    pub category: LootCategory,
    pub rarity: ItemRarity,
    pub quantity: u32,
    pub semantic_tags: Vec<(String, f32)>,
}

/// Loot table entry
#[derive(Debug, Clone)]
struct LootTableEntry {
    category: LootCategory,
    weight: f32,
    min_quantity: u32,
    max_quantity: u32,
    name_prefix: &'static str,
}

/// Generate loot from a drop event
pub fn generate_loot(
    source_tags: &SemanticTags,
    floor_level: u32,
    drop_hash: u64,
) -> Vec<LootItem> {
    let mut items = Vec::new();
    let mut hash = drop_hash;

    // Number of drops (1-4, scaling with floor level)
    let drop_count = 1 + ((hash % 3) as usize).min(3);
    hash = xorshift(hash);

    let table = build_loot_table(source_tags, floor_level);

    for _ in 0..drop_count {
        hash = xorshift(hash);
        if let Some(item) = roll_loot(&table, source_tags, floor_level, hash) {
            items.push(item);
        }
        hash = xorshift(hash);
    }

    items
}

fn build_loot_table(source_tags: &SemanticTags, floor_level: u32) -> Vec<LootTableEntry> {
    let mut table = vec![
        LootTableEntry {
            category: LootCategory::Currency,
            weight: 40.0,
            min_quantity: 5,
            max_quantity: 20u32.saturating_add(floor_level),
            name_prefix: "Tower Shards",
        },
        LootTableEntry {
            category: LootCategory::CombatResource,
            weight: 25.0,
            min_quantity: 1,
            max_quantity: 5,
            name_prefix: "Energy Crystal",
        },
        LootTableEntry {
            category: LootCategory::Material,
            weight: 20.0,
            min_quantity: 1,
            max_quantity: 3,
            name_prefix: "Essence",
        },
        LootTableEntry {
            category: LootCategory::Consumable,
            weight: 10.0,
            min_quantity: 1,
            max_quantity: 2,
            name_prefix: "Potion",
        },
    ];

    // Elemental monsters drop more resources
    if source_tags.get("fire") > 0.5 {
        table.push(LootTableEntry {
            category: LootCategory::CombatResource,
            weight: 15.0,
            min_quantity: 2,
            max_quantity: 8,
            name_prefix: "Thermal Core",
        });
    }
    if source_tags.get("void") > 0.5 || source_tags.get("corruption") > 0.7 {
        table.push(LootTableEntry {
            category: LootCategory::EchoFragment,
            weight: 5.0,
            min_quantity: 1,
            max_quantity: 3,
            name_prefix: "Echo Fragment",
        });
    }

    table
}

fn roll_loot(
    table: &[LootTableEntry],
    source_tags: &SemanticTags,
    floor_level: u32,
    hash: u64,
) -> Option<LootItem> {
    let total_weight: f32 = table.iter().map(|e| e.weight).sum();
    if total_weight <= 0.0 {
        return None;
    }

    let roll = (hash % 10000) as f32 / 10000.0 * total_weight;
    let mut accumulated = 0.0;

    for entry in table {
        accumulated += entry.weight;
        if roll <= accumulated {
            let hash2 = xorshift(hash);

            // Determine rarity
            let rarity = roll_rarity(floor_level, hash2);

            // Determine quantity
            let range = entry.max_quantity.saturating_sub(entry.min_quantity);
            let quantity = entry.min_quantity
                + if range > 0 {
                    hash2 as u32 % (range + 1)
                } else {
                    0
                };

            // Name from element + prefix
            let element_name = dominant_element_name(source_tags);
            let name = format!("{} {}", element_name, entry.name_prefix);

            // Copy relevant tags from source (thematic consistency)
            let item_tags: Vec<(String, f32)> = source_tags
                .tags
                .iter()
                .filter(|(_, v)| *v > 0.3)
                .map(|(k, v)| (k.clone(), v * 0.5))
                .collect();

            return Some(LootItem {
                name,
                category: entry.category,
                rarity,
                quantity,
                semantic_tags: item_tags,
            });
        }
    }

    None
}

fn roll_rarity(floor_level: u32, hash: u64) -> ItemRarity {
    let luck_bonus = (floor_level as f32 * 0.001).min(0.1);
    let roll = (hash % 10000) as f32 / 10000.0;

    if roll < 0.01 + luck_bonus * 0.1 {
        ItemRarity::Legendary
    } else if roll < 0.05 + luck_bonus * 0.5 {
        ItemRarity::Epic
    } else if roll < 0.15 + luck_bonus {
        ItemRarity::Rare
    } else if roll < 0.40 + luck_bonus {
        ItemRarity::Uncommon
    } else {
        ItemRarity::Common
    }
}

fn dominant_element_name(tags: &SemanticTags) -> &'static str {
    let elements = [
        ("fire", "Ember"),
        ("water", "Tide"),
        ("earth", "Stone"),
        ("wind", "Gale"),
        ("void", "Void"),
        ("corruption", "Shadow"),
    ];

    let mut best = ("", 0.0_f32);
    for (tag, _name) in &elements {
        let val = tags.get(tag);
        if val > best.1 {
            best = (tag, val);
        }
    }

    elements
        .iter()
        .find(|(tag, _)| *tag == best.0)
        .map(|(_, name)| *name)
        .unwrap_or("Tower")
}

fn xorshift(mut x: u64) -> u64 {
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

/// Marker for dropped loot on the ground
#[derive(Component, Debug)]
pub struct DroppedLoot {
    pub pickup_radius: f32,
    pub despawn_timer: f32,
}

fn process_loot_drops(mut commands: Commands, mut events: EventReader<LootDropEvent>) {
    for event in events.read() {
        let items = generate_loot(&event.source_tags, event.floor_level, event.drop_hash);

        for (i, item) in items.iter().enumerate() {
            let offset = Vec3::new(i as f32 * 0.5, 0.5, 0.0);
            commands.spawn((
                Transform::from_translation(event.position + offset),
                item.clone(),
                DroppedLoot {
                    pickup_radius: 2.0,
                    despawn_timer: 300.0, // 5 minutes
                },
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_loot_deterministic() {
        let tags = SemanticTags::new(vec![("fire", 0.8), ("corruption", 0.3)]);
        let loot_a = generate_loot(&tags, 10, 42);
        let loot_b = generate_loot(&tags, 10, 42);

        assert_eq!(loot_a.len(), loot_b.len());
        for (a, b) in loot_a.iter().zip(loot_b.iter()) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.quantity, b.quantity);
        }
    }

    #[test]
    fn test_loot_has_items() {
        let tags = SemanticTags::new(vec![("neutral", 0.5)]);
        let loot = generate_loot(&tags, 1, 12345);
        assert!(!loot.is_empty(), "Loot should not be empty");
    }

    #[test]
    fn test_fire_monster_drops_thermal() {
        let tags = SemanticTags::new(vec![("fire", 0.9)]);
        let loot = generate_loot(&tags, 50, 99999);

        let has_ember = loot.iter().any(|item| item.name.contains("Ember"));
        assert!(has_ember, "Fire monsters should drop fire-themed loot");
    }

    #[test]
    fn test_rarity_distribution() {
        let mut common_count = 0;
        let mut rare_plus_count = 0;

        for i in 0..1000 {
            let rarity = roll_rarity(1, i * 7 + 13);
            match rarity {
                ItemRarity::Common | ItemRarity::Uncommon => common_count += 1,
                _ => rare_plus_count += 1,
            }
        }

        assert!(
            common_count > rare_plus_count * 2,
            "Common should be much more frequent"
        );
    }

    #[test]
    fn test_xorshift_nonzero() {
        let mut val = 1u64;
        for _ in 0..100 {
            val = xorshift(val);
            assert_ne!(val, 0, "xorshift should not produce zero");
        }
    }
}
