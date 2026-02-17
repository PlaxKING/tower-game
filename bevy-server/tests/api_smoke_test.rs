//! API Smoke Tests
//!
//! Validates that the HTTP API server starts and responds correctly.
//! Tests endpoints that don't require PostgreSQL (health, generation).
//! Uses a temporary LMDB store, mock ECS bridge, and real PostgreSQL.
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

/// Helper: create a temporary LMDB + API router for testing.
/// Returns (router, temp_dir) — temp_dir must stay alive for the duration.
async fn create_test_router() -> (axum::Router, tempfile::TempDir) {
    let tmp = tempfile::tempdir().expect("Failed to create temp dir");
    let lmdb_path = tmp.path().join("templates");
    std::fs::create_dir_all(&lmdb_path).unwrap();

    // LMDB requires map size to be a multiple of OS page size (4096)
    let lmdb_size = 50 * 1024 * 1024; // 50MB, page-aligned
    let lmdb = Arc::new(
        LmdbTemplateStore::new(lmdb_path.to_str().unwrap(), lmdb_size)
            .expect("Failed to init LMDB"),
    );
    seed_data::seed_all(&lmdb).expect("Failed to seed data");

    let (cmd_sender, _cmd_receiver, world_snapshot) = ecs_bridge::create_bridge();

    // Connect to real PostgreSQL (Docker on port 5433)
    let pg = tower_bevy_server::storage::postgres::PostgresStore::new(
        "postgres://postgres:localdb@localhost:5433/tower_game",
        2,
    )
    .await
    .expect("PostgreSQL not available at localhost:5433 — run 'docker compose up -d postgres'");

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

// ============================================================================
// Health Endpoint
// ============================================================================

#[tokio::test]
async fn test_health_endpoint() {
    let (router, _tmp) = create_test_router().await;

    let req = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
    assert!(!json["version"].as_str().unwrap().is_empty());
}

// ============================================================================
// Generation Endpoints (LMDB-based, no PG needed at runtime)
// ============================================================================

#[tokio::test]
async fn test_generate_floor_endpoint() {
    let (router, _tmp) = create_test_router().await;

    let req = Request::builder()
        .method("POST")
        .uri("/tower.GenerationService/GenerateFloor")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"tower_seed": 12345, "floor_id": 1}"#))
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["floor_id"], 1);
    assert_eq!(json["width"], 50);
    assert_eq!(json["height"], 50);
    assert!(json["tiles"].as_array().unwrap().len() > 0);
    assert!(json["semantic_tags"].as_array().unwrap().len() > 0);

    // Validate tile structure
    let tile = &json["tiles"][0];
    assert!(tile["tile_type"].is_number());
    assert!(tile["grid_x"].is_number());
    assert!(tile["grid_y"].is_number());
    assert!(tile["is_walkable"].is_boolean());
}

#[tokio::test]
async fn test_generate_floor_deterministic() {
    // Same seed + floor_id should produce identical results
    let (router1, _tmp1) = create_test_router().await;
    let (router2, _tmp2) = create_test_router().await;

    let body_str = r#"{"tower_seed": 42, "floor_id": 7}"#;

    let req1 = Request::builder()
        .method("POST")
        .uri("/tower.GenerationService/GenerateFloor")
        .header("content-type", "application/json")
        .body(Body::from(body_str))
        .unwrap();

    let req2 = Request::builder()
        .method("POST")
        .uri("/tower.GenerationService/GenerateFloor")
        .header("content-type", "application/json")
        .body(Body::from(body_str))
        .unwrap();

    let resp1 = router1.oneshot(req1).await.unwrap();
    let resp2 = router2.oneshot(req2).await.unwrap();

    let body1 = axum::body::to_bytes(resp1.into_body(), usize::MAX).await.unwrap();
    let body2 = axum::body::to_bytes(resp2.into_body(), usize::MAX).await.unwrap();

    let json1: Value = serde_json::from_slice(&body1).unwrap();
    let json2: Value = serde_json::from_slice(&body2).unwrap();

    assert_eq!(json1["seed"], json2["seed"]);
    assert_eq!(json1["tiles"], json2["tiles"]);
}

#[tokio::test]
async fn test_generate_loot_endpoint() {
    let (router, _tmp) = create_test_router().await;

    let req = Request::builder()
        .method("POST")
        .uri("/tower.GenerationService/GenerateLoot")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"source_entity_id": 100, "player_id": 1, "source_tags": [{"tag": "fire", "weight": 0.7}], "luck_modifier": 0.0}"#,
        ))
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["items"].is_array());
}

#[tokio::test]
async fn test_world_cycle_endpoint() {
    let (router, _tmp) = create_test_router().await;

    let req = Request::builder()
        .method("POST")
        .uri("/tower.GameStateService/GetWorldCycle")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"tower_seed": 12345}"#))
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["cycle_name"].is_string());
    assert!(json["phase"].is_number());
    assert!(json["phase_progress"].is_number());
    assert!(json["corruption_level"].is_number());
}

#[tokio::test]
async fn test_spawn_monsters_endpoint() {
    let (router, _tmp) = create_test_router().await;

    let req = Request::builder()
        .method("POST")
        .uri("/tower.GenerationService/SpawnMonsters")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"tower_seed": 42, "floor_id": 3, "room_id": 1, "biome_tags": [{"tag": "forest", "weight": 0.8}]}"#,
        ))
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["monsters"].is_array());
    assert!(json["monsters"].as_array().unwrap().len() > 0);
}
