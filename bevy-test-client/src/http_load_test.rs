//! HTTP API Load Test
//!
//! Tests all JSON-over-HTTP endpoints with concurrent requests.
//! Measures latency percentiles (p50/p95/p99), throughput, and error rate.
//!
//! Usage:
//!   cargo run --release --bin http_load_test -- --url http://localhost:50051 --concurrency 10 --duration 30
//!
//! Requires: bevy-server running with HTTP API on the target port.

use reqwest::Client;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

// ============================================================================
// Endpoint definitions
// ============================================================================

struct Endpoint {
    name: &'static str,
    method: &'static str,
    path: &'static str,
    body: Option<Value>,
}

fn all_endpoints() -> Vec<Endpoint> {
    vec![
        // Health
        Endpoint {
            name: "Health",
            method: "GET",
            path: "/health",
            body: None,
        },
        // Generation
        Endpoint {
            name: "GenerateFloor",
            method: "POST",
            path: "/tower.GenerationService/GenerateFloor",
            body: Some(json!({"tower_seed": 42, "floor_id": 1})),
        },
        Endpoint {
            name: "GenerateLoot",
            method: "POST",
            path: "/tower.GenerationService/GenerateLoot",
            body: Some(json!({
                "source_entity_id": 100,
                "player_id": 1,
                "source_tags": [{"tag": "fire", "weight": 0.7}],
                "luck_modifier": 0.0
            })),
        },
        Endpoint {
            name: "SpawnMonsters",
            method: "POST",
            path: "/tower.GenerationService/SpawnMonsters",
            body: Some(json!({
                "tower_seed": 42,
                "floor_id": 3,
                "room_id": 1,
                "biome_tags": [{"tag": "forest", "weight": 0.8}]
            })),
        },
        // GameState
        Endpoint {
            name: "GetWorldCycle",
            method: "POST",
            path: "/tower.GameStateService/GetWorldCycle",
            body: Some(json!({"tower_seed": 12345})),
        },
        Endpoint {
            name: "GetLiveStatus",
            method: "POST",
            path: "/tower.GameStateService/GetLiveStatus",
            body: Some(json!({})),
        },
        // Combat
        Endpoint {
            name: "CalculateDamage",
            method: "POST",
            path: "/tower.CombatService/CalculateDamage",
            body: Some(json!({
                "attacker_id": 1,
                "target_id": 2,
                "ability_id": "basic_attack",
                "hit_angle": 0.0,
                "combo_count": 1,
                "semantic_tags": [{"tag": "fire", "weight": 0.5}]
            })),
        },
        // Mastery
        Endpoint {
            name: "GetMasteryProfile",
            method: "POST",
            path: "/tower.MasteryService/GetMasteryProfile",
            body: Some(json!({"player_id": 1})),
        },
        Endpoint {
            name: "AddMasteryXP",
            method: "POST",
            path: "/tower.MasteryService/AddMasteryXP",
            body: Some(json!({
                "player_id": 1,
                "domain": "sword",
                "xp_amount": 10.0,
                "action_context": "training"
            })),
        },
        // Economy
        Endpoint {
            name: "GetWallet",
            method: "POST",
            path: "/tower.EconomyService/GetWallet",
            body: Some(json!({"player_id": 1})),
        },
        // Destruction
        Endpoint {
            name: "GetFloorState",
            method: "POST",
            path: "/tower.DestructionService/GetFloorState",
            body: Some(json!({"floor_id": 1})),
        },
    ]
}

// ============================================================================
// Per-endpoint statistics
// ============================================================================

struct EndpointStats {
    name: String,
    latencies_us: parking_lot::Mutex<Vec<u64>>,
    success: AtomicU64,
    errors: AtomicU64,
}

