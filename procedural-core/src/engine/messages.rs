use serde::{Deserialize, Serialize};

// =====================================================
// Shared response types (mirror proto messages)
// =====================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vec3Msg {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticTagMsg {
    pub name: String,
    pub value: f32,
}

impl From<(String, f32)> for SemanticTagMsg {
    fn from((name, value): (String, f32)) -> Self {
        Self { name, value }
    }
}

impl From<&(String, f32)> for SemanticTagMsg {
    fn from((name, value): &(String, f32)) -> Self {
        Self {
            name: name.clone(),
            value: *value,
        }
    }
}

// --- GameState types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStateMsg {
    pub entity_id: u64,
    pub position: Vec3Msg,
    pub health: f32,
    pub max_health: f32,
    pub resources: CombatResourcesMsg,
    pub mastery: MasterySnapshotMsg,
    pub abilities: Vec<AbilityStateMsg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatResourcesMsg {
    pub kinetic_energy: f32,
    pub thermal_energy: f32,
    pub semantic_energy: f32,
    pub rage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterySnapshotMsg {
    pub domains: Vec<MasteryDomainMsg>,
    pub active_specializations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasteryDomainMsg {
    pub domain_name: String,
    pub tier: u32,
    pub xp_current: f32,
    pub xp_required: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityStateMsg {
    pub ability_id: String,
    pub cooldown_remaining: f32,
    pub cooldown_total: f32,
    pub slot_index: u32,
}

// --- Combat types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResultMsg {
    pub sequence_number: u64,
    pub accepted: bool,
    pub rejection_reason: String,
    pub changes: Vec<StateChangeMsg>,
    pub server_timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChangeMsg {
    pub entity_id: u64,
    pub change_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageCalcResultMsg {
    pub base_damage: f32,
    pub modified_damage: f32,
    pub crit_chance: f32,
    pub crit_damage: f32,
    pub modifiers: Vec<DamageModifierMsg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageModifierMsg {
    pub source: String,
    pub multiplier: f32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatEventMsg {
    pub tick: u64,
    pub event_type: String,
    pub source_entity: u64,
    pub target_entity: u64,
    pub damage: f32,
    pub hit_position: Vec3Msg,
    pub details: String,
}

// --- Generation types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorResponseMsg {
    pub floor_id: u32,
    pub floor_hash: u64,
    pub biome_tags: Vec<SemanticTagMsg>,
    pub tier: String,
    pub layout: FloorLayoutMsg,
    pub monsters: Vec<MonsterSpawnMsg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorLayoutMsg {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Vec<u8>>,
    pub rooms: Vec<RoomDataMsg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomDataMsg {
    pub room_id: u32,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub room_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterSpawnMsg {
    pub entity_id: u64,
    pub monster_type: String,
    pub position: Vec3Msg,
    pub tags: Vec<SemanticTagMsg>,
    pub health: f32,
    pub max_health: f32,
    pub grammar: MonsterGrammarMsg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterGrammarMsg {
    pub body_type: String,
    pub locomotion: String,
    pub attack_style: String,
    pub modifiers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootItemMsg {
    pub item_name: String,
    pub rarity: String,
    pub tags: Vec<SemanticTagMsg>,
    pub socket_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshDataMsg {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
    pub normals: Vec<f32>,
    pub uvs: Vec<f32>,
    pub material_id: String,
}

// --- Mastery types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasteryProgressResultMsg {
    pub domain: String,
    pub new_tier: u32,
    pub new_xp: f32,
    pub xp_to_next: f32,
    pub tier_up: bool,
    pub newly_unlocked: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasteryProfileMsg {
    pub domains: Vec<DomainProfileMsg>,
    pub specializations: Vec<SpecInfoMsg>,
    pub active_synergies: Vec<SynergyInfoMsg>,
    pub primary_combat_role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainProfileMsg {
    pub domain_name: String,
    pub tier: u32,
    pub xp_current: f32,
    pub xp_required: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecInfoMsg {
    pub branch_id: String,
    pub domain: String,
    pub combat_role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynergyInfoMsg {
    pub synergy_name: String,
    pub required_branches: Vec<String>,
    pub bonus_description: String,
}

// --- World types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldCycleMsg {
    pub current_phase: String,
    pub phase_progress: f32,
    pub monster_spawn_mult: f32,
    pub resource_mult: f32,
    pub semantic_intensity: f32,
}
