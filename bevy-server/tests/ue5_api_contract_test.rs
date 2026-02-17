//! UE5 API Contract Tests
//!
//! Validates that server JSON responses contain all fields that the UE5
//! GRPCClientManager.cpp expects to parse. These tests prevent protocol
//! drift between the Bevy server and the UE5 client.
//!
//! Each test mirrors the parsing logic in GRPCClientManager.cpp:
//! - GenerateFloor (line 496): broadcasts raw JSON
//! - CalculateDamage (lines 514-534): reads damage fields + modifiers
//! - TrackProgress (lines 554-567): reads mastery fields
//! - GetWallet (lines 587-589): reads gold, premium_currency, seasonal_currency
//! - GenerateLoot (lines 622-634): reads item_name, rarity, socket_count, tags
//!
//! Requires: `docker compose up -d postgres` (PostgreSQL on port 5433)

use std::sync::Arc;
use tower_bevy_server::storage::lmdb_templates::LmdbTemplateStore;
use tower_bevy_server::storage::seed_data;
use tower_bevy_server::ecs_bridge;
use tower_bevy_server::api;
use tower_bevy_server::metrics::ServerMetrics;
use serde_json::Value;
use axum::body::Body;
use http::Request;
use tower::ServiceExt;

/// Helper: create test router (same as api_smoke_test)
async fn create_test_router() -> (axum::Router, tempfile::TempDir) {
    let tmp = tempfile::tempdir().expect("Failed to create temp dir");
    let lmdb_path = tmp.path().join("templates");
    std::fs::create_dir_all(&lmdb_path).unwrap();

    let lmdb_size = 50 * 1024 * 1024;
    let lmdb = Arc::new(
        LmdbTemplateStore::new(lmdb_path.to_str().unwrap(), lmdb_size)
            .expect("Failed to init LMDB"),
    );
    seed_data::seed_all(&lmdb).expect("Failed to seed data");

    let (cmd_sender, _cmd_receiver, world_snapshot) = ecs_bridge::create_bridge();

    let pg = tower_bevy_server::storage::postgres::PostgresStore::new(
        "postgres://postgres:localdb@localhost:5433/tower_game",
        2,
    )
    .await
    .expect("PostgreSQL not available at localhost:5433");

    let state = api::ApiState {
        lmdb,
        pg: Arc::new(pg),
        ecs_commands: cmd_sender,
        world_snapshot,
        metrics: ServerMetrics::new(),
    };

    let router = api::build_router(state);
    (router, tmp)
}

async fn post_json(router: axum::Router, path: &str, body: &str) -> Value {
    let req = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        200,
        "Endpoint {} returned {}",
        path,
        resp.status()
    );

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

// ============================================================================
// Contract: GenerateFloor
// UE5 broadcasts raw JSON (line 496) — we validate the shape
// ============================================================================

#[tokio::test]
async fn contract_generate_floor_has_expected_fields() {
    let (router, _tmp) = create_test_router().await;

    let json = post_json(
        router,
        "/tower.GenerationService/GenerateFloor",
        r#"{"tower_seed": 42, "floor_id": 1}"#,
    )
    .await;

    // UE5 ProceduralFloorRenderer expects these fields
    assert!(json["floor_id"].is_number(), "Missing floor_id");
    assert!(json["seed"].is_number(), "Missing seed");
    assert!(json["width"].is_number(), "Missing width");
    assert!(json["height"].is_number(), "Missing height");
    assert!(json["tiles"].is_array(), "Missing tiles array");
    assert!(json["semantic_tags"].is_array(), "Missing semantic_tags");

    // Validate tile structure
    let tile = &json["tiles"][0];
    assert!(tile["tile_type"].is_number(), "Tile missing tile_type");
    assert!(tile["grid_x"].is_number(), "Tile missing grid_x");
    assert!(tile["grid_y"].is_number(), "Tile missing grid_y");
    assert!(tile["is_walkable"].is_boolean(), "Tile missing is_walkable");
}

// ============================================================================
// Contract: CalculateDamage
// UE5 (lines 514-534): reads base_damage, modified_damage, crit_chance,
//   crit_damage, modifiers[].source/multiplier/description
// ============================================================================

#[tokio::test]
async fn contract_calculate_damage_has_ue5_fields() {
    let (router, _tmp) = create_test_router().await;

    let json = post_json(
        router,
        "/tower.CombatService/CalculateDamage",
        r#"{
            "attacker_id": 1,
            "defender_id": 2,
            "weapon_id": "starter_sword",
            "ability_id": "basic_attack"
        }"#,
    )
    .await;

    // Fields that UE5 GRPCClientManager reads (lines 514-534)
    assert!(json["base_damage"].is_number(), "Missing base_damage");
    assert!(
        json["modified_damage"].is_number(),
        "Missing modified_damage"
    );
    assert!(json["crit_chance"].is_number(), "Missing crit_chance");
    assert!(json["crit_damage"].is_number(), "Missing crit_damage");
    assert!(json["modifiers"].is_array(), "Missing modifiers array");

    // Validate modifier structure
    if let Some(modifiers) = json["modifiers"].as_array() {
        for m in modifiers {
            assert!(m["source"].is_string(), "Modifier missing source");
            assert!(m["multiplier"].is_number(), "Modifier missing multiplier");
            assert!(
                m["description"].is_string(),
                "Modifier missing description"
            );
        }
    }
}

