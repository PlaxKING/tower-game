//! Floor Mutator System
//!
//! Random modifiers applied to floors that change gameplay rules.
//! Mutators are deterministically selected from the floor hash,
//! creating unique challenge combinations on every floor.
//!
//! Mutator categories:
//! - Combat: damage modifiers, healing restrictions, crit changes
//! - Environment: darkness, speed, gravity
//! - Economy: loot modifiers, cost changes
//! - Semantic: tag shifts, resonance amplification
//! - Challenge: time limits, no-death, escalation

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

use crate::generation::FloorTier;

pub struct MutatorsPlugin;

impl Plugin for MutatorsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<MutatorActivatedEvent>();
    }
}

/// Event fired when a mutator activates on a floor
#[derive(Event, Debug, Clone)]
pub struct MutatorActivatedEvent {
    pub floor_id: u32,
    pub mutator: FloorMutator,
}

/// Categories of mutators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MutatorCategory {
    Combat,
    Environment,
    Economy,
    Semantic,
    Challenge,
}

/// Individual mutator definitions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MutatorType {
    // Combat mutators
    DoubleDamage,   // All damage dealt/received x2
    GlassCannon,    // +100% damage dealt, +50% damage received
    NoHealing,      // All healing disabled
    CriticalStorm,  // +30% crit chance for everyone
    VampiricCombat, // All hits heal 5% of damage dealt
    ArmoredFoes,    // Monsters have +50% armor
    ElementalChaos, // Random element on each attack

    // Environment mutators
    Darkness,        // Reduced visibility to 30%
    SpeedBoost,      // +40% movement speed for all
    LowGravity,      // Reduced gravity, higher jumps
    ToxicAtmosphere, // Slow DoT (1% HP/sec) unless near shrine
    UnstableGround,  // Random tile collapses every 30s
    MagneticField,   // Projectiles curve unpredictably

    // Economy mutators
    Bountiful,        // +100% loot drops
    Scarcity,         // -50% loot drops, but +1 rarity tier
    GoldenFloor,      // +200% shard drops, no equipment drops
    CursedGold,       // Picking up loot deals 5% HP damage
    MerchantBlessing, // Crafting costs halved

    // Semantic mutators
    SemanticOverload, // All semantic interactions amplified x2
    ElementalPurity,  // Only one element active (dominant from hash)
    TagShift,         // All semantic tags rotate every 60s
    ResonanceLock,    // Synergies always active, conflicts always active
    CorruptionTide,   // Corruption rises 1% per minute

    // Challenge mutators
    TimeTrial,  // Floor must be cleared in 5 minutes
    Ironman,    // Death = restart from floor 1
    Escalation, // Monster difficulty increases every kill
    Pacifist,   // Bonus rewards if no monsters killed
    NoRespite,  // Monster respawn rate x3
}

impl MutatorType {
    /// Get the category for this mutator
    pub fn category(&self) -> MutatorCategory {
        match self {
            Self::DoubleDamage
            | Self::GlassCannon
            | Self::NoHealing
            | Self::CriticalStorm
            | Self::VampiricCombat
            | Self::ArmoredFoes
            | Self::ElementalChaos => MutatorCategory::Combat,

            Self::Darkness
            | Self::SpeedBoost
            | Self::LowGravity
            | Self::ToxicAtmosphere
            | Self::UnstableGround
            | Self::MagneticField => MutatorCategory::Environment,

            Self::Bountiful
            | Self::Scarcity
            | Self::GoldenFloor
            | Self::CursedGold
            | Self::MerchantBlessing => MutatorCategory::Economy,

            Self::SemanticOverload
            | Self::ElementalPurity
            | Self::TagShift
            | Self::ResonanceLock
            | Self::CorruptionTide => MutatorCategory::Semantic,

            Self::TimeTrial
            | Self::Ironman
            | Self::Escalation
            | Self::Pacifist
            | Self::NoRespite => MutatorCategory::Challenge,
        }
    }

