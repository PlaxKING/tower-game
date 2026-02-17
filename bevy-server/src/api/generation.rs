//! GenerationService — Procedural content generation endpoints
//!
//! Endpoints:
//! - POST /tower.GenerationService/GenerateFloor
//! - POST /tower.GenerationService/GenerateLoot
//! - POST /tower.GenerationService/SpawnMonsters
//! - POST /tower.GenerationService/QuerySemanticTags

use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

use super::ApiState;
use crate::destruction::FloorDestructionManager;
use crate::proto::tower::entities::{LootTable, MonsterTemplate};
use crate::proto::tower::game::TagPair as ProtoTagPair;

pub fn routes() -> Router<ApiState> {
    Router::new()
        .route(
            "/tower.GenerationService/GenerateFloor",
            post(generate_floor),
        )
        .route("/tower.GenerationService/GenerateLoot", post(generate_loot))
        .route(
            "/tower.GenerationService/SpawnMonsters",
            post(spawn_monsters),
        )
        .route(
            "/tower.GenerationService/QuerySemanticTags",
            post(query_semantic_tags),
        )
        .route(
            "/tower.GenerationService/GenerateDestructibles",
            post(generate_destructibles),
        )
        .route(
            "/tower.GenerationService/GenerateMonsters",
            post(generate_monsters),
        )
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Deserialize)]
pub struct FloorRequest {
    pub tower_seed: u64,
    pub floor_id: u32,
}

#[derive(Serialize)]
pub struct FloorResponse {
    pub floor_id: u32,
    pub seed: u64,
    pub biome_id: u32,
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<TileData>,
    pub semantic_tags: Vec<TagPair>,
}

