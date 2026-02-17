//! Monster Generation System — Grammar-based procedural monsters with FSM AI
//!
//! ## Architecture
//! ```text
//! Floor Seed + Biome Tags
//!       ↓
//! Grammar-based generation (body × element × corruption × behavior × size)
//!       ↓
//! MonsterInstance component (ECS entity with AI + combat + semantic tags)
//!       ↓
//! FSM AI System: Idle → Patrol → Chase → Attack → Retreat
//! ```
//!
//! Monsters inherit 70% of their floor's semantic tags, ensuring thematic coherence.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::combat::{CombatEnergy, CombatState, EquippedWeapon, WeaponType};
use crate::components::{Monster, Player};
use crate::physics;

// ============================================================================
// Monster Traits (Grammar Axes)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MonsterSize {
    Tiny,
    Small,
    Medium,
    Large,
    Colossal,
}

impl MonsterSize {
    fn from_hash(h: u64) -> Self {
        match h % 100 {
            0..=10 => MonsterSize::Tiny,
            11..=35 => MonsterSize::Small,
            36..=65 => MonsterSize::Medium,
            66..=85 => MonsterSize::Large,
            _ => MonsterSize::Colossal,
        }
    }

    pub fn hp_mult(&self) -> f32 {
        match self {
            MonsterSize::Tiny => 0.3,
            MonsterSize::Small => 0.6,
            MonsterSize::Medium => 1.0,
            MonsterSize::Large => 2.5,
            MonsterSize::Colossal => 5.0,
        }
    }

    pub fn damage_mult(&self) -> f32 {
        match self {
            MonsterSize::Tiny => 0.4,
            MonsterSize::Small => 0.7,
            MonsterSize::Medium => 1.0,
            MonsterSize::Large => 1.5,
            MonsterSize::Colossal => 2.5,
        }
    }

    pub fn speed_mult(&self) -> f32 {
        match self {
            MonsterSize::Tiny => 2.0,
            MonsterSize::Small => 1.5,
            MonsterSize::Medium => 1.0,
            MonsterSize::Large => 0.6,
            MonsterSize::Colossal => 0.3,
        }
    }

