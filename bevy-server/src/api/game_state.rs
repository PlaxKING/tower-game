//! GameStateService — World state management endpoints
//!
//! Endpoints:
//! - POST /tower.GameStateService/GetState
//! - POST /tower.GameStateService/GetWorldCycle
//! - POST /tower.GameStateService/GetPlayerProfile
//! - POST /tower.GameStateService/GetLiveStatus  (reads from Bevy ECS snapshot)
//! - POST /tower.GameStateService/GetLivePlayer   (reads live player from ECS)

use axum::{Router, Json, extract::State, routing::post};
use serde::{Deserialize, Serialize};

use super::ApiState;
use crate::ecs_bridge::GameCommand;

pub fn routes() -> Router<ApiState> {
    Router::new()
        .route("/tower.GameStateService/GetState", post(get_state))
        .route("/tower.GameStateService/GetWorldCycle", post(get_world_cycle))
        .route("/tower.GameStateService/GetPlayerProfile", post(get_player_profile))
        .route("/tower.GameStateService/GetLiveStatus", post(get_live_status))
        .route("/tower.GameStateService/GetLivePlayer", post(get_live_player))
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Deserialize)]
pub struct GetStateRequest {
    pub player_id: u64,
    pub floor_id: u32,
}

#[derive(Serialize)]
pub struct GetStateResponse {
    pub player: Option<PlayerState>,
    pub world_cycle: WorldCycleState,
    pub server_tick: u64,
}

#[derive(Serialize)]
pub struct PlayerState {
    pub id: u64,
    pub username: String,
    pub health: f32,
    pub max_health: f32,
    pub floor_id: u32,
    pub position: [f32; 3],
    pub is_alive: bool,
    pub kinetic_energy: f32,
    pub thermal_energy: f32,
    pub semantic_energy: f32,
}

#[derive(Deserialize)]
pub struct WorldCycleRequest {
    pub tower_seed: u64,
}

#[derive(Serialize, Clone)]
pub struct WorldCycleState {
    pub cycle_name: String,
    pub phase: u32,
    pub phase_progress: f32,
    pub corruption_level: f32,
    pub active_events: Vec<String>,
}

#[derive(Deserialize)]
pub struct PlayerProfileRequest {
    pub player_id: u64,
}

#[derive(Serialize)]
pub struct PlayerProfileResponse {
    pub id: u64,
    pub username: String,
    pub health: f32,
    pub max_health: f32,
    pub floor_id: u32,
    pub position: [f32; 3],
    pub base_stats: BaseStats,
    pub is_alive: bool,
}

#[derive(Serialize)]
pub struct BaseStats {
    pub str_stat: u32,
    pub dex_stat: u32,
    pub int_stat: u32,
    pub vit_stat: u32,
    pub luk_stat: u32,
}

// ============================================================================
// Handlers
// ============================================================================

async fn get_state(
    State(state): State<ApiState>,
    Json(req): Json<GetStateRequest>,
) -> Json<GetStateResponse> {
    let player = match state.pg.get_player(req.player_id as i64).await {
        Ok(Some(row)) => Some(PlayerState {
            id: row.id as u64,
            username: row.username.clone(),
            health: row.health,
            max_health: row.max_health,
            floor_id: row.floor_id as u32,
            position: [row.pos_x, row.pos_y, row.pos_z],
            is_alive: row.is_alive,
            kinetic_energy: row.kinetic_energy,
            thermal_energy: row.thermal_energy,
            semantic_energy: row.semantic_energy,
        }),
        _ => None,
    };

    // Calculate world cycle from current time
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let cycle = compute_world_cycle(now);

    Json(GetStateResponse {
        player,
        world_cycle: cycle,
        server_tick: now,
    })
}

async fn get_world_cycle(
    State(_state): State<ApiState>,
    Json(_req): Json<WorldCycleRequest>,
) -> Json<WorldCycleState> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Json(compute_world_cycle(now))
}

async fn get_player_profile(
    State(state): State<ApiState>,
    Json(req): Json<PlayerProfileRequest>,
) -> Json<PlayerProfileResponse> {
    match state.pg.get_player(req.player_id as i64).await {
        Ok(Some(row)) => Json(PlayerProfileResponse {
            id: row.id as u64,
            username: row.username,
            health: row.health,
            max_health: row.max_health,
            floor_id: row.floor_id as u32,
            position: [row.pos_x, row.pos_y, row.pos_z],
            base_stats: BaseStats {
                str_stat: row.base_str as u32,
                dex_stat: row.base_dex as u32,
                int_stat: row.base_int as u32,
                vit_stat: row.base_vit as u32,
                luk_stat: row.base_luk as u32,
            },
            is_alive: row.is_alive,
        }),
        _ => Json(PlayerProfileResponse {
            id: req.player_id,
            username: String::new(),
            health: 0.0,
            max_health: 0.0,
            floor_id: 0,
            position: [0.0, 0.0, 0.0],
            base_stats: BaseStats {
                str_stat: 0,
                dex_stat: 0,
                int_stat: 0,
                vit_stat: 0,
                luk_stat: 0,
            },
            is_alive: false,
        }),
    }
}

