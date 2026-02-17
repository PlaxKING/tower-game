//! Physics Integration — bevy_rapier3d collision layers, helpers, and knockback
//!
//! Provides:
//! - Collision group constants (PLAYER, ENEMY, WALL, HAZARD, PROJECTILE, DESTRUCTIBLE)
//! - Helper functions for spawning entities with physics components
//! - Knockback component and system for combat knockback
//! - All entities use KinematicPositionBased (server-authoritative positioning)

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::monster_gen::MonsterSize;

// ============================================================================
// Collision Layers
// ============================================================================

/// Collision group constants for physics filtering.
/// Each entity type belongs to a membership group and filters which groups it interacts with.
pub struct PhysicsLayers;

impl PhysicsLayers {
    /// Player characters
    pub const PLAYER: Group = Group::GROUP_1;
    /// Enemy monsters and NPCs
    pub const ENEMY: Group = Group::GROUP_2;
    /// Static walls and floor geometry
    pub const WALL: Group = Group::GROUP_3;
    /// Environmental hazards (lava, traps, etc.)
    pub const HAZARD: Group = Group::GROUP_4;
    /// Projectiles (arrows, spells, etc.)
    pub const PROJECTILE: Group = Group::GROUP_5;
    /// Destructible environment objects
    pub const DESTRUCTIBLE: Group = Group::GROUP_6;
    /// Sensor volumes (aggro range, trigger zones)
    pub const SENSOR: Group = Group::GROUP_7;
}

// ============================================================================
// Physics Component Bundles
// ============================================================================

/// Physics components for a player character.
/// Capsule collider (~1.8m tall), kinematic (server-controlled position).
pub fn player_physics_bundle() -> (RigidBody, Collider, CollisionGroups) {
    (
        RigidBody::KinematicPositionBased,
        Collider::capsule_y(0.9, 0.4), // 1.8m + 0.8m diameter
        CollisionGroups::new(
            PhysicsLayers::PLAYER,
            PhysicsLayers::ENEMY | PhysicsLayers::WALL | PhysicsLayers::HAZARD | PhysicsLayers::PROJECTILE,
        ),
    )
}

/// Physics components for a monster, sized by MonsterSize.
/// Capsule collider scaled to match monster size, kinematic (server-controlled).
pub fn monster_physics_bundle(size: MonsterSize) -> (RigidBody, Collider, CollisionGroups) {
    let (half_height, radius) = monster_capsule_dimensions(size);
    (
        RigidBody::KinematicPositionBased,
        Collider::capsule_y(half_height, radius),
        CollisionGroups::new(
            PhysicsLayers::ENEMY,
            PhysicsLayers::PLAYER | PhysicsLayers::WALL | PhysicsLayers::ENEMY,
        ),
    )
}

/// Get capsule dimensions (half_height, radius) for a monster size.
pub fn monster_capsule_dimensions(size: MonsterSize) -> (f32, f32) {
    match size {
        MonsterSize::Tiny     => (0.3, 0.2),   // ~1.0m total
        MonsterSize::Small    => (0.5, 0.3),   // ~1.6m total
        MonsterSize::Medium   => (0.9, 0.4),   // ~2.6m total
        MonsterSize::Large    => (1.5, 0.8),   // ~4.6m total
        MonsterSize::Colossal => (2.5, 1.5),   // ~8.0m total
    }
}

/// Physics components for a static wall.
/// Fixed rigid body with cuboid collider, collides with everything.
pub fn wall_physics_bundle(half_extents: Vec3) -> (RigidBody, Collider, CollisionGroups) {
    (
        RigidBody::Fixed,
        Collider::cuboid(half_extents.x, half_extents.y, half_extents.z),
        CollisionGroups::new(
            PhysicsLayers::WALL,
            Group::ALL,
        ),
    )
}

/// Physics components for a destructible object.
/// Fixed rigid body with collision events enabled for destruction detection.
pub fn destructible_physics_bundle(half_extents: Vec3) -> (RigidBody, Collider, CollisionGroups, ActiveEvents) {
    (
        RigidBody::Fixed,
        Collider::cuboid(half_extents.x, half_extents.y, half_extents.z),
        CollisionGroups::new(
            PhysicsLayers::DESTRUCTIBLE,
            PhysicsLayers::PLAYER | PhysicsLayers::ENEMY | PhysicsLayers::PROJECTILE,
        ),
        ActiveEvents::COLLISION_EVENTS,
    )
}

/// Physics components for an environmental hazard (lava, spikes, etc.).
/// Sensor collider — detects overlap without physical collision response.
pub fn hazard_sensor_bundle(half_extents: Vec3) -> (Collider, Sensor, CollisionGroups, ActiveEvents) {
    (
        Collider::cuboid(half_extents.x, half_extents.y, half_extents.z),
        Sensor,
        CollisionGroups::new(
            PhysicsLayers::HAZARD,
            PhysicsLayers::PLAYER | PhysicsLayers::ENEMY,
        ),
        ActiveEvents::COLLISION_EVENTS,
    )
}

// ============================================================================
// Knockback System
// ============================================================================

/// Applied to an entity to push it in a direction over time.
/// Removed automatically when duration expires.
#[derive(Component, Debug)]
pub struct Knockback {
    /// Current knockback velocity (units/s)
    pub velocity: Vec3,
    /// Time remaining (seconds)
    pub remaining: f32,
    /// Deceleration rate (units/s^2)
    pub drag: f32,
}

