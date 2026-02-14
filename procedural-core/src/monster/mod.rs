//! Monster generation via combinatorial grammar.
//!
//! Monster = Size x Element x Corruption x Behavior x Faction
//! Each axis contributes to stats, visuals, and semantic tags.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::combat::{CombatResources, CombatState};
use crate::death::Mortal;
use crate::semantic::SemanticTags;

pub mod ai;

pub struct MonsterPlugin;

impl Plugin for MonsterPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnMonsterEvent>().add_systems(
            Update,
            (
                process_monster_spawns,
                ai::update_ai_state,
                ai::execute_ai_movement,
                ai::execute_ai_attacks,
            )
                .chain(),
        );
    }
}

/// Request to spawn a monster
#[derive(Event, Debug)]
pub struct SpawnMonsterEvent {
    pub position: Vec3,
    pub template: MonsterTemplate,
}

/// Size axis — affects HP, damage, speed inversely
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonsterSize {
    Tiny,     // fast, fragile, swarm
    Small,    // quick, moderate HP
    Medium,   // balanced
    Large,    // slow, tanky, high damage
    Colossal, // boss-tier, very slow, massive HP
}

/// Element axis — determines damage type and resistances
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonsterElement {
    Fire,    // high burst damage
    Water,   // healing, crowd control
    Earth,   // defense, armor
    Wind,    // speed, evasion
    Void,    // corruption, life drain
    Neutral, // no element
}

/// Corruption level — how much the Tower has warped this creature
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CorruptionLevel {
    Pure,      // 0-20%: normal creature
    Tainted,   // 20-50%: slightly mutated
    Corrupted, // 50-80%: heavily mutated, extra abilities
    Abyssal,   // 80-100%: fully consumed, very dangerous
}

/// Behavior pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonsterBehavior {
    Passive,    // won't attack unless provoked
    Patrol,     // follows set path, attacks on sight
    Aggressive, // actively seeks players
    Ambush,     // hides, attacks when close
    Pack,       // coordinates with nearby allies
    Guardian,   // protects an area/object
}

/// Full monster template combining all axes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterTemplate {
    pub name: String,
    pub size: MonsterSize,
    pub element: MonsterElement,
    pub corruption: CorruptionLevel,
    pub behavior: MonsterBehavior,
    pub base_level: u32,
}

/// Computed monster stats from template
#[derive(Debug, Clone)]
pub struct MonsterStats {
    pub max_hp: f32,
    pub damage: f32,
    pub speed: f32,
    pub armor: f32,
    pub detection_range: f32,
    pub xp_reward: u32,
}

impl MonsterTemplate {
    /// Generate a monster template from a floor spec and spawn hash
    pub fn from_hash(hash: u64, floor_level: u32) -> Self {
        let size = match hash & 0x7 {
            0 => MonsterSize::Tiny,
            1 | 2 => MonsterSize::Small,
            3 | 4 => MonsterSize::Medium,
            5 | 6 => MonsterSize::Large,
            _ => MonsterSize::Colossal,
        };

        let element = match (hash >> 3) & 0x7 {
            0 => MonsterElement::Fire,
            1 => MonsterElement::Water,
            2 => MonsterElement::Earth,
            3 => MonsterElement::Wind,
            4 => MonsterElement::Void,
            _ => MonsterElement::Neutral,
        };

        let corruption = match (hash >> 6) & 0x3 {
            0 => CorruptionLevel::Pure,
            1 => CorruptionLevel::Tainted,
            2 => CorruptionLevel::Corrupted,
            _ => CorruptionLevel::Abyssal,
        };

        let behavior = match (hash >> 8) & 0x7 {
            0 => MonsterBehavior::Passive,
            1 => MonsterBehavior::Patrol,
            2 | 3 => MonsterBehavior::Aggressive,
            4 => MonsterBehavior::Ambush,
            5 => MonsterBehavior::Pack,
            _ => MonsterBehavior::Guardian,
        };

        let name = generate_name(size, element, corruption);

        Self {
            name,
            size,
            element,
            corruption,
            behavior,
            base_level: floor_level,
        }
    }

