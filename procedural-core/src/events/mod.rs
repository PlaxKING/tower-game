//! Procedural Event System
//!
//! 7 semantic trigger types for dynamic world events:
//! 1. BreathShift — triggered by Breath of Tower phase transitions
//! 2. SemanticResonance — when floor biome tags align with player actions
//! 3. EchoConvergence — multiple death echoes in proximity
//! 4. FloorAnomaly — rare procedural anomaly spawns
//! 5. FactionClash — two factions' influence overlapping
//! 6. CorruptionSurge — corruption level exceeds threshold
//! 7. TowerMemory — the tower "remembers" and reacts to repeated player behavior

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

use crate::semantic::SemanticTags;

pub struct EventsPlugin;

impl Plugin for EventsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EventManager::default())
            .add_event::<WorldEvent>()
            .add_event::<EventTrigger>();
    }
}

/// The 7 semantic trigger types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventTriggerType {
    /// Breath of Tower phase transitions
    BreathShift,
    /// Floor biome tags resonate with player semantic profile
    SemanticResonance,
    /// Multiple death echoes converge in an area
    EchoConvergence,
    /// Rare procedural anomaly (bonus room, NPC, portal)
    FloorAnomaly,
    /// Two factions' influence zones overlap
    FactionClash,
    /// Corruption level exceeds safety threshold
    CorruptionSurge,
    /// Tower remembers and reacts to player patterns
    TowerMemory,
}

/// Severity of the event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventSeverity {
    Minor,    // Visual/audio cue only
    Moderate, // Gameplay modifier
    Major,    // Spawns new entities or changes floor layout
    Critical, // Floor-wide transformation
}

/// Effect that an event applies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventEffect {
    /// Spawn additional monsters with given element bias
    SpawnMonsters { count: u32, element_bias: String },
    /// Buff all players on floor
    PlayerBuff {
        stat: String,
        multiplier: f32,
        duration_secs: f32,
    },
    /// Debuff - environmental hazard
    EnvironmentalHazard {
        damage_per_sec: f32,
        duration_secs: f32,
        element: String,
    },
    /// Spawn bonus loot chest
    BonusLoot { rarity_boost: u32 },
    /// Open a secret passage
    SecretPassage { target_room: u32 },
    /// Semantic tag shift for the floor
    TagShift { tag: String, delta: f32 },
    /// NPC appears with special dialog/quest
    NPCAppearance {
        faction: String,
        quest_available: bool,
    },
    /// Visual/atmospheric change only
    AtmosphericChange { intensity: f32, color_shift: String },
    /// Corruption wave damages all entities
    CorruptionWave {
        damage: f32,
        corruption_increase: f32,
    },
    /// Tower reveals hidden information
    Revelation { hint_type: String, content: String },
}

/// A world event instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEventData {
    pub id: u64,
    pub trigger_type: EventTriggerType,
    pub severity: EventSeverity,
    pub name: String,
    pub description: String,
    pub floor_id: u32,
    pub effects: Vec<EventEffect>,
    pub duration_secs: f32,
    pub semantic_tags: Vec<(String, f32)>,
}

/// Bevy event for when a world event fires
#[derive(Event, Debug, Clone)]
pub struct WorldEvent {
    pub data: WorldEventData,
}

/// Bevy event to request evaluation of a trigger
#[derive(Event, Debug, Clone)]
pub struct EventTrigger {
    pub trigger_type: EventTriggerType,
    pub floor_id: u32,
    pub context: TriggerContext,
}

/// Context data for evaluating triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerContext {
    /// Current breath phase
    pub breath_phase: Option<String>,
    /// Floor biome tags
    pub floor_tags: Vec<(String, f32)>,
    /// Player semantic profile
    pub player_tags: Vec<(String, f32)>,
    /// Number of echoes nearby
    pub echo_count: u32,
    /// Current corruption level (0.0-1.0)
    pub corruption_level: f32,
    /// Active factions on floor
    pub active_factions: Vec<String>,
    /// Player action history (last N actions as tag)
    pub action_history: Vec<String>,
    /// Floor-specific hash for determinism
    pub floor_hash: u64,
}