impl EndpointStats {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            latencies_us: parking_lot::Mutex::new(Vec::with_capacity(10_000)),
            success: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        }
    }

    fn record(&self, duration_us: u64, ok: bool) {
        self.latencies_us.lock().push(duration_us);
        if ok {
            self.success.fetch_add(1, Ordering::Relaxed);
        } else {
            self.errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn percentile(&self, p: f64) -> f64 {
        let mut lat = self.latencies_us.lock().clone();
        if lat.is_empty() {
            return 0.0;
        }
        lat.sort_unstable();
        let idx = ((p / 100.0) * lat.len() as f64) as usize;
        let idx = idx.min(lat.len() - 1);
        lat[idx] as f64 / 1000.0 // return ms
    }
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let base_url = parse_str_arg(&args, "--url").unwrap_or_else(|| "http://localhost:50051".into());
    let concurrency: usize = parse_num_arg(&args, "--concurrency").unwrap_or(10);
    let duration_secs: u64 = parse_num_arg(&args, "--duration").unwrap_or(30);

    println!("=== HTTP API Load Test ===");
    println!("  Target:      {}", base_url);
    println!("  Concurrency: {}", concurrency);
    println!("  Duration:    {}s", duration_secs);
    println!();

    // Verify server is up
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to create HTTP client");

    match client.get(format!("{}/health", base_url)).send().await {
        Ok(resp) if resp.status().is_success() => {
            println!("Server health check: OK");
        }
        Ok(resp) => {
            eprintln!("Server health check failed: HTTP {}", resp.status());
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Cannot reach server at {}: {}", base_url, e);
            std::process::exit(1);
        }
    }

    let endpoints = all_endpoints();
    let stats: Vec<Arc<EndpointStats>> = endpoints
        .iter()
        .map(|e| Arc::new(EndpointStats::new(e.name)))
        .collect();

    let semaphore = Arc::new(Semaphore::new(concurrency));
    let start = Instant::now();
    let deadline = start + Duration::from_secs(duration_secs);
    let total_requests = Arc::new(AtomicU64::new(0));

    println!("Running load test...\n");

    // Spawn worker tasks that cycle through endpoints
    let mut handles = Vec::new();
    for worker_id in 0..concurrency {
        let client = client.clone();
        let base_url = base_url.clone();
        let sem = semaphore.clone();
        let stats = stats.clone();
        let total = total_requests.clone();
        let endpoints_len = endpoints.len();

        // Pre-build request data
        let endpoint_data: Vec<(String, String, Option<String>)> = endpoints
            .iter()
            .map(|e| {
                (
                    e.method.to_string(),
                    format!("{}{}", base_url, e.path),
                    e.body.as_ref().map(|b| b.to_string()),
                )
            })
            .collect();

        let handle = tokio::spawn(async move {
            let mut idx = worker_id % endpoints_len;
            while Instant::now() < deadline {
                let _permit = sem.acquire().await.unwrap();
                let (method, url, body) = &endpoint_data[idx];
                let stat = &stats[idx];

                let req_start = Instant::now();
                let result = if method == "GET" {
                    client.get(url).send().await
                } else {
                    let mut req = client.post(url).header("content-type", "application/json");
                    if let Some(b) = body {
                        req = req.body(b.clone());
                    }
                    req.send().await
                };

                let duration_us = req_start.elapsed().as_micros() as u64;
                let ok = match &result {
                    Ok(resp) => resp.status().is_success(),
                    Err(_) => false,
                };
                stat.record(duration_us, ok);
                total.fetch_add(1, Ordering::Relaxed);

                idx = (idx + 1) % endpoints_len;
            }
        });
        handles.push(handle);
    }

    // Progress reporter
    let total_clone = total_requests.clone();
    let progress = tokio::spawn(async move {
        let mut last_count = 0u64;
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            if Instant::now() >= deadline {
                break;
            }
            let current = total_clone.load(Ordering::Relaxed);
            let delta = current - last_count;
            let elapsed = start.elapsed().as_secs_f64();
            println!(
                "  [{:.0}s] {} requests ({:.0} rps, +{} last 5s)",
                elapsed,
                current,
                current as f64 / elapsed,
                delta
            );
            last_count = current;
        }
    });

    // Wait for completion
    for h in handles {
        h.await.unwrap();
    }
    progress.abort();

    let total_time = start.elapsed();
    let total_reqs = total_requests.load(Ordering::Relaxed);

    // Print results
    println!("\n=== Results ===\n");
    println!(
        "Total: {} requests in {:.2}s ({:.1} rps)\n",
        total_reqs,
        total_time.as_secs_f64(),
        total_reqs as f64 / total_time.as_secs_f64()
    );

    println!(
        "{:<25} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "Endpoint", "Count", "Errors", "p50(ms)", "p95(ms)", "p99(ms)", "Err%"
    );
    println!("{}", "-".repeat(85));

    let mut total_errors = 0u64;
    for stat in &stats {
        let success = stat.success.load(Ordering::Relaxed);
        let errors = stat.errors.load(Ordering::Relaxed);
        let count = success + errors;
        total_errors += errors;
        let err_pct = if count > 0 {
            errors as f64 / count as f64 * 100.0
        } else {
            0.0
        };

        println!(
            "{:<25} {:>8} {:>8} {:>8.2} {:>8.2} {:>8.2} {:>7.1}%",
            stat.name,
            count,
            errors,
            stat.percentile(50.0),
            stat.percentile(95.0),
            stat.percentile(99.0),
            err_pct,
        );
    }

    println!("{}", "-".repeat(85));
    println!(
        "{:<25} {:>8} {:>8}",
        "TOTAL", total_reqs, total_errors
    );

    // Write JSON results file
    let results = json!({
        "test_config": {
            "base_url": base_url,
            "concurrency": concurrency,
            "duration_secs": duration_secs,
        },
        "summary": {
            "total_requests": total_reqs,
            "total_errors": total_errors,
            "duration_secs": total_time.as_secs_f64(),
            "rps": total_reqs as f64 / total_time.as_secs_f64(),
            "error_rate": total_errors as f64 / total_reqs.max(1) as f64,
        },
        "endpoints": stats.iter().map(|s| {
            let success = s.success.load(Ordering::Relaxed);
            let errors = s.errors.load(Ordering::Relaxed);
            json!({
                "name": s.name,
                "count": success + errors,
                "errors": errors,
                "p50_ms": s.percentile(50.0),
                "p95_ms": s.percentile(95.0),
                "p99_ms": s.percentile(99.0),
            })
        }).collect::<Vec<_>>(),
    });

    let results_path = "load_test_results.json";
    if let Err(e) = std::fs::write(results_path, serde_json::to_string_pretty(&results).unwrap()) {
        eprintln!("Failed to write results: {}", e);
    } else {
        println!("\nResults written to {}", results_path);
    }

    // Exit with error code if error rate > 10%
    let error_rate = total_errors as f64 / total_reqs.max(1) as f64;
    if error_rate > 0.10 {
        eprintln!("\nERROR: Error rate {:.1}% exceeds 10% threshold", error_rate * 100.0);
        std::process::exit(1);
    }
}

fn parse_str_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn parse_num_arg<T: std::str::FromStr>(args: &[String], flag: &str) -> Option<T> {
    parse_str_arg(args, flag).and_then(|v| v.parse().ok())
}
