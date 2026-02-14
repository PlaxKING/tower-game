//! Parry, dodge, and block defensive mechanics.
//!
//! - Parry: precise timing window (120ms), reflects portion of damage, staggers attacker
//! - Dodge: i-frames (200ms), costs kinetic energy, directional
//! - Block: reduces damage by %, drains thermal energy per hit

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::{AttackPhase, CombatResources, CombatState};

/// Defensive state for parry/dodge/block
#[derive(Component, Debug)]
pub struct DefenseState {
    pub action: DefenseAction,
    pub timer: f32,
    pub direction: Vec3,
}

impl Default for DefenseState {
    fn default() -> Self {
        Self {
            action: DefenseAction::None,
            timer: 0.0,
            direction: Vec3::ZERO,
        }
    }
}

/// Active defense action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefenseAction {
    None,
    Parry, // reflect + stagger
    Dodge, // i-frames + reposition
    Block, // damage reduction + drain
}

/// Parry timing constants
const PARRY_WINDOW: f32 = 0.12; // 120ms - must match attack Active phase
const PARRY_STARTUP: f32 = 0.05; // 50ms before active window
const PARRY_RECOVERY: f32 = 0.30; // 300ms if missed
const PARRY_TOTAL: f32 = PARRY_STARTUP + PARRY_WINDOW + PARRY_RECOVERY;

/// Dodge constants
const DODGE_IFRAMES: f32 = 0.20; // 200ms invulnerable
const DODGE_DISTANCE: f32 = 4.0;
const DODGE_COST_KINETIC: f32 = 15.0;
const DODGE_COOLDOWN: f32 = 0.5;

/// Block constants
const BLOCK_REDUCTION: f32 = 0.7; // 70% damage reduction
const BLOCK_DRAIN_PER_HIT: f32 = 10.0;

/// Result of a defense attempt
#[derive(Debug, Clone)]
pub enum DefenseResult {
    /// Parry succeeded — attacker is staggered
    ParrySuccess { quality: f32 },
    /// Block absorbed damage
    BlockAbsorb { absorbed: f32, remaining: f32 },
    /// Dodge — attack missed
    DodgeSuccess,
    /// No defense active — take full damage
    NoDefense,
    /// Parry failed (wrong timing)
    ParryFailed,
}

/// Cooldown tracker for dodge
#[derive(Component, Debug)]
pub struct DodgeCooldown {
    pub remaining: f32,
}

impl Default for DodgeCooldown {
    fn default() -> Self {
        Self { remaining: 0.0 }
    }
}

/// Check if incoming damage is defended
pub fn check_defense(
    defense: &DefenseState,
    _incoming_damage: f32,
    resources: &CombatResources,
) -> DefenseResult {
    match defense.action {
        DefenseAction::Parry => {
            let t = defense.timer;
            if (PARRY_STARTUP..PARRY_STARTUP + PARRY_WINDOW).contains(&t) {
                // Perfect parry window
                let window_progress = (t - PARRY_STARTUP) / PARRY_WINDOW;
                // Best quality at center of window
                let quality = 1.0 - (window_progress - 0.5).abs() * 2.0;
                DefenseResult::ParrySuccess {
                    quality: quality.max(0.3),
                }
            } else {
                DefenseResult::ParryFailed
            }
        }
        DefenseAction::Dodge => {
            if defense.timer < DODGE_IFRAMES {
                DefenseResult::DodgeSuccess
            } else {
                DefenseResult::NoDefense
            }
        }
        DefenseAction::Block => {
            if resources.thermal_energy >= BLOCK_DRAIN_PER_HIT {
                let absorbed = _incoming_damage * BLOCK_REDUCTION;
                let remaining = _incoming_damage - absorbed;
                DefenseResult::BlockAbsorb {
                    absorbed,
                    remaining,
                }
            } else {
                // No energy to block — guard break
                DefenseResult::NoDefense
            }
        }
        DefenseAction::None => DefenseResult::NoDefense,
    }
}

/// System: process defense timers
pub fn update_defense_state(
    time: Res<Time>,
    mut query: Query<(&mut DefenseState, &mut DodgeCooldown)>,
) {
    let dt = time.delta_secs();
    for (mut defense, mut cooldown) in &mut query {
        // Tick cooldown
        cooldown.remaining = (cooldown.remaining - dt).max(0.0);

        match defense.action {
            DefenseAction::None => {}
            DefenseAction::Parry => {
                defense.timer += dt;
                if defense.timer >= PARRY_TOTAL {
                    defense.action = DefenseAction::None;
                    defense.timer = 0.0;
                }
            }
            DefenseAction::Dodge => {
                defense.timer += dt;
                if defense.timer >= DODGE_IFRAMES * 1.5 {
                    defense.action = DefenseAction::None;
                    defense.timer = 0.0;
                }
            }
            DefenseAction::Block => {
                // Block stays active as long as held (handled by input)
            }
        }
    }
}

/// System: apply dodge movement
pub fn apply_dodge_movement(time: Res<Time>, mut query: Query<(&mut Transform, &DefenseState)>) {
    let dt = time.delta_secs();
    for (mut transform, defense) in &mut query {
        if defense.action == DefenseAction::Dodge && defense.timer < DODGE_IFRAMES {
            let speed = DODGE_DISTANCE / DODGE_IFRAMES;
            transform.translation += defense.direction * speed * dt;
        }
    }
}

