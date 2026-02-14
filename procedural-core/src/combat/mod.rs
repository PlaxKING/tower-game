use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub mod defense;
pub mod hitbox;
pub mod status;
pub mod weapons;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<hitbox::DamageEvent>().add_systems(
            Update,
            (
                process_attack_timing,
                weapons::weapon_combo_system,
                hitbox::spawn_attack_hitboxes,
                hitbox::update_hitbox_lifetime,
                hitbox::process_hitbox_collisions,
                hitbox::update_invulnerability,
                hitbox::apply_stagger,
                weapons::regenerate_resources,
                defense::update_defense_state,
                defense::apply_dodge_movement,
                status::tick_status_effects,
            )
                .chain(),
        );
    }
}

/// Attack phase for timing-based combat
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttackPhase {
    Idle,
    Windup,   // yellow glow, 300ms
    Active,   // red flash, 120ms - parry window
    Recovery, // blue trail, 400ms - vulnerable
}

/// Quality of player execution (0.0 = miss, 1.0 = perfect)
#[derive(Debug, Clone, Copy)]
pub struct ExecutionQuality(pub f32);

impl ExecutionQuality {
    /// Calculate damage multiplier from quality
    pub fn damage_multiplier(&self) -> f32 {
        1.0 + self.0 * 0.5 // +0% to +50% bonus
    }

    /// Calculate stun duration from parry quality
    pub fn stun_duration_ms(&self) -> u32 {
        (self.0 * 300.0) as u32 // 0 to 300ms
    }
}

/// Angle of attack relative to target facing direction
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AttackAngle {
    Front, // 1.0x damage
    Side,  // 0.7x damage
    Back,  // 1.5x damage
}

impl AttackAngle {
    pub fn multiplier(&self) -> f32 {
        match self {
            Self::Front => 1.0,
            Self::Side => 0.7,
            Self::Back => 1.5,
        }
    }

    /// Determine angle from attacker and target transforms
    pub fn from_transforms(attacker: &Transform, target: &Transform) -> Self {
        let to_attacker = (attacker.translation - target.translation).normalize();
        let target_forward = target.forward().as_vec3();
        let dot = target_forward.dot(to_attacker);

        if dot > 0.5 {
            Self::Front
        } else if dot < -0.5 {
            Self::Back
        } else {
            Self::Side
        }
    }
}

/// Combat state component attached to fighters
#[derive(Component, Debug)]
pub struct CombatState {
    pub phase: AttackPhase,
    pub phase_timer: f32,
    pub combo_step: u32,
    pub max_combo: u32,
}

impl Default for CombatState {
    fn default() -> Self {
        Self {
            phase: AttackPhase::Idle,
            phase_timer: 0.0,
            combo_step: 0,
            max_combo: 3,
        }
    }
}

/// Combat resource types
#[derive(Component, Debug, Serialize, Deserialize)]
pub struct CombatResources {
    pub kinetic_energy: f32,  // from movement/diving
    pub thermal_energy: f32,  // from hovering/defending
    pub semantic_energy: f32, // from reading/analyzing
    pub rage: f32,            // from taking damage
}

impl Default for CombatResources {
    fn default() -> Self {
        Self {
            kinetic_energy: 100.0,
            thermal_energy: 100.0,
            semantic_energy: 50.0,
            rage: 0.0,
        }
    }
}

// Phase durations in seconds
const WINDUP_DURATION: f32 = 0.3;
const ACTIVE_DURATION: f32 = 0.12;
const RECOVERY_DURATION: f32 = 0.4;

fn process_attack_timing(time: Res<Time>, mut query: Query<&mut CombatState>) {
    let dt = time.delta_secs();

    for mut state in &mut query {
        if state.phase == AttackPhase::Idle {
            continue;
        }

        state.phase_timer += dt;

        let phase_duration = match state.phase {
            AttackPhase::Windup => WINDUP_DURATION,
            AttackPhase::Active => ACTIVE_DURATION,
            AttackPhase::Recovery => RECOVERY_DURATION,
            AttackPhase::Idle => continue,
        };

        if state.phase_timer >= phase_duration {
            state.phase_timer = 0.0;
            state.phase = match state.phase {
                AttackPhase::Windup => AttackPhase::Active,
                AttackPhase::Active => AttackPhase::Recovery,
                AttackPhase::Recovery => {
                    state.combo_step = 0;
                    AttackPhase::Idle
                }
                AttackPhase::Idle => AttackPhase::Idle,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_angle_multipliers() {
        assert!((AttackAngle::Front.multiplier() - 1.0).abs() < f32::EPSILON);
        assert!((AttackAngle::Side.multiplier() - 0.7).abs() < f32::EPSILON);
        assert!((AttackAngle::Back.multiplier() - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_execution_quality() {
        let perfect = ExecutionQuality(1.0);
        assert!((perfect.damage_multiplier() - 1.5).abs() < f32::EPSILON);
        assert_eq!(perfect.stun_duration_ms(), 300);

        let miss = ExecutionQuality(0.0);
        assert!((miss.damage_multiplier() - 1.0).abs() < f32::EPSILON);
        assert_eq!(miss.stun_duration_ms(), 0);
    }
}
