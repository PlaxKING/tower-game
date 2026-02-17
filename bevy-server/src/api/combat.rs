//! CombatService — Combat processing endpoints
//!
//! Endpoints:
//! - POST /tower.CombatService/CalculateDamage
//! - POST /tower.CombatService/GetCombatState
//! - POST /tower.CombatService/ProcessAction  (routes through Bevy ECS bridge)

use axum::{Router, Json, extract::State, routing::post};
use serde::{Deserialize, Serialize};

use super::ApiState;
use crate::combat::{self, ActionType};
use crate::ecs_bridge::GameCommand;

pub fn routes() -> Router<ApiState> {
    Router::new()
        .route("/tower.CombatService/CalculateDamage", post(calculate_damage))
        .route("/tower.CombatService/GetCombatState", post(get_combat_state))
        .route("/tower.CombatService/ProcessAction", post(process_action))
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Deserialize)]
pub struct DamageCalcRequest {
    pub attacker_id: u64,
    pub defender_id: u64,
    pub weapon_id: String,
    pub ability_id: String,
    pub attack_angle: Option<String>,
    pub combo_step: Option<u8>,
    pub semantic_affinity: Option<f32>,
}

#[derive(Serialize)]
pub struct DamageCalcResponse {
    pub base_damage: f32,
    pub modified_damage: f32,
    pub crit_chance: f32,
    pub crit_damage: f32,
    pub angle_mult: f32,
    pub combo_mult: f32,
    pub semantic_mult: f32,
    pub was_blocked: bool,
    pub was_parried: bool,
    pub modifiers: Vec<DamageModifier>,
}

#[derive(Serialize)]
pub struct DamageModifier {
    pub source: String,
    pub multiplier: f32,
    pub description: String,
}

#[derive(Deserialize)]
pub struct CombatStateRequest {
    pub entity_id: u64,
}

#[derive(Serialize)]
pub struct CombatStateResponse {
    pub entity_id: u64,
    pub current_phase: String,
    pub phase_timer: f32,
    pub combo_step: u32,
    pub combo_window: f32,
    pub poise: f32,
    pub max_poise: f32,
    pub i_frames: f32,
    pub parry_window: f32,
    pub can_parry: bool,
    pub can_dodge: bool,
    pub can_attack: bool,
    pub energy: Option<CombatEnergyResponse>,
    pub status_effects: Vec<StatusEffect>,
}

#[derive(Serialize)]
pub struct CombatEnergyResponse {
    pub kinetic: f32,
    pub kinetic_max: f32,
    pub thermal: f32,
    pub thermal_max: f32,
    pub semantic: f32,
    pub semantic_max: f32,
}

#[derive(Serialize)]
pub struct StatusEffect {
    pub effect_name: String,
    pub remaining_duration: f32,
    pub stacks: u32,
}

#[derive(Deserialize)]
pub struct ProcessActionRequest {
    pub player_id: u64,
    pub action_type: String,
    pub target_id: u64,
    pub ability_id: String,
    pub position: [f32; 3],
    pub facing: f32,
}

#[derive(Serialize)]
pub struct ActionResult {
    pub success: bool,
    pub action_type: String,
    pub new_phase: String,
    pub combo_step: u8,
    pub damage_dealt: f32,
    pub effects_applied: Vec<String>,
    pub mastery_xp: f32,
    pub mastery_domain: String,
    pub message: String,
}

// ============================================================================
// Handlers
// ============================================================================

async fn calculate_damage(
    State(state): State<ApiState>,
    Json(req): Json<DamageCalcRequest>,
) -> Json<DamageCalcResponse> {
    let mut modifiers = Vec::new();
    let mut base_damage = 10.0f32;

    // Look up weapon from item templates
    if !req.weapon_id.is_empty() {
        if let Ok(Some(weapon)) = state.lmdb.get_item(&req.weapon_id) {
            base_damage = weapon.base_damage;
            modifiers.push(DamageModifier {
                source: format!("weapon:{}", req.weapon_id),
                multiplier: 1.0,
                description: format!("Base weapon damage: {}", weapon.base_damage),
            });
        }
    }

    // Look up ability damage from effects
    if !req.ability_id.is_empty() {
        if let Ok(Some(ability)) = state.lmdb.get_ability(&req.ability_id) {
            let ability_damage: f32 = ability.effects.iter()
                .filter(|e| e.effect_type.starts_with("damage"))
                .map(|e| e.value)
                .sum();
            let ability_mult = if ability_damage > 0.0 { 1.0 + (ability_damage / base_damage).min(5.0) } else { 1.0 };
            base_damage *= ability_mult;
            modifiers.push(DamageModifier {
                source: format!("ability:{}", req.ability_id),
                multiplier: ability_mult,
                description: ability.name.clone(),
            });
        }
    }

    // Parse attack angle
    let angle = match req.attack_angle.as_deref() {
        Some("back") => combat::AttackAngle::Back,
        Some("side") => combat::AttackAngle::Side,
        _ => combat::AttackAngle::Front,
    };

    let combo_step = req.combo_step.unwrap_or(0);
    let semantic_affinity = req.semantic_affinity.unwrap_or(0.0);

    // Use combat module for calculation with default weapon moveset
    let movesets = combat::WeaponMovesets::default();
    let sword_attacks = &movesets.movesets[&combat::WeaponType::Sword];
    let step_idx = (combo_step as usize).min(sword_attacks.len() - 1);
    let attack_data = &sword_attacks[step_idx];

    let result = combat::calculate_damage(
        base_damage, attack_data, angle, combo_step,
        semantic_affinity, false, false,
    );

    modifiers.push(DamageModifier {
        source: "angle".into(),
        multiplier: result.angle_mult,
        description: format!("{:?} attack", result.angle),
    });
    modifiers.push(DamageModifier {
        source: "combo".into(),
        multiplier: result.combo_mult,
        description: format!("Combo step {}", combo_step),
    });

    Json(DamageCalcResponse {
        base_damage,
        modified_damage: result.final_damage,
        crit_chance: 0.05,
        crit_damage: 1.5,
        angle_mult: result.angle_mult,
        combo_mult: result.combo_mult,
        semantic_mult: result.semantic_mult,
        was_blocked: result.was_blocked,
        was_parried: result.was_parried,
        modifiers,
    })
}