    /// Difficulty rating 1-5 (affects reward scaling)
    pub fn difficulty_rating(&self) -> u32 {
        match self {
            Self::SpeedBoost | Self::Bountiful | Self::MerchantBlessing => 1,
            Self::CriticalStorm | Self::VampiricCombat | Self::LowGravity => 2,
            Self::DoubleDamage
            | Self::Darkness
            | Self::Scarcity
            | Self::SemanticOverload
            | Self::Pacifist
            | Self::ElementalPurity => 3,
            Self::GlassCannon
            | Self::ArmoredFoes
            | Self::ToxicAtmosphere
            | Self::GoldenFloor
            | Self::CursedGold
            | Self::TagShift
            | Self::ResonanceLock
            | Self::TimeTrial
            | Self::Escalation
            | Self::NoRespite
            | Self::ElementalChaos
            | Self::UnstableGround
            | Self::MagneticField
            | Self::CorruptionTide => 4,
            Self::NoHealing | Self::Ironman => 5,
        }
    }

    /// Human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::DoubleDamage => "All damage dealt and received is doubled",
            Self::GlassCannon => "Deal +100% damage, take +50% damage",
            Self::NoHealing => "All healing effects are disabled",
            Self::CriticalStorm => "+30% critical hit chance for all combatants",
            Self::VampiricCombat => "All hits heal 5% of damage dealt",
            Self::ArmoredFoes => "Monsters have +50% armor",
            Self::ElementalChaos => "Each attack has a random element",
            Self::Darkness => "Visibility reduced to 30%",
            Self::SpeedBoost => "+40% movement speed for everyone",
            Self::LowGravity => "Reduced gravity, higher jumps",
            Self::ToxicAtmosphere => "1% HP/sec damage unless near a shrine",
            Self::UnstableGround => "Random tiles collapse every 30 seconds",
            Self::MagneticField => "Projectiles curve unpredictably",
            Self::Bountiful => "+100% loot drops",
            Self::Scarcity => "-50% loot quantity, but +1 rarity tier",
            Self::GoldenFloor => "+200% shard drops, no equipment drops",
            Self::CursedGold => "Picking up loot deals 5% HP damage",
            Self::MerchantBlessing => "Crafting costs halved",
            Self::SemanticOverload => "Semantic interactions amplified x2",
            Self::ElementalPurity => "Only one element is active on this floor",
            Self::TagShift => "Semantic tags rotate every 60 seconds",
            Self::ResonanceLock => "Synergies and conflicts are always at maximum",
            Self::CorruptionTide => "Corruption rises 1% per minute",
            Self::TimeTrial => "Clear this floor within 5 minutes",
            Self::Ironman => "Death sends you back to floor 1",
            Self::Escalation => "Each kill increases remaining monster difficulty",
            Self::Pacifist => "Bonus rewards if no monsters are killed",
            Self::NoRespite => "Monster respawn rate tripled",
        }
    }

    /// Icon hint for UI (maps to UE5 texture name)
    pub fn icon_id(&self) -> &'static str {
        match self {
            Self::DoubleDamage => "icon_double_sword",
            Self::GlassCannon => "icon_shattered_shield",
            Self::NoHealing => "icon_broken_heart",
            Self::CriticalStorm => "icon_lightning",
            Self::VampiricCombat => "icon_vampire",
            Self::ArmoredFoes => "icon_heavy_armor",
            Self::ElementalChaos => "icon_chaos_element",
            Self::Darkness => "icon_moon",
            Self::SpeedBoost => "icon_wind",
            Self::LowGravity => "icon_feather",
            Self::ToxicAtmosphere => "icon_poison",
            Self::UnstableGround => "icon_cracked_earth",
            Self::MagneticField => "icon_magnet",
            Self::Bountiful => "icon_treasure",
            Self::Scarcity => "icon_empty_chest",
            Self::GoldenFloor => "icon_gold_coins",
            Self::CursedGold => "icon_cursed_skull",
            Self::MerchantBlessing => "icon_merchant",
            Self::SemanticOverload => "icon_brain",
            Self::ElementalPurity => "icon_crystal",
            Self::TagShift => "icon_cycle",
            Self::ResonanceLock => "icon_lock",
            Self::CorruptionTide => "icon_corruption",
            Self::TimeTrial => "icon_hourglass",
            Self::Ironman => "icon_skull",
            Self::Escalation => "icon_ascending",
            Self::Pacifist => "icon_dove",
            Self::NoRespite => "icon_swarm",
        }
    }
}

