//! Server Metrics — Lightweight request/game metrics with Prometheus + JSON export
//!
//! Uses lock-free atomics for all counters. No external metrics crate needed.
//!
//! ## Endpoints
//! - `GET /metrics` — Prometheus text format (for Grafana/Prometheus scraping)
//! - `GET /metrics/json` — JSON format (for stress test client consumption)

use axum::{
    body::Body,
    extract::State,
    http::Request,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::api::ApiState;

/// Shared metrics state (all lock-free atomics)
#[derive(Debug)]
pub struct ServerMetrics {
    /// Total HTTP requests served
    pub total_requests: AtomicU64,
    /// Total request errors (4xx + 5xx)
    pub total_errors: AtomicU64,
    /// Cumulative request duration in microseconds (for computing average)
    pub total_duration_us: AtomicU64,
    /// Server start time (for uptime calculation)
    pub start_time: Instant,
}

impl Default for ServerMetrics {
    fn default() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
            total_duration_us: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }
}

impl ServerMetrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn record_request(&self, duration_us: u64, is_error: bool) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_duration_us.fetch_add(duration_us, Ordering::Relaxed);
        if is_error {
            self.total_errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn uptime_secs(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    pub fn requests_per_second(&self) -> f64 {
        let total = self.total_requests.load(Ordering::Relaxed) as f64;
        let uptime = self.uptime_secs();
        if uptime > 0.0 { total / uptime } else { 0.0 }
    }

    pub fn avg_duration_ms(&self) -> f64 {
        let total = self.total_requests.load(Ordering::Relaxed);
        let dur_us = self.total_duration_us.load(Ordering::Relaxed);
        if total > 0 {
            (dur_us as f64 / total as f64) / 1000.0
        } else {
            0.0
        }
    }
}

// ============================================================================
// Axum Middleware — Automatic request tracking
// ============================================================================

/// Middleware that records request count and duration for every HTTP request.
pub async fn metrics_middleware(
    State(state): State<ApiState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let start = Instant::now();
    let resp = next.run(req).await;
    let duration_us = start.elapsed().as_micros() as u64;
    let is_error = resp.status().is_client_error() || resp.status().is_server_error();

    state.metrics.record_request(duration_us, is_error);
    resp
}

// ============================================================================
// GET /metrics — Prometheus text exposition format
// ============================================================================

pub async fn prometheus_handler(State(state): State<ApiState>) -> impl IntoResponse {
    let m = &state.metrics;
    let total_requests = m.total_requests.load(Ordering::Relaxed);
    let total_errors = m.total_errors.load(Ordering::Relaxed);
    let total_dur_us = m.total_duration_us.load(Ordering::Relaxed);
    let uptime = m.uptime_secs();
    let rps = m.requests_per_second();

    // Read game state from ECS snapshot
    let (player_count, entity_count, tick, avg_tick_ms) = {
        let snap = state.world_snapshot.read().unwrap_or_else(|e| e.into_inner());
        let players = snap.players.len();
        let entities = snap.entity_count;
        let tick = snap.tick;
        // uptime_secs / ticks gives avg tick duration
        let avg_tick = if tick > 0 {
            (snap.uptime_secs / tick as f64) * 1000.0
        } else {
            0.0
        };
        (players, entities, tick, avg_tick)
    };

    let avg_req_duration_s = if total_requests > 0 {
        (total_dur_us as f64 / total_requests as f64) / 1_000_000.0
    } else {
        0.0
    };

    let body = format!(
        "# HELP tower_requests_total Total HTTP requests served\n\
         # TYPE tower_requests_total counter\n\
         tower_requests_total {total_requests}\n\
         \n\
         # HELP tower_request_errors_total Total HTTP request errors (4xx/5xx)\n\
         # TYPE tower_request_errors_total counter\n\
         tower_request_errors_total {total_errors}\n\
         \n\
         # HELP tower_request_duration_seconds Average request duration\n\
         # TYPE tower_request_duration_seconds gauge\n\
         tower_request_duration_seconds {avg_req_duration_s:.6}\n\
         \n\
         # HELP tower_requests_per_second Current request throughput\n\
         # TYPE tower_requests_per_second gauge\n\
         tower_requests_per_second {rps:.2}\n\
         \n\
         # HELP tower_player_count Current connected player count\n\
         # TYPE tower_player_count gauge\n\
         tower_player_count {player_count}\n\
         \n\
         # HELP tower_entity_count Total active ECS entities\n\
         # TYPE tower_entity_count gauge\n\
         tower_entity_count {entity_count}\n\
         \n\
         # HELP tower_tick_total Total server ticks processed\n\
         # TYPE tower_tick_total counter\n\
         tower_tick_total {tick}\n\
         \n\
         # HELP tower_tick_duration_seconds Average ECS tick duration\n\
         # TYPE tower_tick_duration_seconds gauge\n\
         tower_tick_duration_seconds {avg_tick_s:.6}\n\
         \n\
         # HELP tower_uptime_seconds Server uptime\n\
         # TYPE tower_uptime_seconds gauge\n\
         tower_uptime_seconds {uptime:.2}\n",
        avg_tick_s = avg_tick_ms / 1000.0,
    );

    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}

// ============================================================================
// GET /metrics/json — JSON format for stress test clients
// ============================================================================

#[derive(Serialize)]
pub struct JsonMetrics {
    pub uptime_secs: f64,
    pub player_count: usize,
    pub entity_count: usize,
    pub tick: u64,
    pub avg_tick_time_ms: f64,
    pub total_requests: u64,
    pub total_errors: u64,
    pub rps: f64,
    pub avg_request_duration_ms: f64,
}

pub async fn json_metrics_handler(State(state): State<ApiState>) -> Json<JsonMetrics> {
    let m = &state.metrics;

    let (player_count, entity_count, tick, avg_tick_ms) = {
        let snap = state.world_snapshot.read().unwrap_or_else(|e| e.into_inner());
        let avg_tick = if snap.tick > 0 {
            (snap.uptime_secs / snap.tick as f64) * 1000.0
        } else {
            0.0
        };
        (snap.players.len(), snap.entity_count, snap.tick, avg_tick)
    };

    Json(JsonMetrics {
        uptime_secs: m.uptime_secs(),
        player_count,
        entity_count,
        tick,
        avg_tick_time_ms: avg_tick_ms,
        total_requests: m.total_requests.load(Ordering::Relaxed),
        total_errors: m.total_errors.load(Ordering::Relaxed),
        rps: m.requests_per_second(),
        avg_request_duration_ms: m.avg_duration_ms(),
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_metrics_defaults() {
        let m = ServerMetrics::default();
        assert_eq!(m.total_requests.load(Ordering::Relaxed), 0);
        assert_eq!(m.total_errors.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_record_request() {
        let m = ServerMetrics::default();
        m.record_request(1500, false);
        m.record_request(2500, true);
        m.record_request(1000, false);

        assert_eq!(m.total_requests.load(Ordering::Relaxed), 3);
        assert_eq!(m.total_errors.load(Ordering::Relaxed), 1);
        assert_eq!(m.total_duration_us.load(Ordering::Relaxed), 5000);
    }

    #[test]
    fn test_avg_duration_ms() {
        let m = ServerMetrics::default();
        m.record_request(3000, false); // 3ms
        m.record_request(5000, false); // 5ms
        let avg = m.avg_duration_ms();
        assert!((avg - 4.0).abs() < 0.01); // avg = 4ms
    }

    #[test]
    fn test_rps_zero_uptime() {
        let m = ServerMetrics::default();
        // uptime is near-zero, rps should be finite
        let rps = m.requests_per_second();
        assert!(rps.is_finite());
    }
}