async fn get_combat_state(
    State(state): State<ApiState>,
    Json(req): Json<CombatStateRequest>,
) -> Json<CombatStateResponse> {
    // Read from world snapshot for combat state
    let snap = state.world_snapshot.read()
        .map(|s| s.clone())
        .unwrap_or_default();

    // Check if player exists in snapshot
    let player = snap.players.get(&req.entity_id);
    let phase = if player.is_some() { "idle" } else { "unknown" };

    Json(CombatStateResponse {
        entity_id: req.entity_id,
        current_phase: phase.to_string(),
        phase_timer: 0.0,
        combo_step: 0,
        combo_window: 0.0,
        poise: 100.0,
        max_poise: 100.0,
        i_frames: 0.0,
        parry_window: 0.0,
        can_parry: matches!(phase, "idle" | "blocking"),
        can_dodge: matches!(phase, "idle" | "blocking"),
        can_attack: phase == "idle",
        energy: Some(CombatEnergyResponse {
            kinetic: 0.0,
            kinetic_max: 100.0,
            thermal: 0.0,
            thermal_max: 100.0,
            semantic: 0.0,
            semantic_max: 100.0,
        }),
        status_effects: vec![],
    })
}

async fn process_action(
    State(state): State<ApiState>,
    Json(req): Json<ProcessActionRequest>,
) -> Json<ActionResult> {
    // Parse action type
    let action = match req.action_type.as_str() {
        "attack" => ActionType::Attack,
        "heavy_attack" => ActionType::HeavyAttack,
        "block" => ActionType::Block,
        "block_release" => ActionType::BlockRelease,
        "parry" => ActionType::Parry,
        "dodge" => ActionType::Dodge,
        _ => {
            return Json(ActionResult {
                success: false,
                action_type: req.action_type,
                new_phase: "idle".into(),
                combo_step: 0,
                damage_dealt: 0.0,
                effects_applied: vec![],
                mastery_xp: 0.0,
                mastery_domain: String::new(),
                message: "Unknown action type".into(),
            });
        }
    };

    // Send combat action through ECS bridge
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    let _ = state.ecs_commands.send(GameCommand::CombatAction {
        player_id: req.player_id,
        action,
        position: req.position,
        facing: req.facing,
        reply: reply_tx,
    });

    match reply_rx.await {
        Ok(result) => {
            let mut effects = Vec::new();
            let mut damage = 0.0f32;
            let mut new_phase = "idle".to_string();
            let mut combo_step = 0u8;
            let mut mastery_xp = 0.0f32;
            let mut mastery_domain = String::new();

            if let Some(ref ar) = result.action_result {
                new_phase = format!("{:?}", ar.new_phase);
                combo_step = ar.combo_step;

                if ar.success {
                    effects.push(format!("{:?}", ar.action));

                    // Calculate mastery XP for the action
                    let outcome = combat::DamageOutcome::Hit; // Default for action request
                    let (domain, xp) = combat::mastery_xp_for_action(action, outcome);
                    mastery_domain = domain;
                    mastery_xp = xp;

                    // Award mastery XP
                    let _ = state.pg.add_mastery_experience(
                        req.player_id as i64,
                        &mastery_domain,
                        mastery_xp as i64,
                    ).await;

                    // If attack, look up weapon damage
                    if matches!(action, ActionType::Attack | ActionType::HeavyAttack) {
                        if let Ok(Some(item)) = state.lmdb.get_item(&req.ability_id) {
                            damage = item.base_damage;
                        } else {
                            damage = 15.0; // Default melee
                        }
                    }
                }
            }

            Json(ActionResult {
                success: result.success,
                action_type: req.action_type,
                new_phase,
                combo_step,
                damage_dealt: damage,
                effects_applied: effects,
                mastery_xp,
                mastery_domain,
                message: result.message,
            })
        }
        Err(_) => {
            // Bridge channel closed or timeout — fall back to direct calculation
            Json(ActionResult {
                success: false,
                action_type: req.action_type,
                new_phase: "idle".into(),
                combo_step: 0,
                damage_dealt: 0.0,
                effects_applied: vec![],
                mastery_xp: 0.0,
                mastery_domain: String::new(),
                message: "ECS bridge unavailable".into(),
            })
        }
    }
}