/// Complete list of all mutator types (for random selection)
const ALL_MUTATORS: [MutatorType; 28] = [
    MutatorType::DoubleDamage,
    MutatorType::GlassCannon,
    MutatorType::NoHealing,
    MutatorType::CriticalStorm,
    MutatorType::VampiricCombat,
    MutatorType::ArmoredFoes,
    MutatorType::ElementalChaos,
    MutatorType::Darkness,
    MutatorType::SpeedBoost,
    MutatorType::LowGravity,
    MutatorType::ToxicAtmosphere,
    MutatorType::UnstableGround,
    MutatorType::MagneticField,
    MutatorType::Bountiful,
    MutatorType::Scarcity,
    MutatorType::GoldenFloor,
    MutatorType::CursedGold,
    MutatorType::MerchantBlessing,
    MutatorType::SemanticOverload,
    MutatorType::ElementalPurity,
    MutatorType::TagShift,
    MutatorType::ResonanceLock,
    MutatorType::CorruptionTide,
    MutatorType::TimeTrial,
    MutatorType::Ironman,
    MutatorType::Escalation,
    MutatorType::Pacifist,
    MutatorType::NoRespite,
];

/// A mutator applied to a floor with its multiplier strength
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorMutator {
    pub mutator_type: MutatorType,
    pub category: MutatorCategory,
    pub description: String,
    pub difficulty: u32,
    pub icon_id: String,
    /// Strength modifier (1.0 = normal, higher = more intense)
    pub intensity: f32,
}

impl FloorMutator {
    fn from_type(mt: MutatorType, intensity: f32) -> Self {
        Self {
            category: mt.category(),
            description: mt.description().to_string(),
            difficulty: mt.difficulty_rating(),
            icon_id: mt.icon_id().to_string(),
            intensity,
            mutator_type: mt,
        }
    }
}

/// Gameplay modifiers computed from active mutators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutatorEffects {
    pub damage_dealt_mult: f32,
    pub damage_taken_mult: f32,
    pub healing_mult: f32,
    pub crit_chance_bonus: f32,
    pub lifesteal_percent: f32,
    pub monster_armor_mult: f32,
    pub visibility_mult: f32,
    pub speed_mult: f32,
    pub gravity_mult: f32,
    pub loot_quantity_mult: f32,
    pub loot_rarity_bonus: i32,
    pub shard_mult: f32,
    pub craft_cost_mult: f32,
    pub semantic_mult: f32,
    pub monster_respawn_mult: f32,
    pub time_limit_secs: Option<f32>,
    pub permadeath: bool,
    pub escalation_active: bool,
    pub pacifist_bonus: bool,
    pub toxic_dps_percent: f32,
    pub corruption_rise_per_min: f32,
    pub total_difficulty: u32,
    pub reward_multiplier: f32,
}

impl Default for MutatorEffects {
    fn default() -> Self {
        Self {
            damage_dealt_mult: 1.0,
            damage_taken_mult: 1.0,
            healing_mult: 1.0,
            crit_chance_bonus: 0.0,
            lifesteal_percent: 0.0,
            monster_armor_mult: 1.0,
            visibility_mult: 1.0,
            speed_mult: 1.0,
            gravity_mult: 1.0,
            loot_quantity_mult: 1.0,
            loot_rarity_bonus: 0,
            shard_mult: 1.0,
            craft_cost_mult: 1.0,
            semantic_mult: 1.0,
            monster_respawn_mult: 1.0,
            time_limit_secs: None,
            permadeath: false,
            escalation_active: false,
            pacifist_bonus: false,
            toxic_dps_percent: 0.0,
            corruption_rise_per_min: 0.0,
            total_difficulty: 0,
            reward_multiplier: 1.0,
        }
    }
}