impl Default for TriggerContext {
    fn default() -> Self {
        Self {
            breath_phase: None,
            floor_tags: vec![],
            player_tags: vec![],
            echo_count: 0,
            corruption_level: 0.0,
            active_factions: vec![],
            action_history: vec![],
            floor_hash: 0,
        }
    }
}

/// Manages event generation and cooldowns
#[derive(Resource, Debug, Clone, Default)]
pub struct EventManager {
    pub active_events: Vec<ActiveEvent>,
    pub cooldowns: Vec<(EventTriggerType, f32)>,
    pub events_triggered: u64,
}

#[derive(Debug, Clone)]
pub struct ActiveEvent {
    pub data: WorldEventData,
    pub remaining_secs: f32,
}

impl EventManager {
    /// Check if a trigger type is on cooldown
    pub fn is_on_cooldown(&self, trigger_type: EventTriggerType) -> bool {
        self.cooldowns
            .iter()
            .any(|(t, remaining)| *t == trigger_type && *remaining > 0.0)
    }

    /// Set cooldown for a trigger type
    pub fn set_cooldown(&mut self, trigger_type: EventTriggerType, duration: f32) {
        if let Some(cd) = self.cooldowns.iter_mut().find(|(t, _)| *t == trigger_type) {
            cd.1 = duration;
        } else {
            self.cooldowns.push((trigger_type, duration));
        }
    }

    /// Tick cooldowns and active events
    pub fn tick(&mut self, dt: f32) {
        for (_, remaining) in self.cooldowns.iter_mut() {
            *remaining = (*remaining - dt).max(0.0);
        }
        self.cooldowns.retain(|(_, r)| *r > 0.0);

        for event in self.active_events.iter_mut() {
            event.remaining_secs -= dt;
        }
        self.active_events.retain(|e| e.remaining_secs > 0.0);
    }

    /// Get default cooldown for a trigger type
    pub fn default_cooldown(trigger_type: EventTriggerType) -> f32 {
        match trigger_type {
            EventTriggerType::BreathShift => 60.0,
            EventTriggerType::SemanticResonance => 120.0,
            EventTriggerType::EchoConvergence => 90.0,
            EventTriggerType::FloorAnomaly => 300.0,
            EventTriggerType::FactionClash => 180.0,
            EventTriggerType::CorruptionSurge => 150.0,
            EventTriggerType::TowerMemory => 240.0,
        }
    }
}

/// Evaluate a trigger and potentially generate an event
pub fn evaluate_trigger(
    trigger_type: EventTriggerType,
    context: &TriggerContext,
) -> Option<WorldEventData> {
    match trigger_type {
        EventTriggerType::BreathShift => evaluate_breath_shift(context),
        EventTriggerType::SemanticResonance => evaluate_semantic_resonance(context),
        EventTriggerType::EchoConvergence => evaluate_echo_convergence(context),
        EventTriggerType::FloorAnomaly => evaluate_floor_anomaly(context),
        EventTriggerType::FactionClash => evaluate_faction_clash(context),
        EventTriggerType::CorruptionSurge => evaluate_corruption_surge(context),
        EventTriggerType::TowerMemory => evaluate_tower_memory(context),
    }
}

fn event_hash(context: &TriggerContext, salt: &str) -> u64 {
    let mut hasher = Sha3_256::new();
    hasher.update(context.floor_hash.to_le_bytes());
    hasher.update(salt.as_bytes());
    let result = hasher.finalize();
    u64::from_le_bytes(result[0..8].try_into().unwrap())
}

