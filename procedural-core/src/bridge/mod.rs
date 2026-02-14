//! FFI Bridge Layer: Rust Procedural Core <-> Unreal Engine 5
//!
//! This module exposes C-ABI functions that UE5 can call via DLL loading.
//! Data is serialized as JSON across the boundary.
//! All *_json functions return heap-allocated strings — caller must free with `free_string`.

use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::combat::AttackAngle;
use crate::constants::*;
use crate::events::{self, EventTriggerType, TriggerContext};
use crate::generation::wfc::TileType;
use crate::generation::{FloorSpec, FloorTier, TowerSeed};
use crate::loot;
use crate::monster::MonsterTemplate;
use crate::replication::{DeltaLog, DeltaType, FloorSnapshot};
use crate::semantic::SemanticTags;

// New module imports for extended FFI
use crate::abilities::{default_abilities, AbilityLoadout};
use crate::achievements::AchievementTracker;
use crate::cosmetics::{tower_cosmetics, tower_dyes, CosmeticProfile, CosmeticSlot, DyeChannel};
use crate::mastery::{xp_for_action, MasteryDomain, MasteryProfile, MasteryTier};
use crate::seasons::{
    generate_daily_quests, generate_season_rewards, generate_weekly_quests, SeasonPass,
};
use crate::social::{Guild, Party, PartyRole, Trade, TradeItem};
use crate::sockets::{
    combine_gems, starter_gems, starter_runes, Gem, Rune, SocketColor, SocketContent,
    SocketedEquipment,
};
use crate::specialization::{
    all_specialization_branches, find_active_synergies, SpecializationProfile,
};
use crate::tutorial::{game_hints, tutorial_steps, TutorialProgress};

// Session 20 imports
use crate::gameflow;
use crate::mutators;
use crate::savemigration;

// Session 21 imports
use crate::logging;
use crate::replay;
use crate::towermap;

// Session 22 imports
use crate::analytics;
use crate::hotreload;

// ========================
// Data transfer types
// ========================

/// Floor generation response
#[derive(Debug, Serialize, Deserialize)]
pub struct FloorResponse {
    pub floor_id: u32,
    pub tier: String,
    pub hash: u64,
    pub biome_tags: Vec<(String, f32)>,
}

impl From<FloorSpec> for FloorResponse {
    fn from(spec: FloorSpec) -> Self {
        Self {
            floor_id: spec.id,
            tier: format!("{:?}", spec.tier),
            hash: spec.hash,
            biome_tags: spec.biome_tags.tags.clone(),
        }
    }
}

/// Floor layout for UE5 rendering
#[derive(Debug, Serialize, Deserialize)]
pub struct FloorLayoutResponse {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Vec<u8>>, // TileType as u8
    pub rooms: Vec<RoomInfo>,
    pub spawn_points: Vec<(usize, usize)>,
    pub exit_point: (usize, usize),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomInfo {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub room_type: String,
}

/// Monster template for UE5 spawning
#[derive(Debug, Serialize, Deserialize)]
pub struct MonsterInfo {
    pub name: String,
    pub size: String,
    pub element: String,
    pub corruption: String,
    pub behavior: String,
    pub base_level: u32,
    pub max_hp: f32,
    pub damage: f32,
    pub speed: f32,
    pub armor: f32,
    pub detection_range: f32,
    pub xp_reward: u32,
    pub semantic_tags: Vec<(String, f32)>,
}

/// Loot item for UE5 display
#[derive(Debug, Serialize, Deserialize)]
pub struct LootInfo {
    pub name: String,
    pub category: String,
    pub rarity: String,
    pub quantity: u32,
    pub semantic_tags: Vec<(String, f32)>,
}

/// Combat calculation request
#[derive(Debug, Serialize, Deserialize)]
pub struct CombatCalcRequest {
    pub base_damage: f32,
    pub angle_id: u32, // 0=Front, 1=Side, 2=Back
    pub combo_step: u32,
    pub attacker_tags_json: String,
    pub defender_tags_json: String,
}

/// Combat calculation result
#[derive(Debug, Serialize, Deserialize)]
pub struct CombatCalcResult {
    pub final_damage: f32,
    pub angle_multiplier: f32,
    pub semantic_bonus: f32,
    pub is_synergy: bool,
}

/// Breath of Tower state
#[derive(Debug, Serialize, Deserialize)]
pub struct BreathState {
    pub phase: String,
    pub phase_progress: f32,
    pub monster_spawn_mult: f32,
    pub resource_mult: f32,
    pub semantic_intensity: f32,
}

// ========================
// Helper: safe JSON return
// ========================

fn json_to_cstring<T: Serialize>(value: &T) -> *mut c_char {
    match serde_json::to_string(value) {
        Ok(json) => CString::new(json).unwrap_or_default().into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn parse_cstr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_owned()) }
}

// ========================
// C-ABI: Core
// ========================

/// Version string
#[no_mangle]
pub extern "C" fn get_version() -> *mut c_char {
    CString::new("0.6.0").unwrap_or_default().into_raw()
}

/// Free a string allocated by Rust.
/// Called from C/UE5 — ptr must be from a prior Rust FFI allocation or null.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            drop(CString::from_raw(ptr));
        }
    }
}

// ========================
// C-ABI: Floor Generation
// ========================

/// Generate floor spec and return JSON
#[no_mangle]
pub extern "C" fn generate_floor(seed: u64, floor_id: u32) -> *mut c_char {
    let tower_seed = TowerSeed { seed };
    let spec = FloorSpec::generate(&tower_seed, floor_id);
    let response: FloorResponse = spec.into();
    json_to_cstring(&response)
}

/// Generate full floor layout (tiles + rooms) and return JSON
#[no_mangle]
pub extern "C" fn generate_floor_layout(seed: u64, floor_id: u32) -> *mut c_char {
    let tower_seed = TowerSeed { seed };
    let spec = FloorSpec::generate(&tower_seed, floor_id);
    let layout = crate::generation::wfc::generate_layout(&spec);

    let tile_nums: Vec<Vec<u8>> = layout
        .tiles
        .iter()
        .map(|row| row.iter().map(tile_to_u8).collect())
        .collect();

    let rooms: Vec<RoomInfo> = layout
        .rooms
        .iter()
        .map(|r| RoomInfo {
            x: r.x,
            y: r.y,
            width: r.width,
            height: r.height,
            room_type: format!("{:?}", r.room_type),
        })
        .collect();

    let response = FloorLayoutResponse {
        width: layout.width,
        height: layout.height,
        tiles: tile_nums,
        rooms,
        spawn_points: layout.spawn_points,
        exit_point: layout.exit_point,
    };

    json_to_cstring(&response)
}

/// Get deterministic floor hash
#[no_mangle]
pub extern "C" fn get_floor_hash(seed: u64, floor_id: u32) -> u64 {
    TowerSeed { seed }.floor_hash(floor_id)
}

/// Get floor tier as integer (0=Echelon1, 1=Echelon2, 2=Echelon3, 3=Echelon4)
#[no_mangle]
pub extern "C" fn get_floor_tier(floor_id: u32) -> u32 {
    match FloorTier::from_floor_id(floor_id) {
        FloorTier::Echelon1 => 0,
        FloorTier::Echelon2 => 1,
        FloorTier::Echelon3 => 2,
        FloorTier::Echelon4 => 3,
    }
}

// ========================
// C-ABI: Monster Generation
// ========================

/// Generate a monster from hash and floor level, return JSON
#[no_mangle]
pub extern "C" fn generate_monster(hash: u64, floor_level: u32) -> *mut c_char {
    let template = MonsterTemplate::from_hash(hash, floor_level);
    let stats = template.compute_stats();
    let tags = template.semantic_tags();

    let info = MonsterInfo {
        name: template.name,
        size: format!("{:?}", template.size),
        element: format!("{:?}", template.element),
        corruption: format!("{:?}", template.corruption),
        behavior: format!("{:?}", template.behavior),
        base_level: template.base_level,
        max_hp: stats.max_hp,
        damage: stats.damage,
        speed: stats.speed,
        armor: stats.armor,
        detection_range: stats.detection_range,
        xp_reward: stats.xp_reward,
        semantic_tags: tags.tags,
    };

    json_to_cstring(&info)
}

/// Generate multiple monsters for a floor, return JSON array
#[no_mangle]
pub extern "C" fn generate_floor_monsters(seed: u64, floor_id: u32, count: u32) -> *mut c_char {
    let tower_seed = TowerSeed { seed };
    let base_hash = tower_seed.floor_hash(floor_id);
    let mut monsters = Vec::new();

    for i in 0..count {
        let hash = base_hash.wrapping_add(i as u64 * MONSTER_HASH_PRIME);
        let template = MonsterTemplate::from_hash(hash, floor_id);
        let stats = template.compute_stats();
        let tags = template.semantic_tags();

        monsters.push(MonsterInfo {
            name: template.name,
            size: format!("{:?}", template.size),
            element: format!("{:?}", template.element),
            corruption: format!("{:?}", template.corruption),
            behavior: format!("{:?}", template.behavior),
            base_level: template.base_level,
            max_hp: stats.max_hp,
            damage: stats.damage,
            speed: stats.speed,
            armor: stats.armor,
            detection_range: stats.detection_range,
            xp_reward: stats.xp_reward,
            semantic_tags: tags.tags,
        });
    }

    json_to_cstring(&monsters)
}

// ========================
// C-ABI: Combat
// ========================

/// Get attack angle damage multiplier
#[no_mangle]
pub extern "C" fn get_angle_multiplier(angle_id: u32) -> f32 {
    match angle_id {
        0 => AttackAngle::Front.multiplier(),
        1 => AttackAngle::Side.multiplier(),
        2 => AttackAngle::Back.multiplier(),
        _ => 1.0,
    }
}