    fn name_suffix(&self) -> &'static str {
        match self {
            MonsterSize::Tiny => "Sprite",
            MonsterSize::Small => "Scout",
            MonsterSize::Medium => "Warrior",
            MonsterSize::Large => "Warden",
            MonsterSize::Colossal => "Titan",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MonsterElement {
    Fire,
    Water,
    Earth,
    Wind,
    Void,
    Neutral,
}

impl MonsterElement {
    fn from_hash(h: u64) -> Self {
        match h % 6 {
            0 => MonsterElement::Fire,
            1 => MonsterElement::Water,
            2 => MonsterElement::Earth,
            3 => MonsterElement::Wind,
            4 => MonsterElement::Void,
            _ => MonsterElement::Neutral,
        }
    }

    fn name_prefix(&self) -> &'static str {
        match self {
            MonsterElement::Fire => "Ember",
            MonsterElement::Water => "Tide",
            MonsterElement::Earth => "Stone",
            MonsterElement::Wind => "Gale",
            MonsterElement::Void => "Void",
            MonsterElement::Neutral => "Shadow",
        }
    }

    /// Semantic tag for this element
    fn tag(&self) -> (&'static str, f32) {
        match self {
            MonsterElement::Fire => ("fire", 0.8),
            MonsterElement::Water => ("water", 0.8),
            MonsterElement::Earth => ("earth", 0.8),
            MonsterElement::Wind => ("wind", 0.8),
            MonsterElement::Void => ("void", 0.9),
            MonsterElement::Neutral => ("neutral", 0.5),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CorruptionLevel {
    Pure,
    Tainted,
    Corrupted,
    Abyssal,
}

impl CorruptionLevel {
    fn from_hash(h: u64, floor: u32) -> Self {
        // Higher floors = higher chance of corruption
        let corruption_bias = (floor as u64) / 20;
        let roll = (h % 100).saturating_add(corruption_bias);
        match roll {
            0..=40 => CorruptionLevel::Pure,
            41..=65 => CorruptionLevel::Tainted,
            66..=85 => CorruptionLevel::Corrupted,
            _ => CorruptionLevel::Abyssal,
        }
    }

    pub fn damage_mult(&self) -> f32 {
        match self {
            CorruptionLevel::Pure => 1.0,
            CorruptionLevel::Tainted => 1.2,
            CorruptionLevel::Corrupted => 1.5,
            CorruptionLevel::Abyssal => 2.0,
        }
    }

    fn name_prefix(&self) -> &'static str {
        match self {
            CorruptionLevel::Pure => "",
            CorruptionLevel::Tainted => "Tainted",
            CorruptionLevel::Corrupted => "Corrupted",
            CorruptionLevel::Abyssal => "Abyssal",
        }
    }

    fn corruption_tag_weight(&self) -> f32 {
        match self {
            CorruptionLevel::Pure => 0.0,
            CorruptionLevel::Tainted => 0.3,
            CorruptionLevel::Corrupted => 0.6,
            CorruptionLevel::Abyssal => 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MonsterBodyType {
    Humanoid,
    Beast,
    Insectoid,
    Elemental,
    Undead,
    Construct,
}

impl MonsterBodyType {
    fn from_hash(h: u64) -> Self {
        match h % 6 {
            0 => MonsterBodyType::Humanoid,
            1 => MonsterBodyType::Beast,
            2 => MonsterBodyType::Insectoid,
            3 => MonsterBodyType::Elemental,
            4 => MonsterBodyType::Undead,
            _ => MonsterBodyType::Construct,
        }
    }

    fn name_part(&self) -> &'static str {
        match self {
            MonsterBodyType::Humanoid => "Knight",
            MonsterBodyType::Beast => "Fang",
            MonsterBodyType::Insectoid => "Weaver",
            MonsterBodyType::Elemental => "Core",
            MonsterBodyType::Undead => "Wraith",
            MonsterBodyType::Construct => "Golem",
        }
    }
}

// ============================================================================
// Grammar-based Name Generation
// ============================================================================

/// Generate a monster name from its traits
/// Pattern: [Corruption] [Element] [Body] [Size]
/// Examples: "Corrupted Ember Fang Warden", "Void Core Titan", "Tainted Tide Scout"
fn generate_name(
    corruption: CorruptionLevel,
    element: MonsterElement,
    body: MonsterBodyType,
    size: MonsterSize,
) -> String {
    let mut parts = Vec::new();

    let corruption_prefix = corruption.name_prefix();
    if !corruption_prefix.is_empty() {
        parts.push(corruption_prefix.to_string());
    }

    parts.push(element.name_prefix().to_string());
    parts.push(body.name_part().to_string());
    parts.push(size.name_suffix().to_string());

    parts.join(" ")
}

// ============================================================================
// Monster Blueprint (template generated from seed)
// ============================================================================

/// Generated monster blueprint — all stats and traits deterministically derived from seed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterBlueprint {
    /// Unique ID for this variant (hash-based)
    pub variant_id: u64,
    pub name: String,
    pub size: MonsterSize,
    pub element: MonsterElement,
    pub corruption: CorruptionLevel,
    pub body_type: MonsterBodyType,
    pub floor_level: u32,
    /// Computed stats
    pub max_health: f32,
    pub base_damage: f32,
    pub move_speed: f32,
    pub aggro_range: f32,
    pub leash_range: f32,
    /// Semantic tags (inherited from floor + own traits)
    pub semantic_tags: HashMap<String, f32>,
    /// AI behavior type
    pub ai_behavior: AiBehavior,
    /// Loot table reference
    pub loot_tier: u32,
}

/// Generate a monster blueprint deterministically from a seed and floor
pub fn generate_blueprint(
    seed: u64,
    floor_id: u32,
    biome_tags: &[(String, f32)],
) -> MonsterBlueprint {
    // Use LCG-style hash for each trait axis
    let h1 = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let h2 = h1.wrapping_mul(6364136223846793005).wrapping_add(1);
    let h3 = h2.wrapping_mul(6364136223846793005).wrapping_add(1);
    let h4 = h3.wrapping_mul(6364136223846793005).wrapping_add(1);
    let h5 = h4.wrapping_mul(6364136223846793005).wrapping_add(1);

    let size = MonsterSize::from_hash(h1 >> 32);
    let element = MonsterElement::from_hash(h2 >> 32);
    let corruption = CorruptionLevel::from_hash(h3 >> 32, floor_id);
    let body_type = MonsterBodyType::from_hash(h4 >> 32);
    let ai_behavior = AiBehavior::from_hash(h5 >> 32);

    let name = generate_name(corruption, element, body_type, size);

    // Base stats scaled by floor level
    let floor_scale = 1.0 + (floor_id as f32 - 1.0) * 0.1; // +10% per floor
    let base_hp = 100.0;
    let base_dmg = 15.0;
    let base_speed = 3.0;

    let max_health = base_hp * size.hp_mult() * corruption.damage_mult() * floor_scale;
    let base_damage = base_dmg * size.damage_mult() * corruption.damage_mult() * floor_scale;
    let move_speed = base_speed * size.speed_mult();

    // Aggro/leash ranges depend on AI behavior
    let (aggro_range, leash_range) = match ai_behavior {
        AiBehavior::Passive => (5.0, 10.0),
        AiBehavior::Patrol => (10.0, 20.0),
        AiBehavior::Aggressive => (15.0, 30.0),
        AiBehavior::Ambush => (8.0, 12.0),
        AiBehavior::Guardian => (12.0, 15.0),
    };

    // Build semantic tags: 70% from floor biome, 30% from monster traits
    let mut tags = HashMap::new();

    // Inherit floor biome tags at 70% weight
    for (tag, weight) in biome_tags {
        tags.insert(tag.clone(), weight * 0.7);
    }

    // Add element tag at full weight
    let (elem_tag, elem_weight) = element.tag();
    tags.insert(elem_tag.to_string(), elem_weight);

    // Add corruption tag
    let corruption_weight = corruption.corruption_tag_weight();
    if corruption_weight > 0.0 {
        tags.insert("corruption".to_string(), corruption_weight);
    }

    // Add aggression tag from AI behavior
    let aggression = match ai_behavior {
        AiBehavior::Passive => 0.1,
        AiBehavior::Patrol => 0.3,
        AiBehavior::Aggressive => 0.8,
        AiBehavior::Ambush => 0.6,
        AiBehavior::Guardian => 0.5,
    };
    tags.insert("aggression".to_string(), aggression);

    // Add presence tag from size
    let presence = match size {
        MonsterSize::Tiny => 0.1,
        MonsterSize::Small => 0.3,
        MonsterSize::Medium => 0.5,
        MonsterSize::Large => 0.8,
        MonsterSize::Colossal => 1.0,
    };
    tags.insert("presence".to_string(), presence);

    let loot_tier = (floor_id / 10).max(1);

    MonsterBlueprint {
        variant_id: seed,
        name,
        size,
        element,
        corruption,
        body_type,
        floor_level: floor_id,
        max_health,
        base_damage,
        move_speed,
        aggro_range,
        leash_range,
        semantic_tags: tags,
        ai_behavior,
        loot_tier,
    }
}

/// Generate N monsters for a floor room
pub fn generate_room_monsters(
    tower_seed: u64,
    floor_id: u32,
    room_id: u32,
    biome_tags: &[(String, f32)],
    count: usize,
) -> Vec<MonsterBlueprint> {
    let mut monsters = Vec::with_capacity(count);
    for i in 0..count {
        let monster_seed = tower_seed
            .wrapping_mul(floor_id as u64 + 1)
            .wrapping_mul(room_id as u64 + 1)
            .wrapping_add(i as u64);
        monsters.push(generate_blueprint(monster_seed, floor_id, biome_tags));
    }
    monsters
}

// ============================================================================
// FSM AI System
// ============================================================================

/// AI behavior type (determines patrol/aggro patterns)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AiBehavior {
    /// Won't attack unless provoked
    Passive,
    /// Walks a patrol route, attacks on sight within aggro range
    Patrol,
    /// Actively seeks players, large aggro range
    Aggressive,
    /// Hides and attacks when player is close
    Ambush,
    /// Guards a specific area, won't leave leash range
    Guardian,
}

impl AiBehavior {
    fn from_hash(h: u64) -> Self {
        match h % 10 {
            0..=1 => AiBehavior::Passive,
            2..=4 => AiBehavior::Patrol,
            5..=7 => AiBehavior::Aggressive,
            8 => AiBehavior::Ambush,
            _ => AiBehavior::Guardian,
        }
    }
}

/// Finite State Machine AI component
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct MonsterAi {
    pub behavior: AiBehavior,
    pub state: AiState,
    /// Time spent in current state
    pub state_timer: f32,
    /// Spawn position (for leash/patrol)
    pub home_position: Vec3,
    /// Current target player ID
    pub target_player: Option<u64>,
    /// Aggro range (detect players)
    pub aggro_range: f32,
    /// Leash range (return home if too far)
    pub leash_range: f32,
    /// Attack range (melee/ranged)
    pub attack_range: f32,
    /// Movement speed
    pub move_speed: f32,
    /// Patrol waypoint index
    pub patrol_index: u8,
    /// Patrol waypoints (offsets from home)
    pub patrol_offsets: Vec<[f32; 3]>,
    /// Health threshold to retreat (fraction of max)
    pub retreat_threshold: f32,
    /// Current health fraction
    pub health_fraction: f32,
}

impl Default for MonsterAi {
    fn default() -> Self {
        Self {
            behavior: AiBehavior::Patrol,
            state: AiState::Idle,
            state_timer: 0.0,
            home_position: Vec3::ZERO,
            target_player: None,
            aggro_range: 10.0,
            leash_range: 20.0,
            attack_range: 2.0,
            move_speed: 3.0,
            patrol_index: 0,
            patrol_offsets: vec![
                [5.0, 0.0, 0.0],
                [5.0, 0.0, 5.0],
                [0.0, 0.0, 5.0],
                [0.0, 0.0, 0.0],
            ],
            retreat_threshold: 0.2,
            health_fraction: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiState {
    /// Standing still, waiting for stimulus
    Idle,
    /// Walking between patrol waypoints
    Patrol,
    /// Moving toward detected player
    Chase,
    /// In attack range, executing attacks
    Attack,
    /// Low health, moving away from player
    Retreat,
    /// Returning to home after leash or combat
    ReturnHome,
    /// Dead, awaiting despawn
    Dead,
}

/// Monster semantic tags component (for loot/interaction systems)
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct MonsterSemanticTags {
    pub tags: HashMap<String, f32>,
}

// ============================================================================
// AI System — FSM State Transitions
// ============================================================================

/// System: Update monster AI states every tick
pub fn update_monster_ai(
    time: Res<Time>,
    players: Query<(&Player, &Transform), Without<MonsterAi>>,
    mut monsters: Query<(&mut MonsterAi, &mut Monster, &mut Transform)>,
) {
    let dt = time.delta_secs();

    for (mut ai, mut monster, mut transform) in &mut monsters {
        ai.state_timer += dt;
        ai.health_fraction = monster.health / monster.max_health;

        // Check for death
        if monster.health <= 0.0 {
            ai.state = AiState::Dead;
            continue;
        }

        // Find nearest player
        let (nearest_player_id, nearest_distance) =
            find_nearest_player(&transform.translation, &players);

        // FSM transitions
        let new_state = match ai.state {
            AiState::Idle => {
                if let Some(dist) = nearest_distance {
                    if dist <= ai.aggro_range && ai.behavior != AiBehavior::Passive {
                        ai.target_player = nearest_player_id;
                        AiState::Chase
                    } else if ai.state_timer > 3.0 && ai.behavior == AiBehavior::Patrol {
                        AiState::Patrol
                    } else {
                        AiState::Idle
                    }
                } else if ai.state_timer > 3.0 && ai.behavior == AiBehavior::Patrol {
                    AiState::Patrol
                } else {
                    AiState::Idle
                }
            }

            AiState::Patrol => {
                // Check for player detection
                if let Some(dist) = nearest_distance {
                    if dist <= ai.aggro_range && ai.behavior != AiBehavior::Passive {
                        ai.target_player = nearest_player_id;
                        AiState::Chase
                    } else {
                        // Move toward current patrol waypoint
                        move_toward_patrol(&mut ai, &mut transform, dt);
                        AiState::Patrol
                    }
                } else {
                    move_toward_patrol(&mut ai, &mut transform, dt);
                    AiState::Patrol
                }
            }

            AiState::Chase => {
                // Check retreat
                if ai.health_fraction <= ai.retreat_threshold {
                    AiState::Retreat
                }
                // Check leash
                else if transform.translation.distance(ai.home_position) > ai.leash_range {
                    ai.target_player = None;
                    AiState::ReturnHome
                }
                // Check attack range
                else if let Some(dist) = nearest_distance {
                    if dist <= ai.attack_range {
                        AiState::Attack
                    } else if dist > ai.aggro_range * 1.5 {
                        // Lost target
                        ai.target_player = None;
                        AiState::ReturnHome
                    } else {
                        // Move toward target
                        if let Some(target_pos) = get_player_position(ai.target_player, &players) {
                            move_toward(&mut transform, target_pos, ai.move_speed, dt);
                        }
                        AiState::Chase
                    }
                } else {
                    ai.target_player = None;
                    AiState::ReturnHome
                }
            }

            AiState::Attack => {
                // Check retreat
                if ai.health_fraction <= ai.retreat_threshold {
                    AiState::Retreat
                }
                // Check leash
                else if transform.translation.distance(ai.home_position) > ai.leash_range {
                    ai.target_player = None;
                    AiState::ReturnHome
                }
                // Check if target moved out of attack range
                else if let Some(dist) = nearest_distance {
                    if dist > ai.attack_range * 1.2 {
                        AiState::Chase
                    } else {
                        // Continue attacking (combat system handles actual damage)
                        AiState::Attack
                    }
                } else {
                    ai.target_player = None;
                    AiState::ReturnHome
                }
            }

            AiState::Retreat => {
                // Move away from player, toward home
                let retreat_target = ai.home_position;
                move_toward(&mut transform, retreat_target, ai.move_speed * 1.5, dt);

                if transform.translation.distance(ai.home_position) < 2.0 {
                    AiState::Idle
                } else if ai.health_fraction > ai.retreat_threshold * 1.5 {
                    // Health recovered enough, re-engage
                    AiState::Chase
                } else {
                    AiState::Retreat
                }
            }

            AiState::ReturnHome => {
                move_toward(&mut transform, ai.home_position, ai.move_speed, dt);
                if transform.translation.distance(ai.home_position) < 1.0 {
                    // Heal on return home
                    monster.health = monster.max_health;
                    ai.health_fraction = 1.0;
                    AiState::Idle
                } else {
                    AiState::ReturnHome
                }
            }

            AiState::Dead => AiState::Dead,
        };

        if new_state != ai.state {
            ai.state = new_state;
            ai.state_timer = 0.0;
        }

        // Sync position back to Monster component
        monster.position = transform.translation;
    }
}

// ============================================================================
// AI Helpers
// ============================================================================

fn find_nearest_player(
    monster_pos: &Vec3,
    players: &Query<(&Player, &Transform), Without<MonsterAi>>,
) -> (Option<u64>, Option<f32>) {
    let mut nearest_id = None;
    let mut nearest_dist = f32::MAX;

    for (player, player_transform) in players.iter() {
        let dist = monster_pos.distance(player_transform.translation);
        if dist < nearest_dist {
            nearest_dist = dist;
            nearest_id = Some(player.id);
        }
    }

    if nearest_id.is_some() {
        (nearest_id, Some(nearest_dist))
    } else {
        (None, None)
    }
}

fn get_player_position(
    target_id: Option<u64>,
    players: &Query<(&Player, &Transform), Without<MonsterAi>>,
) -> Option<Vec3> {
    let target = target_id?;
    players
        .iter()
        .find(|(p, _)| p.id == target)
        .map(|(_, t)| t.translation)
}

fn move_toward(transform: &mut Transform, target: Vec3, speed: f32, dt: f32) {
    let dir = (target - transform.translation).normalize_or_zero();
    transform.translation += dir * speed * dt;
}

fn move_toward_patrol(ai: &mut MonsterAi, transform: &mut Transform, dt: f32) {
    if ai.patrol_offsets.is_empty() {
        return;
    }

    let idx = ai.patrol_index as usize % ai.patrol_offsets.len();
    let offset = ai.patrol_offsets[idx];
    let target = ai.home_position + Vec3::new(offset[0], offset[1], offset[2]);

    move_toward(transform, target, ai.move_speed * 0.5, dt);

    if transform.translation.distance(target) < 1.0 {
        ai.patrol_index = (ai.patrol_index + 1) % ai.patrol_offsets.len() as u8;
    }
}

// ============================================================================
// Spawning — Blueprint → ECS Entity
// ============================================================================

/// Spawn a monster entity from a blueprint at a given position.
/// Returns the spawned entity.
pub fn spawn_monster_from_blueprint(
    commands: &mut Commands,
    blueprint: &MonsterBlueprint,
    position: Vec3,
) -> Entity {
    let entity = commands
        .spawn((
            Monster {
                monster_type: blueprint.name.clone(),
                position,
                health: blueprint.max_health,
                max_health: blueprint.max_health,
            },
            Transform::from_translation(position),
            MonsterAi {
                behavior: blueprint.ai_behavior,
                state: AiState::Idle,
                state_timer: 0.0,
                home_position: position,
                target_player: None,
                aggro_range: blueprint.aggro_range,
                leash_range: blueprint.leash_range,
                attack_range: 2.5,
                move_speed: blueprint.move_speed,
                patrol_index: 0,
                patrol_offsets: generate_patrol_offsets(blueprint.variant_id),
                retreat_threshold: 0.2,
                health_fraction: 1.0,
            },
            CombatState::default(),
            EquippedWeapon {
                weapon_type: WeaponType::Sword, // Monsters use generic melee
                weapon_id: format!("monster_weapon_{}", blueprint.variant_id),
                base_damage: blueprint.base_damage,
                attack_speed: blueprint.move_speed / 3.0, // Faster monsters attack faster
                range: 2.5,
            },
            CombatEnergy::default(),
            MonsterSemanticTags {
                tags: blueprint.semantic_tags.clone(),
            },
            // Physics collider — capsule sized by monster size
            physics::monster_physics_bundle(blueprint.size),
        ))
        .id();

    entity
}

/// Generate patrol waypoints from monster seed
fn generate_patrol_offsets(seed: u64) -> Vec<[f32; 3]> {
    let mut offsets = Vec::new();
    let mut h = seed;
    let count = 3 + (h % 3) as usize; // 3-5 waypoints

    for _ in 0..count {
        h = h.wrapping_mul(6364136223846793005).wrapping_add(1);
        let x = ((h >> 32) % 10) as f32 - 5.0; // -5 to +5
        h = h.wrapping_mul(6364136223846793005).wrapping_add(1);
        let z = ((h >> 32) % 10) as f32 - 5.0;
        offsets.push([x, 0.0, z]);
    }

    offsets
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_blueprint_deterministic() {
        let biome = vec![("forest".to_string(), 0.8), ("nature".to_string(), 0.6)];
        let bp1 = generate_blueprint(42, 5, &biome);
        let bp2 = generate_blueprint(42, 5, &biome);
        assert_eq!(bp1.name, bp2.name);
        assert_eq!(bp1.max_health, bp2.max_health);
        assert_eq!(bp1.base_damage, bp2.base_damage);
    }

    #[test]
    fn test_different_seeds_different_monsters() {
        let biome = vec![("dungeon".to_string(), 0.7)];
        let bp1 = generate_blueprint(100, 1, &biome);
        let bp2 = generate_blueprint(200, 1, &biome);
        // Different seeds should produce different monsters (very high probability)
        assert_ne!(bp1.variant_id, bp2.variant_id);
    }

    #[test]
    fn test_floor_scaling() {
        let biome = vec![];
        let floor1 = generate_blueprint(42, 1, &biome);
        let floor10 = generate_blueprint(42, 10, &biome);
        // Floor 10 should have higher stats than floor 1
        assert!(floor10.max_health > floor1.max_health);
        assert!(floor10.base_damage > floor1.base_damage);
    }

    #[test]
    fn test_semantic_tag_inheritance() {
        let biome = vec![("fire".to_string(), 0.9), ("volcano".to_string(), 0.8)];
        let bp = generate_blueprint(42, 5, &biome);

        // Should have inherited floor tags at 70% weight
        let fire_weight = bp.semantic_tags.get("fire");
        assert!(fire_weight.is_some());

        let volcano_weight = bp.semantic_tags.get("volcano");
        assert!(volcano_weight.is_some());
        // 0.8 * 0.7 = 0.56
        assert!((volcano_weight.unwrap() - 0.56).abs() < 0.01);

        // Should also have own tags
        assert!(bp.semantic_tags.contains_key("aggression"));
        assert!(bp.semantic_tags.contains_key("presence"));
    }

    #[test]
    fn test_name_generation() {
        let name = generate_name(
            CorruptionLevel::Corrupted,
            MonsterElement::Fire,
            MonsterBodyType::Beast,
            MonsterSize::Large,
        );
        assert_eq!(name, "Corrupted Ember Fang Warden");
    }

    #[test]
    fn test_name_pure_no_prefix() {
        let name = generate_name(
            CorruptionLevel::Pure,
            MonsterElement::Water,
            MonsterBodyType::Humanoid,
            MonsterSize::Small,
        );
        assert_eq!(name, "Tide Knight Scout");
    }

    #[test]
    fn test_size_multipliers() {
        // Tiny: high speed, low HP
        assert!(MonsterSize::Tiny.speed_mult() > MonsterSize::Colossal.speed_mult());
        assert!(MonsterSize::Tiny.hp_mult() < MonsterSize::Colossal.hp_mult());
    }

    #[test]
    fn test_corruption_increases_with_floor() {
        // On floor 1, corruption should mostly be Pure
        // On floor 100, corruption should mostly be Corrupted/Abyssal
        let mut high_corruption_count_f1 = 0;
        let mut high_corruption_count_f100 = 0;
        for seed in 0..100u64 {
            let h = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let h = h.wrapping_mul(6364136223846793005).wrapping_add(1);
            let h = h.wrapping_mul(6364136223846793005).wrapping_add(1);
            if matches!(
                CorruptionLevel::from_hash(h >> 32, 1),
                CorruptionLevel::Corrupted | CorruptionLevel::Abyssal
            ) {
                high_corruption_count_f1 += 1;
            }
            if matches!(
                CorruptionLevel::from_hash(h >> 32, 100),
                CorruptionLevel::Corrupted | CorruptionLevel::Abyssal
            ) {
                high_corruption_count_f100 += 1;
            }
        }
        assert!(high_corruption_count_f100 > high_corruption_count_f1);
    }

    #[test]
    fn test_generate_room_monsters() {
        let biome = vec![("dungeon".to_string(), 0.7)];
        let monsters = generate_room_monsters(12345, 5, 1, &biome, 5);
        assert_eq!(monsters.len(), 5);
        // All should have valid stats
        for m in &monsters {
            assert!(m.max_health > 0.0);
            assert!(m.base_damage > 0.0);
            assert!(m.move_speed > 0.0);
            assert!(!m.name.is_empty());
        }
    }

    #[test]
    fn test_ai_behavior_distribution() {
        // Ensure all behaviors are reachable
        let mut found = std::collections::HashSet::new();
        for i in 0..100u64 {
            found.insert(AiBehavior::from_hash(i));
        }
        assert!(found.contains(&AiBehavior::Passive));
        assert!(found.contains(&AiBehavior::Patrol));
        assert!(found.contains(&AiBehavior::Aggressive));
        assert!(found.contains(&AiBehavior::Ambush));
        assert!(found.contains(&AiBehavior::Guardian));
    }

    #[test]
    fn test_patrol_offsets_generation() {
        let offsets = generate_patrol_offsets(42);
        assert!(offsets.len() >= 3 && offsets.len() <= 5);
        for o in &offsets {
            assert!(o[0] >= -5.0 && o[0] <= 5.0);
            assert_eq!(o[1], 0.0);
            assert!(o[2] >= -5.0 && o[2] <= 5.0);
        }
    }

    #[test]
    fn test_element_coverage() {
        let mut found = std::collections::HashSet::new();
        for i in 0..6u64 {
            found.insert(MonsterElement::from_hash(i));
        }
        assert_eq!(found.len(), 6); // All 6 elements reachable
    }

    #[test]
    fn test_body_type_coverage() {
        let mut found = std::collections::HashSet::new();
        for i in 0..6u64 {
            found.insert(MonsterBodyType::from_hash(i));
        }
        assert_eq!(found.len(), 6);
    }
}