impl Knockback {
    /// Create knockback in a direction with given force and duration.
    /// Force is initial speed (units/s), decelerates linearly to zero over duration.
    pub fn new(direction: Vec3, force: f32, duration: f32) -> Self {
        Self {
            velocity: direction.normalize_or_zero() * force,
            remaining: duration,
            drag: if duration > 0.0 { force / duration } else { 0.0 },
        }
    }
}

/// System: Apply knockback movement to entities and remove when finished.
pub fn apply_knockback(
    time: Res<Time>,
    mut commands: Commands,
    mut entities: Query<(Entity, &mut Transform, &mut Knockback)>,
) {
    let dt = time.delta_secs();

    for (entity, mut transform, mut kb) in &mut entities {
        kb.remaining -= dt;
        if kb.remaining <= 0.0 {
            commands.entity(entity).remove::<Knockback>();
            continue;
        }

        // Apply velocity to position
        transform.translation += kb.velocity * dt;

        // Linear deceleration
        let speed = kb.velocity.length();
        let drag_amount = kb.drag * dt;
        if speed > drag_amount {
            kb.velocity = kb.velocity.normalize() * (speed - drag_amount);
        } else {
            kb.velocity = Vec3::ZERO;
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collision_layers_distinct() {
        let layers = [
            PhysicsLayers::PLAYER,
            PhysicsLayers::ENEMY,
            PhysicsLayers::WALL,
            PhysicsLayers::HAZARD,
            PhysicsLayers::PROJECTILE,
            PhysicsLayers::DESTRUCTIBLE,
            PhysicsLayers::SENSOR,
        ];
        for i in 0..layers.len() {
            for j in (i + 1)..layers.len() {
                assert_ne!(layers[i], layers[j], "Layers {} and {} should be distinct", i, j);
            }
        }
    }

    #[test]
    fn test_player_physics_is_kinematic() {
        let (rigid_body, _collider, groups) = player_physics_bundle();
        assert!(matches!(rigid_body, RigidBody::KinematicPositionBased));
        assert!(groups.filters.contains(PhysicsLayers::ENEMY));
        assert!(groups.filters.contains(PhysicsLayers::WALL));
        assert!(!groups.filters.contains(PhysicsLayers::PLAYER));
    }

    #[test]
    fn test_monster_physics_is_kinematic() {
        let (rigid_body, _collider, groups) = monster_physics_bundle(MonsterSize::Medium);
        assert!(matches!(rigid_body, RigidBody::KinematicPositionBased));
        assert!(groups.filters.contains(PhysicsLayers::PLAYER));
        assert!(groups.filters.contains(PhysicsLayers::WALL));
    }

    #[test]
    fn test_monster_size_colliders_scale() {
        let dims: Vec<(f32, f32)> = [
            MonsterSize::Tiny,
            MonsterSize::Small,
            MonsterSize::Medium,
            MonsterSize::Large,
            MonsterSize::Colossal,
        ].iter().map(|s| monster_capsule_dimensions(*s)).collect();

        for i in 1..dims.len() {
            assert!(dims[i].0 > dims[i - 1].0, "Half-height should increase with size");
            assert!(dims[i].1 > dims[i - 1].1, "Radius should increase with size");
        }
    }

    #[test]
    fn test_wall_is_fixed() {
        let (rigid_body, _collider, groups) = wall_physics_bundle(Vec3::ONE);
        assert!(matches!(rigid_body, RigidBody::Fixed));
        assert_eq!(groups.filters, Group::ALL);
    }

    #[test]
    fn test_destructible_has_events() {
        let (_rb, _collider, _groups, events) = destructible_physics_bundle(Vec3::ONE);
        assert!(events.contains(ActiveEvents::COLLISION_EVENTS));
    }

    #[test]
    fn test_hazard_is_sensor() {
        let (_collider, _sensor, groups, events) = hazard_sensor_bundle(Vec3::ONE);
        assert!(groups.filters.contains(PhysicsLayers::PLAYER));
        assert!(groups.filters.contains(PhysicsLayers::ENEMY));
        assert!(events.contains(ActiveEvents::COLLISION_EVENTS));
    }

    #[test]
    fn test_knockback_creation() {
        let kb = Knockback::new(Vec3::X, 10.0, 0.5);
        assert!((kb.velocity.length() - 10.0).abs() < 0.01);
        assert!((kb.remaining - 0.5).abs() < 0.01);
        assert!((kb.drag - 20.0).abs() < 0.01); // 10.0 / 0.5
    }

    #[test]
    fn test_knockback_direction_normalized() {
        let kb = Knockback::new(Vec3::new(3.0, 0.0, 4.0), 10.0, 1.0);
        let dir = kb.velocity.normalize();
        assert!((dir.x - 0.6).abs() < 0.01);
        assert!((dir.z - 0.8).abs() < 0.01);
        assert!((kb.velocity.length() - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_knockback_zero_direction() {
        let kb = Knockback::new(Vec3::ZERO, 10.0, 1.0);
        assert_eq!(kb.velocity, Vec3::ZERO);
    }

    #[test]
    fn test_knockback_zero_duration() {
        let kb = Knockback::new(Vec3::X, 10.0, 0.0);
        assert_eq!(kb.drag, 0.0);
    }
}
