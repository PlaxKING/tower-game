//! HTTP/JSON API Layer
//!
//! Provides REST-like endpoints following gRPC path conventions.
//! The UE5 client calls these endpoints via JSON-over-HTTP transport.
//!
//! ## Architecture
//! ```text
//! UE5 Client (GRPCClientManager, JSON mode)
//!       ↓ HTTP POST, JSON body
//! Axum Router (port 50051)
//!       ↓
//! Service Handlers (generation, mastery, economy, combat, game_state)
//!       ↓
//! StorageManager (LMDB + PostgreSQL)
//! ```
//!
//! ## Endpoint Convention
//! All endpoints follow gRPC path pattern: `POST /tower.<Service>/<Method>`
//! Example: `POST /tower.GenerationService/GenerateFloor`

pub mod combat;
pub mod destruction;
pub mod economy;
pub mod game_state;
pub mod generation;
pub mod mastery;

use axum::{middleware, routing::get, Json, Router};
use serde::Serialize;
use std::sync::Arc;
use tracing::info;

use crate::ecs_bridge::{CommandSender, SharedWorldSnapshot};
use crate::metrics::ServerMetrics;
use crate::storage::lmdb_templates::LmdbTemplateStore;
use crate::storage::postgres::PostgresStore;

/// Shared state available to all API handlers
#[derive(Clone)]
pub struct ApiState {
    pub lmdb: Arc<LmdbTemplateStore>,
    pub pg: Arc<PostgresStore>,
    /// Channel to send commands to Bevy ECS (write operations)
    pub ecs_commands: CommandSender,
    /// Shared snapshot of live game world (read operations)
    pub world_snapshot: SharedWorldSnapshot,
    /// Server-wide metrics (lock-free atomics)
    pub metrics: Arc<ServerMetrics>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Build the full API router with all service endpoints
pub fn build_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(crate::metrics::prometheus_handler))
        .route("/metrics/json", get(crate::metrics::json_metrics_handler))
        .merge(generation::routes())
        .merge(mastery::routes())
        .merge(economy::routes())
        .merge(game_state::routes())
        .merge(combat::routes())
        .merge(destruction::routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::metrics::metrics_middleware,
        ))
        .with_state(state)
}

/// Start the HTTP API server on the given port
///
/// Runs alongside the Bevy app (spawned on tokio runtime).
pub async fn start_api_server(
    lmdb: Arc<LmdbTemplateStore>,
    pg: Arc<PostgresStore>,
    ecs_commands: CommandSender,
    world_snapshot: SharedWorldSnapshot,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let metrics = ServerMetrics::new();
    let state = ApiState {
        lmdb,
        pg,
        ecs_commands,
        world_snapshot,
        metrics,
    };
    let app = build_router(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("API server listening on {}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}
