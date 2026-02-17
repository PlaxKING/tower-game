//! MasteryService — Skill mastery and progression endpoints
//!
//! Endpoints:
//! - POST /tower.MasteryService/TrackProgress
//! - POST /tower.MasteryService/GetMasteryProfile
//! - POST /tower.MasteryService/ChooseSpecialization
//! - POST /tower.MasteryService/UpdateAbilityLoadout

use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

use super::ApiState;
use crate::proto::tower::entities::AbilityTemplate;

pub fn routes() -> Router<ApiState> {
    Router::new()
        .route("/tower.MasteryService/TrackProgress", post(track_progress))
        .route(
            "/tower.MasteryService/GetMasteryProfile",
            post(get_mastery_profile),
        )
        .route(
            "/tower.MasteryService/ChooseSpecialization",
            post(choose_specialization),
        )
        .route(
            "/tower.MasteryService/UpdateAbilityLoadout",
            post(update_ability_loadout),
        )
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Deserialize)]
pub struct MasteryProgressRequest {
    pub player_id: u64,
    pub domain: String,
    pub action_type: String,
    pub xp_amount: f32,
}

#[derive(Serialize)]
pub struct MasteryProgressResponse {
    pub domain: String,
    pub new_tier: u32,
    pub new_xp: f64,
    pub xp_to_next: f64,
    pub tier_up: bool,
    pub newly_unlocked: Vec<String>,
}

#[derive(Deserialize)]
pub struct MasteryProfileRequest {
    pub player_id: u64,
}

#[derive(Serialize)]
pub struct MasteryProfileResponse {
    pub domains: Vec<DomainProfile>,
    pub primary_combat_role: String,
}

#[derive(Serialize)]
pub struct DomainProfile {
    pub domain_name: String,
    pub tier: u32,
    pub xp_current: f64,
    pub xp_required: f64,
    pub specialization: String,
}

#[derive(Deserialize)]
pub struct ChooseSpecRequest {
    pub player_id: u64,
    pub domain: String,
    pub branch_id: String,
}

#[derive(Serialize)]
pub struct ChooseSpecResponse {
    pub success: bool,
    pub failure_reason: String,
    pub combat_role: String,
}

#[derive(Deserialize)]
pub struct AbilityLoadoutRequest {
    pub player_id: u64,
    pub slots: Vec<AbilitySlot>,
}

#[derive(Deserialize)]
pub struct AbilitySlot {
    pub slot_index: u32,
    pub ability_id: String,
}

#[derive(Serialize)]
pub struct AbilityLoadoutResponse {
    pub success: bool,
    pub validation_errors: Vec<String>,
}

// ============================================================================
// Handlers
// ============================================================================

/// XP tier thresholds
const TIER_THRESHOLDS: [u64; 6] = [0, 5_000, 25_000, 100_000, 500_000, 1_000_000];

#[allow(dead_code)]
pub fn tier_name(tier: u32) -> &'static str {
    match tier {
        0 => "Novice",
        1 => "Apprentice",
        2 => "Journeyman",
        3 => "Expert",
        4 => "Master",
        5 => "Grandmaster",
        _ => "Unknown",
    }
}

async fn track_progress(
    State(state): State<ApiState>,
    Json(req): Json<MasteryProgressRequest>,
) -> Json<MasteryProgressResponse> {
    let xp = req.xp_amount.max(0.0) as i64;

    match state
        .pg
        .add_mastery_experience(req.player_id as i64, &req.domain, xp)
        .await
    {
        Ok(row) => {
            let new_tier = row.tier as u32;
            let xp_to_next = if (new_tier as usize) < TIER_THRESHOLDS.len() - 1 {
                TIER_THRESHOLDS[new_tier as usize + 1] as f64 - row.experience as f64
            } else {
                0.0
            };

            // Check if we crossed a tier boundary
            let old_xp = row.experience - xp;
            let old_tier = TIER_THRESHOLDS
                .iter()
                .rposition(|&t| old_xp as u64 >= t)
                .unwrap_or(0);
            let tier_up = new_tier as usize > old_tier;

            // Unlock abilities for new tier
            let newly_unlocked = if tier_up {
                get_unlocked_abilities(&req.domain, new_tier, &state)
            } else {
                vec![]
            };

            Json(MasteryProgressResponse {
                domain: req.domain,
                new_tier,
                new_xp: row.experience as f64,
                xp_to_next,
                tier_up,
                newly_unlocked,
            })
        }
        Err(e) => {
            tracing::error!("Failed to add mastery XP: {}", e);
            Json(MasteryProgressResponse {
                domain: req.domain,
                new_tier: 0,
                new_xp: 0.0,
                xp_to_next: 5000.0,
                tier_up: false,
                newly_unlocked: vec![],
            })
        }
    }
}

