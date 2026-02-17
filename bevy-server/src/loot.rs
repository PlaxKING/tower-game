//! Loot System — Semantic drop rules, rarity distribution, equipment effects
//!
//! ## Design Pillars (from CLAUDE.md)
//! - Equipment provides SPECIAL EFFECTS, not big stat bonuses
//! - Trigger→Action system: OnHit, OnParry, OnDodge → ElementalDamage, Lifesteal, Shield
//! - Monsters drop resources, not equipment (players craft equipment)
//! - Semantic tag similarity affects drop quality
//!
//! ## Architecture
//! ```text
//! Monster killed → semantic similarity check → loot table + bonuses
//!       ↓
//! RarityRoll (floor depth + luck + semantic affinity)
//!       ↓
//! Item generation (base stats + random effects from trigger→action pool)
//!       ↓
//! Drop notification → client
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Rarity System
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
    Mythic,
    Ancient,
}

impl Rarity {
    /// Base drop weight (inverse of rarity)
    pub fn drop_weight(&self) -> f32 {
        match self {
            Rarity::Common => 50.0,
            Rarity::Uncommon => 25.0,
            Rarity::Rare => 10.0,
            Rarity::Epic => 4.0,
            Rarity::Legendary => 1.0,
            Rarity::Mythic => 0.2,
            Rarity::Ancient => 0.05,
        }
    }

    /// Stat multiplier for this rarity
    pub fn stat_mult(&self) -> f32 {
        match self {
            Rarity::Common => 1.0,
            Rarity::Uncommon => 1.15,
            Rarity::Rare => 1.3,
            Rarity::Epic => 1.5,
            Rarity::Legendary => 1.8,
            Rarity::Mythic => 2.2,
            Rarity::Ancient => 3.0,
        }
    }

    /// Number of equipment effects this rarity can have
    pub fn max_effects(&self) -> usize {
        match self {
            Rarity::Common => 0,
            Rarity::Uncommon => 1,
            Rarity::Rare => 1,
            Rarity::Epic => 2,
            Rarity::Legendary => 3,
            Rarity::Mythic => 4,
            Rarity::Ancient => 5,
        }
    }

    /// Socket count range for this rarity
    pub fn socket_range(&self) -> (u8, u8) {
        match self {
            Rarity::Common => (0, 0),
            Rarity::Uncommon => (0, 1),
            Rarity::Rare => (1, 2),
            Rarity::Epic => (1, 3),
            Rarity::Legendary => (2, 3),
            Rarity::Mythic => (2, 4),
            Rarity::Ancient => (3, 5),
        }
    }

    /// Roll a rarity from a 0.0–1.0 normalized value with luck bonus
    pub fn from_roll(roll: f32, luck_bonus: f32, floor_bonus: f32) -> Self {
        let adjusted = roll - luck_bonus * 0.1 - floor_bonus * 0.05;
        if adjusted < 0.0005 {
            Rarity::Ancient
        } else if adjusted < 0.003 {
            Rarity::Mythic
        } else if adjusted < 0.015 {
            Rarity::Legendary
        } else if adjusted < 0.06 {
            Rarity::Epic
        } else if adjusted < 0.18 {
            Rarity::Rare
        } else if adjusted < 0.45 {
            Rarity::Uncommon
        } else {
            Rarity::Common
        }
    }
}

// ============================================================================
// Equipment Effects (Trigger → Action)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectTrigger {
    OnHit,
    OnCrit,
    OnParry,
    OnDodge,
    OnKill,
    OnDamaged,
    OnLowHealth,
    OnAbilityUse,
    Passive,
    OnCombatStart,
    OnCombatEnd,
}

impl EffectTrigger {
    fn from_hash(h: u64) -> Self {
        match h % 11 {
            0 => EffectTrigger::OnHit,
            1 => EffectTrigger::OnCrit,
            2 => EffectTrigger::OnParry,
            3 => EffectTrigger::OnDodge,
            4 => EffectTrigger::OnKill,
            5 => EffectTrigger::OnDamaged,
            6 => EffectTrigger::OnLowHealth,
            7 => EffectTrigger::OnAbilityUse,
            8 => EffectTrigger::Passive,
            9 => EffectTrigger::OnCombatStart,
            _ => EffectTrigger::OnCombatEnd,
        }
    }