/// Initiate a parry
pub fn start_parry(defense: &mut DefenseState, combat: &CombatState) -> bool {
    if defense.action != DefenseAction::None {
        return false;
    }
    if combat.phase != AttackPhase::Idle {
        return false; // Can't parry while attacking
    }
    defense.action = DefenseAction::Parry;
    defense.timer = 0.0;
    true
}

/// Initiate a dodge
pub fn start_dodge(
    defense: &mut DefenseState,
    cooldown: &mut DodgeCooldown,
    resources: &mut CombatResources,
    direction: Vec3,
) -> bool {
    if defense.action != DefenseAction::None {
        return false;
    }
    if cooldown.remaining > 0.0 {
        return false;
    }
    if resources.kinetic_energy < DODGE_COST_KINETIC {
        return false;
    }

    resources.kinetic_energy -= DODGE_COST_KINETIC;
    cooldown.remaining = DODGE_COOLDOWN;
    defense.action = DefenseAction::Dodge;
    defense.timer = 0.0;
    defense.direction = if direction.length_squared() > 0.01 {
        direction.normalize()
    } else {
        Vec3::NEG_Z // default backward dodge
    };
    true
}

/// Initiate block
pub fn start_block(defense: &mut DefenseState) -> bool {
    if defense.action != DefenseAction::None && defense.action != DefenseAction::Block {
        return false;
    }
    defense.action = DefenseAction::Block;
    defense.timer = 0.0;
    true
}

/// Release block
pub fn stop_block(defense: &mut DefenseState) {
    if defense.action == DefenseAction::Block {
        defense.action = DefenseAction::None;
        defense.timer = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parry_timing_perfect() {
        let defense = DefenseState {
            action: DefenseAction::Parry,
            timer: PARRY_STARTUP + PARRY_WINDOW / 2.0, // center of window
            direction: Vec3::ZERO,
        };
        let resources = CombatResources::default();
        match check_defense(&defense, 50.0, &resources) {
            DefenseResult::ParrySuccess { quality } => {
                assert!(
                    quality > 0.8,
                    "Center of parry window should give high quality, got {quality}"
                );
            }
            other => panic!("Expected ParrySuccess, got {:?}", other),
        }
    }

    #[test]
    fn test_parry_timing_miss() {
        let defense = DefenseState {
            action: DefenseAction::Parry,
            timer: PARRY_TOTAL - 0.01, // in recovery
            direction: Vec3::ZERO,
        };
        let resources = CombatResources::default();
        match check_defense(&defense, 50.0, &resources) {
            DefenseResult::ParryFailed => {} // expected
            other => panic!("Expected ParryFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_dodge_iframes() {
        let defense = DefenseState {
            action: DefenseAction::Dodge,
            timer: 0.05, // within i-frames
            direction: Vec3::NEG_Z,
        };
        let resources = CombatResources::default();
        match check_defense(&defense, 100.0, &resources) {
            DefenseResult::DodgeSuccess => {} // expected
            other => panic!("Expected DodgeSuccess, got {:?}", other),
        }
    }

    #[test]
    fn test_block_absorb() {
        let defense = DefenseState {
            action: DefenseAction::Block,
            timer: 0.0,
            direction: Vec3::ZERO,
        };
        let resources = CombatResources::default();
        match check_defense(&defense, 100.0, &resources) {
            DefenseResult::BlockAbsorb {
                absorbed,
                remaining,
            } => {
                assert!((absorbed - 70.0).abs() < 0.1, "Should absorb 70%");
                assert!((remaining - 30.0).abs() < 0.1, "Should pass 30%");
            }
            other => panic!("Expected BlockAbsorb, got {:?}", other),
        }
    }

    #[test]
    fn test_block_guard_break() {
        let defense = DefenseState {
            action: DefenseAction::Block,
            timer: 0.0,
            direction: Vec3::ZERO,
        };
        let resources = CombatResources {
            thermal_energy: 0.0, // no energy
            ..Default::default()
        };
        match check_defense(&defense, 100.0, &resources) {
            DefenseResult::NoDefense => {} // guard break
            other => panic!("Expected NoDefense (guard break), got {:?}", other),
        }
    }

    #[test]
    fn test_start_dodge_costs_energy() {
        let mut defense = DefenseState::default();
        let mut cooldown = DodgeCooldown::default();
        let mut resources = CombatResources::default();
        let initial_kinetic = resources.kinetic_energy;

        let success = start_dodge(&mut defense, &mut cooldown, &mut resources, Vec3::X);
        assert!(success);
        assert!(resources.kinetic_energy < initial_kinetic);
        assert_eq!(defense.action, DefenseAction::Dodge);
    }

    #[test]
    fn test_dodge_cooldown() {
        let mut defense = DefenseState::default();
        let mut cooldown = DodgeCooldown::default();
        let mut resources = CombatResources::default();

        // First dodge succeeds
        assert!(start_dodge(
            &mut defense,
            &mut cooldown,
            &mut resources,
            Vec3::X
        ));

        // Reset defense for second attempt
        defense.action = DefenseAction::None;

        // Second dodge fails (cooldown)
        assert!(!start_dodge(
            &mut defense,
            &mut cooldown,
            &mut resources,
            Vec3::X
        ));
    }

    #[test]
    fn test_parry_during_attack_fails() {
        let mut defense = DefenseState::default();
        let combat = CombatState {
            phase: AttackPhase::Active,
            ..Default::default()
        };
        assert!(!start_parry(&mut defense, &combat));
    }
}