fn evaluate_breath_shift(ctx: &TriggerContext) -> Option<WorldEventData> {
    let phase = ctx.breath_phase.as_deref()?;
    let hash = event_hash(ctx, "breath_shift");

    let (name, desc, effects, severity) = match phase {
        "Hold" => (
            "Tower's Peak Resonance",
            "The tower reaches maximum power. Rare creatures stir.",
            vec![
                EventEffect::SpawnMonsters {
                    count: 2,
                    element_bias: "void".into(),
                },
                EventEffect::PlayerBuff {
                    stat: "damage".into(),
                    multiplier: 1.2,
                    duration_secs: 60.0,
                },
            ],
            EventSeverity::Major,
        ),
        "Exhale" => (
            "Tower's Exhalation",
            "Energy flows outward. Hidden paths reveal themselves.",
            vec![
                EventEffect::SecretPassage {
                    target_room: (hash % 10) as u32,
                },
                EventEffect::AtmosphericChange {
                    intensity: 0.7,
                    color_shift: "blue".into(),
                },
            ],
            EventSeverity::Moderate,
        ),
        "Pause" => (
            "Tower's Rest",
            "A peaceful lull. Echoes become visible.",
            vec![
                EventEffect::PlayerBuff {
                    stat: "healing".into(),
                    multiplier: 1.5,
                    duration_secs: 120.0,
                },
                EventEffect::AtmosphericChange {
                    intensity: 0.3,
                    color_shift: "golden".into(),
                },
            ],
            EventSeverity::Minor,
        ),
        _ => (
            "Tower's Inhalation",
            "The tower draws energy inward. Resources shimmer.",
            vec![
                EventEffect::BonusLoot { rarity_boost: 1 },
                EventEffect::TagShift {
                    tag: "energy".into(),
                    delta: 0.2,
                },
            ],
            EventSeverity::Moderate,
        ),
    };

    Some(WorldEventData {
        id: hash,
        trigger_type: EventTriggerType::BreathShift,
        severity,
        name: name.to_string(),
        description: desc.to_string(),
        floor_id: ctx.floor_hash as u32 & 0xFFFF,
        effects,
        duration_secs: 30.0,
        semantic_tags: vec![("breath".into(), 0.9), ("energy".into(), 0.5)],
    })
}

fn evaluate_semantic_resonance(ctx: &TriggerContext) -> Option<WorldEventData> {
    let floor_tags = SemanticTags {
        tags: ctx.floor_tags.clone(),
    };
    let player_tags = SemanticTags {
        tags: ctx.player_tags.clone(),
    };
    let similarity = floor_tags.similarity(&player_tags);

    // Only trigger when high resonance
    if similarity < 0.6 {
        return None;
    }

    let hash = event_hash(ctx, "resonance");
    let dominant = floor_tags.dominant().map(|(t, _)| t).unwrap_or("neutral");

    Some(WorldEventData {
        id: hash,
        trigger_type: EventTriggerType::SemanticResonance,
        severity: if similarity > 0.8 {
            EventSeverity::Major
        } else {
            EventSeverity::Moderate
        },
        name: format!("{} Resonance", capitalize(dominant)),
        description: format!(
            "Your affinity with {} resonates through the floor.",
            dominant
        ),
        floor_id: ctx.floor_hash as u32 & 0xFFFF,
        effects: vec![
            EventEffect::PlayerBuff {
                stat: dominant.to_string(),
                multiplier: 1.0 + similarity * 0.5,
                duration_secs: 45.0,
            },
            EventEffect::TagShift {
                tag: dominant.to_string(),
                delta: 0.15,
            },
        ],
        duration_secs: 45.0,
        semantic_tags: ctx.floor_tags.clone(),
    })
}

fn evaluate_echo_convergence(ctx: &TriggerContext) -> Option<WorldEventData> {
    if ctx.echo_count < 3 {
        return None;
    }

    let hash = event_hash(ctx, "echo_conv");
    let severity = if ctx.echo_count >= 5 {
        EventSeverity::Critical
    } else {
        EventSeverity::Major
    };

    Some(WorldEventData {
        id: hash,
        trigger_type: EventTriggerType::EchoConvergence,
        severity,
        name: "Echo Convergence".to_string(),
        description: format!(
            "{} death echoes converge, distorting reality.",
            ctx.echo_count
        ),
        floor_id: ctx.floor_hash as u32 & 0xFFFF,
        effects: vec![
            EventEffect::SpawnMonsters {
                count: ctx.echo_count / 2,
                element_bias: "void".into(),
            },
            EventEffect::BonusLoot { rarity_boost: 2 },
            EventEffect::AtmosphericChange {
                intensity: 0.9,
                color_shift: "purple".into(),
            },
        ],
        duration_secs: 60.0,
        semantic_tags: vec![
            ("death".into(), 0.8),
            ("void".into(), 0.6),
            ("echo".into(), 1.0),
        ],
    })
}