/// Compute aggregate gameplay effects from a list of mutators
pub fn compute_effects(mutators: &[FloorMutator]) -> MutatorEffects {
    let mut fx = MutatorEffects::default();
    let mut total_diff: u32 = 0;

    for m in mutators {
        let i = m.intensity;
        total_diff = total_diff.saturating_add(m.difficulty);

        match &m.mutator_type {
            MutatorType::DoubleDamage => {
                fx.damage_dealt_mult *= 1.0 + i;
                fx.damage_taken_mult *= 1.0 + i;
            }
            MutatorType::GlassCannon => {
                fx.damage_dealt_mult *= 1.0 + i;
                fx.damage_taken_mult *= 1.0 + 0.5 * i;
            }
            MutatorType::NoHealing => {
                fx.healing_mult = 0.0;
            }
            MutatorType::CriticalStorm => {
                fx.crit_chance_bonus += 0.3 * i;
            }
            MutatorType::VampiricCombat => {
                fx.lifesteal_percent += 0.05 * i;
            }
            MutatorType::ArmoredFoes => {
                fx.monster_armor_mult *= 1.0 + 0.5 * i;
            }
            MutatorType::ElementalChaos => {
                // Handled by combat system — flag only
            }
            MutatorType::Darkness => {
                fx.visibility_mult *= 0.3_f32.powf(i);
            }
            MutatorType::SpeedBoost => {
                fx.speed_mult *= 1.0 + 0.4 * i;
            }
            MutatorType::LowGravity => {
                fx.gravity_mult *= 1.0 - 0.5 * i.min(0.9);
            }
            MutatorType::ToxicAtmosphere => {
                fx.toxic_dps_percent += 0.01 * i;
            }
            MutatorType::UnstableGround | MutatorType::MagneticField => {
                // Handled by floor/combat systems — flags only
            }
            MutatorType::Bountiful => {
                fx.loot_quantity_mult *= 1.0 + i;
            }
            MutatorType::Scarcity => {
                fx.loot_quantity_mult *= 1.0 - 0.5 * i.min(0.9);
                fx.loot_rarity_bonus += 1;
            }
            MutatorType::GoldenFloor => {
                fx.shard_mult *= 1.0 + 2.0 * i;
                fx.loot_quantity_mult = 0.0; // no equipment
            }
            MutatorType::CursedGold => {
                // Handled by pickup system
            }
            MutatorType::MerchantBlessing => {
                fx.craft_cost_mult *= 1.0 - 0.5 * i.min(0.9);
            }
            MutatorType::SemanticOverload => {
                fx.semantic_mult *= 1.0 + i;
            }
            MutatorType::ElementalPurity | MutatorType::TagShift | MutatorType::ResonanceLock => {
                // Handled by semantic system
            }
            MutatorType::CorruptionTide => {
                fx.corruption_rise_per_min += 0.01 * i;
            }
            MutatorType::TimeTrial => {
                fx.time_limit_secs = Some(300.0 / i.max(0.5));
            }
            MutatorType::Ironman => {
                fx.permadeath = true;
            }
            MutatorType::Escalation => {
                fx.escalation_active = true;
            }
            MutatorType::Pacifist => {
                fx.pacifist_bonus = true;
            }
            MutatorType::NoRespite => {
                fx.monster_respawn_mult *= 1.0 + 2.0 * i;
            }
        }
    }

    fx.total_difficulty = total_diff;
    // Reward scaling: +10% per difficulty point
    fx.reward_multiplier = 1.0 + total_diff as f32 * 0.1;
    fx
}