    /// Compute final stats
    pub fn compute_stats(&self) -> MonsterStats {
        let (hp_mult, dmg_mult, spd_mult) = match self.size {
            MonsterSize::Tiny => (0.3, 0.4, 2.0),
            MonsterSize::Small => (0.6, 0.7, 1.5),
            MonsterSize::Medium => (1.0, 1.0, 1.0),
            MonsterSize::Large => (2.0, 1.5, 0.6),
            MonsterSize::Colossal => (5.0, 2.5, 0.3),
        };

        let armor = match self.element {
            MonsterElement::Earth => 20.0,
            MonsterElement::Water => 10.0,
            MonsterElement::Fire => 5.0,
            _ => 0.0,
        };

        let corruption_mult = match self.corruption {
            CorruptionLevel::Pure => 1.0,
            CorruptionLevel::Tainted => 1.2,
            CorruptionLevel::Corrupted => 1.5,
            CorruptionLevel::Abyssal => 2.0,
        };

        let level_scale = 1.0 + (self.base_level as f32 * 0.05);

        MonsterStats {
            max_hp: 100.0 * hp_mult * corruption_mult * level_scale,
            damage: 10.0 * dmg_mult * corruption_mult * level_scale,
            speed: 5.0 * spd_mult,
            armor,
            detection_range: match self.behavior {
                MonsterBehavior::Passive => 5.0,
                MonsterBehavior::Patrol => 15.0,
                MonsterBehavior::Aggressive => 25.0,
                MonsterBehavior::Ambush => 8.0,
                MonsterBehavior::Pack => 20.0,
                MonsterBehavior::Guardian => 12.0,
            },
            xp_reward: (10.0 * hp_mult * corruption_mult * level_scale) as u32,
        }
    }

    /// Generate semantic tags for this monster
    pub fn semantic_tags(&self) -> SemanticTags {
        let mut tags = vec![];

        // Element tag
        match self.element {
            MonsterElement::Fire => tags.push(("fire", 0.8)),
            MonsterElement::Water => tags.push(("water", 0.8)),
            MonsterElement::Earth => tags.push(("earth", 0.8)),
            MonsterElement::Wind => tags.push(("wind", 0.8)),
            MonsterElement::Void => tags.push(("void", 0.8)),
            MonsterElement::Neutral => tags.push(("neutral", 0.5)),
        }

        // Corruption tag
        let corruption_val = match self.corruption {
            CorruptionLevel::Pure => 0.0,
            CorruptionLevel::Tainted => 0.3,
            CorruptionLevel::Corrupted => 0.6,
            CorruptionLevel::Abyssal => 1.0,
        };
        tags.push(("corruption", corruption_val));

        // Aggression based on behavior
        let aggression = match self.behavior {
            MonsterBehavior::Passive => 0.1,
            MonsterBehavior::Patrol => 0.4,
            MonsterBehavior::Aggressive => 0.9,
            MonsterBehavior::Ambush => 0.7,
            MonsterBehavior::Pack => 0.6,
            MonsterBehavior::Guardian => 0.5,
        };
        tags.push(("aggression", aggression));

        // Size as "presence"
        let presence = match self.size {
            MonsterSize::Tiny => 0.1,
            MonsterSize::Small => 0.3,
            MonsterSize::Medium => 0.5,
            MonsterSize::Large => 0.8,
            MonsterSize::Colossal => 1.0,
        };
        tags.push(("presence", presence));

        SemanticTags::new(tags)
    }
}

/// Generate a name from grammar: [Corruption Prefix] + [Element] + [Size Suffix]
fn generate_name(
    size: MonsterSize,
    element: MonsterElement,
    corruption: CorruptionLevel,
) -> String {
    let prefix = match corruption {
        CorruptionLevel::Pure => "",
        CorruptionLevel::Tainted => "Shadow ",
        CorruptionLevel::Corrupted => "Void-Touched ",
        CorruptionLevel::Abyssal => "Abyssal ",
    };

    let core = match element {
        MonsterElement::Fire => "Ember",
        MonsterElement::Water => "Tide",
        MonsterElement::Earth => "Stone",
        MonsterElement::Wind => "Gale",
        MonsterElement::Void => "Hollow",
        MonsterElement::Neutral => "Tower",
    };

    let suffix = match size {
        MonsterSize::Tiny => " Wisp",
        MonsterSize::Small => " Scout",
        MonsterSize::Medium => " Guardian",
        MonsterSize::Large => " Warden",
        MonsterSize::Colossal => " Colossus",
    };

    format!("{prefix}{core}{suffix}")
}

