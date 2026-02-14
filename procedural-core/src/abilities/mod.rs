//! Active Abilities System
//!
//! From ddopensource.txt Category 7:
//! Ultimate skills, signature moves, combo extensions, and synergies.
//!
//! Abilities are unlocked through:
//! 1. Mastery skill tree nodes (SkillEffect::UnlockAbility)
//! 2. Specialization branches (ultimate abilities)
//! 3. Equipment effects (free ability procs)
//!
//! Each ability has cooldown, resource cost, targeting, and effects.
//! Players equip up to 6 abilities in their hotbar.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Targeting type for abilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbilityTarget {
    /// Hits in front arc
    Melee,
    /// Fires projectile
    Ranged,
    /// Area of effect around self
    SelfAoE,
    /// Area of effect at target location
    GroundTarget,
    /// Single ally target
    AllyTarget,
    /// All allies in radius
    PartyAoE,
    /// Self only
    SelfOnly,
}

/// Resource cost for an ability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityCost {
    pub kinetic: f32,
    pub thermal: f32,
    pub semantic: f32,
    /// HP cost (blood magic style abilities)
    pub hp_percent: f32,
}

impl AbilityCost {
    pub fn kinetic(amount: f32) -> Self {
        Self {
            kinetic: amount,
            thermal: 0.0,
            semantic: 0.0,
            hp_percent: 0.0,
        }
    }

    pub fn thermal(amount: f32) -> Self {
        Self {
            kinetic: 0.0,
            thermal: amount,
            semantic: 0.0,
            hp_percent: 0.0,
        }
    }

    pub fn semantic(amount: f32) -> Self {
        Self {
            kinetic: 0.0,
            thermal: 0.0,
            semantic: amount,
            hp_percent: 0.0,
        }
    }

    pub fn free() -> Self {
        Self {
            kinetic: 0.0,
            thermal: 0.0,
            semantic: 0.0,
            hp_percent: 0.0,
        }
    }

    pub fn total_cost(&self) -> f32 {
        self.kinetic + self.thermal + self.semantic
    }
}

/// An active ability that players can use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ability {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon_tag: String,
    /// Cooldown in seconds
    pub cooldown: f32,
    pub cost: AbilityCost,
    pub target: AbilityTarget,
    /// Range in game units (for ranged/ground target)
    pub range: f32,
    /// Radius for AoE abilities
    pub radius: f32,
    /// Cast time in seconds (0 = instant)
    pub cast_time: f32,
    /// Effects when ability activates
    pub effects: Vec<AbilityEffect>,
    /// Source: mastery node, specialization, or equipment
    pub source: AbilitySource,
}

/// Where the ability was unlocked from
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AbilitySource {
    MasteryNode(String),
    Specialization(String),
    Equipment(String),
    Innate,
}

/// What an ability does on activation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AbilityEffect {
    /// Deal damage to targets
    Damage {
        base: f32,
        scaling_stat: String,
        element: String,
    },
    /// Heal targets
    Heal { base: f32, scaling_stat: String },
    /// Apply status effect
    ApplyStatus {
        status: String,
        duration: f32,
        stacks: u32,
    },
    /// Shield self or allies
    Shield { amount: f32, duration: f32 },
    /// Buff stat
    Buff {
        stat: String,
        amount: f32,
        duration: f32,
    },
    /// Debuff enemies
    Debuff {
        stat: String,
        amount: f32,
        duration: f32,
    },
    /// Dash/teleport
    Displacement { distance: f32 },
    /// Spawn entity (summon, turret, trap)
    Summon {
        entity_type: String,
        duration: f32,
        count: u32,
    },
    /// Pull enemies toward point
    Pull { radius: f32, force: f32 },
    /// Knockback enemies
    Knockback { force: f32, radius: f32 },
    /// Cleanse debuffs
    Cleanse { count: u32 },
    /// Resource regeneration
    ResourceRegen {
        resource: String,
        amount: f32,
        duration: f32,
    },
}

/// Runtime state for tracking cooldowns
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AbilityCooldownTracker {
    /// ability_id → remaining cooldown seconds
    pub cooldowns: HashMap<String, f32>,
}