/// How many mutators a floor gets based on tier
fn mutator_count_for_tier(tier: FloorTier) -> usize {
    match tier {
        FloorTier::Echelon1 => 1, // Tutorial floors: 1 mild mutator
        FloorTier::Echelon2 => 2, // Mid game: 2 mutators
        FloorTier::Echelon3 => 3, // Late game: 3 mutators
        FloorTier::Echelon4 => 4, // Endgame: 4 mutators
    }
}

/// Deterministically generate mutators for a floor from its seed and ID
pub fn generate_floor_mutators(seed: u64, floor_id: u32) -> Vec<FloorMutator> {
    // Use a separate hash stream for mutators so we don't affect other generation
    let mut hasher = Sha3_256::new();
    hasher.update(b"mutators");
    hasher.update(seed.to_le_bytes());
    hasher.update(floor_id.to_le_bytes());
    let result = hasher.finalize();

    let tier = FloorTier::from_floor_id(floor_id);
    let count = mutator_count_for_tier(tier);

    let mut mutators = Vec::with_capacity(count);
    let mut used_categories = Vec::new();

    for i in 0..count {
        // Extract bytes for selection
        let byte_offset = (i * 4) % 28; // 32 bytes in SHA3-256, use 4 per mutator
        let selector = u32::from_le_bytes([
            result[byte_offset],
            result[byte_offset + 1],
            result[byte_offset + 2],
            result[byte_offset + 3],
        ]);

        // Select mutator, avoiding duplicate categories for variety
        let mut idx = (selector as usize) % ALL_MUTATORS.len();
        let mut attempts = 0;
        while attempts < ALL_MUTATORS.len() {
            let candidate = &ALL_MUTATORS[idx];
            let cat = candidate.category();

            // Skip if we already have this category (unless we've exhausted options)
            if !used_categories.contains(&cat) || attempts >= ALL_MUTATORS.len() / 2 {
                // Tier-based filtering: Echelon1 avoids difficulty 5 mutators
                if tier == FloorTier::Echelon1 && candidate.difficulty_rating() >= 5 {
                    idx = (idx + 1) % ALL_MUTATORS.len();
                    attempts += 1;
                    continue;
                }
                break;
            }
            idx = (idx + 1) % ALL_MUTATORS.len();
            attempts += 1;
        }

        let mt = ALL_MUTATORS[idx].clone();
        used_categories.push(mt.category());

        // Intensity scales with tier
        let base_intensity = match tier {
            FloorTier::Echelon1 => 0.5,
            FloorTier::Echelon2 => 0.75,
            FloorTier::Echelon3 => 1.0,
            FloorTier::Echelon4 => 1.25,
        };

        // Minor per-mutator variation from hash
        let variation = ((result[(i * 2 + 16) % 32] as f32) / 255.0) * 0.4 - 0.2; // -0.2..+0.2
        let intensity = (base_intensity + variation).clamp(0.3, 2.0);

        mutators.push(FloorMutator::from_type(mt, intensity));
    }

    mutators
}

/// Get the full list of all available mutator types (for UI display)
pub fn all_mutator_types() -> Vec<FloorMutator> {
    ALL_MUTATORS
        .iter()
        .map(|mt| FloorMutator::from_type(mt.clone(), 1.0))
        .collect()
}

/// Serializable floor mutator set for FFI transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorMutatorSet {
    pub floor_id: u32,
    pub tier: String,
    pub mutators: Vec<FloorMutator>,
    pub effects: MutatorEffects,
}

