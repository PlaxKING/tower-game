use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::semantic::SemanticTags;

pub struct DeathPlugin;

impl Plugin for DeathPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DeathEvent>()
            .add_event::<EchoSpawnEvent>()
            .add_systems(
                Update,
                (process_death_events, update_echoes, decay_echoes).chain(),
            );
    }
}

/// Death event fired when an entity dies
#[derive(Event, Debug)]
pub struct DeathEvent {
    pub entity: Entity,
    pub death_location: Vec3,
    pub killer: Option<Entity>,
    pub cause: DeathCause,
}

/// How the entity died - affects echo properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeathCause {
    Combat { final_blow_damage: f32 },
    Fall { height: f32 },
    Environment { damage_type: String },
    Void, // fell below tower
}

/// Echo spawn request
#[derive(Event, Debug)]
pub struct EchoSpawnEvent {
    pub position: Vec3,
    pub original_entity: Entity,
    pub semantic_tags: SemanticTags,
    pub echo_type: EchoType,
    pub power: f32,
}

/// Types of echoes left behind
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EchoType {
    /// Replay of last actions before death (ghost playback)
    ActionReplay { duration_secs: f32 },
    /// Resource cache that others can collect
    ResourceCache {
        kinetic: f32,
        thermal: f32,
        semantic: f32,
    },
    /// Hostile echo that attacks nearby players
    Vengeful { aggression: f32, damage: f32 },
    /// Helpful echo that heals/buffs passersby
    Benevolent {
        heal_per_sec: f32,
        buff_strength: f32,
    },
}

/// Echo component on spawned echo entities
#[derive(Component, Debug)]
pub struct Echo {
    pub echo_type: EchoType,
    pub lifetime: f32, // seconds remaining
    pub max_lifetime: f32,
    pub power: f32, // decreases over time
    pub original_player_id: Option<u64>,
}

/// Marker for entities that can die and leave echoes
#[derive(Component, Debug)]
pub struct Mortal {
    pub hp: f32,
    pub max_hp: f32,
    pub echo_power_factor: f32, // how powerful echoes they leave (based on achievements)
}

impl Default for Mortal {
    fn default() -> Self {
        Self {
            hp: 100.0,
            max_hp: 100.0,
            echo_power_factor: 1.0,
        }
    }
}

/// Determines echo type based on death cause and semantic tags
fn determine_echo_type(cause: &DeathCause, tags: &SemanticTags) -> EchoType {
    let aggression = tags.get("aggression");
    let corruption = tags.get("corruption");
    let healing = tags.get("healing");

    match cause {
        DeathCause::Combat { final_blow_damage } => {
            if aggression > 0.6 || corruption > 0.5 {
                EchoType::Vengeful {
                    aggression: aggression.max(0.3),
                    damage: final_blow_damage * 0.3,
                }
            } else {
                EchoType::ActionReplay {
                    duration_secs: 10.0,
                }
            }
        }
        DeathCause::Fall { height } => EchoType::ResourceCache {
            kinetic: height * 2.0,
            thermal: 0.0,
            semantic: 0.0,
        },
        DeathCause::Environment { .. } => {
            if healing > 0.5 {
                EchoType::Benevolent {
                    heal_per_sec: healing * 10.0,
                    buff_strength: 0.2,
                }
            } else {
                EchoType::ResourceCache {
                    kinetic: 10.0,
                    thermal: 10.0,
                    semantic: 10.0,
                }
            }
        }
        DeathCause::Void => {
            // Void deaths leave vengeful echoes
            EchoType::Vengeful {
                aggression: 0.8,
                damage: 20.0,
            }
        }
    }
}

fn process_death_events(
    mut death_events: EventReader<DeathEvent>,
    mut echo_spawn: EventWriter<EchoSpawnEvent>,
    query: Query<(&SemanticTags, &Mortal)>,
) {
    for event in death_events.read() {
        if let Ok((tags, mortal)) = query.get(event.entity) {
            let echo_type = determine_echo_type(&event.cause, tags);
            let power = mortal.echo_power_factor
                * match &event.cause {
                    DeathCause::Combat { final_blow_damage } => final_blow_damage / mortal.max_hp,
                    DeathCause::Fall { height } => (height / 100.0).min(2.0),
                    DeathCause::Environment { .. } => 1.0,
                    DeathCause::Void => 1.5,
                };

            echo_spawn.send(EchoSpawnEvent {
                position: event.death_location,
                original_entity: event.entity,
                semantic_tags: tags.clone(),
                echo_type,
                power,
            });
        }
    }
}

fn update_echoes(time: Res<Time>, mut query: Query<&mut Echo>) {
    let dt = time.delta_secs();
    for mut echo in &mut query {
        echo.lifetime -= dt;
        // Power decays as lifetime decreases
        let life_ratio = (echo.lifetime / echo.max_lifetime).max(0.0);
        echo.power *= life_ratio.powf(0.1); // slow decay curve
    }
}

fn decay_echoes(mut commands: Commands, query: Query<(Entity, &Echo)>) {
    for (entity, echo) in &query {
        if echo.lifetime <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Calculate echo lifetime based on floor tier and echo power
pub fn echo_lifetime(floor_echelon: u32, power: f32) -> f32 {
    let base = match floor_echelon {
        1 => 120.0,  // 2 minutes in tutorial zones
        2 => 300.0,  // 5 minutes
        3 => 600.0,  // 10 minutes
        _ => 1800.0, // 30 minutes in endgame
    };
    base * power.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_lifetime_scaling() {
        assert!((echo_lifetime(1, 1.0) - 120.0).abs() < f32::EPSILON);
        assert!((echo_lifetime(4, 1.0) - 1800.0).abs() < f32::EPSILON);
        // Power 4.0 should double lifetime (sqrt(4) = 2)
        assert!((echo_lifetime(1, 4.0) - 240.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_determine_echo_type_combat_aggressive() {
        let tags = SemanticTags::new(vec![("aggression", 0.9), ("corruption", 0.7)]);
        let cause = DeathCause::Combat {
            final_blow_damage: 50.0,
        };
        match determine_echo_type(&cause, &tags) {
            EchoType::Vengeful { aggression, .. } => {
                assert!(aggression > 0.5);
            }
            _ => panic!("Expected Vengeful echo for aggressive entity"),
        }
    }

    #[test]
    fn test_determine_echo_type_fall() {
        let tags = SemanticTags::new(vec![("neutral", 0.5)]);
        let cause = DeathCause::Fall { height: 50.0 };
        match determine_echo_type(&cause, &tags) {
            EchoType::ResourceCache { kinetic, .. } => {
                assert!((kinetic - 100.0).abs() < f32::EPSILON);
            }
            _ => panic!("Expected ResourceCache echo for fall death"),
        }
    }

    #[test]
    fn test_mortal_defaults() {
        let mortal = Mortal::default();
        assert!((mortal.hp - 100.0).abs() < f32::EPSILON);
        assert!((mortal.echo_power_factor - 1.0).abs() < f32::EPSILON);
    }
}