fn evaluate_floor_anomaly(ctx: &TriggerContext) -> Option<WorldEventData> {
    let hash = event_hash(ctx, "anomaly");

    // Low probability: hash must end in specific pattern
    if hash % 100 > 15 {
        return None; // 15% chance
    }

    let anomaly_type = (hash / 100) % 4;
    let (name, desc, effects) = match anomaly_type {
        0 => (
            "Dimensional Rift",
            "A tear in the tower's fabric reveals a hidden chamber.",
            vec![
                EventEffect::SecretPassage {
                    target_room: (hash % 20) as u32,
                },
                EventEffect::BonusLoot { rarity_boost: 3 },
            ],
        ),
        1 => (
            "Wandering Merchant",
            "A mysterious trader appears between the walls.",
            vec![EventEffect::NPCAppearance {
                faction: "neutral".into(),
                quest_available: false,
            }],
        ),
        2 => (
            "Crystalline Growth",
            "Strange crystals emerge from the floor, pulsing with energy.",
            vec![
                EventEffect::BonusLoot { rarity_boost: 1 },
                EventEffect::TagShift {
                    tag: "crystal".into(),
                    delta: 0.3,
                },
                EventEffect::AtmosphericChange {
                    intensity: 0.5,
                    color_shift: "cyan".into(),
                },
            ],
        ),
        _ => (
            "Temporal Echo",
            "Time stutters. The tower shows a glimpse of its past.",
            vec![
                EventEffect::Revelation {
                    hint_type: "history".into(),
                    content: "An ancient floor configuration flickers into view.".into(),
                },
                EventEffect::PlayerBuff {
                    stat: "perception".into(),
                    multiplier: 1.3,
                    duration_secs: 30.0,
                },
            ],
        ),
    };

    Some(WorldEventData {
        id: hash,
        trigger_type: EventTriggerType::FloorAnomaly,
        severity: EventSeverity::Major,
        name: name.to_string(),
        description: desc.to_string(),
        floor_id: ctx.floor_hash as u32 & 0xFFFF,
        effects,
        duration_secs: 90.0,
        semantic_tags: vec![("anomaly".into(), 1.0), ("mystery".into(), 0.7)],
    })
}

fn evaluate_faction_clash(ctx: &TriggerContext) -> Option<WorldEventData> {
    if ctx.active_factions.len() < 2 {
        return None;
    }

    let hash = event_hash(ctx, "faction_clash");
    let f1 = &ctx.active_factions[0];
    let f2 = &ctx.active_factions[1];

    Some(WorldEventData {
        id: hash,
        trigger_type: EventTriggerType::FactionClash,
        severity: EventSeverity::Moderate,
        name: format!("{} vs {} Clash", capitalize(f1), capitalize(f2)),
        description: format!("The {} and {} factions contest this territory.", f1, f2),
        floor_id: ctx.floor_hash as u32 & 0xFFFF,
        effects: vec![
            EventEffect::SpawnMonsters {
                count: 3,
                element_bias: f1.clone(),
            },
            EventEffect::NPCAppearance {
                faction: f2.clone(),
                quest_available: true,
            },
            EventEffect::TagShift {
                tag: "conflict".into(),
                delta: 0.25,
            },
        ],
        duration_secs: 120.0,
        semantic_tags: vec![
            ("faction".into(), 0.9),
            ("conflict".into(), 0.7),
            (f1.clone(), 0.5),
            (f2.clone(), 0.5),
        ],
    })
}