impl AbilityCooldownTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if ability is off cooldown
    pub fn is_ready(&self, ability_id: &str) -> bool {
        self.cooldowns.get(ability_id).is_none_or(|cd| *cd <= 0.0)
    }

    /// Start cooldown for an ability
    pub fn start_cooldown(&mut self, ability: &Ability) {
        self.cooldowns.insert(ability.id.clone(), ability.cooldown);
    }

    /// Tick all cooldowns by delta time
    pub fn tick(&mut self, delta: f32) {
        for cd in self.cooldowns.values_mut() {
            *cd = (*cd - delta).max(0.0);
        }
    }

    /// Get remaining cooldown for an ability
    pub fn remaining(&self, ability_id: &str) -> f32 {
        self.cooldowns.get(ability_id).copied().unwrap_or(0.0)
    }

    /// Apply cooldown reduction (from passives/equipment)
    pub fn apply_cdr(&mut self, ability_id: &str, reduction_percent: f32) {
        if let Some(cd) = self.cooldowns.get_mut(ability_id) {
            *cd *= 1.0 - reduction_percent;
        }
    }
}

/// Player's ability loadout (hotbar)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityLoadout {
    /// Equipped abilities (max 6 slots)
    pub slots: Vec<Option<String>>,
    /// All learned abilities (id → Ability)
    pub known_abilities: HashMap<String, Ability>,
    pub max_slots: usize,
}

impl Default for AbilityLoadout {
    fn default() -> Self {
        Self::new()
    }
}

impl AbilityLoadout {
    pub fn new() -> Self {
        Self {
            slots: vec![None; 6],
            known_abilities: HashMap::new(),
            max_slots: 6,
        }
    }

    /// Learn a new ability
    pub fn learn(&mut self, ability: Ability) -> bool {
        if self.known_abilities.contains_key(&ability.id) {
            return false;
        }
        self.known_abilities.insert(ability.id.clone(), ability);
        true
    }

    /// Equip ability to a slot
    pub fn equip(&mut self, slot: usize, ability_id: &str) -> bool {
        if slot >= self.max_slots {
            return false;
        }
        if !self.known_abilities.contains_key(ability_id) {
            return false;
        }
        // Remove from any existing slot
        for s in &mut self.slots {
            if s.as_deref() == Some(ability_id) {
                *s = None;
            }
        }
        self.slots[slot] = Some(ability_id.to_string());
        true
    }

    /// Unequip from a slot
    pub fn unequip(&mut self, slot: usize) -> bool {
        if slot >= self.max_slots {
            return false;
        }
        self.slots[slot] = None;
        true
    }

    /// Get ability in a slot
    pub fn get_slot(&self, slot: usize) -> Option<&Ability> {
        self.slots
            .get(slot)
            .and_then(|s| s.as_ref())
            .and_then(|id| self.known_abilities.get(id))
    }

