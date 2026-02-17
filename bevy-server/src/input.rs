//! Player Input Processing — types, validation, and anti-cheat
//!
//! Input arrives from two paths:
//! 1. **Renet UDP** — real-time, via bevy_replicon client events (for bevy test client)
//! 2. **HTTP API** — request/response, via ECS bridge GameCommands (for UE5 client)
//!
//! Both paths use the same validation logic defined here.

use bevy::prelude::*;
use serde::{Serialize, Deserialize};

// ============================================================================
// Input Types
// ============================================================================

/// Player input event sent from client to server.
/// Contains movement direction, facing, and optional combat/interaction actions.
#[derive(Event, Serialize, Deserialize, Debug, Clone)]
pub struct PlayerInput {
    /// Movement direction (world-space). Magnitude = desired speed (clamped server-side).
    pub movement: [f32; 3],
    /// Facing direction (yaw in radians)
    pub facing: f32,
    /// Combat/interaction action (None = just moving)
    pub action: Option<InputAction>,
    /// Input sequence number (for client-side prediction reconciliation)
    pub sequence: u32,
}

/// Available player actions sent as input
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputAction {
    Attack,
    Block,
    BlockRelease,
    Parry,
    Dodge,
    HeavyAttack,
    Interact,
}

impl InputAction {
    /// Convert to combat system ActionType. Returns None for non-combat actions.
    pub fn to_combat_action(self) -> Option<crate::combat::ActionType> {
        match self {
            InputAction::Attack => Some(crate::combat::ActionType::Attack),
            InputAction::Block => Some(crate::combat::ActionType::Block),
            InputAction::BlockRelease => Some(crate::combat::ActionType::BlockRelease),
            InputAction::Parry => Some(crate::combat::ActionType::Parry),
            InputAction::Dodge => Some(crate::combat::ActionType::Dodge),
            InputAction::HeavyAttack => Some(crate::combat::ActionType::HeavyAttack),
            InputAction::Interact => None,
        }
    }
}

// ============================================================================
// Validation Constants
// ============================================================================

/// Maximum player movement speed (units per second)
pub const MAX_MOVE_SPEED: f32 = 10.0;
/// Tolerance multiplier for network lag compensation (allows 50% over max)
pub const SPEED_TOLERANCE: f32 = 1.5;
/// Maximum facing change per tick (anti-spinbot, radians)
pub const MAX_FACING_DELTA: f32 = std::f32::consts::TAU;

// ============================================================================
// Validation Functions
// ============================================================================

/// Validate and sanitize player movement input.
/// Returns validated movement vector (clamped to max speed) or None if rejected.
pub fn validate_movement(movement: [f32; 3]) -> Option<Vec3> {
    let mv = Vec3::new(movement[0], movement[1], movement[2]);

    // Reject NaN/Inf
    if !mv.x.is_finite() || !mv.y.is_finite() || !mv.z.is_finite() {
        return None;
    }

    let speed = mv.length();

    // Reject impossible speeds (anti-speed-hack)
    if speed > MAX_MOVE_SPEED * SPEED_TOLERANCE {
        return None;
    }

    // Clamp to max speed
    if speed > MAX_MOVE_SPEED {
        Some(mv.normalize() * MAX_MOVE_SPEED)
    } else {
        Some(mv)
    }
}

/// Validate facing direction (must be finite, within valid range)
pub fn validate_facing(facing: f32) -> f32 {
    if !facing.is_finite() {
        return 0.0;
    }
    // Normalize to [0, TAU)
    facing.rem_euclid(std::f32::consts::TAU)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_movement_normal() {
        let result = validate_movement([3.0, 0.0, 4.0]); // speed = 5
        assert!(result.is_some());
        let v = result.unwrap();
        assert!((v.length() - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_validate_movement_clamped() {
        let result = validate_movement([12.0, 0.0, 0.0]); // speed = 12 > MAX_MOVE_SPEED
        assert!(result.is_some());
        let v = result.unwrap();
        assert!((v.length() - MAX_MOVE_SPEED).abs() < 0.01);
    }

    #[test]
    fn test_validate_movement_rejected() {
        // Speed > MAX_MOVE_SPEED * SPEED_TOLERANCE = 15
        let result = validate_movement([20.0, 0.0, 0.0]);
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_movement_zero() {
        let result = validate_movement([0.0, 0.0, 0.0]);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), Vec3::ZERO);
    }

    #[test]
    fn test_validate_movement_nan_rejected() {
        assert!(validate_movement([f32::NAN, 0.0, 0.0]).is_none());
        assert!(validate_movement([0.0, f32::INFINITY, 0.0]).is_none());
        assert!(validate_movement([0.0, 0.0, f32::NEG_INFINITY]).is_none());
    }

    #[test]
    fn test_validate_movement_preserves_direction() {
        let result = validate_movement([6.0, 0.0, 8.0]).unwrap(); // speed 10, exactly MAX
        assert!((result.x - 6.0).abs() < 0.01);
        assert!((result.z - 8.0).abs() < 0.01);
    }

    #[test]
    fn test_validate_facing_normal() {
        assert!((validate_facing(1.5) - 1.5).abs() < 0.01);
    }

    #[test]
    fn test_validate_facing_negative() {
        let result = validate_facing(-1.0);
        assert!(result >= 0.0);
        assert!(result < std::f32::consts::TAU);
    }

    #[test]
    fn test_validate_facing_nan() {
        assert_eq!(validate_facing(f32::NAN), 0.0);
        assert_eq!(validate_facing(f32::INFINITY), 0.0);
    }

    #[test]
    fn test_input_action_to_combat() {
        assert_eq!(
            InputAction::Attack.to_combat_action(),
            Some(crate::combat::ActionType::Attack)
        );
        assert_eq!(
            InputAction::Parry.to_combat_action(),
            Some(crate::combat::ActionType::Parry)
        );
        assert_eq!(
            InputAction::Dodge.to_combat_action(),
            Some(crate::combat::ActionType::Dodge)
        );
        assert_eq!(InputAction::Interact.to_combat_action(), None);
    }

    #[test]
    fn test_all_combat_actions_convert() {
        // All combat actions should convert to Some
        let combat_actions = [
            InputAction::Attack,
            InputAction::Block,
            InputAction::BlockRelease,
            InputAction::Parry,
            InputAction::Dodge,
            InputAction::HeavyAttack,
        ];
        for action in combat_actions {
            assert!(action.to_combat_action().is_some(), "{:?} should convert", action);
        }
    }

    #[test]
    fn test_player_input_serialization() {
        let input = PlayerInput {
            movement: [1.0, 0.0, -1.0],
            facing: 1.57,
            action: Some(InputAction::Attack),
            sequence: 42,
        };
        let bytes = bincode::serialize(&input).unwrap();
        let decoded: PlayerInput = bincode::deserialize(&bytes).unwrap();
        assert_eq!(decoded.sequence, 42);
        assert_eq!(decoded.action, Some(InputAction::Attack));
        assert!((decoded.facing - 1.57).abs() < 0.001);
    }
}
