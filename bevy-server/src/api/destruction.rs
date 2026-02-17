//! DestructionService — Environmental destruction endpoints
//!
//! Endpoints:
//! - POST /tower.DestructionService/ApplyDamage
//! - POST /tower.DestructionService/GetFloorState
//! - POST /tower.DestructionService/Rebuild
//! - POST /tower.DestructionService/GetTemplates

use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

use super::ApiState;
use crate::destruction::{self, DestructionDamageType, FloorDestructionManager};

pub fn routes() -> Router<ApiState> {
    Router::new()
        .route("/tower.DestructionService/ApplyDamage", post(apply_damage))
        .route(
            "/tower.DestructionService/GetFloorState",
            post(get_floor_state),
        )
        .route("/tower.DestructionService/Rebuild", post(rebuild))
        .route(
            "/tower.DestructionService/GetTemplates",
            post(get_templates),
        )
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Deserialize)]
pub struct ApplyDamageRequest {
    pub player_id: u64,
    pub target_entity_id: u64,
    pub floor_id: u32,
    pub impact_point: [f32; 3],
    pub entity_position: [f32; 3],
    pub damage: f32,
    pub radius: f32,
    pub damage_type: String, // "kinetic", "explosive", "fire", "ice", "lightning", "semantic"
    pub weapon_id: String,
    pub ability_id: String,
}