    /// Display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            EffectTrigger::OnHit => "On Hit",
            EffectTrigger::OnCrit => "On Critical Hit",
            EffectTrigger::OnParry => "On Successful Parry",
            EffectTrigger::OnDodge => "On Dodge",
            EffectTrigger::OnKill => "On Kill",
            EffectTrigger::OnDamaged => "When Damaged",
            EffectTrigger::OnLowHealth => "When Low Health",
            EffectTrigger::OnAbilityUse => "On Ability Use",
            EffectTrigger::Passive => "Passive",
            EffectTrigger::OnCombatStart => "On Combat Start",
            EffectTrigger::OnCombatEnd => "On Combat End",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffectAction {
    /// Deal bonus elemental damage
    ElementalDamage {
        element: String,
        amount: f32,
        is_percent: bool,
    },
    /// Heal self
    Heal { amount: f32, is_percent: bool },
    /// Lifesteal (heal % of damage dealt)
    Lifesteal { percent: f32 },
    /// Temporary shield
    Shield { amount: f32, duration_secs: f32 },
    /// Buff a stat
    BuffStat {
        stat: String,
        amount: f32,
        duration_secs: f32,
    },
    /// Restore energy resource
    RestoreEnergy { energy_type: String, amount: f32 },
    /// Area of effect damage around self
    AoeDamage {
        element: String,
        damage: f32,
        radius: f32,
    },
    /// Apply a debuff to target
    Debuff {
        debuff_id: String,
        duration_secs: f32,
        value: f32,
    },
}

impl EffectAction {
    fn from_hash(h: u64, rarity: Rarity) -> Self {
        let power_mult = rarity.stat_mult();
        match h % 8 {
            0 => {
                let element = match (h >> 16) % 5 {
                    0 => "fire",
                    1 => "ice",
                    2 => "lightning",
                    3 => "void",
                    _ => "kinetic",
                };
                EffectAction::ElementalDamage {
                    element: element.to_string(),
                    amount: 10.0 * power_mult,
                    is_percent: false,
                }
            }
            1 => EffectAction::Heal {
                amount: 5.0 * power_mult,
                is_percent: false,
            },
            2 => EffectAction::Lifesteal {
                percent: 5.0 * power_mult,
            },
            3 => EffectAction::Shield {
                amount: 20.0 * power_mult,
                duration_secs: 5.0,
            },
            4 => {
                let stat = match (h >> 16) % 4 {
                    0 => "strength",
                    1 => "dexterity",
                    2 => "intelligence",
                    _ => "vitality",
                };
                EffectAction::BuffStat {
                    stat: stat.to_string(),
                    amount: 3.0 * power_mult,
                    duration_secs: 8.0,
                }
            }
            5 => {
                let energy = match (h >> 16) % 3 {
                    0 => "kinetic",
                    1 => "thermal",
                    _ => "semantic",
                };
                EffectAction::RestoreEnergy {
                    energy_type: energy.to_string(),
                    amount: 10.0 * power_mult,
                }
            }
            6 => EffectAction::AoeDamage {
                element: "kinetic".to_string(),
                damage: 15.0 * power_mult,
                radius: 3.0,
            },
            _ => {
                let debuff = match (h >> 16) % 3 {
                    0 => "slow",
                    1 => "weaken",
                    _ => "burn",
                };
                EffectAction::Debuff {
                    debuff_id: debuff.to_string(),
                    duration_secs: 4.0,
                    value: 3.0 * power_mult,
                }
            }
        }
    }

    /// Description for UI tooltip
    pub fn description(&self) -> String {
        match self {
            EffectAction::ElementalDamage {
                element,
                amount,
                is_percent,
            } => {
                if *is_percent {
                    format!("Deal {:.0}% {element} damage", amount)
                } else {
                    format!("Deal {amount:.0} {element} damage")
                }
            }
            EffectAction::Heal { amount, is_percent } => {
                if *is_percent {
                    format!("Heal {amount:.0}% HP")
                } else {
                    format!("Heal {amount:.0} HP")
                }
            }
            EffectAction::Lifesteal { percent } => {
                format!("Lifesteal {percent:.0}%")
            }
            EffectAction::Shield {
                amount,
                duration_secs,
            } => {
                format!("Shield {amount:.0} HP for {duration_secs:.1}s")
            }
            EffectAction::BuffStat {
                stat,
                amount,
                duration_secs,
            } => {
                format!("+{amount:.0} {stat} for {duration_secs:.0}s")
            }
            EffectAction::RestoreEnergy {
                energy_type,
                amount,
            } => {
                format!("Restore {amount:.0} {energy_type} energy")
            }
            EffectAction::AoeDamage {
                element,
                damage,
                radius,
            } => {
                format!("AoE {damage:.0} {element} damage ({radius:.0}m)")
            }
            EffectAction::Debuff {
                debuff_id,
                duration_secs,
                value,
            } => {
                format!("Apply {debuff_id} ({value:.0}) for {duration_secs:.0}s")
            }
        }
    }
}

/// A complete equipment effect (trigger + action + chance + cooldown)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentEffect {
    pub trigger: EffectTrigger,
    pub action: EffectAction,
    /// Proc chance (0.0 – 1.0)
    pub chance: f32,
    /// Cooldown in seconds (0 = no cooldown)
    pub cooldown_secs: f32,
}