async fn get_mastery_profile(
    State(state): State<ApiState>,
    Json(req): Json<MasteryProfileRequest>,
) -> Json<MasteryProfileResponse> {
    let rows = state
        .pg
        .get_all_mastery(req.player_id as i64)
        .await
        .unwrap_or_default();

    let domains: Vec<DomainProfile> = rows
        .iter()
        .map(|r| {
            let tier = r.tier as u32;
            let xp_required = if (tier as usize) < TIER_THRESHOLDS.len() - 1 {
                TIER_THRESHOLDS[tier as usize + 1] as f64
            } else {
                TIER_THRESHOLDS.last().copied().unwrap_or(0) as f64
            };

            DomainProfile {
                domain_name: r.domain.clone(),
                tier,
                xp_current: r.experience as f64,
                xp_required,
                specialization: r.specialization.clone().unwrap_or_default(),
            }
        })
        .collect();

    // Determine combat role from highest-tier specialization
    let primary_combat_role = domains
        .iter()
        .filter(|d| !d.specialization.is_empty())
        .max_by_key(|d| d.tier)
        .map(|d| d.specialization.clone())
        .unwrap_or_else(|| "None".to_string());

    Json(MasteryProfileResponse {
        domains,
        primary_combat_role,
    })
}

async fn choose_specialization(
    State(state): State<ApiState>,
    Json(req): Json<ChooseSpecRequest>,
) -> Json<ChooseSpecResponse> {
    match state
        .pg
        .set_mastery_specialization(req.player_id as i64, &req.domain, &req.branch_id)
        .await
    {
        Ok(()) => Json(ChooseSpecResponse {
            success: true,
            failure_reason: String::new(),
            combat_role: req.branch_id.clone(),
        }),
        Err(e) => Json(ChooseSpecResponse {
            success: false,
            failure_reason: e.to_string(),
            combat_role: String::new(),
        }),
    }
}

async fn update_ability_loadout(
    State(_state): State<ApiState>,
    Json(req): Json<AbilityLoadoutRequest>,
) -> Json<AbilityLoadoutResponse> {
    let mut errors = Vec::new();

    // Validate slot indices (0-9)
    for slot in &req.slots {
        if slot.slot_index >= 10 {
            errors.push(format!("Invalid slot index: {}", slot.slot_index));
        }
    }

    // Check for duplicate slot assignments
    let mut seen_slots = std::collections::HashSet::new();
    for slot in &req.slots {
        if !seen_slots.insert(slot.slot_index) {
            errors.push(format!("Duplicate slot index: {}", slot.slot_index));
        }
    }

    Json(AbilityLoadoutResponse {
        success: errors.is_empty(),
        validation_errors: errors,
    })
}

// ============================================================================
// Helpers
// ============================================================================

fn get_unlocked_abilities(domain: &str, tier: u32, state: &ApiState) -> Vec<String> {
    // Query ability templates from LMDB
    let abilities: Vec<AbilityTemplate> =
        state.lmdb.get_all(state.lmdb.abilities).unwrap_or_default();

    abilities
        .iter()
        .filter(|a| a.required_mastery_domain == domain && a.required_mastery_tier <= tier as i32)
        .map(|a| a.id.clone())
        .collect()
}