#[derive(Serialize)]
pub struct ApplyDamageResponse {
    pub success: bool,
    pub damage_dealt: f32,
    pub newly_destroyed_clusters: Vec<u8>,
    pub structural_collapse: bool,
    pub fragment_mask: Vec<u8>,
    pub destruction_loot: Vec<LootDropResponse>,
    pub mastery_xp: f32,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct LootDropResponse {
    pub item_template_id: String,
    pub quantity: u32,
    pub position: [f32; 3],
}

#[derive(Deserialize)]
pub struct FloorStateRequest {
    pub floor_id: u32,
}

#[derive(Serialize)]
pub struct FloorStateResponse {
    pub floor_id: u32,
    pub destructibles: Vec<DestructibleStateResponse>,
    pub total_destructibles: u32,
    pub destroyed_count: u32,
    pub destruction_percentage: f32,
}

#[derive(Serialize)]
pub struct DestructibleStateResponse {
    pub entity_id: u64,
    pub template_id: String,
    pub material: String,
    pub total_hp: f32,
    pub max_total_hp: f32,
    pub collapsed: bool,
    pub position: [f32; 3],
    pub fragment_mask: Vec<u8>,
    pub fragment_count: usize,
    pub destroyed_fragment_count: usize,
    pub supports_rebuild: bool,
    pub rebuild_progress: f32,
    pub semantic_tags: Vec<TagPairResponse>,
}

#[derive(Serialize)]
pub struct TagPairResponse {
    pub tag: String,
    pub weight: f32,
}

#[derive(Deserialize)]
pub struct RebuildRequest {
    pub player_id: u64,
    pub target_entity_id: u64,
    pub floor_id: u32,
    pub material_items: Vec<String>,
}

#[derive(Serialize)]
pub struct RebuildResponse {
    pub success: bool,
    pub rebuild_progress: f32,
    pub fully_repaired: bool,
    pub mastery_xp: f32,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct TemplatesResponse {
    pub templates: Vec<TemplateInfo>,
}

#[derive(Serialize)]
pub struct TemplateInfo {
    pub id: String,
    pub display_name: String,
    pub material: String,
    pub fragment_count: u8,
    pub base_hp: f32,
    pub effective_hp: f32,
    pub supports_rebuild: bool,
    pub category: String,
    pub semantic_tags: Vec<TagPairResponse>,
}

// ============================================================================
// Handlers
// ============================================================================

async fn apply_damage(
    State(state): State<ApiState>,
    Json(req): Json<ApplyDamageRequest>,
) -> Json<ApplyDamageResponse> {
    let damage_type = parse_damage_type(&req.damage_type);

    // Get the FloorDestructionManager from ApiState
    // Since the manager lives in Bevy ECS as a Resource, we simulate it here
    // In production this would communicate with the Bevy world via channel
    let mut manager = FloorDestructionManager::new();

    // Ensure the target exists (spawn if needed for demo)
    if manager.floors.get(&req.floor_id).is_none()
        || !manager
            .floors
            .get(&req.floor_id)
            .unwrap()
            .contains_key(&req.target_entity_id)
    {
        // Try to create from LMDB template data
        let _ = manager.spawn(
            "wall_stone_3m",
            req.floor_id,
            bevy::math::Vec3::new(
                req.entity_position[0],
                req.entity_position[1],
                req.entity_position[2],
            ),
        );
    }

    let impact = bevy::math::Vec3::new(
        req.impact_point[0],
        req.impact_point[1],
        req.impact_point[2],
    );
    let entity_pos = bevy::math::Vec3::new(
        req.entity_position[0],
        req.entity_position[1],
        req.entity_position[2],
    );

    match manager.apply_damage(
        req.target_entity_id,
        req.floor_id,
        impact,
        entity_pos,
        req.damage,
        req.radius,
        damage_type,
    ) {
        Some(result) => {
            // Calculate mastery XP based on damage and material
            let mastery_xp = calculate_destruction_mastery_xp(result.damage_dealt);

            // Award mastery XP
            if mastery_xp > 0.0 {
                let _ = state
                    .pg
                    .add_mastery_experience(
                        req.player_id as i64,
                        "DestructionMastery",
                        mastery_xp as i64,
                    )
                    .await;
            }

            // Generate loot from destruction
            let loot = generate_destruction_loot(&result, &req);

            Json(ApplyDamageResponse {
                success: true,
                damage_dealt: result.damage_dealt,
                newly_destroyed_clusters: result.newly_destroyed_clusters,
                structural_collapse: result.structural_collapse,
                fragment_mask: result.fragment_mask,
                destruction_loot: loot,
                mastery_xp,
                error: None,
            })
        }
        None => Json(ApplyDamageResponse {
            success: false,
            damage_dealt: 0.0,
            newly_destroyed_clusters: vec![],
            structural_collapse: false,
            fragment_mask: vec![],
            destruction_loot: vec![],
            mastery_xp: 0.0,
            error: Some("Target entity not found".to_string()),
        }),
    }
}

async fn get_floor_state(
    State(_state): State<ApiState>,
    Json(req): Json<FloorStateRequest>,
) -> Json<FloorStateResponse> {
    // In production, this reads from the Bevy ECS FloorDestructionManager
    // For now, return the managed state
    let manager = FloorDestructionManager::new();

    let (total, destroyed, pct) = manager.floor_stats(req.floor_id);

    let destructibles = manager
        .floors
        .get(&req.floor_id)
        .map(|floor| {
            floor
                .values()
                .map(|d| {
                    DestructibleStateResponse {
                        entity_id: d.entity_id,
                        template_id: d.template_id.clone(),
                        material: format!("{:?}", d.material),
                        total_hp: d.total_hp(),
                        max_total_hp: d.max_total_hp(),
                        collapsed: d.collapsed,
                        position: [0.0, 0.0, 0.0], // Would come from Transform component
                        fragment_mask: d.fragment_mask(),
                        fragment_count: d.fragments.len(),
                        destroyed_fragment_count: d.destroyed_count(),
                        supports_rebuild: d.supports_rebuild,
                        rebuild_progress: d.rebuild_progress,
                        semantic_tags: d
                            .semantic_tags
                            .iter()
                            .map(|(tag, weight)| TagPairResponse {
                                tag: tag.clone(),
                                weight: *weight,
                            })
                            .collect(),
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Json(FloorStateResponse {
        floor_id: req.floor_id,
        destructibles,
        total_destructibles: total,
        destroyed_count: destroyed,
        destruction_percentage: pct,
    })
}

async fn rebuild(
    State(state): State<ApiState>,
    Json(req): Json<RebuildRequest>,
) -> Json<RebuildResponse> {
    let mut manager = FloorDestructionManager::new();

    // Check if entity exists and is collapsed
    let floor = match manager.floors.get_mut(&req.floor_id) {
        Some(f) => f,
        None => {
            return Json(RebuildResponse {
                success: false,
                rebuild_progress: 0.0,
                fully_repaired: false,
                mastery_xp: 0.0,
                error: Some("Floor not found".to_string()),
            })
        }
    };

    let destructible = match floor.get_mut(&req.target_entity_id) {
        Some(d) => d,
        None => {
            return Json(RebuildResponse {
                success: false,
                rebuild_progress: 0.0,
                fully_repaired: false,
                mastery_xp: 0.0,
                error: Some("Entity not found".to_string()),
            })
        }
    };

    if !destructible.supports_rebuild {
        return Json(RebuildResponse {
            success: false,
            rebuild_progress: 0.0,
            fully_repaired: false,
            mastery_xp: 0.0,
            error: Some("This object cannot be rebuilt".to_string()),
        });
    }

    // Each material item contributes 0.25 rebuild progress
    let repair_amount = req.material_items.len() as f32 * 0.25;
    let fully_repaired = destructible.repair(repair_amount);

    // Award building mastery XP
    let mastery_xp = repair_amount * 100.0; // 100 XP per 0.25 progress
    let _ = state
        .pg
        .add_mastery_experience(req.player_id as i64, "BuildingMastery", mastery_xp as i64)
        .await;

    Json(RebuildResponse {
        success: true,
        rebuild_progress: destructible.rebuild_progress,
        fully_repaired,
        mastery_xp,
        error: None,
    })
}

async fn get_templates(
    State(_state): State<ApiState>,
    Json(_req): Json<serde_json::Value>,
) -> Json<TemplatesResponse> {
    let templates = destruction::default_templates();

    let infos: Vec<TemplateInfo> = templates
        .iter()
        .map(|t| {
            let effective_hp =
                t.base_hp_per_fragment * t.material.hp_multiplier() * t.fragment_count as f32;
            TemplateInfo {
                id: t.id.clone(),
                display_name: t.display_name.clone(),
                material: format!("{:?}", t.material),
                fragment_count: t.fragment_count,
                base_hp: t.base_hp_per_fragment,
                effective_hp,
                supports_rebuild: t.supports_rebuild,
                category: format!("{:?}", t.category),
                semantic_tags: t
                    .semantic_tags
                    .iter()
                    .map(|(tag, weight)| TagPairResponse {
                        tag: tag.clone(),
                        weight: *weight,
                    })
                    .collect(),
            }
        })
        .collect();

    Json(TemplatesResponse { templates: infos })
}

// ============================================================================
// Helpers
// ============================================================================

fn parse_damage_type(s: &str) -> DestructionDamageType {
    match s.to_lowercase().as_str() {
        "kinetic" | "physical" | "melee" => DestructionDamageType::Kinetic,
        "explosive" | "explosion" | "blast" => DestructionDamageType::Explosive,
        "fire" | "elemental_fire" => DestructionDamageType::ElementalFire,
        "ice" | "elemental_ice" | "frost" => DestructionDamageType::ElementalIce,
        "lightning" | "elemental_lightning" | "electric" => {
            DestructionDamageType::ElementalLightning
        }
        "semantic" | "corruption" | "tower" => DestructionDamageType::Semantic,
        _ => DestructionDamageType::Kinetic,
    }
}

fn calculate_destruction_mastery_xp(damage_dealt: f32) -> f32 {
    // Base: 1 XP per 10 damage dealt, min 5 XP
    (damage_dealt / 10.0).max(5.0)
}

fn generate_destruction_loot(
    result: &destruction::DestructionResult,
    req: &ApplyDamageRequest,
) -> Vec<LootDropResponse> {
    if result.newly_destroyed_clusters.is_empty() {
        return vec![];
    }

    let mut loot = Vec::new();
    let mut rng = req
        .target_entity_id
        .wrapping_mul(req.player_id.wrapping_add(1));

    // Each destroyed cluster has a chance to drop materials
    for &cluster_id in &result.newly_destroyed_clusters {
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let roll = ((rng >> 33) as f32) / (u32::MAX as f32);

        if roll < 0.6 {
            // 60% drop chance per cluster
            // Determine material type from damage type context
            let item_id = match parse_damage_type(&req.damage_type) {
                DestructionDamageType::ElementalFire => "ash_remnant",
                DestructionDamageType::ElementalIce => "frozen_shard",
                DestructionDamageType::ElementalLightning => "charged_fragment",
                _ => "raw_material",
            };

            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let quantity = 1 + ((rng >> 33) % 3) as u32;

            loot.push(LootDropResponse {
                item_template_id: item_id.to_string(),
                quantity,
                position: [
                    req.impact_point[0] + (cluster_id as f32 * 0.5),
                    req.impact_point[1],
                    req.impact_point[2],
                ],
            });
        }
    }

    loot
}