#[derive(Serialize)]
pub struct TileData {
    pub tile_type: u32,
    pub grid_x: i32,
    pub grid_y: i32,
    pub biome_id: u32,
    pub is_walkable: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TagPair {
    pub tag: String,
    pub weight: f32,
}

#[derive(Deserialize)]
pub struct LootRequest {
    pub source_entity_id: u64,
    pub player_id: u64,
    pub source_tags: Vec<TagPair>,
    pub luck_modifier: f32,
}

#[derive(Serialize)]
pub struct LootResponse {
    pub items: Vec<LootItem>,
}

#[derive(Serialize)]
pub struct LootItem {
    pub item_template_id: String,
    /// Alias for item_template_id — UE5 GRPCClientManager reads this field
    pub item_name: String,
    pub quantity: u32,
    pub rarity: i32,
    /// Socket slots on this item (UE5 reads this field)
    pub socket_count: u32,
    /// Semantic tags for this item (UE5 reads this field)
    pub tags: Vec<TagPair>,
}

#[derive(Deserialize)]
pub struct SpawnMonstersRequest {
    pub tower_seed: u64,
    pub floor_id: u32,
    pub room_id: u32,
    pub biome_tags: Vec<TagPair>,
}

#[derive(Serialize)]
pub struct SpawnMonstersResponse {
    pub monsters: Vec<MonsterSpawn>,
}

#[derive(Serialize)]
pub struct MonsterSpawn {
    pub template_id: String,
    pub position: [f32; 3],
    pub health: f32,
    pub tier: u32,
}

#[derive(Deserialize)]
pub struct SemanticQueryRequest {
    pub query_tags: Vec<TagPair>,
    pub similarity_threshold: f32,
    pub max_results: u32,
}

#[derive(Serialize)]
pub struct SemanticQueryResponse {
    pub matches: Vec<SemanticMatch>,
}

#[derive(Serialize)]
pub struct SemanticMatch {
    pub entity_id: String,
    pub entity_type: String,
    pub similarity: f32,
    pub tags: Vec<TagPair>,
}

// ============================================================================
// Handlers
// ============================================================================

async fn generate_floor(
    State(_state): State<ApiState>,
    Json(req): Json<FloorRequest>,
) -> Json<FloorResponse> {
    let seed = req.tower_seed.wrapping_add(req.floor_id as u64);
    let size = 50u32;

    // Generate tiles procedurally from seed
    let mut tiles = Vec::with_capacity((size * size) as usize);
    let mut rng = seed;
    for y in 0..size as i32 {
        for x in 0..size as i32 {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let tile_type = ((rng >> 33) % 5) as u32;
            tiles.push(TileData {
                tile_type,
                grid_x: x,
                grid_y: y,
                biome_id: determine_biome(req.floor_id),
                is_walkable: tile_type != 4, // type 4 = wall
            });
        }
    }

    let biome_id = determine_biome(req.floor_id);
    let tags = generate_floor_tags(req.floor_id, biome_id, seed);

    Json(FloorResponse {
        floor_id: req.floor_id,
        seed,
        biome_id,
        width: size,
        height: size,
        tiles,
        semantic_tags: tags,
    })
}

async fn generate_loot(
    State(state): State<ApiState>,
    Json(req): Json<LootRequest>,
) -> Json<LootResponse> {
    // Look up loot tables from LMDB
    let all_tables: Vec<LootTable> = state
        .lmdb
        .get_all(state.lmdb.loot_tables)
        .unwrap_or_default();

    let mut items = Vec::new();
    let mut rng = req
        .source_entity_id
        .wrapping_mul(req.player_id.wrapping_add(1));

    for table in &all_tables {
        for entry in &table.entries {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let roll = ((rng >> 33) as f32) / (u32::MAX as f32);
            let adjusted_chance = entry.drop_chance * (1.0 + req.luck_modifier * 0.1);

            if roll < adjusted_chance {
                let template_id = entry.item_template_id.clone();
                // Derive socket count from rarity (higher rarity = more sockets)
                rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                let rarity = ((rng >> 33) % 5) as i32; // 0-4: Common to Legendary
                let socket_count = match rarity {
                    0 | 1 => 0,
                    2 => ((rng >> 40) % 2) as u32,
                    3 => 1 + ((rng >> 44) % 2) as u32,
                    _ => 2 + ((rng >> 48) % 2) as u32,
                };

                items.push(LootItem {
                    item_name: template_id.clone(),
                    item_template_id: template_id,
                    quantity: entry.min_quantity.max(1),
                    rarity,
                    socket_count,
                    tags: req.source_tags.clone(),
                });
            }
        }
        if !items.is_empty() {
            break;
        } // Use first matching table
    }

    Json(LootResponse { items })
}

async fn spawn_monsters(
    State(state): State<ApiState>,
    Json(req): Json<SpawnMonstersRequest>,
) -> Json<SpawnMonstersResponse> {
    // Get monster templates from LMDB
    let all_monsters: Vec<MonsterTemplate> =
        state.lmdb.get_all(state.lmdb.monsters).unwrap_or_default();

    // Filter monsters by tier based on floor
    let tier = (req.floor_id / 10).min(5);
    let candidates: Vec<&MonsterTemplate> =
        all_monsters.iter().filter(|m| m.tier <= tier + 1).collect();

    let mut spawns = Vec::new();
    let mut rng = req
        .tower_seed
        .wrapping_add(req.floor_id as u64 * 1000 + req.room_id as u64);
    let spawn_count = 3 + (rng % 5) as usize;

    for _i in 0..spawn_count.min(candidates.len()) {
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let idx = (rng as usize) % candidates.len();
        let monster = &candidates[idx];

        let x = ((rng >> 16) % 20) as f32 - 10.0;
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let z = ((rng >> 16) % 20) as f32 - 10.0;

        spawns.push(MonsterSpawn {
            template_id: monster.id.clone(),
            position: [x, 0.0, z],
            health: monster.base_health,
            tier: monster.tier,
        });
    }

    Json(SpawnMonstersResponse { monsters: spawns })
}

async fn query_semantic_tags(
    State(state): State<ApiState>,
    Json(req): Json<SemanticQueryRequest>,
) -> Json<SemanticQueryResponse> {
    // Search all monster templates for semantic similarity
    let all_monsters: Vec<MonsterTemplate> =
        state.lmdb.get_all(state.lmdb.monsters).unwrap_or_default();

    let mut matches = Vec::new();

    for monster in &all_monsters {
        let proto_tags: Vec<&ProtoTagPair> = monster
            .semantic_tags
            .as_ref()
            .map(|st| st.tags.iter().collect())
            .unwrap_or_default();
        let similarity = compute_similarity(&req.query_tags, &proto_tags);
        if similarity >= req.similarity_threshold {
            matches.push(SemanticMatch {
                entity_id: monster.id.clone(),
                entity_type: "monster".to_string(),
                similarity,
                tags: proto_tags
                    .iter()
                    .map(|t| TagPair {
                        tag: t.tag.clone(),
                        weight: t.weight,
                    })
                    .collect(),
            });
        }
    }

    // Sort by similarity descending
    matches.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    matches.truncate(req.max_results as usize);

    Json(SemanticQueryResponse { matches })
}

// ============================================================================
// Helpers
// ============================================================================

fn determine_biome(floor_id: u32) -> u32 {
    match floor_id % 5 {
        0 => 1, // Dungeon
        1 => 2, // Forest
        2 => 3, // Volcano
        3 => 4, // Ice
        4 => 5, // Corruption
        _ => 1,
    }
}

fn generate_floor_tags(floor_id: u32, biome_id: u32, _seed: u64) -> Vec<TagPair> {
    let mut tags = vec![TagPair {
        tag: format!("floor_{}", floor_id),
        weight: 1.0,
    }];

    match biome_id {
        1 => tags.push(TagPair {
            tag: "dungeon".into(),
            weight: 0.8,
        }),
        2 => tags.push(TagPair {
            tag: "nature".into(),
            weight: 0.9,
        }),
        3 => {
            tags.push(TagPair {
                tag: "fire".into(),
                weight: 0.9,
            });
            tags.push(TagPair {
                tag: "volcanic".into(),
                weight: 0.7,
            });
        }
        4 => tags.push(TagPair {
            tag: "ice".into(),
            weight: 0.9,
        }),
        5 => tags.push(TagPair {
            tag: "corruption".into(),
            weight: 0.8,
        }),
        _ => {}
    }

    // Corruption increases with floor depth
    let corruption = (floor_id as f32 / 100.0).min(1.0);
    if corruption > 0.1 {
        tags.push(TagPair {
            tag: "corruption".into(),
            weight: corruption,
        });
    }

    tags
}

fn compute_similarity(query: &[TagPair], entity_tags: &[&ProtoTagPair]) -> f32 {
    if query.is_empty() || entity_tags.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0f32;
    let mut mag_q = 0.0f32;
    let mut mag_e = 0.0f32;

    for q in query {
        mag_q += q.weight * q.weight;
        for e in entity_tags {
            if q.tag == e.tag {
                dot += q.weight * e.weight;
            }
        }
    }

    for e in entity_tags {
        mag_e += e.weight * e.weight;
    }

    let mag = (mag_q.sqrt()) * (mag_e.sqrt());
    if mag < 0.0001 {
        0.0
    } else {
        dot / mag
    }
}

// ============================================================================
// Destructible Generation
// ============================================================================

#[derive(Deserialize)]
pub struct GenerateDestructiblesRequest {
    pub tower_seed: u64,
    pub floor_id: u32,
    pub biome_tags: Vec<TagPair>,
}

#[derive(Serialize)]
pub struct GenerateDestructiblesResponse {
    pub destructibles: Vec<SpawnedDestructible>,
    pub total_count: usize,
}

#[derive(Serialize)]
pub struct SpawnedDestructible {
    pub entity_id: u64,
    pub template_id: String,
    pub position: [f32; 3],
    pub rotation_yaw: f32,
    pub material: String,
    pub fragment_count: u8,
    pub total_hp: f32,
    pub category: String,
    pub semantic_tags: Vec<TagPair>,
}

async fn generate_destructibles(
    State(_state): State<ApiState>,
    Json(req): Json<GenerateDestructiblesRequest>,
) -> Json<GenerateDestructiblesResponse> {
    let mut manager = FloorDestructionManager::new();
    let mut rng = req.tower_seed.wrapping_add(req.floor_id as u64 * 7919);

    // Determine biome-appropriate destructibles
    let biome_id = determine_biome(req.floor_id);
    let templates_for_biome = select_templates_for_biome(biome_id, &req.biome_tags);

    // Calculate spawn count based on floor (deeper = more complex)
    let base_count = 15;
    let depth_bonus = (req.floor_id as usize / 10).min(20);
    let total_spawn_count = base_count + depth_bonus;

    let mut spawned = Vec::new();

    for _ in 0..total_spawn_count {
        rng = rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let template_idx = (rng as usize) % templates_for_biome.len();
        let template_id = &templates_for_biome[template_idx];

        // Random position within floor bounds (50x50 grid)
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let x = ((rng >> 33) % 50) as f32 * 3.0 - 75.0;
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let z = ((rng >> 33) % 50) as f32 * 3.0 - 75.0;
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let yaw = ((rng >> 33) as f32 / u32::MAX as f32) * std::f32::consts::TAU;

        let pos = bevy::math::Vec3::new(x, 0.0, z);

        if let Some(entity_id) = manager.spawn(template_id, req.floor_id, pos) {
            let template = manager.templates.get(template_id.as_str()).unwrap();
            let d = manager
                .floors
                .get(&req.floor_id)
                .unwrap()
                .get(&entity_id)
                .unwrap();

            spawned.push(SpawnedDestructible {
                entity_id,
                template_id: template_id.clone(),
                position: [x, 0.0, z],
                rotation_yaw: yaw,
                material: format!("{:?}", template.material),
                fragment_count: template.fragment_count,
                total_hp: d.total_hp(),
                category: format!("{:?}", template.category),
                semantic_tags: template
                    .semantic_tags
                    .iter()
                    .map(|(t, w)| TagPair {
                        tag: t.clone(),
                        weight: *w,
                    })
                    .collect(),
            });
        }
    }

    let total_count = spawned.len();
    Json(GenerateDestructiblesResponse {
        destructibles: spawned,
        total_count,
    })
}

/// Select appropriate destructible templates based on biome
fn select_templates_for_biome(biome_id: u32, biome_tags: &[TagPair]) -> Vec<String> {
    let has_tag = |name: &str| biome_tags.iter().any(|t| t.tag == name);
    let mut templates = Vec::new();

    // Universal templates (appear everywhere)
    templates.push("crate_wooden".to_string());
    templates.push("barrel_metal".to_string());

    match biome_id {
        1 => {
            // Dungeon
            templates.push("wall_stone_3m".to_string());
            templates.push("pillar_stone".to_string());
            templates.push("crystal_cluster".to_string());
        }
        2 => {
            // Forest
            templates.push("tree_forest".to_string());
            templates.push("tree_forest".to_string()); // Higher weight
            templates.push("wall_wood_3m".to_string());
            templates.push("bridge_wood_section".to_string());
        }
        3 => {
            // Volcano
            templates.push("wall_stone_3m".to_string());
            templates.push("pillar_stone".to_string());
            templates.push("crystal_cluster".to_string());
        }
        4 => {
            // Ice
            templates.push("wall_stone_3m".to_string());
            templates.push("crystal_cluster".to_string());
            templates.push("bridge_wood_section".to_string());
        }
        5 => {
            // Corruption
            templates.push("tree_corrupted".to_string());
            templates.push("corruption_node".to_string());
            templates.push("crystal_cluster".to_string());
            templates.push("wall_stone_3m".to_string());
        }
        _ => {
            templates.push("wall_stone_3m".to_string());
        }
    }

    // Corruption floors always get corruption nodes
    if has_tag("corruption") {
        templates.push("corruption_node".to_string());
    }

    templates
}

// ============================================================================
// Grammar-based Monster Generation
// ============================================================================

#[derive(Deserialize)]
pub struct GenerateMonstersRequest {
    pub tower_seed: u64,
    pub floor_id: u32,
    pub room_id: u32,
    pub biome_tags: Vec<TagPair>,
    pub count: Option<usize>,
}

#[derive(Serialize)]
pub struct GenerateMonstersResponse {
    pub monsters: Vec<GeneratedMonster>,
    pub total_count: usize,
}

#[derive(Serialize)]
pub struct GeneratedMonster {
    pub variant_id: u64,
    pub name: String,
    pub size: String,
    pub element: String,
    pub corruption: String,
    pub body_type: String,
    pub max_health: f32,
    pub base_damage: f32,
    pub move_speed: f32,
    pub ai_behavior: String,
    pub position: [f32; 3],
    pub semantic_tags: Vec<TagPair>,
    pub loot_tier: u32,
}

async fn generate_monsters(
    State(_state): State<ApiState>,
    Json(req): Json<GenerateMonstersRequest>,
) -> Json<GenerateMonstersResponse> {
    let biome: Vec<(String, f32)> = req
        .biome_tags
        .iter()
        .map(|t| (t.tag.clone(), t.weight))
        .collect();

    let count = req.count.unwrap_or_else(|| {
        // Default: 3-8 based on floor depth
        3 + ((req.floor_id as usize / 5) % 6)
    });

    let blueprints = crate::monster_gen::generate_room_monsters(
        req.tower_seed,
        req.floor_id,
        req.room_id,
        &biome,
        count,
    );

    let mut rng = req
        .tower_seed
        .wrapping_mul(req.floor_id as u64 + 1)
        .wrapping_mul(req.room_id as u64 + 1);

    let monsters: Vec<GeneratedMonster> = blueprints
        .iter()
        .map(|bp| {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let x = ((rng >> 33) % 20) as f32 - 10.0;
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let z = ((rng >> 33) % 20) as f32 - 10.0;

            GeneratedMonster {
                variant_id: bp.variant_id,
                name: bp.name.clone(),
                size: format!("{:?}", bp.size),
                element: format!("{:?}", bp.element),
                corruption: format!("{:?}", bp.corruption),
                body_type: format!("{:?}", bp.body_type),
                max_health: bp.max_health,
                base_damage: bp.base_damage,
                move_speed: bp.move_speed,
                ai_behavior: format!("{:?}", bp.ai_behavior),
                position: [x, 0.0, z],
                semantic_tags: bp
                    .semantic_tags
                    .iter()
                    .map(|(tag, weight)| TagPair {
                        tag: tag.clone(),
                        weight: *weight,
                    })
                    .collect(),
                loot_tier: bp.loot_tier,
            }
        })
        .collect();

    let total_count = monsters.len();
    Json(GenerateMonstersResponse {
        monsters,
        total_count,
    })
}