/// Marker component for monster entities
#[derive(Component, Debug)]
pub struct Monster {
    pub template: MonsterTemplate,
    pub current_hp: f32,
    pub aggro_target: Option<Entity>,
}

fn process_monster_spawns(mut commands: Commands, mut events: EventReader<SpawnMonsterEvent>) {
    for event in events.read() {
        let stats = event.template.compute_stats();
        let tags = event.template.semantic_tags();

        commands.spawn((
            Transform::from_translation(event.position),
            Monster {
                template: event.template.clone(),
                current_hp: stats.max_hp,
                aggro_target: None,
            },
            Mortal {
                hp: stats.max_hp,
                max_hp: stats.max_hp,
                echo_power_factor: match event.template.corruption {
                    CorruptionLevel::Abyssal => 2.0,
                    CorruptionLevel::Corrupted => 1.5,
                    _ => 1.0,
                },
            },
            CombatState::default(),
            CombatResources::default(),
            tags,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monster_from_hash_deterministic() {
        let m1 = MonsterTemplate::from_hash(12345, 1);
        let m2 = MonsterTemplate::from_hash(12345, 1);
        assert_eq!(m1.name, m2.name);
        assert_eq!(m1.size, m2.size);
        assert_eq!(m1.element, m2.element);
    }

    #[test]
    fn test_different_hashes_different_monsters() {
        let m1 = MonsterTemplate::from_hash(12345, 1);
        let m2 = MonsterTemplate::from_hash(67890, 1);
        // Very unlikely to be identical across all axes
        assert!(
            m1.size != m2.size
                || m1.element != m2.element
                || m1.corruption != m2.corruption
                || m1.behavior != m2.behavior,
            "Different hashes should produce different monsters"
        );
    }

    #[test]
    fn test_stats_scaling() {
        let tiny = MonsterTemplate {
            name: "Test".into(),
            size: MonsterSize::Tiny,
            element: MonsterElement::Neutral,
            corruption: CorruptionLevel::Pure,
            behavior: MonsterBehavior::Passive,
            base_level: 1,
        };
        let colossal = MonsterTemplate {
            name: "Test".into(),
            size: MonsterSize::Colossal,
            element: MonsterElement::Neutral,
            corruption: CorruptionLevel::Pure,
            behavior: MonsterBehavior::Passive,
            base_level: 1,
        };

        let tiny_stats = tiny.compute_stats();
        let colossal_stats = colossal.compute_stats();

        assert!(colossal_stats.max_hp > tiny_stats.max_hp * 5.0);
        assert!(tiny_stats.speed > colossal_stats.speed * 3.0);
    }

    #[test]
    fn test_corruption_multiplier() {
        let pure = MonsterTemplate {
            name: "T".into(),
            size: MonsterSize::Medium,
            element: MonsterElement::Neutral,
            corruption: CorruptionLevel::Pure,
            behavior: MonsterBehavior::Patrol,
            base_level: 1,
        };
        let abyssal = MonsterTemplate {
            name: "T".into(),
            size: MonsterSize::Medium,
            element: MonsterElement::Neutral,
            corruption: CorruptionLevel::Abyssal,
            behavior: MonsterBehavior::Patrol,
            base_level: 1,
        };

        assert!(abyssal.compute_stats().max_hp > pure.compute_stats().max_hp * 1.5);
    }

    #[test]
    fn test_name_generation() {
        let name = generate_name(
            MonsterSize::Large,
            MonsterElement::Fire,
            CorruptionLevel::Corrupted,
        );
        assert_eq!(name, "Void-Touched Ember Warden");
    }

    #[test]
    fn test_semantic_tags_element() {
        let template = MonsterTemplate {
            name: "T".into(),
            size: MonsterSize::Medium,
            element: MonsterElement::Fire,
            corruption: CorruptionLevel::Tainted,
            behavior: MonsterBehavior::Aggressive,
            base_level: 1,
        };
        let tags = template.semantic_tags();
        assert!(tags.get("fire") > 0.5);
        assert!(tags.get("aggression") > 0.8);
        assert!(tags.get("corruption") > 0.2);
    }
}