// ============================================================================
// Live ECS Endpoints (read from Bevy world snapshot / command channel)
// ============================================================================

#[derive(Serialize)]
pub struct LiveStatusResponse {
    pub server_tick: u64,
    pub uptime_secs: f64,
    pub player_count: usize,
    pub entity_count: usize,
    pub players: Vec<LivePlayerInfo>,
    pub destruction_stats: std::collections::HashMap<u32, DestructionFloorStats>,
    pub world_cycle: WorldCycleState,
}

#[derive(Serialize)]
pub struct LivePlayerInfo {
    pub id: u64,
    pub position: [f32; 3],
    pub health: f32,
    pub floor: u32,
    pub in_combat: bool,
}

#[derive(Serialize)]
pub struct DestructionFloorStats {
    pub total: u32,
    pub destroyed: u32,
    pub percentage: f32,
}

async fn get_live_status(
    State(state): State<ApiState>,
    Json(_req): Json<serde_json::Value>,
) -> Json<LiveStatusResponse> {
    let snap = state.world_snapshot.read()
        .map(|s| s.clone())
        .unwrap_or_default();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let players: Vec<LivePlayerInfo> = snap.players.values().map(|p| LivePlayerInfo {
        id: p.id,
        position: p.position,
        health: p.health,
        floor: p.current_floor,
        in_combat: p.in_combat,
    }).collect();

    let destruction_stats = snap.destruction_stats.iter().map(|(&floor, &(total, destroyed, pct))| {
        (floor, DestructionFloorStats { total, destroyed, percentage: pct })
    }).collect();

    Json(LiveStatusResponse {
        server_tick: snap.tick,
        uptime_secs: snap.uptime_secs,
        player_count: snap.players.len(),
        entity_count: snap.entity_count,
        players,
        destruction_stats,
        world_cycle: compute_world_cycle(now),
    })
}

#[derive(Deserialize)]
pub struct LivePlayerRequest {
    pub player_id: u64,
}

#[derive(Serialize)]
pub struct LivePlayerResponse {
    pub found: bool,
    pub player: Option<LivePlayerInfo>,
}

async fn get_live_player(
    State(state): State<ApiState>,
    Json(req): Json<LivePlayerRequest>,
) -> Json<LivePlayerResponse> {
    // Send a command to Bevy ECS and await the response
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    let _ = state.ecs_commands.send(GameCommand::GetPlayer {
        player_id: req.player_id,
        reply: reply_tx,
    });

    match reply_rx.await {
        Ok(Some(snap)) => Json(LivePlayerResponse {
            found: true,
            player: Some(LivePlayerInfo {
                id: snap.id,
                position: snap.position,
                health: snap.health,
                floor: snap.current_floor,
                in_combat: snap.in_combat,
            }),
        }),
        _ => Json(LivePlayerResponse {
            found: false,
            player: None,
        }),
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Breath of the Tower cycle (4-phase, 24h period)
fn compute_world_cycle(timestamp_secs: u64) -> WorldCycleState {
    let cycle_duration = 86400u64; // 24 hours
    let phase_duration = cycle_duration / 4;

    let cycle_position = timestamp_secs % cycle_duration;
    let phase = (cycle_position / phase_duration) as u32;
    let phase_progress = (cycle_position % phase_duration) as f32 / phase_duration as f32;

    let (cycle_name, corruption) = match phase {
        0 => ("Dawn Breath", 0.1),
        1 => ("Noon Surge", 0.3),
        2 => ("Dusk Whisper", 0.5),
        3 => ("Midnight Pulse", 0.8),
        _ => ("Unknown", 0.0),
    };

    let mut events = Vec::new();
    if corruption > 0.5 {
        events.push("corruption_surge".to_string());
    }
    if phase == 3 {
        events.push("shadow_realm_active".to_string());
    }

    WorldCycleState {
        cycle_name: cycle_name.to_string(),
        phase,
        phase_progress,
        corruption_level: corruption,
        active_events: events,
    }
}
