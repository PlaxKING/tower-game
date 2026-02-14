//! Monster AI behavior system.
//!
//! Simple state-machine AI driven by monster behavior type and semantic tags.
//! Uses distance/angle checks to determine actions.

use bevy::prelude::*;

use super::{Monster, MonsterBehavior};
use crate::combat::{AttackPhase, CombatState};
use crate::player::Player;

/// AI state machine
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub enum AiState {
    #[default]
    Idle,
    Patrol {
        waypoint_idx: usize,
    },
    Chase {
        target: Entity,
    },
    Attack {
        target: Entity,
    },
    Flee,
    Ambush,
    Regroup,
}

/// AI configuration per monster (derived from MonsterBehavior)
#[derive(Component, Debug)]
pub struct AiConfig {
    pub detection_range: f32,
    pub attack_range: f32,
    pub flee_health_pct: f32,
    pub aggro_persistence: f32,
    pub patrol_speed: f32,
    pub chase_speed: f32,
}

impl AiConfig {
    pub fn from_behavior(behavior: MonsterBehavior, detection: f32, speed: f32) -> Self {
        match behavior {
            MonsterBehavior::Passive => Self {
                detection_range: detection * 0.5,
                attack_range: 2.0,
                flee_health_pct: 0.5,
                aggro_persistence: 3.0,
                patrol_speed: speed * 0.3,
                chase_speed: speed * 0.6,
            },
            MonsterBehavior::Patrol => Self {
                detection_range: detection,
                attack_range: 2.5,
                flee_health_pct: 0.2,
                aggro_persistence: 8.0,
                patrol_speed: speed * 0.5,
                chase_speed: speed * 0.8,
            },
            MonsterBehavior::Aggressive => Self {
                detection_range: detection * 1.5,
                attack_range: 3.0,
                flee_health_pct: 0.0,
                aggro_persistence: 15.0,
                patrol_speed: speed * 0.4,
                chase_speed: speed,
            },
            MonsterBehavior::Ambush => Self {
                detection_range: detection * 0.8,
                attack_range: 4.0,
                flee_health_pct: 0.3,
                aggro_persistence: 5.0,
                patrol_speed: 0.0,
                chase_speed: speed * 1.3,
            },
            MonsterBehavior::Pack => Self {
                detection_range: detection * 1.2,
                attack_range: 2.5,
                flee_health_pct: 0.15,
                aggro_persistence: 12.0,
                patrol_speed: speed * 0.5,
                chase_speed: speed * 0.9,
            },
            MonsterBehavior::Guardian => Self {
                detection_range: detection * 0.7,
                attack_range: 3.5,
                flee_health_pct: 0.0,
                aggro_persistence: 999.0,
                patrol_speed: speed * 0.2,
                chase_speed: speed * 0.7,
            },
        }
    }
}

/// Patrol waypoints
#[derive(Component, Debug)]
pub struct PatrolPath {
    pub waypoints: Vec<Vec3>,
}

