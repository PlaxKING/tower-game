//! Inventory and equipment system.
//!
//! Players have a fixed-size inventory grid and equipment slots.
//! Items are semantic-tagged for thematic consistency.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::combat::weapons::Weapon;
use crate::economy::ItemRarity;
use crate::loot::LootItem;

/// Inventory capacity tiers
const BASE_INVENTORY_SIZE: usize = 20;
const MAX_INVENTORY_SIZE: usize = 60;

/// Equipment slots
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipSlot {
    MainHand,
    OffHand,
    Head,
    Chest,
    Legs,
    Boots,
    Accessory1,
    Accessory2,
}

/// An equippable item with stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentItem {
    pub name: String,
    pub slot: EquipSlot,
    pub rarity: ItemRarity,
    pub defense: f32,
    pub health_bonus: f32,
    pub resource_bonus: ResourceBonus,
    pub semantic_tags: Vec<(String, f32)>,
}

/// Bonus resources from equipment
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceBonus {
    pub kinetic: f32,
    pub thermal: f32,
    pub semantic: f32,
}

/// Player inventory component
#[derive(Component, Debug)]
pub struct Inventory {
    pub items: Vec<InventorySlot>,
    pub capacity: usize,
}

/// A single inventory slot
#[derive(Debug, Clone)]
pub enum InventorySlot {
    Empty,
    Loot(LootItem),
    Equipment(EquipmentItem),
    WeaponItem(Weapon),
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            items: vec![InventorySlot::Empty; BASE_INVENTORY_SIZE],
            capacity: BASE_INVENTORY_SIZE,
        }
    }
}

impl Inventory {
    /// Try to add an item to the first empty slot
    pub fn add_loot(&mut self, item: LootItem) -> bool {
        for slot in &mut self.items {
            if matches!(slot, InventorySlot::Empty) {
                *slot = InventorySlot::Loot(item);
                return true;
            }
        }
        false // inventory full
    }

    /// Try to add equipment
    pub fn add_equipment(&mut self, item: EquipmentItem) -> bool {
        for slot in &mut self.items {
            if matches!(slot, InventorySlot::Empty) {
                *slot = InventorySlot::Equipment(item);
                return true;
            }
        }
        false
    }

    /// Try to add weapon
    pub fn add_weapon(&mut self, weapon: Weapon) -> bool {
        for slot in &mut self.items {
            if matches!(slot, InventorySlot::Empty) {
                *slot = InventorySlot::WeaponItem(weapon);
                return true;
            }
        }
        false
    }

    /// Remove item at index
    pub fn remove(&mut self, index: usize) -> InventorySlot {
        if index < self.items.len() {
            std::mem::replace(&mut self.items[index], InventorySlot::Empty)
        } else {
            InventorySlot::Empty
        }
    }

    /// Count non-empty slots
    pub fn used_slots(&self) -> usize {
        self.items
            .iter()
            .filter(|s| !matches!(s, InventorySlot::Empty))
            .count()
    }

    /// Expand inventory (from upgrades)
    pub fn expand(&mut self, additional: usize) {
        let new_cap = (self.capacity + additional).min(MAX_INVENTORY_SIZE);
        let to_add = new_cap - self.capacity;
        for _ in 0..to_add {
            self.items.push(InventorySlot::Empty);
        }
        self.capacity = new_cap;
    }
}

/// Currently equipped gear
#[derive(Component, Debug, Default)]
pub struct Equipment {
    pub main_hand: Option<Weapon>,
    pub off_hand: Option<EquipmentItem>,
    pub head: Option<EquipmentItem>,
    pub chest: Option<EquipmentItem>,
    pub legs: Option<EquipmentItem>,
    pub boots: Option<EquipmentItem>,
    pub accessory1: Option<EquipmentItem>,
    pub accessory2: Option<EquipmentItem>,
}

impl Equipment {
    /// Total defense from all equipped items
    pub fn total_defense(&self) -> f32 {
        let mut def = 0.0;
        for eq in [
            &self.off_hand,
            &self.head,
            &self.chest,
            &self.legs,
            &self.boots,
            &self.accessory1,
            &self.accessory2,
        ]
        .into_iter()
        .flatten()
        {
            def += eq.defense;
        }
        def
    }