/// Generate a complete mutator set for a floor (used by FFI)
pub fn generate_mutator_set(seed: u64, floor_id: u32) -> FloorMutatorSet {
    let mutators = generate_floor_mutators(seed, floor_id);
    let effects = compute_effects(&mutators);
    let tier = FloorTier::from_floor_id(floor_id);

    FloorMutatorSet {
        floor_id,
        tier: format!("{:?}", tier),
        mutators,
        effects,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mutator_generation_deterministic() {
        let m1 = generate_floor_mutators(42, 10);
        let m2 = generate_floor_mutators(42, 10);
        assert_eq!(m1.len(), m2.len());
        for (a, b) in m1.iter().zip(m2.iter()) {
            assert_eq!(a.mutator_type, b.mutator_type);
            assert!((a.intensity - b.intensity).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_mutator_count_by_tier() {
        // Echelon1 (floor 50) → 1 mutator
        let m = generate_floor_mutators(42, 50);
        assert_eq!(m.len(), 1);

        // Echelon2 (floor 200) → 2 mutators
        let m = generate_floor_mutators(42, 200);
        assert_eq!(m.len(), 2);

        // Echelon3 (floor 400) → 3 mutators
        let m = generate_floor_mutators(42, 400);
        assert_eq!(m.len(), 3);

        // Echelon4 (floor 600) → 4 mutators
        let m = generate_floor_mutators(42, 600);
        assert_eq!(m.len(), 4);
    }

    #[test]
    fn test_different_floors_get_different_mutators() {
        let m1 = generate_floor_mutators(42, 200);
        let m2 = generate_floor_mutators(42, 201);
        // Very unlikely to be identical
        let same = m1
            .iter()
            .zip(m2.iter())
            .all(|(a, b)| a.mutator_type == b.mutator_type);
        assert!(
            !same,
            "Different floors should generally have different mutators"
        );
    }

    #[test]
    fn test_different_seeds_get_different_mutators() {
        let m1 = generate_floor_mutators(42, 200);
        let m2 = generate_floor_mutators(99, 200);
        let same = m1
            .iter()
            .zip(m2.iter())
            .all(|(a, b)| a.mutator_type == b.mutator_type);
        assert!(
            !same,
            "Different seeds should generally have different mutators"
        );
    }

    #[test]
    fn test_echelon1_no_difficulty5() {
        // Run many floor IDs to check Echelon1 never gets difficulty 5
        for floor_id in 1..=100 {
            let mutators = generate_floor_mutators(42, floor_id);
            for m in &mutators {
                assert!(
                    m.difficulty < 5,
                    "Echelon1 floor {} got difficulty 5 mutator: {:?}",
                    floor_id,
                    m.mutator_type
                );
            }
        }
    }

    #[test]
    fn test_intensity_range() {
        for floor_id in [1, 50, 200, 400, 600, 999] {
            let mutators = generate_floor_mutators(42, floor_id);
            for m in &mutators {
                assert!(
                    m.intensity >= 0.3 && m.intensity <= 2.0,
                    "Intensity {} out of range for floor {}",
                    m.intensity,
                    floor_id
                );
            }
        }
    }

    #[test]
    fn test_compute_effects_default() {
        let fx = compute_effects(&[]);
        assert!((fx.damage_dealt_mult - 1.0).abs() < f32::EPSILON);
        assert!((fx.healing_mult - 1.0).abs() < f32::EPSILON);
        assert!((fx.reward_multiplier - 1.0).abs() < f32::EPSILON);
        assert!(!fx.permadeath);
        assert!(fx.time_limit_secs.is_none());
    }

    #[test]
    fn test_compute_effects_double_damage() {
        let mutators = vec![FloorMutator::from_type(MutatorType::DoubleDamage, 1.0)];
        let fx = compute_effects(&mutators);
        assert!((fx.damage_dealt_mult - 2.0).abs() < f32::EPSILON);
        assert!((fx.damage_taken_mult - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_compute_effects_no_healing() {
        let mutators = vec![FloorMutator::from_type(MutatorType::NoHealing, 1.0)];
        let fx = compute_effects(&mutators);
        assert!((fx.healing_mult).abs() < f32::EPSILON);
    }

    #[test]
    fn test_compute_effects_ironman() {
        let mutators = vec![FloorMutator::from_type(MutatorType::Ironman, 1.0)];
        let fx = compute_effects(&mutators);
        assert!(fx.permadeath);
    }

    #[test]
    fn test_compute_effects_time_trial() {
        let mutators = vec![FloorMutator::from_type(MutatorType::TimeTrial, 1.0)];
        let fx = compute_effects(&mutators);
        assert!(fx.time_limit_secs.is_some());
        assert!((fx.time_limit_secs.unwrap() - 300.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_compute_effects_stacking() {
        let mutators = vec![
            FloorMutator::from_type(MutatorType::DoubleDamage, 1.0),
            FloorMutator::from_type(MutatorType::Darkness, 1.0),
            FloorMutator::from_type(MutatorType::Bountiful, 1.0),
        ];
        let fx = compute_effects(&mutators);
        assert!(fx.damage_dealt_mult > 1.5);
        assert!(fx.visibility_mult < 0.5);
        assert!(fx.loot_quantity_mult > 1.5);
        assert!(fx.total_difficulty > 5);
        assert!(fx.reward_multiplier > 1.5);
    }

    #[test]
    fn test_compute_effects_reward_scaling() {
        // Higher difficulty = higher rewards
        let easy = vec![FloorMutator::from_type(MutatorType::SpeedBoost, 1.0)]; // diff 1
        let hard = vec![FloorMutator::from_type(MutatorType::Ironman, 1.0)]; // diff 5
        let fx_easy = compute_effects(&easy);
        let fx_hard = compute_effects(&hard);
        assert!(fx_hard.reward_multiplier > fx_easy.reward_multiplier);
    }

    #[test]
    fn test_all_mutator_types_complete() {
        let all = all_mutator_types();
        assert_eq!(all.len(), 28);

        // Every category represented
        let cats: std::collections::HashSet<_> = all.iter().map(|m| m.category).collect();
        assert_eq!(cats.len(), 5);
    }

    #[test]
    fn test_mutator_descriptions_not_empty() {
        for mt in &ALL_MUTATORS {
            assert!(
                !mt.description().is_empty(),
                "{:?} has empty description",
                mt
            );
            assert!(!mt.icon_id().is_empty(), "{:?} has empty icon_id", mt);
        }
    }

    #[test]
    fn test_generate_mutator_set() {
        let set = generate_mutator_set(42, 300);
        assert_eq!(set.floor_id, 300);
        assert_eq!(set.tier, "Echelon2");
        assert_eq!(set.mutators.len(), 2);
        assert!(set.effects.reward_multiplier >= 1.0);
    }

    #[test]
    fn test_mutator_category_consistency() {
        for mt in &ALL_MUTATORS {
            let mutator = FloorMutator::from_type(mt.clone(), 1.0);
            assert_eq!(
                mutator.category,
                mt.category(),
                "Category mismatch for {:?}",
                mt
            );
        }
    }

    #[test]
    fn test_difficulty_rating_range() {
        for mt in &ALL_MUTATORS {
            let d = mt.difficulty_rating();
            assert!(
                (1..=5).contains(&d),
                "{:?} has invalid difficulty {}",
                mt,
                d
            );
        }
    }

    #[test]
    fn test_scarcity_rarity_bonus() {
        let mutators = vec![FloorMutator::from_type(MutatorType::Scarcity, 1.0)];
        let fx = compute_effects(&mutators);
        assert!(fx.loot_quantity_mult < 1.0);
        assert!(fx.loot_rarity_bonus > 0);
    }

    #[test]
    fn test_golden_floor_no_equipment() {
        let mutators = vec![FloorMutator::from_type(MutatorType::GoldenFloor, 1.0)];
        let fx = compute_effects(&mutators);
        assert!((fx.loot_quantity_mult).abs() < f32::EPSILON);
        assert!(fx.shard_mult > 2.0);
    }
}