    /// Count equipped abilities
    pub fn equipped_count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    /// Get all known ability IDs
    pub fn known_ids(&self) -> Vec<&str> {
        self.known_abilities.keys().map(|s| s.as_str()).collect()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Predefined abilities from mastery trees and specializations
pub fn default_abilities() -> Vec<Ability> {
    vec![
        // === Sword Abilities ===
        Ability {
            id: "rising_slash".into(),
            name: "Rising Slash".into(),
            description: "Launch an enemy upward with a powerful upswing, enabling aerial combos."
                .into(),
            icon_tag: "sword_aerial".into(),
            cooldown: 12.0,
            cost: AbilityCost::kinetic(25.0),
            target: AbilityTarget::Melee,
            range: 3.0,
            radius: 0.0,
            cast_time: 0.0,
            effects: vec![
                AbilityEffect::Damage {
                    base: 80.0,
                    scaling_stat: "strength".into(),
                    element: "physical".into(),
                },
                AbilityEffect::Knockback {
                    force: 15.0,
                    radius: 0.0,
                },
            ],
            source: AbilitySource::MasteryNode("sword_rising_slash".into()),
        },
        // === Parry Abilities ===
        Ability {
            id: "riposte".into(),
            name: "Riposte".into(),
            description: "After a perfect parry, instantly counter with a devastating strike."
                .into(),
            icon_tag: "parry_counter".into(),
            cooldown: 0.0, // triggered by perfect parry
            cost: AbilityCost::free(),
            target: AbilityTarget::Melee,
            range: 3.0,
            radius: 0.0,
            cast_time: 0.0,
            effects: vec![AbilityEffect::Damage {
                base: 150.0,
                scaling_stat: "agility".into(),
                element: "physical".into(),
            }],
            source: AbilitySource::MasteryNode("parry_counter".into()),
        },
        // === Staff Abilities ===
        Ability {
            id: "healing_wave".into(),
            name: "Healing Wave".into(),
            description: "Send a wave of restorative energy that heals all allies in range.".into(),
            icon_tag: "staff_heal".into(),
            cooldown: 18.0,
            cost: AbilityCost::semantic(40.0),
            target: AbilityTarget::PartyAoE,
            range: 0.0,
            radius: 12.0,
            cast_time: 1.5,
            effects: vec![AbilityEffect::Heal {
                base: 120.0,
                scaling_stat: "spirit".into(),
            }],
            source: AbilitySource::Specialization("staff_mender".into()),
        },
        // === Gauntlet Abilities ===
        Ability {
            id: "ground_slam".into(),
            name: "Ground Slam".into(),
            description: "Smash the ground, dealing AoE damage and stunning nearby enemies.".into(),
            icon_tag: "gauntlet_slam".into(),
            cooldown: 15.0,
            cost: AbilityCost::kinetic(35.0),
            target: AbilityTarget::SelfAoE,
            range: 0.0,
            radius: 8.0,
            cast_time: 0.5,
            effects: vec![
                AbilityEffect::Damage {
                    base: 100.0,
                    scaling_stat: "strength".into(),
                    element: "earth".into(),
                },
                AbilityEffect::ApplyStatus {
                    status: "stun".into(),
                    duration: 1.5,
                    stacks: 1,
                },
            ],
            source: AbilitySource::MasteryNode("gauntlet_slam".into()),
        },
        // === Dodge Abilities ===
        Ability {
            id: "shadow_strike".into(),
            name: "Shadow Strike".into(),
            description: "Dash through an enemy, dealing damage and appearing behind them.".into(),
            icon_tag: "dodge_dash".into(),
            cooldown: 10.0,
            cost: AbilityCost::kinetic(20.0),
            target: AbilityTarget::Melee,
            range: 12.0,
            radius: 0.0,
            cast_time: 0.0,
            effects: vec![
                AbilityEffect::Displacement { distance: 12.0 },
                AbilityEffect::Damage {
                    base: 60.0,
                    scaling_stat: "agility".into(),
                    element: "void".into(),
                },
            ],
            source: AbilitySource::Specialization("dodge_shadow".into()),
        },
        // === General Abilities ===
        Ability {
            id: "war_cry".into(),
            name: "War Cry".into(),
            description: "Let out a fierce shout, buffing party attack damage for 10 seconds."
                .into(),
            icon_tag: "support_buff".into(),
            cooldown: 30.0,
            cost: AbilityCost::kinetic(15.0),
            target: AbilityTarget::PartyAoE,
            range: 0.0,
            radius: 15.0,
            cast_time: 0.0,
            effects: vec![AbilityEffect::Buff {
                stat: "damage".into(),
                amount: 0.15,
                duration: 10.0,
            }],
            source: AbilitySource::Innate,
        },
        Ability {
            id: "elemental_burst".into(),
            name: "Elemental Burst".into(),
            description: "Release stored elemental energy in an explosive blast.".into(),
            icon_tag: "element_burst".into(),
            cooldown: 20.0,
            cost: AbilityCost::thermal(30.0),
            target: AbilityTarget::GroundTarget,
            range: 20.0,
            radius: 6.0,
            cast_time: 0.8,
            effects: vec![
                AbilityEffect::Damage {
                    base: 200.0,
                    scaling_stat: "mind".into(),
                    element: "fire".into(),
                },
                AbilityEffect::ApplyStatus {
                    status: "burning".into(),
                    duration: 4.0,
                    stacks: 2,
                },
            ],
            source: AbilitySource::Innate,
        },
        Ability {
            id: "semantic_shield".into(),
            name: "Semantic Shield".into(),
            description: "Weave tower energy into a protective barrier that absorbs damage.".into(),
            icon_tag: "shield_magic".into(),
            cooldown: 25.0,
            cost: AbilityCost::semantic(35.0),
            target: AbilityTarget::SelfOnly,
            range: 0.0,
            radius: 0.0,
            cast_time: 0.3,
            effects: vec![
                AbilityEffect::Shield {
                    amount: 200.0,
                    duration: 8.0,
                },
                AbilityEffect::Cleanse { count: 2 },
            ],
            source: AbilitySource::Innate,
        },
    ]
}

/// Bevy plugin stub
pub struct AbilitiesPlugin;
impl bevy::prelude::Plugin for AbilitiesPlugin {
    fn build(&self, _app: &mut bevy::prelude::App) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_abilities() {
        let abilities = default_abilities();
        assert!(abilities.len() >= 8, "Should have at least 8 abilities");
    }

    #[test]
    fn test_ability_loadout_learn() {
        let mut loadout = AbilityLoadout::new();
        let abilities = default_abilities();

        assert!(loadout.learn(abilities[0].clone()));
        assert!(!loadout.learn(abilities[0].clone())); // can't learn twice
        assert_eq!(loadout.known_abilities.len(), 1);
    }

    #[test]
    fn test_ability_loadout_equip() {
        let mut loadout = AbilityLoadout::new();
        let abilities = default_abilities();

        loadout.learn(abilities[0].clone());
        assert!(loadout.equip(0, &abilities[0].id));
        assert_eq!(loadout.equipped_count(), 1);
        assert!(loadout.get_slot(0).is_some());
        assert_eq!(loadout.get_slot(0).unwrap().id, abilities[0].id);
    }

    #[test]
    fn test_equip_moves_between_slots() {
        let mut loadout = AbilityLoadout::new();
        let abilities = default_abilities();

        loadout.learn(abilities[0].clone());
        loadout.equip(0, &abilities[0].id);
        loadout.equip(3, &abilities[0].id); // move to slot 3

        assert!(loadout.get_slot(0).is_none());
        assert!(loadout.get_slot(3).is_some());
        assert_eq!(loadout.equipped_count(), 1);
    }

    #[test]
    fn test_cannot_equip_unknown() {
        let mut loadout = AbilityLoadout::new();
        assert!(!loadout.equip(0, "nonexistent"));
    }

    #[test]
    fn test_unequip() {
        let mut loadout = AbilityLoadout::new();
        let abilities = default_abilities();

        loadout.learn(abilities[0].clone());
        loadout.equip(0, &abilities[0].id);
        assert!(loadout.unequip(0));
        assert_eq!(loadout.equipped_count(), 0);
    }

    #[test]
    fn test_cooldown_tracker() {
        let mut tracker = AbilityCooldownTracker::new();
        let abilities = default_abilities();
        let ability = &abilities[0]; // Rising Slash, 12s cooldown

        assert!(tracker.is_ready(&ability.id));

        tracker.start_cooldown(ability);
        assert!(!tracker.is_ready(&ability.id));
        assert!((tracker.remaining(&ability.id) - 12.0).abs() < 0.01);

        // Tick 5 seconds
        tracker.tick(5.0);
        assert!(!tracker.is_ready(&ability.id));
        assert!((tracker.remaining(&ability.id) - 7.0).abs() < 0.01);

        // Tick 7 more seconds
        tracker.tick(7.0);
        assert!(tracker.is_ready(&ability.id));
    }

    #[test]
    fn test_cooldown_reduction() {
        let mut tracker = AbilityCooldownTracker::new();
        let abilities = default_abilities();
        let ability = &abilities[0];

        tracker.start_cooldown(ability);
        tracker.apply_cdr(&ability.id, 0.25); // 25% CDR
                                              // 12 * 0.75 = 9
        assert!((tracker.remaining(&ability.id) - 9.0).abs() < 0.01);
    }

    #[test]
    fn test_ability_cost_types() {
        let k = AbilityCost::kinetic(20.0);
        assert_eq!(k.kinetic, 20.0);
        assert_eq!(k.thermal, 0.0);

        let t = AbilityCost::thermal(30.0);
        assert_eq!(t.thermal, 30.0);

        let s = AbilityCost::semantic(15.0);
        assert_eq!(s.semantic, 15.0);

        let f = AbilityCost::free();
        assert_eq!(f.total_cost(), 0.0);
    }

    #[test]
    fn test_max_6_slots() {
        let mut loadout = AbilityLoadout::new();
        assert_eq!(loadout.max_slots, 6);
        assert!(!loadout.equip(6, "test")); // out of bounds
    }

    #[test]
    fn test_loadout_json() {
        let mut loadout = AbilityLoadout::new();
        let abilities = default_abilities();
        loadout.learn(abilities[0].clone());
        loadout.equip(0, &abilities[0].id);

        let json = loadout.to_json();
        assert!(!json.is_empty());
        assert!(json.contains("rising_slash"));
    }

    #[test]
    fn test_ability_targets() {
        let abilities = default_abilities();
        let targets: Vec<AbilityTarget> = abilities.iter().map(|a| a.target).collect();
        // Should have multiple target types
        assert!(targets.contains(&AbilityTarget::Melee));
        assert!(targets.contains(&AbilityTarget::PartyAoE));
        assert!(targets.contains(&AbilityTarget::SelfOnly));
    }
}