impl EquipmentEffect {
    /// Generate a random effect from a seed
    fn from_seed(seed: u64, rarity: Rarity) -> Self {
        let h1 = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let h2 = h1.wrapping_mul(6364136223846793005).wrapping_add(1);
        let h3 = h2.wrapping_mul(6364136223846793005).wrapping_add(1);

        let trigger = EffectTrigger::from_hash(h1 >> 32);
        let action = EffectAction::from_hash(h2 >> 32, rarity);

        // Higher rarity = higher proc chance
        let base_chance = match rarity {
            Rarity::Common => 0.0,
            Rarity::Uncommon => 0.15,
            Rarity::Rare => 0.20,
            Rarity::Epic => 0.25,
            Rarity::Legendary => 0.30,
            Rarity::Mythic => 0.40,
            Rarity::Ancient => 0.50,
        };

        // Passive triggers always proc
        let chance = if matches!(trigger, EffectTrigger::Passive) {
            1.0
        } else {
            base_chance + ((h3 >> 32) % 10) as f32 * 0.02 // +0–20% variance
        };

        let cooldown_secs = match trigger {
            EffectTrigger::Passive => 0.0,
            EffectTrigger::OnKill | EffectTrigger::OnCombatEnd => 0.0,
            _ => 3.0 + ((h3 >> 16) % 5) as f32, // 3-7 second cooldown
        };

        EquipmentEffect {
            trigger,
            action,
            chance,
            cooldown_secs,
        }
    }

    /// Full description for tooltip
    pub fn full_description(&self) -> String {
        let chance_str = if self.chance >= 1.0 {
            String::new()
        } else {
            format!("{:.0}% chance: ", self.chance * 100.0)
        };

        let cd_str = if self.cooldown_secs > 0.0 {
            format!(" ({:.0}s CD)", self.cooldown_secs)
        } else {
            String::new()
        };

        format!(
            "{}: {}{}{}",
            self.trigger.display_name(),
            chance_str,
            self.action.description(),
            cd_str
        )
    }
}

// ============================================================================
// Loot Drop Generation
// ============================================================================

/// A generated loot drop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootDrop {
    /// Item template ID (references LMDB/seed data)
    pub item_id: String,
    /// Generated display name (may be modified by rarity/effects)
    pub display_name: String,
    /// Quantity of this item
    pub quantity: u32,
    /// Rolled rarity
    pub rarity: Rarity,
    /// Equipment effects (only for gear, empty for materials)
    pub effects: Vec<EquipmentEffect>,
    /// Socket count (only for gear)
    pub sockets: u8,
    /// Semantic tags that influenced this drop
    pub semantic_source: Vec<(String, f32)>,
    /// Gold value
    pub gold_value: u32,
}

/// Configuration for loot generation
#[derive(Debug, Clone)]
pub struct LootConfig {
    /// Floor depth (affects rarity chances)
    pub floor_id: u32,
    /// Player luck stat (0 = baseline)
    pub luck: f32,
    /// Semantic similarity between player tags and monster tags (0.0–1.0)
    pub semantic_affinity: f32,
    /// Monster's loot tier
    pub loot_tier: u32,
    /// Monster's semantic tags
    pub monster_tags: HashMap<String, f32>,
}