fn evaluate_corruption_surge(ctx: &TriggerContext) -> Option<WorldEventData> {
    if ctx.corruption_level < 0.6 {
        return None;
    }

    let hash = event_hash(ctx, "corruption_surge");
    let severity = if ctx.corruption_level > 0.85 {
        EventSeverity::Critical
    } else {
        EventSeverity::Major
    };

    Some(WorldEventData {
        id: hash,
        trigger_type: EventTriggerType::CorruptionSurge,
        severity,
        name: "Corruption Surge".to_string(),
        description: format!(
            "Corruption reaches {:.0}%. The tower writhes.",
            ctx.corruption_level * 100.0
        ),
        floor_id: ctx.floor_hash as u32 & 0xFFFF,
        effects: vec![
            EventEffect::CorruptionWave {
                damage: ctx.corruption_level * 20.0,
                corruption_increase: 0.1,
            },
            EventEffect::EnvironmentalHazard {
                damage_per_sec: ctx.corruption_level * 5.0,
                duration_secs: 30.0,
                element: "corruption".into(),
            },
            EventEffect::SpawnMonsters {
                count: 4,
                element_bias: "corruption".into(),
            },
        ],
        duration_secs: 30.0,
        semantic_tags: vec![
            ("corruption".into(), ctx.corruption_level),
            ("danger".into(), 0.9),
        ],
    })
}

