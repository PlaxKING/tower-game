//! Physics-based hitbox system using bevy_rapier3d.
//!
//! Each attack spawns a hitbox collider with a short lifetime.
//! Hitboxes carry damage info and detect overlaps with hurtboxes.

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use super::{AttackAngle, AttackPhase, CombatState};

/// Hitbox spawned during attack Active phase
#[derive(Component, Debug)]
pub struct Hitbox {
    pub owner: Entity,
    pub base_damage: f32,
    pub knockback: f32,
    pub hit_entities: Vec<Entity>,
    pub lifetime: f32,
}

/// Hurtbox attached to damageable entities
#[derive(Component, Debug)]
pub struct Hurtbox {
    pub owner: Entity,
}

/// Component tracking health and damage
#[derive(Component, Debug)]
pub struct Health {
    pub current: f32,
    pub max: f32,
    pub invulnerable_timer: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self {
            current: max,
            max,
            invulnerable_timer: 0.0,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.current > 0.0
    }

    pub fn take_damage(&mut self, amount: f32) -> f32 {
        if self.invulnerable_timer > 0.0 {
            return 0.0;
        }
        let actual = amount.min(self.current);
        self.current -= actual;
        self.invulnerable_timer = 0.1; // brief i-frames after hit
        actual
    }
}

/// Damage event for UI/effects
#[derive(Event, Debug)]
pub struct DamageEvent {
    pub target: Entity,
    pub attacker: Entity,
    pub amount: f32,
    pub angle: AttackAngle,
    pub position: Vec3,
}

/// Stagger/knockback state
#[derive(Component, Debug)]
pub struct Stagger {
    pub timer: f32,
    pub direction: Vec3,
    pub strength: f32,
}

impl Default for Stagger {
    fn default() -> Self {
        Self {
            timer: 0.0,
            direction: Vec3::ZERO,
            strength: 0.0,
        }
    }
}

/// Spawn a melee hitbox in front of an entity during Active phase
pub fn spawn_attack_hitboxes(
    mut commands: Commands,
    query: Query<(Entity, &CombatState, &Transform), Changed<CombatState>>,
) {
    for (entity, combat, transform) in &query {
        if combat.phase != AttackPhase::Active {
            continue;
        }

        let forward = transform.forward().as_vec3();
        let hitbox_pos = transform.translation + forward * 1.5;

        // Combo step affects hitbox size and damage
        let (size, damage, knockback) = match combat.combo_step {
            0 => (Vec3::new(1.2, 1.0, 1.5), 25.0, 3.0), // light
            1 => (Vec3::new(1.5, 1.2, 2.0), 35.0, 5.0), // medium
            _ => (Vec3::new(2.0, 1.5, 2.5), 50.0, 8.0), // heavy finisher
        };

        commands.spawn((
            Transform::from_translation(hitbox_pos).with_rotation(transform.rotation),
            Collider::cuboid(size.x / 2.0, size.y / 2.0, size.z / 2.0),
            Sensor,
            ActiveEvents::COLLISION_EVENTS,
            Hitbox {
                owner: entity,
                base_damage: damage,
                knockback,
                hit_entities: Vec::new(),
                lifetime: 0.15,
            },
        ));
    }
}