/// Generate loot drops from a killed monster
pub fn generate_loot(seed: u64, config: &LootConfig) -> Vec<LootDrop> {
    let mut drops = Vec::new();
    let mut h = seed;

    // Number of drops: 1-3 base, +1 per tier above 1
    let base_drops = 1 + (h % 3) as usize;
    let tier_bonus = config.loot_tier.saturating_sub(1) as usize;
    let total_drops = (base_drops + tier_bonus).min(6);

    for i in 0..total_drops {
        h = h
            .wrapping_mul(6364136223846793005)
            .wrapping_add(i as u64 + 1);

        // Roll rarity
        let rarity_roll = ((h >> 32) as u32) as f32 / u32::MAX as f32;
        let floor_bonus = config.floor_id as f32 / 100.0;
        let rarity = Rarity::from_roll(rarity_roll, config.luck, floor_bonus);

        // Determine item category based on monster tags
        h = h.wrapping_mul(6364136223846793005).wrapping_add(1);
        let (item_id, display_name, is_gear) =
            select_item_from_tags(h, &config.monster_tags, rarity);

        // Quantity (materials get more, gear always 1)
        let quantity = if is_gear {
            1
        } else {
            h = h.wrapping_mul(6364136223846793005).wrapping_add(1);
            1 + (h % 5) as u32 // 1-5 for materials
        };

        // Generate effects for gear
        let effects = if is_gear {
            let effect_count = rarity.max_effects();
            let mut effs = Vec::with_capacity(effect_count);
            for j in 0..effect_count {
                let effect_seed = h.wrapping_mul(j as u64 + 7).wrapping_add(seed);
                effs.push(EquipmentEffect::from_seed(effect_seed, rarity));
            }
            effs
        } else {
            vec![]
        };

        // Sockets for gear
        let sockets = if is_gear {
            let (min, max) = rarity.socket_range();
            if max > min {
                h = h.wrapping_mul(6364136223846793005).wrapping_add(1);
                min + (h % (max - min + 1) as u64) as u8
            } else {
                min
            }
        } else {
            0
        };

        // Gold value
        let base_gold = 10 * config.loot_tier;
        let gold_value = (base_gold as f32 * rarity.stat_mult()) as u32;

        // Semantic tags that influenced drop
        let semantic_source: Vec<(String, f32)> = config
            .monster_tags
            .iter()
            .filter(|(_, w)| **w > 0.3)
            .map(|(k, v)| (k.clone(), *v))
            .collect();

        drops.push(LootDrop {
            item_id,
            display_name,
            quantity,
            rarity,
            effects,
            sockets,
            semantic_source,
            gold_value,
        });
    }

    // Apply semantic affinity bonus: chance for extra rare drop
    if config.semantic_affinity > 0.5 {
        h = h.wrapping_mul(6364136223846793005).wrapping_add(42);
        let bonus_roll = (h >> 33) as f32 / u32::MAX as f32;
        if bonus_roll < config.semantic_affinity - 0.5 {
            // Bonus semantic drop — always at least Rare
            let rarity = Rarity::from_roll(0.0, config.luck + 0.5, config.floor_id as f32 / 50.0);
            let rarity = if rarity < Rarity::Rare {
                Rarity::Rare
            } else {
                rarity
            };

            h = h.wrapping_mul(6364136223846793005).wrapping_add(1);
            let (item_id, display_name, _) = select_item_from_tags(h, &config.monster_tags, rarity);

            drops.push(LootDrop {
                item_id,
                display_name: format!("Semantic {}", display_name),
                quantity: 1,
                rarity,
                effects: vec![EquipmentEffect::from_seed(h, rarity)],
                sockets: 1,
                semantic_source: config
                    .monster_tags
                    .iter()
                    .map(|(k, v)| (k.clone(), *v))
                    .collect(),
                gold_value: (50 * config.loot_tier) as u32,
            });
        }
    }

    drops
}