    /// Total health bonus
    pub fn total_health_bonus(&self) -> f32 {
        let mut bonus = 0.0;
        for eq in [
            &self.off_hand,
            &self.head,
            &self.chest,
            &self.legs,
            &self.boots,
            &self.accessory1,
            &self.accessory2,
        ]
        .into_iter()
        .flatten()
        {
            bonus += eq.health_bonus;
        }
        bonus
    }

    /// Total resource bonuses
    pub fn total_resource_bonus(&self) -> ResourceBonus {
        let mut total = ResourceBonus::default();
        for eq in [
            &self.off_hand,
            &self.head,
            &self.chest,
            &self.legs,
            &self.boots,
            &self.accessory1,
            &self.accessory2,
        ]
        .into_iter()
        .flatten()
        {
            total.kinetic += eq.resource_bonus.kinetic;
            total.thermal += eq.resource_bonus.thermal;
            total.semantic += eq.resource_bonus.semantic;
        }
        total
    }
}

/// System: auto-pickup loot near player
pub fn auto_pickup_loot(
    mut commands: Commands,
    mut players: Query<(&Transform, &mut Inventory), With<crate::player::Player>>,
    loot_query: Query<(Entity, &Transform, &LootItem, &crate::loot::DroppedLoot)>,
) {
    for (player_tf, mut inventory) in &mut players {
        for (loot_entity, loot_tf, item, dropped) in &loot_query {
            let distance = player_tf.translation.distance(loot_tf.translation);
            if distance <= dropped.pickup_radius && inventory.add_loot(item.clone()) {
                commands.entity(loot_entity).despawn();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loot::LootCategory;

    #[test]
    fn test_inventory_default() {
        let inv = Inventory::default();
        assert_eq!(inv.capacity, BASE_INVENTORY_SIZE);
        assert_eq!(inv.used_slots(), 0);
    }

    #[test]
    fn test_inventory_add_and_remove() {
        let mut inv = Inventory::default();
        let item = LootItem {
            name: "Test Shard".into(),
            category: LootCategory::Currency,
            rarity: ItemRarity::Common,
            quantity: 10,
            semantic_tags: vec![],
        };

        assert!(inv.add_loot(item.clone()));
        assert_eq!(inv.used_slots(), 1);

        let removed = inv.remove(0);
        assert!(matches!(removed, InventorySlot::Loot(_)));
        assert_eq!(inv.used_slots(), 0);
    }

    #[test]
    fn test_inventory_full() {
        let mut inv = Inventory {
            items: vec![
                InventorySlot::Loot(LootItem {
                    name: "Fill".into(),
                    category: LootCategory::Material,
                    rarity: ItemRarity::Common,
                    quantity: 1,
                    semantic_tags: vec![],
                });
                3
            ],
            capacity: 3,
        };

        let extra = LootItem {
            name: "Extra".into(),
            category: LootCategory::Currency,
            rarity: ItemRarity::Common,
            quantity: 1,
            semantic_tags: vec![],
        };

        assert!(!inv.add_loot(extra), "Should fail when full");
    }

    #[test]
    fn test_inventory_expand() {
        let mut inv = Inventory::default();
        let old_cap = inv.capacity;
        inv.expand(10);
        assert_eq!(inv.capacity, old_cap + 10);
        assert_eq!(inv.items.len(), old_cap + 10);
    }

    #[test]
    fn test_equipment_defense() {
        let mut eq = Equipment::default();
        assert_eq!(eq.total_defense(), 0.0);

        eq.chest = Some(EquipmentItem {
            name: "Iron Plate".into(),
            slot: EquipSlot::Chest,
            rarity: ItemRarity::Common,
            defense: 20.0,
            health_bonus: 50.0,
            resource_bonus: ResourceBonus::default(),
            semantic_tags: vec![],
        });

        assert!((eq.total_defense() - 20.0).abs() < f32::EPSILON);
        assert!((eq.total_health_bonus() - 50.0).abs() < f32::EPSILON);
    }
}