/// System: update AI state based on environment
pub fn update_ai_state(
    mut monsters: Query<(
        Entity,
        &Transform,
        &Monster,
        &AiConfig,
        &mut AiState,
        &crate::combat::hitbox::Health,
    )>,
    players: Query<(Entity, &Transform), With<Player>>,
) {
    for (_entity, monster_tf, monster, config, mut ai_state, health) in &mut monsters {
        // Check flee condition
        let health_pct = health.current / health.max;
        if health_pct < config.flee_health_pct && config.flee_health_pct > 0.0 {
            if *ai_state != AiState::Flee {
                *ai_state = AiState::Flee;
            }
            continue;
        }

        // Find nearest player
        let nearest = players
            .iter()
            .map(|(e, tf)| {
                let dist = tf.translation.distance(monster_tf.translation);
                (e, dist)
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let (nearest_player, distance) = match nearest {
            Some(n) => n,
            None => {
                // No players — return to patrol/idle
                match monster.template.behavior {
                    MonsterBehavior::Patrol => {
                        if !matches!(*ai_state, AiState::Patrol { .. }) {
                            *ai_state = AiState::Patrol { waypoint_idx: 0 };
                        }
                    }
                    MonsterBehavior::Ambush => *ai_state = AiState::Ambush,
                    _ => *ai_state = AiState::Idle,
                }
                continue;
            }
        };

        // State transitions
        match *ai_state {
            AiState::Idle | AiState::Patrol { .. } | AiState::Ambush => {
                if distance <= config.detection_range {
                    if distance <= config.attack_range {
                        *ai_state = AiState::Attack {
                            target: nearest_player,
                        };
                    } else {
                        *ai_state = AiState::Chase {
                            target: nearest_player,
                        };
                    }
                }
            }
            AiState::Chase { .. } => {
                if distance <= config.attack_range {
                    *ai_state = AiState::Attack {
                        target: nearest_player,
                    };
                } else if distance > config.detection_range * 1.5 {
                    // Lost target
                    *ai_state = AiState::Idle;
                }
            }
            AiState::Attack { .. } => {
                if distance > config.attack_range * 1.5 {
                    *ai_state = AiState::Chase {
                        target: nearest_player,
                    };
                }
            }
            AiState::Flee => {
                if health_pct > config.flee_health_pct * 1.5 {
                    *ai_state = AiState::Idle;
                }
            }
            AiState::Regroup => {
                // Pack behavior: find allies and group up
                *ai_state = AiState::Idle;
            }
        }
    }
}

/// System: execute AI movement
pub fn execute_ai_movement(
    time: Res<Time>,
    mut monsters: Query<(&mut Transform, &AiState, &AiConfig, Option<&PatrolPath>), With<Monster>>,
    player_transforms: Query<&Transform, (With<Player>, Without<Monster>)>,
) {
    let dt = time.delta_secs();

    for (mut transform, ai_state, config, patrol_path) in &mut monsters {
        match ai_state {
            AiState::Idle => {}
            AiState::Patrol { waypoint_idx } => {
                if let Some(path) = patrol_path {
                    if let Some(target) = path.waypoints.get(*waypoint_idx) {
                        let dir = (*target - transform.translation).normalize_or_zero();
                        transform.translation += dir * config.patrol_speed * dt;

                        // Look towards movement direction
                        if dir.length_squared() > 0.01 {
                            let look_target = transform.translation + dir;
                            transform.look_at(look_target, Vec3::Y);
                        }
                    }
                }
            }
            AiState::Chase { target } => {
                if let Ok(player_tf) = player_transforms.get(*target) {
                    let dir = (player_tf.translation - transform.translation).normalize_or_zero();
                    transform.translation += dir * config.chase_speed * dt;

                    if dir.length_squared() > 0.01 {
                        let look_target = transform.translation + dir;
                        transform.look_at(look_target, Vec3::Y);
                    }
                }
            }
            AiState::Flee => {
                // Run away from nearest player (simplified: just reverse chase)
                // In a real system you'd use navmesh pathfinding
                let nearest_player_pos =
                    player_transforms
                        .iter()
                        .map(|t| t.translation)
                        .min_by(|a, b| {
                            let da = a.distance(transform.translation);
                            let db = b.distance(transform.translation);
                            da.partial_cmp(&db).unwrap()
                        });

                if let Some(player_pos) = nearest_player_pos {
                    let dir = (transform.translation - player_pos).normalize_or_zero();
                    transform.translation += dir * config.chase_speed * 1.2 * dt;
                }
            }
            AiState::Attack { target } => {
                // Face the target
                if let Ok(player_tf) = player_transforms.get(*target) {
                    let look_target = Vec3::new(
                        player_tf.translation.x,
                        transform.translation.y,
                        player_tf.translation.z,
                    );
                    transform.look_at(look_target, Vec3::Y);
                }
            }
            AiState::Ambush => {
                // Stay still, wait
            }
            AiState::Regroup => {}
        }
    }
}

/// System: AI attacks when in Attack state
pub fn execute_ai_attacks(mut query: Query<(&AiState, &mut CombatState), With<Monster>>) {
    for (ai_state, mut combat) in &mut query {
        if let AiState::Attack { .. } = ai_state {
            // Start attack if idle
            if combat.phase == AttackPhase::Idle {
                combat.phase = AttackPhase::Windup;
                combat.phase_timer = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_config_aggressive() {
        let config = AiConfig::from_behavior(MonsterBehavior::Aggressive, 10.0, 5.0);
        assert!(
            config.detection_range > 10.0,
            "Aggressive should have extended detection"
        );
        assert_eq!(config.flee_health_pct, 0.0, "Aggressive never flees");
    }

    #[test]
    fn test_ai_config_passive() {
        let config = AiConfig::from_behavior(MonsterBehavior::Passive, 10.0, 5.0);
        assert!(
            config.detection_range < 10.0,
            "Passive should have reduced detection"
        );
        assert!(config.flee_health_pct > 0.0, "Passive should flee");
    }

    #[test]
    fn test_ai_config_guardian() {
        let config = AiConfig::from_behavior(MonsterBehavior::Guardian, 10.0, 5.0);
        assert_eq!(config.flee_health_pct, 0.0, "Guardian never flees");
        assert!(
            config.aggro_persistence > 100.0,
            "Guardian is very persistent"
        );
    }

    #[test]
    fn test_ai_state_default() {
        let state = AiState::default();
        assert_eq!(state, AiState::Idle);
    }
}