// ============================================================================
// Contract: TrackProgress (MasteryService)
// UE5 (lines 554-567): reads domain, new_tier, new_xp, xp_to_next,
//   tier_up, newly_unlocked
// ============================================================================

#[tokio::test]
async fn contract_track_progress_has_ue5_fields() {
    let (router, _tmp) = create_test_router().await;

    let json = post_json(
        router,
        "/tower.MasteryService/TrackProgress",
        r#"{
            "player_id": 1,
            "domain": "sword",
            "action_type": "attack",
            "xp_amount": 10.0
        }"#,
    )
    .await;

    // Fields that UE5 GRPCClientManager reads (lines 554-567)
    assert!(json["domain"].is_string(), "Missing domain");
    assert!(json["new_tier"].is_number(), "Missing new_tier");
    assert!(json["new_xp"].is_number(), "Missing new_xp");
    assert!(json["xp_to_next"].is_number(), "Missing xp_to_next");
    assert!(json["tier_up"].is_boolean(), "Missing tier_up");
    assert!(
        json["newly_unlocked"].is_array(),
        "Missing newly_unlocked array"
    );
}

// ============================================================================
// Contract: GetWallet
// UE5 (lines 587-589): reads gold, premium_currency, seasonal_currency
// Previously: server sent honor_points, UE5 expected seasonal_currency
// ============================================================================

#[tokio::test]
async fn contract_get_wallet_has_ue5_fields() {
    let (router, _tmp) = create_test_router().await;

    let json = post_json(
        router,
        "/tower.EconomyService/GetWallet",
        r#"{"player_id": 1}"#,
    )
    .await;

    // Fields that UE5 GRPCClientManager reads (lines 587-589)
    assert!(json["gold"].is_number(), "Missing gold");
    assert!(
        json["premium_currency"].is_number(),
        "Missing premium_currency"
    );
    assert!(
        json["seasonal_currency"].is_number(),
        "Missing seasonal_currency (was honor_points)"
    );

    // Server also returns honor_points for non-UE5 clients
    assert!(
        json["honor_points"].is_number(),
        "Missing honor_points (legacy field)"
    );
}

// ============================================================================
// Contract: GenerateLoot
// UE5 (lines 622-634): reads items[].item_name, rarity, socket_count, tags[]
// Previously: server sent item_template_id/quantity, missing item_name/socket_count/tags
// ============================================================================

#[tokio::test]
async fn contract_generate_loot_has_ue5_fields() {
    let (router, _tmp) = create_test_router().await;

    let json = post_json(
        router,
        "/tower.GenerationService/GenerateLoot",
        r#"{
            "source_entity_id": 100,
            "player_id": 1,
            "source_tags": [{"tag": "fire", "weight": 0.7}],
            "luck_modifier": 0.0
        }"#,
    )
    .await;

    assert!(json["items"].is_array(), "Missing items array");

    let items = json["items"].as_array().unwrap();
    assert!(!items.is_empty(), "Loot should generate at least one item");

    for item in items {
        // Fields that UE5 GRPCClientManager reads (lines 622-634)
        assert!(
            item["item_name"].is_string(),
            "Loot item missing item_name"
        );
        assert!(item["rarity"].is_number(), "Loot item missing rarity");
        assert!(
            item["socket_count"].is_number(),
            "Loot item missing socket_count"
        );
        assert!(item["tags"].is_array(), "Loot item missing tags array");

        // Server also returns these for internal use
        assert!(
            item["item_template_id"].is_string(),
            "Loot item missing item_template_id"
        );
        assert!(item["quantity"].is_number(), "Loot item missing quantity");
    }
}

// ============================================================================
// Contract: Metrics endpoints (for load test consumption)
// ============================================================================

#[tokio::test]
async fn contract_metrics_json_has_expected_fields() {
    let (router, _tmp) = create_test_router().await;

    let req = Request::builder()
        .method("GET")
        .uri("/metrics/json")
        .body(Body::empty())
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["uptime_secs"].is_number(), "Missing uptime_secs");
    assert!(json["player_count"].is_number(), "Missing player_count");
    assert!(json["entity_count"].is_number(), "Missing entity_count");
    assert!(json["tick"].is_number(), "Missing tick");
    assert!(
        json["total_requests"].is_number(),
        "Missing total_requests"
    );
    assert!(json["rps"].is_number(), "Missing rps");
}