/// Tick hitbox lifetime and despawn expired
pub fn update_hitbox_lifetime(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Hitbox)>,
) {
    let dt = time.delta_secs();
    for (entity, mut hitbox) in &mut query {
        hitbox.lifetime -= dt;
        if hitbox.lifetime <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Detect hitbox-hurtbox collisions and apply damage
pub fn process_hitbox_collisions(
    mut collision_events: EventReader<CollisionEvent>,
    mut hitbox_query: Query<&mut Hitbox>,
    hurtbox_query: Query<&Hurtbox>,
    transform_query: Query<&Transform>,
    mut health_query: Query<&mut Health>,
    mut damage_events: EventWriter<DamageEvent>,
) {
    for event in collision_events.read() {
        let (e1, e2, started) = match event {
            CollisionEvent::Started(e1, e2, _) => (*e1, *e2, true),
            CollisionEvent::Stopped(_, _, _) => continue,
        };

        if !started {
            continue;
        }

        // Determine which is hitbox and which is hurtbox
        let (hitbox_entity, hurtbox_entity) =
            if hitbox_query.get(e1).is_ok() && hurtbox_query.get(e2).is_ok() {
                (e1, e2)
            } else if hitbox_query.get(e2).is_ok() && hurtbox_query.get(e1).is_ok() {
                (e2, e1)
            } else {
                continue;
            };

        let mut hitbox = hitbox_query.get_mut(hitbox_entity).unwrap();
        let hurtbox = hurtbox_query.get(hurtbox_entity).unwrap();

        // Don't hit self
        if hitbox.owner == hurtbox.owner {
            continue;
        }

        // Don't hit same entity twice
        if hitbox.hit_entities.contains(&hurtbox.owner) {
            continue;
        }

        // Calculate angle-based damage
        let angle = if let (Ok(attacker_tf), Ok(target_tf)) = (
            transform_query.get(hitbox.owner),
            transform_query.get(hurtbox.owner),
        ) {
            AttackAngle::from_transforms(attacker_tf, target_tf)
        } else {
            AttackAngle::Front
        };

        let final_damage = hitbox.base_damage * angle.multiplier();

        // Apply damage
        if let Ok(mut health) = health_query.get_mut(hurtbox.owner) {
            let actual = health.take_damage(final_damage);
            if actual > 0.0 {
                let position = transform_query
                    .get(hurtbox.owner)
                    .map(|t| t.translation)
                    .unwrap_or(Vec3::ZERO);

                damage_events.send(DamageEvent {
                    target: hurtbox.owner,
                    attacker: hitbox.owner,
                    amount: actual,
                    angle,
                    position,
                });
            }
        }

        hitbox.hit_entities.push(hurtbox.owner);
    }
}

/// Update invulnerability timers
pub fn update_invulnerability(time: Res<Time>, mut query: Query<&mut Health>) {
    let dt = time.delta_secs();
    for mut health in &mut query {
        if health.invulnerable_timer > 0.0 {
            health.invulnerable_timer = (health.invulnerable_timer - dt).max(0.0);
        }
    }
}

/// Apply stagger/knockback
pub fn apply_stagger(time: Res<Time>, mut query: Query<(&mut Transform, &mut Stagger)>) {
    let dt = time.delta_secs();
    for (mut transform, mut stagger) in &mut query {
        if stagger.timer <= 0.0 {
            continue;
        }
        stagger.timer -= dt;
        transform.translation += stagger.direction * stagger.strength * dt;
        stagger.strength *= 0.9; // decay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_basic() {
        let mut hp = Health::new(100.0);
        assert!(hp.is_alive());
        assert_eq!(hp.current, 100.0);

        let dmg = hp.take_damage(30.0);
        assert!((dmg - 30.0).abs() < f32::EPSILON);
        assert!((hp.current - 70.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_health_overkill() {
        let mut hp = Health::new(50.0);
        let dmg = hp.take_damage(200.0);
        assert!((dmg - 50.0).abs() < f32::EPSILON);
        assert!(!hp.is_alive());
    }

    #[test]
    fn test_invulnerability() {
        let mut hp = Health::new(100.0);
        hp.take_damage(10.0);
        // Should have i-frames now
        assert!(hp.invulnerable_timer > 0.0);

        let blocked = hp.take_damage(50.0);
        assert!((blocked - 0.0).abs() < f32::EPSILON);
        assert!((hp.current - 90.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_stagger_default() {
        let s = Stagger::default();
        assert!(s.timer <= 0.0);
        assert_eq!(s.direction, Vec3::ZERO);
    }

    #[test]
    fn test_damage_multipliers_with_angle() {
        let base = 100.0;
        assert!((base * AttackAngle::Front.multiplier() - 100.0).abs() < f32::EPSILON);
        assert!((base * AttackAngle::Side.multiplier() - 70.0).abs() < f32::EPSILON);
        assert!((base * AttackAngle::Back.multiplier() - 150.0).abs() < f32::EPSILON);
    }
}