/// Calculate combat damage with semantic bonuses
#[no_mangle]
pub extern "C" fn calculate_combat(request_json: *const c_char) -> *mut c_char {
    let json_str = match parse_cstr(request_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let request: CombatCalcRequest = match serde_json::from_str(&json_str) {
        Ok(r) => r,
        Err(_) => return std::ptr::null_mut(),
    };

    let angle_mult = match request.angle_id {
        0 => AttackAngle::Front.multiplier(),
        1 => AttackAngle::Side.multiplier(),
        2 => AttackAngle::Back.multiplier(),
        _ => 1.0,
    };

    // Semantic bonus from tag similarity
    let attacker_tags: Vec<(String, f32)> =
        serde_json::from_str(&request.attacker_tags_json).unwrap_or_default();
    let defender_tags: Vec<(String, f32)> =
        serde_json::from_str(&request.defender_tags_json).unwrap_or_default();

    let sem_a = SemanticTags {
        tags: attacker_tags,
    };
    let sem_b = SemanticTags {
        tags: defender_tags,
    };
    let similarity = sem_a.similarity(&sem_b);

    let semantic_bonus = if similarity > SEMANTIC_HIGH_THRESHOLD {
        SEMANTIC_SYNERGY_BONUS
    } else if similarity < SEMANTIC_LOW_THRESHOLD {
        SEMANTIC_CONFLICT_PENALTY
    } else {
        0.0
    };

    let combo_mult = 1.0 + request.combo_step as f32 * COMBO_STEP_MULT;
    let final_damage = request.base_damage * angle_mult * combo_mult * (1.0 + semantic_bonus);

    let result = CombatCalcResult {
        final_damage,
        angle_multiplier: angle_mult,
        semantic_bonus,
        is_synergy: similarity > SEMANTIC_HIGH_THRESHOLD,
    };

    json_to_cstring(&result)
}

// ========================
// C-ABI: Semantic
// ========================

/// Cosine similarity between two tag arrays (JSON)
#[no_mangle]
pub extern "C" fn semantic_similarity(
    tags_a_json: *const c_char,
    tags_b_json: *const c_char,
) -> f32 {
    let a_str = match parse_cstr(tags_a_json) {
        Some(s) => s,
        None => return 0.0,
    };
    let b_str = match parse_cstr(tags_b_json) {
        Some(s) => s,
        None => return 0.0,
    };

    let tags_a: Vec<(String, f32)> = serde_json::from_str(&a_str).unwrap_or_default();
    let tags_b: Vec<(String, f32)> = serde_json::from_str(&b_str).unwrap_or_default();

    let sem_a = SemanticTags { tags: tags_a };
    let sem_b = SemanticTags { tags: tags_b };
    sem_a.similarity(&sem_b)
}

// ========================
// C-ABI: Loot
// ========================

/// Generate loot drops from monster death
#[no_mangle]
pub extern "C" fn generate_loot(
    source_tags_json: *const c_char,
    floor_level: u32,
    drop_hash: u64,
) -> *mut c_char {
    let tags_str = match parse_cstr(source_tags_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let tags_vec: Vec<(String, f32)> = serde_json::from_str(&tags_str).unwrap_or_default();
    let source_tags = SemanticTags { tags: tags_vec };

    let items = loot::generate_loot(&source_tags, floor_level, drop_hash);

    let loot_infos: Vec<LootInfo> = items
        .iter()
        .map(|item| LootInfo {
            name: item.name.clone(),
            category: format!("{:?}", item.category),
            rarity: format!("{:?}", item.rarity),
            quantity: item.quantity,
            semantic_tags: item.semantic_tags.clone(),
        })
        .collect();

    json_to_cstring(&loot_infos)
}

// ========================
// C-ABI: World
// ========================

/// Get current Breath of Tower phase based on elapsed seconds
#[no_mangle]
pub extern "C" fn get_breath_state(elapsed_seconds: f32) -> *mut c_char {
    use crate::world::BreathPhase;

    let cycle_pos = elapsed_seconds % BREATH_CYCLE_TOTAL;

    let hold_start = BREATH_INHALE_SECS;
    let exhale_start = hold_start + BREATH_HOLD_SECS;
    let pause_start = exhale_start + BREATH_EXHALE_SECS;

    let (phase, phase_progress) = if cycle_pos < hold_start {
        (BreathPhase::Inhale, cycle_pos / BREATH_INHALE_SECS)
    } else if cycle_pos < exhale_start {
        (
            BreathPhase::Hold,
            (cycle_pos - hold_start) / BREATH_HOLD_SECS,
        )
    } else if cycle_pos < pause_start {
        (
            BreathPhase::Exhale,
            (cycle_pos - exhale_start) / BREATH_EXHALE_SECS,
        )
    } else {
        (
            BreathPhase::Pause,
            (cycle_pos - pause_start) / BREATH_PAUSE_SECS,
        )
    };

    let state = BreathState {
        phase: format!("{:?}", phase),
        phase_progress,
        monster_spawn_mult: phase.monster_spawn_multiplier(),
        resource_mult: phase.resource_multiplier(),
        semantic_intensity: phase.semantic_intensity(),
    };

    json_to_cstring(&state)
}

// ========================
// C-ABI: Replication
// ========================

/// Record a delta (world mutation) and return its sequence number
#[no_mangle]
pub extern "C" fn record_delta(
    delta_type_id: u32,
    floor_id: u32,
    entity_hash: u64,
    player_id: *const c_char,
    payload: *const c_char,
    tick: u64,
) -> *mut c_char {
    let player = parse_cstr(player_id).unwrap_or_default();
    let payload_str = parse_cstr(payload).unwrap_or_default();

    let dt = match delta_type_id {
        0 => DeltaType::MonsterKill,
        1 => DeltaType::ChestOpen,
        2 => DeltaType::ShrineActivate,
        3 => DeltaType::LootPickup,
        4 => DeltaType::TrapDisarm,
        5 => DeltaType::DoorUnlock,
        6 => DeltaType::EnvironmentChange,
        7 => DeltaType::PlayerSpawn,
        8 => DeltaType::PlayerDeath,
        9 => DeltaType::StairsUnlock,
        10 => DeltaType::CraftComplete,
        11 => DeltaType::QuestProgress,
        _ => DeltaType::EnvironmentChange,
    };

    let mut log = DeltaLog::default();
    let seq = log.record(tick, dt, floor_id, entity_hash, &player, &payload_str);
    json_to_cstring(&seq)
}

/// Create a floor snapshot (seed + deltas) for network sync
#[no_mangle]
pub extern "C" fn create_floor_snapshot(
    seed: u64,
    floor_id: u32,
    deltas_json: *const c_char,
) -> *mut c_char {
    let json_str = match parse_cstr(deltas_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let deltas: Vec<crate::replication::Delta> =
        serde_json::from_str(&json_str).unwrap_or_default();

    let mut log = DeltaLog::default();
    for delta in deltas {
        log.push(delta);
    }

    let tower_seed = TowerSeed { seed };
    let snapshot = FloorSnapshot::capture(&tower_seed, floor_id, &log, 0);
    json_to_cstring(&snapshot)
}

// ========================
// C-ABI: Events
// ========================

/// Evaluate a procedural event trigger, return event JSON or null
#[no_mangle]
pub extern "C" fn evaluate_event_trigger(
    trigger_type_id: u32,
    context_json: *const c_char,
) -> *mut c_char {
    let json_str = match parse_cstr(context_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let context: TriggerContext = match serde_json::from_str(&json_str) {
        Ok(c) => c,
        Err(_) => return std::ptr::null_mut(),
    };

    let trigger = match trigger_type_id {
        0 => EventTriggerType::BreathShift,
        1 => EventTriggerType::SemanticResonance,
        2 => EventTriggerType::EchoConvergence,
        3 => EventTriggerType::FloorAnomaly,
        4 => EventTriggerType::FactionClash,
        5 => EventTriggerType::CorruptionSurge,
        6 => EventTriggerType::TowerMemory,
        _ => return std::ptr::null_mut(),
    };

    match events::evaluate_trigger(trigger, &context) {
        Some(event) => json_to_cstring(&event),
        None => std::ptr::null_mut(),
    }
}

// ========================
// C-ABI: Mastery System
// ========================

/// Create a new empty mastery profile, return JSON
#[no_mangle]
pub extern "C" fn mastery_create_profile() -> *mut c_char {
    let profile = MasteryProfile::new();
    json_to_cstring(&profile)
}

/// Gain XP in a mastery domain, return updated profile JSON
/// domain_id: 0-20 mapping to MasteryDomain variants
#[no_mangle]
pub extern "C" fn mastery_gain_xp(
    profile_json: *const c_char,
    domain_id: u32,
    amount: u64,
) -> *mut c_char {
    let json_str = match parse_cstr(profile_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let mut profile: MasteryProfile = match serde_json::from_str(&json_str) {
        Ok(p) => p,
        Err(_) => return std::ptr::null_mut(),
    };

    let domain = match domain_from_id(domain_id) {
        Some(d) => d,
        None => return std::ptr::null_mut(),
    };

    profile.gain_xp(domain, amount);
    json_to_cstring(&profile)
}

/// Get mastery tier for a domain (0=Novice..5=Grandmaster), -1 if invalid
#[no_mangle]
pub extern "C" fn mastery_get_tier(profile_json: *const c_char, domain_id: u32) -> i32 {
    let json_str = match parse_cstr(profile_json) {
        Some(s) => s,
        None => return -1,
    };
    let profile: MasteryProfile = match serde_json::from_str(&json_str) {
        Ok(p) => p,
        Err(_) => return -1,
    };

    let domain = match domain_from_id(domain_id) {
        Some(d) => d,
        None => return -1,
    };

    match profile.tier(domain) {
        MasteryTier::Novice => 0,
        MasteryTier::Apprentice => 1,
        MasteryTier::Journeyman => 2,
        MasteryTier::Expert => 3,
        MasteryTier::Master => 4,
        MasteryTier::Grandmaster => 5,
    }
}

/// Get XP amount for a game action by name
#[no_mangle]
pub extern "C" fn mastery_xp_for_action(action_name: *const c_char) -> u64 {
    match parse_cstr(action_name) {
        Some(name) => xp_for_action(&name),
        None => 0,
    }
}

/// Get all mastery domain names as JSON array
#[no_mangle]
pub extern "C" fn mastery_get_all_domains() -> *mut c_char {
    let domains: Vec<&str> = vec![
        "SwordMastery",
        "GreatswordMastery",
        "DaggerMastery",
        "SpearMastery",
        "GauntletMastery",
        "StaffMastery",
        "ParryMastery",
        "DodgeMastery",
        "BlockMastery",
        "AerialMastery",
        "Blacksmithing",
        "Alchemy",
        "Enchanting",
        "Tailoring",
        "Cooking",
        "Mining",
        "Herbalism",
        "Salvaging",
        "Trading",
        "Exploration",
        "SemanticAttunement",
    ];
    json_to_cstring(&domains)
}

// ========================
// C-ABI: Specialization
// ========================

/// Get all specialization branches as JSON
#[no_mangle]
pub extern "C" fn spec_get_all_branches() -> *mut c_char {
    let branches = all_specialization_branches();
    let infos: Vec<serde_json::Value> = branches
        .iter()
        .map(|b| {
            serde_json::json!({
                "id": b.id,
                "name": b.name,
                "domain": format!("{:?}", b.domain),
                "description": b.description,
                "required_tier": format!("{:?}", b.required_tier),
                "role_affinity": format!("{:?}", b.role_affinity),
            })
        })
        .collect();
    json_to_cstring(&infos)
}

/// Create a new specialization profile, return JSON
#[no_mangle]
pub extern "C" fn spec_create_profile() -> *mut c_char {
    let profile = SpecializationProfile::new();
    json_to_cstring(&profile)
}

/// Choose a specialization branch, return updated profile JSON or null on failure
#[no_mangle]
pub extern "C" fn spec_choose_branch(
    profile_json: *const c_char,
    mastery_json: *const c_char,
    branch_id: *const c_char,
) -> *mut c_char {
    let prof_str = match parse_cstr(profile_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let mast_str = match parse_cstr(mastery_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let bid_str = match parse_cstr(branch_id) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut profile: SpecializationProfile = match serde_json::from_str(&prof_str) {
        Ok(p) => p,
        Err(_) => return std::ptr::null_mut(),
    };
    let mastery: MasteryProfile = match serde_json::from_str(&mast_str) {
        Ok(m) => m,
        Err(_) => return std::ptr::null_mut(),
    };

    let branches = all_specialization_branches();
    let branch = match branches.iter().find(|b| b.id == bid_str) {
        Some(b) => b,
        None => return std::ptr::null_mut(),
    };

    match profile.choose_branch(branch, &mastery) {
        Ok(()) => json_to_cstring(&profile),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Find active synergies for chosen branches, return JSON
#[no_mangle]
pub extern "C" fn spec_find_synergies(branch_ids_json: *const c_char) -> *mut c_char {
    let json_str = match parse_cstr(branch_ids_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let branch_ids: Vec<String> = match serde_json::from_str(&json_str) {
        Ok(ids) => ids,
        Err(_) => return std::ptr::null_mut(),
    };

    let synergies = find_active_synergies(&branch_ids);
    let infos: Vec<serde_json::Value> = synergies
        .iter()
        .map(|s| {
            serde_json::json!({
                "name": s.name,
                "branch_a": s.branch_a,
                "branch_b": s.branch_b,
                "description": s.description,
            })
        })
        .collect();
    json_to_cstring(&infos)
}

// ========================
// C-ABI: Abilities
// ========================

/// Get all default abilities as JSON
#[no_mangle]
pub extern "C" fn ability_get_defaults() -> *mut c_char {
    let abilities = default_abilities();
    let infos: Vec<serde_json::Value> = abilities
        .iter()
        .map(|a| {
            serde_json::json!({
                "id": a.id,
                "name": a.name,
                "description": a.description,
                "cooldown": a.cooldown,
                "target": format!("{:?}", a.target),
                "range": a.range,
                "radius": a.radius,
                "cast_time": a.cast_time,
                "cost_kinetic": a.cost.kinetic,
                "cost_thermal": a.cost.thermal,
                "cost_semantic": a.cost.semantic,
            })
        })
        .collect();
    json_to_cstring(&infos)
}

/// Create a new ability loadout, return JSON
#[no_mangle]
pub extern "C" fn ability_create_loadout() -> *mut c_char {
    let loadout = AbilityLoadout::new();
    json_to_cstring(&loadout)
}

/// Learn an ability (by id) and return updated loadout JSON
#[no_mangle]
pub extern "C" fn ability_learn(
    loadout_json: *const c_char,
    ability_id: *const c_char,
) -> *mut c_char {
    let load_str = match parse_cstr(loadout_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let aid_str = match parse_cstr(ability_id) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut loadout: AbilityLoadout = match serde_json::from_str(&load_str) {
        Ok(l) => l,
        Err(_) => return std::ptr::null_mut(),
    };

    let defaults = default_abilities();
    let ability = match defaults.iter().find(|a| a.id == aid_str) {
        Some(a) => a.clone(),
        None => return std::ptr::null_mut(),
    };

    loadout.learn(ability);
    json_to_cstring(&loadout)
}

/// Equip an ability to a hotbar slot (0-5), return updated loadout JSON
#[no_mangle]
pub extern "C" fn ability_equip(
    loadout_json: *const c_char,
    slot: u32,
    ability_id: *const c_char,
) -> *mut c_char {
    let load_str = match parse_cstr(loadout_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let aid_str = match parse_cstr(ability_id) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut loadout: AbilityLoadout = match serde_json::from_str(&load_str) {
        Ok(l) => l,
        Err(_) => return std::ptr::null_mut(),
    };

    loadout.equip(slot as usize, &aid_str);
    json_to_cstring(&loadout)
}

// ========================
// C-ABI: Socket System
// ========================

/// Get starter gems as JSON
#[no_mangle]
pub extern "C" fn socket_get_starter_gems() -> *mut c_char {
    let gems = starter_gems();
    json_to_cstring(&gems)
}

/// Get starter runes as JSON
#[no_mangle]
pub extern "C" fn socket_get_starter_runes() -> *mut c_char {
    let runes = starter_runes();
    json_to_cstring(&runes)
}

/// Create a socketed equipment piece with given socket colors
/// colors_json: JSON array of color ids (0=Red, 1=Blue, 2=Yellow, 3=Prismatic)
#[no_mangle]
pub extern "C" fn socket_create_equipment(
    name: *const c_char,
    colors_json: *const c_char,
) -> *mut c_char {
    let name_str = match parse_cstr(name) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let colors_str = match parse_cstr(colors_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let color_ids: Vec<u32> = match serde_json::from_str(&colors_str) {
        Ok(ids) => ids,
        Err(_) => return std::ptr::null_mut(),
    };

    let colors: Vec<SocketColor> = color_ids
        .iter()
        .map(|&id| socket_color_from_id(id))
        .collect();
    let equip = SocketedEquipment::new(name_str, colors);
    json_to_cstring(&equip)
}

/// Insert a gem into a socket, return updated equipment JSON or null
#[no_mangle]
pub extern "C" fn socket_insert_gem(
    equipment_json: *const c_char,
    slot: u32,
    gem_json: *const c_char,
) -> *mut c_char {
    let equip_str = match parse_cstr(equipment_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let gem_str = match parse_cstr(gem_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut equip: SocketedEquipment = match serde_json::from_str(&equip_str) {
        Ok(e) => e,
        Err(_) => return std::ptr::null_mut(),
    };
    let gem: Gem = match serde_json::from_str(&gem_str) {
        Ok(g) => g,
        Err(_) => return std::ptr::null_mut(),
    };

    match equip.insert_at(slot as usize, SocketContent::Gem(gem)) {
        Ok(_) => json_to_cstring(&equip),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Insert a rune into a socket, return updated equipment JSON or null
#[no_mangle]
pub extern "C" fn socket_insert_rune(
    equipment_json: *const c_char,
    slot: u32,
    rune_json: *const c_char,
) -> *mut c_char {
    let equip_str = match parse_cstr(equipment_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let rune_str = match parse_cstr(rune_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut equip: SocketedEquipment = match serde_json::from_str(&equip_str) {
        Ok(e) => e,
        Err(_) => return std::ptr::null_mut(),
    };
    let rune: Rune = match serde_json::from_str(&rune_str) {
        Ok(r) => r,
        Err(_) => return std::ptr::null_mut(),
    };

    match equip.insert_at(slot as usize, SocketContent::Rune(rune)) {
        Ok(_) => json_to_cstring(&equip),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Combine 3 gems of same tier into next tier, return new gem JSON or null
#[no_mangle]
pub extern "C" fn socket_combine_gems(gems_json: *const c_char) -> *mut c_char {
    let json_str = match parse_cstr(gems_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let gems: Vec<Gem> = match serde_json::from_str(&json_str) {
        Ok(g) => g,
        Err(_) => return std::ptr::null_mut(),
    };

    if gems.len() != 3 {
        return std::ptr::null_mut();
    }
    let arr: [Gem; 3] = match gems.try_into() {
        Ok(a) => a,
        Err(_) => return std::ptr::null_mut(),
    };

    match combine_gems(&arr) {
        Some(gem) => json_to_cstring(&gem),
        None => std::ptr::null_mut(),
    }
}

// ========================
// C-ABI: Cosmetics
// ========================

/// Get all available cosmetics as JSON
#[no_mangle]
pub extern "C" fn cosmetic_get_all() -> *mut c_char {
    let cosmetics = tower_cosmetics();
    json_to_cstring(&cosmetics)
}

/// Get all available dyes as JSON
#[no_mangle]
pub extern "C" fn cosmetic_get_all_dyes() -> *mut c_char {
    let dyes = tower_dyes();
    json_to_cstring(&dyes)
}

/// Create a new cosmetic profile, return JSON
#[no_mangle]
pub extern "C" fn cosmetic_create_profile() -> *mut c_char {
    let profile = CosmeticProfile::new();
    json_to_cstring(&profile)
}

/// Unlock a cosmetic item, return updated profile JSON
#[no_mangle]
pub extern "C" fn cosmetic_unlock(
    profile_json: *const c_char,
    cosmetic_id: *const c_char,
) -> *mut c_char {
    let prof_str = match parse_cstr(profile_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let cid_str = match parse_cstr(cosmetic_id) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut profile: CosmeticProfile = match serde_json::from_str(&prof_str) {
        Ok(p) => p,
        Err(_) => return std::ptr::null_mut(),
    };

    profile.unlock_cosmetic(&cid_str);
    json_to_cstring(&profile)
}

/// Apply transmog override, return updated profile JSON
/// slot_id: 0-11 mapping to CosmeticSlot variants
#[no_mangle]
pub extern "C" fn cosmetic_apply_transmog(
    profile_json: *const c_char,
    slot_id: u32,
    cosmetic_id: *const c_char,
) -> *mut c_char {
    let prof_str = match parse_cstr(profile_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let cid_str = match parse_cstr(cosmetic_id) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut profile: CosmeticProfile = match serde_json::from_str(&prof_str) {
        Ok(p) => p,
        Err(_) => return std::ptr::null_mut(),
    };

    let slot = match cosmetic_slot_from_id(slot_id) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    profile.apply_transmog(slot, &cid_str);
    json_to_cstring(&profile)
}

/// Apply dye to a transmog slot, return updated profile JSON
/// channel_id: 0=Primary, 1=Secondary, 2=Accent
#[no_mangle]
pub extern "C" fn cosmetic_apply_dye(
    profile_json: *const c_char,
    slot_id: u32,
    channel_id: u32,
    dye_id: *const c_char,
) -> *mut c_char {
    let prof_str = match parse_cstr(profile_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let did_str = match parse_cstr(dye_id) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut profile: CosmeticProfile = match serde_json::from_str(&prof_str) {
        Ok(p) => p,
        Err(_) => return std::ptr::null_mut(),
    };

    let slot = match cosmetic_slot_from_id(slot_id) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let channel = match channel_id {
        0 => DyeChannel::Primary,
        1 => DyeChannel::Secondary,
        2 => DyeChannel::Accent,
        _ => return std::ptr::null_mut(),
    };

    profile.apply_dye(slot, channel, &did_str);
    json_to_cstring(&profile)
}

// ========================
// C-ABI: Tutorial
// ========================

/// Get all tutorial steps as JSON
#[no_mangle]
pub extern "C" fn tutorial_get_steps() -> *mut c_char {
    let steps = tutorial_steps();
    json_to_cstring(&steps)
}

/// Get all game hints as JSON
#[no_mangle]
pub extern "C" fn tutorial_get_hints() -> *mut c_char {
    let hints = game_hints();
    json_to_cstring(&hints)
}

/// Create a new tutorial progress, return JSON
#[no_mangle]
pub extern "C" fn tutorial_create_progress() -> *mut c_char {
    let progress = TutorialProgress::new();
    json_to_cstring(&progress)
}

/// Complete a tutorial step, return updated progress JSON
#[no_mangle]
pub extern "C" fn tutorial_complete_step(
    progress_json: *const c_char,
    step_id: *const c_char,
) -> *mut c_char {
    let prog_str = match parse_cstr(progress_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let sid_str = match parse_cstr(step_id) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut progress: TutorialProgress = match serde_json::from_str(&prog_str) {
        Ok(p) => p,
        Err(_) => return std::ptr::null_mut(),
    };

    progress.complete(&sid_str);
    json_to_cstring(&progress)
}

/// Get tutorial completion percent (0-100)
#[no_mangle]
pub extern "C" fn tutorial_completion_percent(progress_json: *const c_char) -> f32 {
    let prog_str = match parse_cstr(progress_json) {
        Some(s) => s,
        None => return 0.0,
    };
    let progress: TutorialProgress = match serde_json::from_str(&prog_str) {
        Ok(p) => p,
        Err(_) => return 0.0,
    };
    let total = tutorial_steps().len();
    progress.completion_percent(total) * 100.0
}

// ========================
// C-ABI: Achievements
// ========================

/// Create a new achievement tracker with all predefined achievements, return JSON
#[no_mangle]
pub extern "C" fn achievement_create_tracker() -> *mut c_char {
    let tracker = AchievementTracker::new();
    json_to_cstring(&tracker)
}

/// Increment an achievement counter, return updated tracker JSON
#[no_mangle]
pub extern "C" fn achievement_increment(
    tracker_json: *const c_char,
    achievement_id: *const c_char,
    amount: u64,
) -> *mut c_char {
    let trk_str = match parse_cstr(tracker_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let aid_str = match parse_cstr(achievement_id) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut tracker: AchievementTracker = match serde_json::from_str(&trk_str) {
        Ok(t) => t,
        Err(_) => return std::ptr::null_mut(),
    };

    tracker.increment_counter(&aid_str, amount);
    json_to_cstring(&tracker)
}

/// Check all achievements and unlock completed ones, return updated tracker JSON
#[no_mangle]
pub extern "C" fn achievement_check_all(
    tracker_json: *const c_char,
    current_tick: u64,
) -> *mut c_char {
    let trk_str = match parse_cstr(tracker_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut tracker: AchievementTracker = match serde_json::from_str(&trk_str) {
        Ok(t) => t,
        Err(_) => return std::ptr::null_mut(),
    };

    tracker.check_all(current_tick);
    json_to_cstring(&tracker)
}

/// Get achievement completion percentage (0.0 - 1.0)
#[no_mangle]
pub extern "C" fn achievement_completion_percent(tracker_json: *const c_char) -> f32 {
    let trk_str = match parse_cstr(tracker_json) {
        Some(s) => s,
        None => return 0.0,
    };
    let tracker: AchievementTracker = match serde_json::from_str(&trk_str) {
        Ok(t) => t,
        Err(_) => return 0.0,
    };
    tracker.completion_percent() as f32
}

// ========================
// C-ABI: Season Pass
// ========================

/// Create a new season pass, return JSON
#[no_mangle]
pub extern "C" fn season_create_pass(season_number: u32, name: *const c_char) -> *mut c_char {
    let name_str = match parse_cstr(name) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let pass = SeasonPass::new(season_number, name_str);
    json_to_cstring(&pass)
}

/// Add XP to season pass, return updated pass JSON
#[no_mangle]
pub extern "C" fn season_add_xp(pass_json: *const c_char, amount: u64) -> *mut c_char {
    let pass_str = match parse_cstr(pass_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut pass: SeasonPass = match serde_json::from_str(&pass_str) {
        Ok(p) => p,
        Err(_) => return std::ptr::null_mut(),
    };

    pass.add_xp(amount);
    json_to_cstring(&pass)
}

/// Generate daily quests for a day seed, return JSON array
#[no_mangle]
pub extern "C" fn season_generate_dailies(day_seed: u64) -> *mut c_char {
    let quests = generate_daily_quests(day_seed);
    json_to_cstring(&quests)
}

/// Generate weekly quests for a week seed, return JSON array
#[no_mangle]
pub extern "C" fn season_generate_weeklies(week_seed: u64) -> *mut c_char {
    let quests = generate_weekly_quests(week_seed);
    json_to_cstring(&quests)
}

/// Get all season rewards for a season, return JSON array
#[no_mangle]
pub extern "C" fn season_get_rewards(season_number: u32) -> *mut c_char {
    let rewards = generate_season_rewards(season_number);
    json_to_cstring(&rewards)
}

// ========================
// C-ABI: Social — Guild
// ========================

/// Create a new guild, return JSON
#[no_mangle]
pub extern "C" fn social_create_guild(
    name: *const c_char,
    tag: *const c_char,
    leader_id: *const c_char,
    leader_name: *const c_char,
    faction: *const c_char,
) -> *mut c_char {
    let name_str = match parse_cstr(name) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let tag_str = match parse_cstr(tag) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let lid_str = match parse_cstr(leader_id) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let lname_str = match parse_cstr(leader_name) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let faction_str = match parse_cstr(faction) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let guild_id = format!("guild_{}_{}", tag_str, lid_str);
    let mut guild = Guild::new(guild_id, name_str, tag_str, lid_str, lname_str);
    guild.faction_affinity = Some(faction_str);
    json_to_cstring(&guild)
}

/// Add a member to guild, return updated guild JSON or null
#[no_mangle]
pub extern "C" fn social_guild_add_member(
    guild_json: *const c_char,
    user_id: *const c_char,
    user_name: *const c_char,
) -> *mut c_char {
    let guild_str = match parse_cstr(guild_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let uid_str = match parse_cstr(user_id) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let uname_str = match parse_cstr(user_name) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut guild: Guild = match serde_json::from_str(&guild_str) {
        Ok(g) => g,
        Err(_) => return std::ptr::null_mut(),
    };

    if guild.add_member(uid_str, uname_str) {
        json_to_cstring(&guild)
    } else {
        std::ptr::null_mut()
    }
}

// ========================
// C-ABI: Social — Party
// ========================

/// Create a new party, return JSON
#[no_mangle]
pub extern "C" fn social_create_party(
    leader_id: *const c_char,
    leader_name: *const c_char,
) -> *mut c_char {
    let lid_str = match parse_cstr(leader_id) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let lname_str = match parse_cstr(leader_name) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let party = Party::new(lid_str, lname_str);
    json_to_cstring(&party)
}

/// Add a member to party, return updated party JSON or null (max 4)
/// role_id: 0=Vanguard, 1=Striker, 2=Support, 3=Tactician
#[no_mangle]
pub extern "C" fn social_party_add_member(
    party_json: *const c_char,
    user_id: *const c_char,
    user_name: *const c_char,
    role_id: u32,
) -> *mut c_char {
    let party_str = match parse_cstr(party_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let uid_str = match parse_cstr(user_id) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let uname_str = match parse_cstr(user_name) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut party: Party = match serde_json::from_str(&party_str) {
        Ok(p) => p,
        Err(_) => return std::ptr::null_mut(),
    };

    let role = match role_id {
        0 => PartyRole::Vanguard,
        1 => PartyRole::Striker,
        2 => PartyRole::Support,
        3 => PartyRole::Tactician,
        _ => PartyRole::Striker,
    };

    if party.add_member(uid_str, uname_str, role) {
        json_to_cstring(&party)
    } else {
        std::ptr::null_mut()
    }
}

// ========================
// C-ABI: Social — Trade
// ========================

/// Create a new trade between two players, return JSON
#[no_mangle]
pub extern "C" fn social_create_trade(
    player_a: *const c_char,
    player_b: *const c_char,
) -> *mut c_char {
    let pa_str = match parse_cstr(player_a) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let pb_str = match parse_cstr(player_b) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let trade = Trade::new(pa_str, pb_str);
    json_to_cstring(&trade)
}

/// Add an item to trade, return updated trade JSON
#[no_mangle]
pub extern "C" fn social_trade_add_item(
    trade_json: *const c_char,
    player_id: *const c_char,
    item_name: *const c_char,
    quantity: u32,
    rarity: *const c_char,
) -> *mut c_char {
    let trade_str = match parse_cstr(trade_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let pid_str = match parse_cstr(player_id) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let iname_str = match parse_cstr(item_name) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let rar_str = match parse_cstr(rarity) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut trade: Trade = match serde_json::from_str(&trade_str) {
        Ok(t) => t,
        Err(_) => return std::ptr::null_mut(),
    };

    let item = TradeItem {
        item_name: iname_str,
        quantity,
        rarity: rar_str,
    };
    trade.add_item(&pid_str, item);
    json_to_cstring(&trade)
}

/// Lock a player's side of the trade, return updated trade JSON
#[no_mangle]
pub extern "C" fn social_trade_lock(
    trade_json: *const c_char,
    player_id: *const c_char,
) -> *mut c_char {
    let trade_str = match parse_cstr(trade_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let pid_str = match parse_cstr(player_id) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut trade: Trade = match serde_json::from_str(&trade_str) {
        Ok(t) => t,
        Err(_) => return std::ptr::null_mut(),
    };

    trade.lock(&pid_str);
    json_to_cstring(&trade)
}

/// Confirm a player's side of the trade, return updated trade JSON
#[no_mangle]
pub extern "C" fn social_trade_confirm(
    trade_json: *const c_char,
    player_id: *const c_char,
) -> *mut c_char {
    let trade_str = match parse_cstr(trade_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let pid_str = match parse_cstr(player_id) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut trade: Trade = match serde_json::from_str(&trade_str) {
        Ok(t) => t,
        Err(_) => return std::ptr::null_mut(),
    };

    trade.confirm(&pid_str);
    json_to_cstring(&trade)
}

/// Execute a confirmed trade, return result JSON (completed or error)
#[no_mangle]
pub extern "C" fn social_trade_execute(trade_json: *const c_char) -> *mut c_char {
    let trade_str = match parse_cstr(trade_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut trade: Trade = match serde_json::from_str(&trade_str) {
        Ok(t) => t,
        Err(_) => return std::ptr::null_mut(),
    };

    trade.execute();
    json_to_cstring(&trade)
}

// ========================
// Helpers
// ========================

fn domain_from_id(id: u32) -> Option<MasteryDomain> {
    match id {
        0 => Some(MasteryDomain::SwordMastery),
        1 => Some(MasteryDomain::GreatswordMastery),
        2 => Some(MasteryDomain::DaggerMastery),
        3 => Some(MasteryDomain::SpearMastery),
        4 => Some(MasteryDomain::GauntletMastery),
        5 => Some(MasteryDomain::StaffMastery),
        6 => Some(MasteryDomain::ParryMastery),
        7 => Some(MasteryDomain::DodgeMastery),
        8 => Some(MasteryDomain::BlockMastery),
        9 => Some(MasteryDomain::AerialMastery),
        10 => Some(MasteryDomain::Blacksmithing),
        11 => Some(MasteryDomain::Alchemy),
        12 => Some(MasteryDomain::Enchanting),
        13 => Some(MasteryDomain::Tailoring),
        14 => Some(MasteryDomain::Cooking),
        15 => Some(MasteryDomain::Mining),
        16 => Some(MasteryDomain::Herbalism),
        17 => Some(MasteryDomain::Salvaging),
        18 => Some(MasteryDomain::Trading),
        19 => Some(MasteryDomain::Exploration),
        20 => Some(MasteryDomain::SemanticAttunement),
        _ => None,
    }
}

fn socket_color_from_id(id: u32) -> SocketColor {
    match id {
        0 => SocketColor::Red,
        1 => SocketColor::Blue,
        2 => SocketColor::Yellow,
        3 => SocketColor::Prismatic,
        _ => SocketColor::Red,
    }
}

fn cosmetic_slot_from_id(id: u32) -> Option<CosmeticSlot> {
    match id {
        0 => Some(CosmeticSlot::HeadOverride),
        1 => Some(CosmeticSlot::ChestOverride),
        2 => Some(CosmeticSlot::LegsOverride),
        3 => Some(CosmeticSlot::BootsOverride),
        4 => Some(CosmeticSlot::GlovesOverride),
        5 => Some(CosmeticSlot::WeaponSkin),
        6 => Some(CosmeticSlot::BackAccessory),
        7 => Some(CosmeticSlot::Aura),
        8 => Some(CosmeticSlot::Emote),
        9 => Some(CosmeticSlot::Title),
        10 => Some(CosmeticSlot::ProfileBorder),
        11 => Some(CosmeticSlot::NameplateStyle),
        _ => None,
    }
}

fn tile_to_u8(tile: &TileType) -> u8 {
    match tile {
        TileType::Empty => 0,
        TileType::Floor => 1,
        TileType::Wall => 2,
        TileType::Door => 3,
        TileType::StairsUp => 4,
        TileType::StairsDown => 5,
        TileType::Chest => 6,
        TileType::Trap => 7,
        TileType::Spawner => 8,
        TileType::Shrine => 9,
        TileType::WindColumn => 10,
        TileType::VoidPit => 11,
    }
}

// ========================
// C-ABI: Floor Mutators (Session 20)
// ========================

/// Generate mutators for a floor, return JSON with mutator set + effects
#[no_mangle]
pub extern "C" fn generate_floor_mutators(seed: u64, floor_id: u32) -> *mut c_char {
    let set = mutators::generate_mutator_set(seed, floor_id);
    json_to_cstring(&set)
}

/// Get all available mutator types as JSON
#[no_mangle]
pub extern "C" fn get_all_mutator_types() -> *mut c_char {
    let all = mutators::all_mutator_types();
    json_to_cstring(&all)
}

/// Compute aggregate effects from a JSON array of mutators
#[no_mangle]
pub extern "C" fn compute_mutator_effects(mutators_json: *const c_char) -> *mut c_char {
    let json_str = match parse_cstr(mutators_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let mutators: Vec<mutators::FloorMutator> = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(_) => return std::ptr::null_mut(),
    };
    let effects = mutators::compute_effects(&mutators);
    json_to_cstring(&effects)
}

// ========================
// C-ABI: Game Flow (Session 20)
// ========================

/// Get all valid game states as JSON array of strings
#[no_mangle]
pub extern "C" fn get_all_game_states() -> *mut c_char {
    let states = gameflow::all_game_states();
    json_to_cstring(&states)
}

/// Get all valid in-game sub-states as JSON array of strings
#[no_mangle]
pub extern "C" fn get_all_sub_states() -> *mut c_char {
    let states = gameflow::all_sub_states();
    json_to_cstring(&states)
}

// ========================
// C-ABI: Save Migration (Session 20)
// ========================

/// Migrate a save file to current version, return JSON MigrationResult
#[no_mangle]
pub extern "C" fn migrate_save(save_json: *const c_char) -> *mut c_char {
    let json_str = match parse_cstr(save_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let result = savemigration::migrate_save(&json_str);
    json_to_cstring(&result)
}

/// Get the save version from a JSON string, returns 0 if invalid
#[no_mangle]
pub extern "C" fn get_save_version(save_json: *const c_char) -> u32 {
    let json_str = match parse_cstr(save_json) {
        Some(s) => s,
        None => return 0,
    };
    savemigration::get_save_version(&json_str).unwrap_or(0)
}

/// Create a new empty save at current version, return JSON
#[no_mangle]
pub extern "C" fn create_new_save(player_name: *const c_char) -> *mut c_char {
    let name = match parse_cstr(player_name) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let save = savemigration::create_new_save(&name);
    json_to_cstring(&save)
}

/// Get current save format version
#[no_mangle]
pub extern "C" fn get_current_save_version() -> u32 {
    savemigration::CURRENT_SAVE_VERSION
}

/// Validate a save file is at the current version, returns 1 if valid, 0 if not
#[no_mangle]
pub extern "C" fn validate_save(save_json: *const c_char) -> u32 {
    let json_str = match parse_cstr(save_json) {
        Some(s) => s,
        None => return 0,
    };
    if savemigration::validate_save(&json_str) {
        1
    } else {
        0
    }
}

// ========================
// C-ABI: Logging (Session 21)
// ========================

/// Get default logging configuration as JSON
#[no_mangle]
pub extern "C" fn logging_get_default_config() -> *mut c_char {
    let config = logging::TracingConfig::default();
    json_to_cstring(&config)
}

/// Initialize logging with JSON config
#[no_mangle]
pub extern "C" fn logging_init(config_json: *const c_char) {
    if let Some(json_str) = parse_cstr(config_json) {
        if let Some(config) = logging::TracingConfig::from_json(&json_str) {
            logging::init_tracing(&config);
        }
    }
}

/// Get current logging snapshot as JSON
#[no_mangle]
pub extern "C" fn logging_get_snapshot() -> *mut c_char {
    let config = logging::TracingConfig::default();
    let snapshot = logging::LoggingSnapshot::capture(&config);
    json_to_cstring(&snapshot)
}

/// Log a message at the specified level (0=Trace, 1=Debug, 2=Info, 3=Warn, 4=Error)
#[no_mangle]
pub extern "C" fn logging_log_message(level: u32, target: *const c_char, message: *const c_char) {
    let target_str = match parse_cstr(target) {
        Some(s) => s,
        None => return,
    };
    let msg_str = match parse_cstr(message) {
        Some(s) => s,
        None => return,
    };

    match level {
        0 | 1 => logging::log_debug(&target_str, &msg_str),
        2 => logging::log_info(&target_str, &msg_str),
        3 => logging::log_warn(&target_str, &msg_str),
        4 => logging::log_error(&target_str, &msg_str),
        _ => logging::log_info(&target_str, &msg_str),
    }
}

// ========================
// C-ABI: Replay System (Session 21)
// ========================

/// Start recording a replay
#[no_mangle]
pub extern "C" fn replay_start_recording(
    seed: u64,
    floor_id: u32,
    player_name: *const c_char,
    player_build_json: *const c_char,
    current_tick: u64,
) -> u32 {
    let name = match parse_cstr(player_name) {
        Some(s) => s,
        None => return 0,
    };
    let build = match parse_cstr(player_build_json) {
        Some(s) => s,
        None => return 0,
    };

    let tower_seed = TowerSeed { seed };
    let mut recorder = replay::ReplayRecorder::default();
    recorder.start_recording(&tower_seed, floor_id, &name, &build, current_tick);

    1 // Success
}

/// Record an input frame
#[no_mangle]
pub extern "C" fn replay_record_frame(tick: u64, input_type: u32, payload_json: *const c_char) {
    let payload = match parse_cstr(payload_json) {
        Some(s) => s,
        None => return,
    };

    let input = match input_type {
        0 => replay::InputType::Move,
        1 => replay::InputType::Attack,
        2 => replay::InputType::Parry,
        3 => replay::InputType::Dodge,
        4 => replay::InputType::UseAbility,
        5 => replay::InputType::Interact,
        6 => replay::InputType::Jump,
        7 => replay::InputType::ChangeWeapon,
        _ => return,
    };

    let mut recorder = replay::ReplayRecorder::default();
    recorder.record_frame(tick, input, &payload);
}

/// Stop recording and get the replay as JSON
#[no_mangle]
pub extern "C" fn replay_stop_recording(outcome: u32, current_tick: u64) -> *mut c_char {
    let outcome_enum = match outcome {
        0 => replay::ReplayOutcome::InProgress,
        1 => replay::ReplayOutcome::Victory,
        2 => replay::ReplayOutcome::Death,
        3 => replay::ReplayOutcome::Abandoned,
        _ => replay::ReplayOutcome::Abandoned,
    };

    let mut recorder = replay::ReplayRecorder::default();
    let recording = recorder.stop_recording(outcome_enum, vec![], current_tick);

    match recording {
        Some(rec) => json_to_cstring(&rec),
        None => std::ptr::null_mut(),
    }
}

/// Create a playback controller from recording JSON
#[no_mangle]
pub extern "C" fn replay_create_playback(recording_json: *const c_char) -> *mut c_char {
    let json_str = match parse_cstr(recording_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let recording = match replay::ReplayRecording::from_json(&json_str) {
        Some(r) => r,
        None => return std::ptr::null_mut(),
    };

    let playback = replay::ReplayPlayback::new(&recording);
    json_to_cstring(&playback)
}

/// Get replay snapshot for FFI
#[no_mangle]
pub extern "C" fn replay_get_snapshot() -> *mut c_char {
    let recorder = replay::ReplayRecorder::default();
    let snapshot = replay::ReplaySnapshot::capture(&recorder);
    json_to_cstring(&snapshot)
}

/// Get all input types as JSON
#[no_mangle]
pub extern "C" fn replay_get_input_types() -> *mut c_char {
    let types = vec![
        "Move",
        "Attack",
        "Parry",
        "Dodge",
        "UseAbility",
        "Interact",
        "Jump",
        "ChangeWeapon",
    ];
    json_to_cstring(&types)
}

// ========================
// C-ABI: Tower Map (Session 21)
// ========================

/// Create a new empty tower map
#[no_mangle]
pub extern "C" fn towermap_create() -> *mut c_char {
    let map = towermap::TowerMap::default();
    json_to_cstring(&map)
}

/// Discover a floor in the map, returns updated map JSON
#[no_mangle]
pub extern "C" fn towermap_discover_floor(
    map_json: *const c_char,
    floor_id: u32,
    tier: u32,
    total_rooms: u32,
    total_monsters: u32,
    total_chests: u32,
) -> *mut c_char {
    let json_str = match parse_cstr(map_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut map = match towermap::TowerMap::from_json(&json_str) {
        Some(m) => m,
        None => return std::ptr::null_mut(),
    };

    let tier_enum = match tier {
        0 => FloorTier::Echelon1,
        1 => FloorTier::Echelon2,
        2 => FloorTier::Echelon3,
        3 => FloorTier::Echelon4,
        _ => FloorTier::from_floor_id(floor_id),
    };

    map.discover_floor(
        floor_id,
        tier_enum,
        total_rooms,
        total_monsters,
        total_chests,
    );
    json_to_cstring(&map)
}

/// Clear a floor in the map, returns updated map JSON
#[no_mangle]
pub extern "C" fn towermap_clear_floor(
    map_json: *const c_char,
    floor_id: u32,
    clear_time_secs: f32,
) -> *mut c_char {
    let json_str = match parse_cstr(map_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut map = match towermap::TowerMap::from_json(&json_str) {
        Some(m) => m,
        None => return std::ptr::null_mut(),
    };

    map.clear_floor(floor_id, clear_time_secs);
    json_to_cstring(&map)
}

/// Record a death on a floor, returns updated map JSON
#[no_mangle]
pub extern "C" fn towermap_record_death(map_json: *const c_char, floor_id: u32) -> *mut c_char {
    let json_str = match parse_cstr(map_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut map = match towermap::TowerMap::from_json(&json_str) {
        Some(m) => m,
        None => return std::ptr::null_mut(),
    };

    map.record_death(floor_id);
    json_to_cstring(&map)
}

/// Get a specific floor entry as JSON
#[no_mangle]
pub extern "C" fn towermap_get_floor(map_json: *const c_char, floor_id: u32) -> *mut c_char {
    let json_str = match parse_cstr(map_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let map = match towermap::TowerMap::from_json(&json_str) {
        Some(m) => m,
        None => return std::ptr::null_mut(),
    };

    match map.get_floor(floor_id) {
        Some(entry) => json_to_cstring(entry),
        None => std::ptr::null_mut(),
    }
}

/// Get tower map overview as JSON
#[no_mangle]
pub extern "C" fn towermap_get_overview(map_json: *const c_char) -> *mut c_char {
    let json_str = match parse_cstr(map_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let map = match towermap::TowerMap::from_json(&json_str) {
        Some(m) => m,
        None => return std::ptr::null_mut(),
    };

    let overview = towermap::TowerMapOverview::from_map(&map);
    json_to_cstring(&overview)
}

/// Update floor progress (room discovered), returns updated map JSON
#[no_mangle]
pub extern "C" fn towermap_discover_room(map_json: *const c_char, floor_id: u32) -> *mut c_char {
    let json_str = match parse_cstr(map_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut map = match towermap::TowerMap::from_json(&json_str) {
        Some(m) => m,
        None => return std::ptr::null_mut(),
    };

    if let Some(entry) = map.get_floor_mut(floor_id) {
        entry.discover_room();
    }

    json_to_cstring(&map)
}

/// Update floor progress (monster killed), returns updated map JSON
#[no_mangle]
pub extern "C" fn towermap_kill_monster(map_json: *const c_char, floor_id: u32) -> *mut c_char {
    let json_str = match parse_cstr(map_json) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    let mut map = match towermap::TowerMap::from_json(&json_str) {
        Some(m) => m,
        None => return std::ptr::null_mut(),
    };

    if let Some(entry) = map.get_floor_mut(floor_id) {
        entry.kill_monster();
    }

    json_to_cstring(&map)
}

// ========================
// C-ABI: Hot-Reload (Session 22)
// ========================

/// Get hot-reload status
#[no_mangle]
pub extern "C" fn hotreload_get_status() -> *mut c_char {
    // Create a default status since we can't access Bevy resources from FFI
    let status = hotreload::HotReloadStatus {
        enabled: true,
        watched_file: Some("config/engine.json".to_string()),
        reload_count: 0,
        last_reload_success: false,
        last_reload_time: 0.0,
        last_error: None,
    };
    json_to_cstring(&status)
}

/// Trigger manual config reload
#[no_mangle]
pub extern "C" fn hotreload_trigger_reload() -> u32 {
    // In a real implementation, this would post an event to Bevy
    // For now, just return success
    1
}

// ========================
// C-ABI: Analytics (Session 22)
// ========================

/// Get analytics snapshot
#[no_mangle]
pub extern "C" fn analytics_get_snapshot() -> *mut c_char {
    // Create an empty snapshot
    let snapshot = analytics::AnalyticsSnapshot {
        combat: analytics::CombatStats::default(),
        progression: analytics::ProgressionStats::default(),
        equipment: analytics::EquipmentStats::default(),
        economy: analytics::EconomyStats::default(),
        behavior: analytics::BehaviorStats::default(),
    };
    json_to_cstring(&snapshot)
}

/// Reset analytics
#[no_mangle]
pub extern "C" fn analytics_reset() {
    // In a real implementation, this would post an event to Bevy
    // For now, this is a no-op
}

/// Record combat event
#[no_mangle]
pub extern "C" fn analytics_record_damage(weapon: *const c_char, _amount: u32) {
    let _weapon_name = match parse_cstr(weapon) {
        Some(w) => w,
        None => return,
    };
    // In a real implementation, this would send an AnalyticsEvent
    // For now, this is a no-op
}

/// Record floor cleared
#[no_mangle]
pub extern "C" fn analytics_record_floor_cleared(floor_id: u32, tier: u8, time_secs: f32) {
    // In a real implementation, this would send an AnalyticsEvent
    // For now, this is a no-op
    let _ = (floor_id, tier, time_secs);
}

/// Record gold transaction
#[no_mangle]
pub extern "C" fn analytics_record_gold(amount: u64, earned: u32) {
    // earned: 1 = earned, 0 = spent
    // In a real implementation, this would send an AnalyticsEvent
    let _ = (amount, earned);
}

/// Get analytics event types
#[no_mangle]
pub extern "C" fn analytics_get_event_types() -> *mut c_char {
    let types = vec![
        "CombatDamageDealt",
        "CombatDamageTaken",
        "CombatKill",
        "CombatDeath",
        "CombatParry",
        "CombatDodge",
        "CombatAbilityUsed",
        "FloorCleared",
        "RoomExplored",
        "SecretFound",
        "WeaponSwitched",
        "GoldEarned",
        "GoldSpent",
        "ItemCrafted",
        "ItemTraded",
        "Action",
    ];
    json_to_cstring(&types)
}

// ========================
// Tests
// ========================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_generate_floor_ffi() {
        let result_ptr = generate_floor(42, 1);
        assert!(!result_ptr.is_null());
        let json_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
        let response: FloorResponse = serde_json::from_str(json_str).unwrap();
        assert_eq!(response.floor_id, 1);
        assert!(!response.biome_tags.is_empty());
        free_string(result_ptr);
    }

    #[test]
    fn test_generate_floor_layout_ffi() {
        let result_ptr = generate_floor_layout(42, 1);
        assert!(!result_ptr.is_null());
        let json_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
        let response: FloorLayoutResponse = serde_json::from_str(json_str).unwrap();
        assert!(response.width > 0);
        assert!(response.height > 0);
        assert!(!response.rooms.is_empty());
        free_string(result_ptr);
    }

    #[test]
    fn test_generate_monster_ffi() {
        let result_ptr = generate_monster(12345, 10);
        assert!(!result_ptr.is_null());
        let json_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
        let info: MonsterInfo = serde_json::from_str(json_str).unwrap();
        assert!(!info.name.is_empty());
        assert!(info.max_hp > 0.0);
        free_string(result_ptr);
    }

    #[test]
    fn test_generate_floor_monsters_ffi() {
        let result_ptr = generate_floor_monsters(42, 5, 3);
        assert!(!result_ptr.is_null());
        let json_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
        let monsters: Vec<MonsterInfo> = serde_json::from_str(json_str).unwrap();
        assert_eq!(monsters.len(), 3);
        free_string(result_ptr);
    }

    #[test]
    fn test_floor_hash_ffi() {
        let hash1 = get_floor_hash(42, 1);
        let hash2 = get_floor_hash(42, 1);
        assert_eq!(hash1, hash2);
        let hash3 = get_floor_hash(42, 2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_floor_tier_ffi() {
        assert_eq!(get_floor_tier(50), 0); // Echelon1
        assert_eq!(get_floor_tier(200), 1); // Echelon2
        assert_eq!(get_floor_tier(400), 2); // Echelon3
        assert_eq!(get_floor_tier(600), 3); // Echelon4
    }

    #[test]
    fn test_angle_multiplier_ffi() {
        assert!((get_angle_multiplier(0) - 1.0).abs() < f32::EPSILON);
        assert!((get_angle_multiplier(1) - 0.7).abs() < f32::EPSILON);
        assert!((get_angle_multiplier(2) - 1.5).abs() < f32::EPSILON);
        assert!((get_angle_multiplier(99) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_version_ffi() {
        let ptr = get_version();
        let version = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        assert_eq!(version, "0.6.0");
        free_string(ptr);
    }

    #[test]
    fn test_generate_loot_ffi() {
        let tags_json = CString::new(r#"[["fire", 0.8], ["corruption", 0.3]]"#).unwrap();
        let result_ptr = generate_loot(tags_json.as_ptr(), 10, 42);
        assert!(!result_ptr.is_null());
        let json_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
        let items: Vec<LootInfo> = serde_json::from_str(json_str).unwrap();
        assert!(!items.is_empty());
        free_string(result_ptr);
    }

    #[test]
    fn test_breath_state_ffi() {
        let ptr = get_breath_state(100.0); // early in Inhale phase
        assert!(!ptr.is_null());
        let json_str = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        let state: BreathState = serde_json::from_str(json_str).unwrap();
        assert_eq!(state.phase, "Inhale");
        assert!(state.phase_progress > 0.0 && state.phase_progress < 1.0);
        free_string(ptr);
    }

    #[test]
    fn test_record_delta_ffi() {
        let player = CString::new("player1").unwrap();
        let payload = CString::new(r#"{"xp":50}"#).unwrap();
        let result = record_delta(0, 5, 12345, player.as_ptr(), payload.as_ptr(), 100);
        assert!(!result.is_null());
        free_string(result);
    }

    #[test]
    fn test_create_floor_snapshot_ffi() {
        let deltas_json = CString::new("[]").unwrap();
        let result = create_floor_snapshot(42, 1, deltas_json.as_ptr());
        assert!(!result.is_null());
        let json_str = unsafe { CStr::from_ptr(result).to_str().unwrap() };
        assert!(json_str.contains("\"seed\":42"));
        assert!(json_str.contains("\"floor_id\":1"));
        free_string(result);
    }

    #[test]
    fn test_evaluate_event_breath_shift() {
        let ctx = crate::events::TriggerContext {
            breath_phase: Some("Hold".into()),
            floor_tags: vec![("fire".into(), 0.7)],
            floor_hash: 42,
            ..Default::default()
        };
        let ctx_json = CString::new(serde_json::to_string(&ctx).unwrap()).unwrap();
        let result = evaluate_event_trigger(0, ctx_json.as_ptr()); // 0 = BreathShift
        assert!(!result.is_null());
        let json_str = unsafe { CStr::from_ptr(result).to_str().unwrap() };
        assert!(json_str.contains("BreathShift"));
        free_string(result);
    }

    #[test]
    fn test_evaluate_event_no_trigger() {
        let ctx = crate::events::TriggerContext {
            corruption_level: 0.1, // too low
            floor_hash: 42,
            ..Default::default()
        };
        let ctx_json = CString::new(serde_json::to_string(&ctx).unwrap()).unwrap();
        let result = evaluate_event_trigger(5, ctx_json.as_ptr()); // 5 = CorruptionSurge
        assert!(result.is_null(), "Low corruption should not trigger event");
    }

    #[test]
    fn test_combat_calc_ffi() {
        let request = CombatCalcRequest {
            base_damage: 100.0,
            angle_id: 2, // Back
            combo_step: 1,
            attacker_tags_json: r#"[["fire", 0.8]]"#.into(),
            defender_tags_json: r#"[["water", 0.9]]"#.into(),
        };
        let request_json = CString::new(serde_json::to_string(&request).unwrap()).unwrap();
        let result_ptr = calculate_combat(request_json.as_ptr());
        assert!(!result_ptr.is_null());
        let json_str = unsafe { CStr::from_ptr(result_ptr).to_str().unwrap() };
        let result: CombatCalcResult = serde_json::from_str(json_str).unwrap();
        assert!(
            result.final_damage > 100.0,
            "Back attack + combo should increase damage"
        );
        assert!((result.angle_multiplier - 1.5).abs() < f32::EPSILON);
        free_string(result_ptr);
    }

    // ========================
    // Mastery FFI Tests
    // ========================

    #[test]
    fn test_mastery_create_profile() {
        let ptr = mastery_create_profile();
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        assert!(json.contains("masteries") || json.contains("{"));
        free_string(ptr);
    }

    #[test]
    fn test_mastery_gain_xp() {
        let profile_ptr = mastery_create_profile();
        assert!(!profile_ptr.is_null());

        let updated = mastery_gain_xp(profile_ptr, 0, 500); // SwordMastery + 500 XP
        assert!(!updated.is_null());

        let tier = mastery_get_tier(updated, 0);
        assert!(tier >= 0, "Tier should be valid after gaining XP");

        free_string(profile_ptr);
        free_string(updated);
    }

    #[test]
    fn test_mastery_xp_for_action() {
        let action = CString::new("sword_attack").unwrap();
        let xp = mastery_xp_for_action(action.as_ptr());
        assert!(xp > 0, "sword_attack should give XP");
    }

    #[test]
    fn test_mastery_get_all_domains() {
        let ptr = mastery_get_all_domains();
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        let domains: Vec<String> = serde_json::from_str(json).unwrap();
        assert_eq!(domains.len(), 21);
        assert!(domains.contains(&"SwordMastery".to_string()));
        free_string(ptr);
    }

    #[test]
    fn test_mastery_invalid_domain() {
        let profile_ptr = mastery_create_profile();
        let result = mastery_gain_xp(profile_ptr, 99, 100); // invalid domain
        assert!(result.is_null());
        assert_eq!(mastery_get_tier(profile_ptr, 99), -1);
        free_string(profile_ptr);
    }

    // ========================
    // Specialization FFI Tests
    // ========================

    #[test]
    fn test_spec_get_all_branches() {
        let ptr = spec_get_all_branches();
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        let branches: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(
            branches.len() >= 10,
            "Should have many specialization branches"
        );
        free_string(ptr);
    }

    #[test]
    fn test_spec_create_profile() {
        let ptr = spec_create_profile();
        assert!(!ptr.is_null());
        free_string(ptr);
    }

    #[test]
    fn test_spec_find_synergies() {
        let ids = CString::new(r#"["sword_berserker","parry_counter"]"#).unwrap();
        let ptr = spec_find_synergies(ids.as_ptr());
        assert!(!ptr.is_null());
        free_string(ptr);
    }

    // ========================
    // Abilities FFI Tests
    // ========================

    #[test]
    fn test_ability_get_defaults() {
        let ptr = ability_get_defaults();
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        let abilities: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(abilities.len() >= 5, "Should have default abilities");
        free_string(ptr);
    }

    #[test]
    fn test_ability_loadout_workflow() {
        // Create loadout
        let loadout_ptr = ability_create_loadout();
        assert!(!loadout_ptr.is_null());

        // Learn an ability
        let defaults = default_abilities();
        if !defaults.is_empty() {
            let aid = CString::new(defaults[0].id.as_str()).unwrap();
            let learned = ability_learn(loadout_ptr, aid.as_ptr());
            assert!(!learned.is_null());

            // Equip to slot 0
            let equipped = ability_equip(learned, 0, aid.as_ptr());
            assert!(!equipped.is_null());

            free_string(learned);
            free_string(equipped);
        }

        free_string(loadout_ptr);
    }

    // ========================
    // Socket FFI Tests
    // ========================

    #[test]
    fn test_socket_starter_gems() {
        let ptr = socket_get_starter_gems();
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        let gems: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(gems.len() >= 3, "Should have starter gems");
        free_string(ptr);
    }

    #[test]
    fn test_socket_starter_runes() {
        let ptr = socket_get_starter_runes();
        assert!(!ptr.is_null());
        free_string(ptr);
    }

    #[test]
    fn test_socket_create_equipment() {
        let name = CString::new("Test Sword").unwrap();
        let colors = CString::new("[0, 1, 3]").unwrap(); // Red, Blue, Prismatic
        let ptr = socket_create_equipment(name.as_ptr(), colors.as_ptr());
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        assert!(json.contains("Test Sword"));
        free_string(ptr);
    }

    #[test]
    fn test_socket_insert_gem() {
        let name = CString::new("Gemmed Blade").unwrap();
        let colors = CString::new("[0]").unwrap();
        let equip_ptr = socket_create_equipment(name.as_ptr(), colors.as_ptr());
        assert!(!equip_ptr.is_null());

        let gems = starter_gems();
        if !gems.is_empty() {
            let gem_json_str = serde_json::to_string(&gems[0]).unwrap();
            let gem_cstr = CString::new(gem_json_str).unwrap();
            let result = socket_insert_gem(equip_ptr, 0, gem_cstr.as_ptr());
            // May be null if color doesn't match, that's ok
            if !result.is_null() {
                free_string(result);
            }
        }

        free_string(equip_ptr);
    }

    // ========================
    // Cosmetics FFI Tests
    // ========================

    #[test]
    fn test_cosmetic_get_all() {
        let ptr = cosmetic_get_all();
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        let cosmetics: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(!cosmetics.is_empty());
        free_string(ptr);
    }

    #[test]
    fn test_cosmetic_get_all_dyes() {
        let ptr = cosmetic_get_all_dyes();
        assert!(!ptr.is_null());
        free_string(ptr);
    }

    #[test]
    fn test_cosmetic_profile_workflow() {
        let profile_ptr = cosmetic_create_profile();
        assert!(!profile_ptr.is_null());

        let cosmetics = tower_cosmetics();
        if !cosmetics.is_empty() {
            let cid = CString::new(cosmetics[0].id.as_str()).unwrap();
            let unlocked = cosmetic_unlock(profile_ptr, cid.as_ptr());
            assert!(!unlocked.is_null());
            free_string(unlocked);
        }

        free_string(profile_ptr);
    }

    // ========================
    // Tutorial FFI Tests
    // ========================

    #[test]
    fn test_tutorial_get_steps() {
        let ptr = tutorial_get_steps();
        assert!(!ptr.is_null());
        free_string(ptr);
    }

    #[test]
    fn test_tutorial_get_hints() {
        let ptr = tutorial_get_hints();
        assert!(!ptr.is_null());
        free_string(ptr);
    }

    #[test]
    fn test_tutorial_workflow() {
        let progress_ptr = tutorial_create_progress();
        assert!(!progress_ptr.is_null());

        let pct = tutorial_completion_percent(progress_ptr);
        assert!((0.0..=100.0).contains(&pct));

        let steps = tutorial_steps();
        if !steps.is_empty() {
            let sid = CString::new(steps[0].id.as_str()).unwrap();
            let updated = tutorial_complete_step(progress_ptr, sid.as_ptr());
            assert!(!updated.is_null());

            let new_pct = tutorial_completion_percent(updated);
            assert!(
                new_pct > pct || pct == 0.0,
                "Completing step should increase progress"
            );
            free_string(updated);
        }

        free_string(progress_ptr);
    }

    // ========================
    // Achievement FFI Tests
    // ========================

    #[test]
    fn test_achievement_create_tracker() {
        let ptr = achievement_create_tracker();
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        assert!(json.len() > 10, "Tracker JSON should not be empty");
        free_string(ptr);
    }

    #[test]
    fn test_achievement_increment_and_check() {
        let tracker_ptr = achievement_create_tracker();
        assert!(!tracker_ptr.is_null());

        let aid = CString::new("monster_slayer_1").unwrap();
        let incremented = achievement_increment(tracker_ptr, aid.as_ptr(), 50);
        assert!(!incremented.is_null());

        let checked = achievement_check_all(incremented, 1000);
        assert!(!checked.is_null());

        let pct = achievement_completion_percent(checked);
        assert!((0.0..=1.0).contains(&pct));

        free_string(tracker_ptr);
        free_string(incremented);
        free_string(checked);
    }

    // ========================
    // Season Pass FFI Tests
    // ========================

    #[test]
    fn test_season_create_pass() {
        let name = CString::new("Season of the Tower").unwrap();
        let ptr = season_create_pass(1, name.as_ptr());
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        assert!(json.contains("Season of the Tower"));
        free_string(ptr);
    }

    #[test]
    fn test_season_add_xp() {
        let name = CString::new("Test Season").unwrap();
        let pass_ptr = season_create_pass(1, name.as_ptr());
        let updated = season_add_xp(pass_ptr, 5000);
        assert!(!updated.is_null());
        free_string(pass_ptr);
        free_string(updated);
    }

    #[test]
    fn test_season_generate_dailies() {
        let ptr = season_generate_dailies(42);
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        let quests: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert_eq!(quests.len(), 3, "Should generate 3 daily quests");
        free_string(ptr);
    }

    #[test]
    fn test_season_generate_weeklies() {
        let ptr = season_generate_weeklies(42);
        assert!(!ptr.is_null());
        free_string(ptr);
    }

    #[test]
    fn test_season_get_rewards() {
        let ptr = season_get_rewards(1);
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        let rewards: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(!rewards.is_empty(), "Should have season rewards");
        free_string(ptr);
    }

    // ========================
    // Social FFI Tests
    // ========================

    #[test]
    fn test_social_create_guild() {
        let name = CString::new("Test Guild").unwrap();
        let tag = CString::new("TG").unwrap();
        let lid = CString::new("leader1").unwrap();
        let lname = CString::new("LeaderName").unwrap();
        let faction = CString::new("AscendingOrder").unwrap();

        let ptr = social_create_guild(
            name.as_ptr(),
            tag.as_ptr(),
            lid.as_ptr(),
            lname.as_ptr(),
            faction.as_ptr(),
        );
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        assert!(json.contains("Test Guild"));
        free_string(ptr);
    }

    #[test]
    fn test_social_guild_add_member() {
        let name = CString::new("My Guild").unwrap();
        let tag = CString::new("MG").unwrap();
        let lid = CString::new("leader1").unwrap();
        let lname = CString::new("Leader").unwrap();
        let faction = CString::new("DeepDwellers").unwrap();

        let guild_ptr = social_create_guild(
            name.as_ptr(),
            tag.as_ptr(),
            lid.as_ptr(),
            lname.as_ptr(),
            faction.as_ptr(),
        );
        assert!(!guild_ptr.is_null());

        let uid = CString::new("member1").unwrap();
        let uname = CString::new("MemberOne").unwrap();
        let updated = social_guild_add_member(guild_ptr, uid.as_ptr(), uname.as_ptr());
        assert!(!updated.is_null());

        free_string(guild_ptr);
        free_string(updated);
    }

    #[test]
    fn test_social_create_party() {
        let lid = CString::new("player1").unwrap();
        let lname = CString::new("Player One").unwrap();
        let ptr = social_create_party(lid.as_ptr(), lname.as_ptr());
        assert!(!ptr.is_null());
        free_string(ptr);
    }

    #[test]
    fn test_social_party_add_member() {
        let lid = CString::new("player1").unwrap();
        let lname = CString::new("Player One").unwrap();
        let party_ptr = social_create_party(lid.as_ptr(), lname.as_ptr());

        let uid = CString::new("player2").unwrap();
        let uname = CString::new("Player Two").unwrap();
        let updated = social_party_add_member(party_ptr, uid.as_ptr(), uname.as_ptr(), 1); // Striker
        assert!(!updated.is_null());

        free_string(party_ptr);
        free_string(updated);
    }

    #[test]
    fn test_social_trade_workflow() {
        let pa = CString::new("player_a").unwrap();
        let pb = CString::new("player_b").unwrap();
        let trade_ptr = social_create_trade(pa.as_ptr(), pb.as_ptr());
        assert!(!trade_ptr.is_null());

        // Add item
        let iname = CString::new("Fire Sword").unwrap();
        let rarity = CString::new("Rare").unwrap();
        let with_item =
            social_trade_add_item(trade_ptr, pa.as_ptr(), iname.as_ptr(), 1, rarity.as_ptr());
        assert!(!with_item.is_null());

        // Lock
        let locked = social_trade_lock(with_item, pa.as_ptr());
        assert!(!locked.is_null());

        free_string(trade_ptr);
        free_string(with_item);
        free_string(locked);
    }

    // ========================
    // Version test (updated)
    // ========================

    #[test]
    fn test_version_ffi_v030() {
        let ptr = get_version();
        let version = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        assert_eq!(version, "0.6.0");
        free_string(ptr);
    }

    // ========================
    // Null safety tests
    // ========================

    #[test]
    fn test_null_input_safety() {
        // All functions should handle null inputs gracefully
        assert!(mastery_gain_xp(std::ptr::null(), 0, 100).is_null());
        assert_eq!(mastery_get_tier(std::ptr::null(), 0), -1);
        assert_eq!(mastery_xp_for_action(std::ptr::null()), 0);
        assert!(spec_choose_branch(std::ptr::null(), std::ptr::null(), std::ptr::null()).is_null());
        assert!(ability_learn(std::ptr::null(), std::ptr::null()).is_null());
        assert!(socket_create_equipment(std::ptr::null(), std::ptr::null()).is_null());
        assert!(cosmetic_unlock(std::ptr::null(), std::ptr::null()).is_null());
        assert!(tutorial_complete_step(std::ptr::null(), std::ptr::null()).is_null());
        assert!(achievement_increment(std::ptr::null(), std::ptr::null(), 1).is_null());
        assert!(season_add_xp(std::ptr::null(), 100).is_null());
        assert!(social_create_guild(
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null()
        )
        .is_null());
        assert_eq!(tutorial_completion_percent(std::ptr::null()), 0.0);
        assert_eq!(achievement_completion_percent(std::ptr::null()), 0.0);
        // Session 20 null safety
        assert!(compute_mutator_effects(std::ptr::null()).is_null());
        assert!(migrate_save(std::ptr::null()).is_null());
        assert_eq!(get_save_version(std::ptr::null()), 0);
        assert!(create_new_save(std::ptr::null()).is_null());
        assert_eq!(validate_save(std::ptr::null()), 0);
        // Session 21 null safety
        assert_eq!(
            replay_start_recording(42, 1, std::ptr::null(), std::ptr::null(), 0),
            0
        );
        assert!(replay_create_playback(std::ptr::null()).is_null());
        assert!(towermap_discover_floor(std::ptr::null(), 1, 0, 5, 10, 3).is_null());
        assert!(towermap_clear_floor(std::ptr::null(), 1, 100.0).is_null());
        assert!(towermap_record_death(std::ptr::null(), 1).is_null());
        assert!(towermap_get_floor(std::ptr::null(), 1).is_null());
        assert!(towermap_get_overview(std::ptr::null()).is_null());
    }

    // ========================
    // Floor Mutators FFI Tests (Session 20)
    // ========================

    #[test]
    fn test_generate_floor_mutators_ffi() {
        let ptr = generate_floor_mutators(42, 200);
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        let set: mutators::FloorMutatorSet = serde_json::from_str(json).unwrap();
        assert_eq!(set.floor_id, 200);
        assert_eq!(set.mutators.len(), 2); // Echelon2 → 2 mutators
        assert!(set.effects.reward_multiplier >= 1.0);
        free_string(ptr);
    }

    #[test]
    fn test_generate_floor_mutators_echelon4_ffi() {
        let ptr = generate_floor_mutators(42, 600);
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        let set: mutators::FloorMutatorSet = serde_json::from_str(json).unwrap();
        assert_eq!(set.mutators.len(), 4); // Echelon4 → 4 mutators
        free_string(ptr);
    }

    #[test]
    fn test_get_all_mutator_types_ffi() {
        let ptr = get_all_mutator_types();
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        let types: Vec<mutators::FloorMutator> = serde_json::from_str(json).unwrap();
        assert_eq!(types.len(), 28);
        free_string(ptr);
    }

    #[test]
    fn test_compute_mutator_effects_ffi() {
        let input = serde_json::json!([
            {"mutator_type": "DoubleDamage", "category": "Combat", "description": "test", "difficulty": 3, "icon_id": "test", "intensity": 1.0},
            {"mutator_type": "Bountiful", "category": "Economy", "description": "test", "difficulty": 1, "icon_id": "test", "intensity": 1.0}
        ]);
        let cstr = CString::new(input.to_string()).unwrap();
        let ptr = compute_mutator_effects(cstr.as_ptr());
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        let fx: mutators::MutatorEffects = serde_json::from_str(json).unwrap();
        assert!(fx.damage_dealt_mult > 1.5);
        assert!(fx.loot_quantity_mult > 1.5);
        free_string(ptr);
    }

    // ========================
    // Game Flow FFI Tests (Session 20)
    // ========================

    #[test]
    fn test_get_all_game_states_ffi() {
        let ptr = get_all_game_states();
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        let states: Vec<String> = serde_json::from_str(json).unwrap();
        assert_eq!(states.len(), 7);
        assert!(states.contains(&"InGame".to_string()));
        free_string(ptr);
    }

    #[test]
    fn test_get_all_sub_states_ffi() {
        let ptr = get_all_sub_states();
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        let states: Vec<String> = serde_json::from_str(json).unwrap();
        assert_eq!(states.len(), 7);
        assert!(states.contains(&"Exploring".to_string()));
        free_string(ptr);
    }

    // ========================
    // Save Migration FFI Tests (Session 20)
    // ========================

    #[test]
    fn test_create_new_save_ffi() {
        let name = CString::new("TestHero").unwrap();
        let ptr = create_new_save(name.as_ptr());
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        assert!(json.contains("TestHero"));
        assert!(json.contains(&format!(
            "\"version\":{}",
            savemigration::CURRENT_SAVE_VERSION
        )));
        free_string(ptr);
    }

    #[test]
    fn test_migrate_save_ffi() {
        let v1_save = serde_json::json!({
            "version": 1,
            "player_name": "Migrator",
            "player_level": 10,
            "inventory": {"items": [], "shards": 500}
        });
        let cstr = CString::new(v1_save.to_string()).unwrap();
        let ptr = migrate_save(cstr.as_ptr());
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        let result: savemigration::MigrationResult = serde_json::from_str(json).unwrap();
        assert!(result.success);
        assert_eq!(result.original_version, 1);
        assert_eq!(result.final_version, savemigration::CURRENT_SAVE_VERSION);
        free_string(ptr);
    }

    #[test]
    fn test_get_save_version_ffi() {
        let save = serde_json::json!({"version": 2}).to_string();
        let cstr = CString::new(save).unwrap();
        assert_eq!(get_save_version(cstr.as_ptr()), 2);
    }

    #[test]
    fn test_validate_save_ffi() {
        let name = CString::new("Validator").unwrap();
        let save_ptr = create_new_save(name.as_ptr());
        assert!(!save_ptr.is_null());
        assert_eq!(validate_save(save_ptr), 1); // current version → valid
        free_string(save_ptr);

        let old = CString::new(r#"{"version":1}"#).unwrap();
        assert_eq!(validate_save(old.as_ptr()), 0); // old version → invalid
    }

    #[test]
    fn test_get_current_save_version_ffi() {
        assert_eq!(
            get_current_save_version(),
            savemigration::CURRENT_SAVE_VERSION
        );
    }

    // ========================
    // Logging FFI Tests (Session 21)
    // ========================

    #[test]
    fn test_logging_get_default_config_ffi() {
        let ptr = logging_get_default_config();
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        assert!(json.contains("default_level"));
        free_string(ptr);
    }

    #[test]
    fn test_logging_get_snapshot_ffi() {
        let ptr = logging_get_snapshot();
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        assert!(json.contains("available_levels"));
        free_string(ptr);
    }

    #[test]
    fn test_logging_log_message_ffi() {
        let target = CString::new("test").unwrap();
        let message = CString::new("test message").unwrap();
        logging_log_message(2, target.as_ptr(), message.as_ptr()); // Info level
                                                                   // Should not panic
    }

    // ========================
    // Replay FFI Tests (Session 21)
    // ========================

    #[test]
    fn test_replay_get_snapshot_ffi() {
        let ptr = replay_get_snapshot();
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        assert!(json.contains("is_recording"));
        free_string(ptr);
    }

    #[test]
    fn test_replay_get_input_types_ffi() {
        let ptr = replay_get_input_types();
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        let types: Vec<String> = serde_json::from_str(json).unwrap();
        assert_eq!(types.len(), 8);
        free_string(ptr);
    }

    // ========================
    // Tower Map FFI Tests (Session 21)
    // ========================

    #[test]
    fn test_towermap_create_ffi() {
        let ptr = towermap_create();
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        let map: towermap::TowerMap = serde_json::from_str(json).unwrap();
        assert_eq!(map.highest_floor_reached, 0);
        free_string(ptr);
    }

    #[test]
    fn test_towermap_discover_floor_ffi() {
        let map_ptr = towermap_create();
        let updated = towermap_discover_floor(map_ptr, 10, 1, 5, 10, 3);
        assert!(!updated.is_null());

        let json = unsafe { CStr::from_ptr(updated).to_str().unwrap() };
        let map: towermap::TowerMap = serde_json::from_str(json).unwrap();
        assert_eq!(map.highest_floor_reached, 10);
        assert_eq!(map.total_floors_discovered, 1);

        free_string(map_ptr);
        free_string(updated);
    }

    #[test]
    fn test_towermap_clear_floor_ffi() {
        let map_ptr = towermap_create();
        let discovered = towermap_discover_floor(map_ptr, 1, 0, 5, 10, 3);
        let cleared = towermap_clear_floor(discovered, 1, 120.5);
        assert!(!cleared.is_null());

        let json = unsafe { CStr::from_ptr(cleared).to_str().unwrap() };
        let map: towermap::TowerMap = serde_json::from_str(json).unwrap();
        assert_eq!(map.total_floors_cleared, 1);

        free_string(map_ptr);
        free_string(discovered);
        free_string(cleared);
    }

    #[test]
    fn test_towermap_get_overview_ffi() {
        let map_ptr = towermap_create();
        let discovered = towermap_discover_floor(map_ptr, 1, 0, 5, 10, 3);
        let overview_ptr = towermap_get_overview(discovered);
        assert!(!overview_ptr.is_null());

        let json = unsafe { CStr::from_ptr(overview_ptr).to_str().unwrap() };
        assert!(json.contains("highest_floor"));
        assert!(json.contains("total_discovered"));

        free_string(map_ptr);
        free_string(discovered);
        free_string(overview_ptr);
    }
}