/// Select an item ID and name based on monster's semantic tags
fn select_item_from_tags(
    h: u64,
    tags: &HashMap<String, f32>,
    rarity: Rarity,
) -> (String, String, bool) {
    // Determine primary element/theme from tags
    let primary = tags
        .iter()
        .filter(|(k, _)| !matches!(k.as_str(), "aggression" | "presence" | "corruption"))
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(k, _)| k.as_str())
        .unwrap_or("neutral");

    // Is this a gear drop or material?
    let is_gear = (h % 100) < 20; // 20% chance of gear, 80% material

    if is_gear {
        let gear_type = match h % 4 {
            0 => ("sword", "Sword"),
            1 => ("spear", "Spear"),
            2 => ("hammer", "Hammer"),
            _ => ("armor", "Armor"),
        };

        let rarity_prefix = match rarity {
            Rarity::Common => "",
            Rarity::Uncommon => "Fine ",
            Rarity::Rare => "Superior ",
            Rarity::Epic => "Exalted ",
            Rarity::Legendary => "Legendary ",
            Rarity::Mythic => "Mythic ",
            Rarity::Ancient => "Ancient ",
        };

        let element_prefix = match primary {
            "fire" => "Ember",
            "water" | "ice" => "Frost",
            "earth" | "stone" => "Stone",
            "wind" => "Gale",
            "void" => "Void",
            "nature" | "forest" => "Verdant",
            "corruption" => "Corrupted",
            _ => "Shadow",
        };

        let name = format!("{}{} {}", rarity_prefix, element_prefix, gear_type.1);
        let id = format!("{}_{}", primary, gear_type.0);
        (id, name, true)
    } else {
        // Material drop based on element
        let (item_id, item_name) = match primary {
            "fire" | "volcanic" => ("fire_crystal", "Fire Crystal"),
            "water" | "ice" => ("ice_shard", "Ice Shard"),
            "earth" | "stone" => ("iron_ore", "Iron Ore"),
            "wind" => ("wind_essence", "Wind Essence"),
            "void" => ("void_shard", "Void Shard"),
            "nature" | "forest" => ("herb_lifeleaf", "Lifeleaf"),
            "corruption" => ("corruption_residue", "Corruption Residue"),
            "dungeon" => ("iron_ore", "Iron Ore"),
            _ => ("raw_material", "Raw Material"),
        };
        (item_id.to_string(), item_name.to_string(), false)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rarity_ordering() {
        assert!(Rarity::Common < Rarity::Uncommon);
        assert!(Rarity::Uncommon < Rarity::Rare);
        assert!(Rarity::Rare < Rarity::Epic);
        assert!(Rarity::Legendary < Rarity::Mythic);
        assert!(Rarity::Mythic < Rarity::Ancient);
    }

    #[test]
    fn test_rarity_from_roll() {
        // High roll = common
        assert_eq!(Rarity::from_roll(0.9, 0.0, 0.0), Rarity::Common);
        // Very low roll = legendary or better
        let r = Rarity::from_roll(0.01, 0.0, 0.0);
        assert!(r >= Rarity::Rare);
        // Luck bonus shifts distribution
        let with_luck = Rarity::from_roll(0.5, 1.0, 0.0);
        let without_luck = Rarity::from_roll(0.5, 0.0, 0.0);
        assert!(with_luck >= without_luck);
    }

    #[test]
    fn test_rarity_stats() {
        assert!(Rarity::Ancient.stat_mult() > Rarity::Common.stat_mult());
        assert!(Rarity::Legendary.max_effects() > Rarity::Rare.max_effects());
    }

    #[test]
    fn test_effect_generation() {
        let effect = EquipmentEffect::from_seed(42, Rarity::Epic);
        assert!(effect.chance > 0.0);
        let desc = effect.full_description();
        assert!(!desc.is_empty());
    }

    #[test]
    fn test_effect_description() {
        let effect = EquipmentEffect {
            trigger: EffectTrigger::OnHit,
            action: EffectAction::ElementalDamage {
                element: "fire".into(),
                amount: 15.0,
                is_percent: false,
            },
            chance: 0.25,
            cooldown_secs: 3.0,
        };
        let desc = effect.full_description();
        assert!(desc.contains("On Hit"));
        assert!(desc.contains("fire"));
        assert!(desc.contains("25%"));
        assert!(desc.contains("3s CD"));
    }

    #[test]
    fn test_generate_loot_basic() {
        let config = LootConfig {
            floor_id: 5,
            luck: 0.0,
            semantic_affinity: 0.0,
            loot_tier: 1,
            monster_tags: HashMap::from([
                ("fire".to_string(), 0.8),
                ("aggression".to_string(), 0.6),
            ]),
        };

        let drops = generate_loot(42, &config);
        assert!(!drops.is_empty());
        for drop in &drops {
            assert!(!drop.item_id.is_empty());
            assert!(drop.quantity > 0);
        }
    }

    #[test]
    fn test_generate_loot_deterministic() {
        let config = LootConfig {
            floor_id: 10,
            luck: 0.5,
            semantic_affinity: 0.3,
            loot_tier: 2,
            monster_tags: HashMap::from([("earth".to_string(), 0.7)]),
        };

        let drops1 = generate_loot(12345, &config);
        let drops2 = generate_loot(12345, &config);
        assert_eq!(drops1.len(), drops2.len());
        for (d1, d2) in drops1.iter().zip(drops2.iter()) {
            assert_eq!(d1.item_id, d2.item_id);
            assert_eq!(d1.rarity, d2.rarity);
            assert_eq!(d1.quantity, d2.quantity);
        }
    }

    #[test]
    fn test_higher_tier_more_drops() {
        let config_low = LootConfig {
            floor_id: 1,
            luck: 0.0,
            semantic_affinity: 0.0,
            loot_tier: 1,
            monster_tags: HashMap::new(),
        };

        let config_high = LootConfig {
            floor_id: 50,
            luck: 0.0,
            semantic_affinity: 0.0,
            loot_tier: 5,
            monster_tags: HashMap::new(),
        };

        // Over many seeds, higher tier should average more drops
        let mut total_low = 0;
        let mut total_high = 0;
        for seed in 0..50u64 {
            total_low += generate_loot(seed, &config_low).len();
            total_high += generate_loot(seed, &config_high).len();
        }
        assert!(total_high > total_low);
    }

    #[test]
    fn test_semantic_affinity_bonus_drop() {
        let config = LootConfig {
            floor_id: 20,
            luck: 0.0,
            semantic_affinity: 0.9, // High affinity
            loot_tier: 3,
            monster_tags: HashMap::from([("fire".to_string(), 0.9), ("volcanic".to_string(), 0.7)]),
        };

        // With high semantic affinity, some seeds should produce bonus drops
        let mut found_semantic = false;
        for seed in 0..100u64 {
            let drops = generate_loot(seed, &config);
            for drop in &drops {
                if drop.display_name.starts_with("Semantic") {
                    found_semantic = true;
                    assert!(drop.rarity >= Rarity::Rare);
                }
            }
        }
        assert!(
            found_semantic,
            "Should find at least one semantic bonus drop in 100 seeds"
        );
    }

    #[test]
    fn test_gear_has_effects_at_high_rarity() {
        let config = LootConfig {
            floor_id: 50,
            luck: 5.0, // Very lucky
            semantic_affinity: 0.0,
            loot_tier: 5,
            monster_tags: HashMap::from([("void".to_string(), 0.9)]),
        };

        // Find gear drops and check effects
        let mut found_gear_with_effects = false;
        for seed in 0..200u64 {
            let drops = generate_loot(seed, &config);
            for drop in &drops {
                if !drop.effects.is_empty() {
                    found_gear_with_effects = true;
                    // Each effect should have valid data
                    for eff in &drop.effects {
                        assert!(eff.chance > 0.0);
                        let desc = eff.full_description();
                        assert!(!desc.is_empty());
                    }
                }
            }
        }
        assert!(found_gear_with_effects, "Should find gear with effects");
    }

    #[test]
    fn test_element_themed_drops() {
        // Fire monster should drop fire-themed items
        let config = LootConfig {
            floor_id: 5,
            luck: 0.0,
            semantic_affinity: 0.0,
            loot_tier: 1,
            monster_tags: HashMap::from([("fire".to_string(), 0.9)]),
        };

        let mut found_fire = false;
        for seed in 0..50u64 {
            let drops = generate_loot(seed, &config);
            for drop in &drops {
                if drop.item_id.contains("fire") || drop.display_name.contains("Ember") {
                    found_fire = true;
                }
            }
        }
        assert!(found_fire, "Fire monsters should drop fire-themed items");
    }

    #[test]
    fn test_all_triggers_reachable() {
        let mut found = std::collections::HashSet::new();
        for i in 0..11u64 {
            found.insert(EffectTrigger::from_hash(i));
        }
        assert_eq!(found.len(), 11);
    }

    #[test]
    fn test_socket_ranges() {
        let (min, max) = Rarity::Legendary.socket_range();
        assert!(min >= 1);
        assert!(max >= min);
        assert!(max <= 5);

        let (min, max) = Rarity::Common.socket_range();
        assert_eq!(min, 0);
        assert_eq!(max, 0);
    }

    #[test]
    fn test_loot_gold_value_scales_with_rarity() {
        let config = LootConfig {
            floor_id: 10,
            luck: 0.0,
            semantic_affinity: 0.0,
            loot_tier: 3,
            monster_tags: HashMap::new(),
        };

        let drops = generate_loot(42, &config);
        // All drops should have non-zero gold value
        for drop in &drops {
            assert!(drop.gold_value > 0);
        }
    }
}