fn evaluate_tower_memory(ctx: &TriggerContext) -> Option<WorldEventData> {
    if ctx.action_history.len() < 5 {
        return None;
    }

    // Detect repeated action patterns
    let mut action_counts = std::collections::HashMap::new();
    for action in &ctx.action_history {
        *action_counts.entry(action.as_str()).or_insert(0u32) += 1;
    }

    let (dominant_action, count) = action_counts
        .iter()
        .max_by_key(|(_, c)| *c)
        .map(|(a, c)| (*a, *c))?;

    // Need at least 3 repetitions of same action
    if count < 3 {
        return None;
    }

    let hash = event_hash(ctx, "tower_memory");

    let (name, desc, effects) = match dominant_action {
        "attack" | "combat" => (
            "Tower Remembers Violence",
            "The tower recognizes your aggressive nature and responds.",
            vec![
                EventEffect::SpawnMonsters {
                    count: 5,
                    element_bias: "aggression".into(),
                },
                EventEffect::PlayerBuff {
                    stat: "damage".into(),
                    multiplier: 1.3,
                    duration_secs: 60.0,
                },
            ],
        ),
        "explore" | "discover" => (
            "Tower Guides the Curious",
            "The tower senses your explorative spirit and reveals secrets.",
            vec![
                EventEffect::SecretPassage {
                    target_room: (hash % 15) as u32,
                },
                EventEffect::Revelation {
                    hint_type: "map".into(),
                    content: "Hidden passages glow faintly.".into(),
                },
            ],
        ),
        "craft" | "gather" => (
            "Tower Nourishes the Crafter",
            "The tower recognizes your creative efforts.",
            vec![
                EventEffect::BonusLoot { rarity_boost: 2 },
                EventEffect::TagShift {
                    tag: "crafting".into(),
                    delta: 0.2,
                },
            ],
        ),
        _ => (
            "Tower's Whisper",
            "The tower stirs, acknowledging your presence.",
            vec![
                EventEffect::AtmosphericChange {
                    intensity: 0.4,
                    color_shift: "white".into(),
                },
                EventEffect::Revelation {
                    hint_type: "lore".into(),
                    content: "Ancient writing appears on the walls.".into(),
                },
            ],
        ),
    };

    Some(WorldEventData {
        id: hash,
        trigger_type: EventTriggerType::TowerMemory,
        severity: EventSeverity::Moderate,
        name: name.to_string(),
        description: desc.to_string(),
        floor_id: ctx.floor_hash as u32 & 0xFFFF,
        effects,
        duration_secs: 60.0,
        semantic_tags: vec![
            ("memory".into(), 0.8),
            ("tower".into(), 0.6),
            (dominant_action.to_string(), 0.5),
        ],
    })
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_context() -> TriggerContext {
        TriggerContext {
            breath_phase: Some("Hold".into()),
            floor_tags: vec![("fire".into(), 0.7), ("corruption".into(), 0.3)],
            player_tags: vec![("fire".into(), 0.8), ("combat".into(), 0.5)],
            echo_count: 0,
            corruption_level: 0.0,
            active_factions: vec![],
            action_history: vec![],
            floor_hash: 42,
        }
    }

    #[test]
    fn test_breath_shift_hold() {
        let ctx = base_context();
        let event = evaluate_trigger(EventTriggerType::BreathShift, &ctx);
        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(event.trigger_type, EventTriggerType::BreathShift);
        assert_eq!(event.name, "Tower's Peak Resonance");
        assert_eq!(event.severity, EventSeverity::Major);
    }

    #[test]
    fn test_breath_shift_exhale() {
        let mut ctx = base_context();
        ctx.breath_phase = Some("Exhale".into());
        let event = evaluate_trigger(EventTriggerType::BreathShift, &ctx);
        assert!(event.is_some());
        assert_eq!(event.unwrap().name, "Tower's Exhalation");
    }

    #[test]
    fn test_breath_shift_pause() {
        let mut ctx = base_context();
        ctx.breath_phase = Some("Pause".into());
        let event = evaluate_trigger(EventTriggerType::BreathShift, &ctx);
        assert!(event.is_some());
        assert_eq!(event.unwrap().name, "Tower's Rest");
    }

    #[test]
    fn test_semantic_resonance_high() {
        let ctx = TriggerContext {
            floor_tags: vec![("fire".into(), 0.8), ("aggression".into(), 0.5)],
            player_tags: vec![("fire".into(), 0.9), ("aggression".into(), 0.4)],
            floor_hash: 42,
            ..Default::default()
        };
        let event = evaluate_trigger(EventTriggerType::SemanticResonance, &ctx);
        assert!(event.is_some());
        let event = event.unwrap();
        assert!(event.name.contains("Resonance"));
    }

    #[test]
    fn test_semantic_resonance_low_no_trigger() {
        let ctx = TriggerContext {
            floor_tags: vec![("fire".into(), 0.9)],
            player_tags: vec![("water".into(), 0.9)],
            floor_hash: 42,
            ..Default::default()
        };
        let event = evaluate_trigger(EventTriggerType::SemanticResonance, &ctx);
        assert!(
            event.is_none(),
            "Low similarity should not trigger resonance"
        );
    }

    #[test]
    fn test_echo_convergence() {
        let mut ctx = base_context();
        ctx.echo_count = 4;
        let event = evaluate_trigger(EventTriggerType::EchoConvergence, &ctx);
        assert!(event.is_some());
        assert_eq!(event.unwrap().severity, EventSeverity::Major);
    }

    #[test]
    fn test_echo_convergence_critical() {
        let mut ctx = base_context();
        ctx.echo_count = 6;
        let event = evaluate_trigger(EventTriggerType::EchoConvergence, &ctx);
        assert!(event.is_some());
        assert_eq!(event.unwrap().severity, EventSeverity::Critical);
    }

    #[test]
    fn test_echo_convergence_too_few() {
        let mut ctx = base_context();
        ctx.echo_count = 2;
        let event = evaluate_trigger(EventTriggerType::EchoConvergence, &ctx);
        assert!(event.is_none());
    }

    #[test]
    fn test_faction_clash() {
        let mut ctx = base_context();
        ctx.active_factions = vec!["seekers".into(), "breakers".into()];
        let event = evaluate_trigger(EventTriggerType::FactionClash, &ctx);
        assert!(event.is_some());
        let event = event.unwrap();
        assert!(event.name.contains("Seekers"));
        assert!(event.name.contains("Breakers"));
    }

    #[test]
    fn test_faction_clash_single_faction() {
        let mut ctx = base_context();
        ctx.active_factions = vec!["seekers".into()];
        let event = evaluate_trigger(EventTriggerType::FactionClash, &ctx);
        assert!(event.is_none(), "Need at least 2 factions");
    }

    #[test]
    fn test_corruption_surge() {
        let mut ctx = base_context();
        ctx.corruption_level = 0.7;
        let event = evaluate_trigger(EventTriggerType::CorruptionSurge, &ctx);
        assert!(event.is_some());
        assert_eq!(event.unwrap().severity, EventSeverity::Major);
    }

    #[test]
    fn test_corruption_surge_critical() {
        let mut ctx = base_context();
        ctx.corruption_level = 0.9;
        let event = evaluate_trigger(EventTriggerType::CorruptionSurge, &ctx);
        assert!(event.is_some());
        assert_eq!(event.unwrap().severity, EventSeverity::Critical);
    }

    #[test]
    fn test_corruption_surge_low() {
        let mut ctx = base_context();
        ctx.corruption_level = 0.3;
        let event = evaluate_trigger(EventTriggerType::CorruptionSurge, &ctx);
        assert!(event.is_none(), "Low corruption should not trigger surge");
    }

    #[test]
    fn test_tower_memory_combat() {
        let mut ctx = base_context();
        ctx.action_history = vec!["attack".into(); 5];
        let event = evaluate_trigger(EventTriggerType::TowerMemory, &ctx);
        assert!(event.is_some());
        assert!(event.unwrap().name.contains("Violence"));
    }

    #[test]
    fn test_tower_memory_explore() {
        let mut ctx = base_context();
        ctx.action_history = vec!["explore".into(); 5];
        let event = evaluate_trigger(EventTriggerType::TowerMemory, &ctx);
        assert!(event.is_some());
        assert!(event.unwrap().name.contains("Curious"));
    }

    #[test]
    fn test_tower_memory_too_few_actions() {
        let mut ctx = base_context();
        ctx.action_history = vec!["attack".into(); 2];
        let event = evaluate_trigger(EventTriggerType::TowerMemory, &ctx);
        assert!(event.is_none());
    }

    #[test]
    fn test_event_manager_cooldown() {
        let mut mgr = EventManager::default();
        mgr.set_cooldown(EventTriggerType::BreathShift, 60.0);
        assert!(mgr.is_on_cooldown(EventTriggerType::BreathShift));
        assert!(!mgr.is_on_cooldown(EventTriggerType::CorruptionSurge));

        mgr.tick(61.0);
        assert!(!mgr.is_on_cooldown(EventTriggerType::BreathShift));
    }

    #[test]
    fn test_event_manager_active_events() {
        let mut mgr = EventManager::default();
        mgr.active_events.push(ActiveEvent {
            data: WorldEventData {
                id: 1,
                trigger_type: EventTriggerType::BreathShift,
                severity: EventSeverity::Minor,
                name: "Test".into(),
                description: "Test".into(),
                floor_id: 1,
                effects: vec![],
                duration_secs: 10.0,
                semantic_tags: vec![],
            },
            remaining_secs: 10.0,
        });

        assert_eq!(mgr.active_events.len(), 1);
        mgr.tick(5.0);
        assert_eq!(mgr.active_events.len(), 1);
        mgr.tick(6.0);
        assert_eq!(
            mgr.active_events.len(),
            0,
            "Expired event should be removed"
        );
    }

    #[test]
    fn test_all_trigger_types_exist() {
        let types = [
            EventTriggerType::BreathShift,
            EventTriggerType::SemanticResonance,
            EventTriggerType::EchoConvergence,
            EventTriggerType::FloorAnomaly,
            EventTriggerType::FactionClash,
            EventTriggerType::CorruptionSurge,
            EventTriggerType::TowerMemory,
        ];
        assert_eq!(types.len(), 7, "Must have 7 trigger types");
    }

    #[test]
    fn test_default_cooldowns() {
        for trigger in [
            EventTriggerType::BreathShift,
            EventTriggerType::SemanticResonance,
            EventTriggerType::FloorAnomaly,
        ] {
            let cd = EventManager::default_cooldown(trigger);
            assert!(cd > 0.0, "All triggers should have positive cooldowns");
        }
    }

    #[test]
    fn test_event_serialization() {
        let event = WorldEventData {
            id: 42,
            trigger_type: EventTriggerType::BreathShift,
            severity: EventSeverity::Major,
            name: "Test Event".into(),
            description: "Testing".into(),
            floor_id: 5,
            effects: vec![EventEffect::PlayerBuff {
                stat: "damage".into(),
                multiplier: 1.2,
                duration_secs: 30.0,
            }],
            duration_secs: 30.0,
            semantic_tags: vec![("test".into(), 0.5)],
        };

        let json = serde_json::to_string(&event).unwrap();
        let restored: WorldEventData = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, 42);
        assert_eq!(restored.name, "Test Event");
    }

    #[test]
    fn test_capitalize() {
        assert_eq!(capitalize("seekers"), "Seekers");
        assert_eq!(capitalize(""), "");
        assert_eq!(capitalize("a"), "A");
    }
}
