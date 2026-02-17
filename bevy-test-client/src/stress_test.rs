/// Stress test utility for spawning multiple renet clients
///
/// Usage: cargo run --bin stress_test -- --clients 10 --duration 60
///
/// Enhanced with shared atomic counters and structured summary output.
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    RepliconRenetPlugins,
    renet::RenetClient,
    netcode::{ClientAuthentication, NetcodeClientTransport},
};
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use tracing::info;

// Shared component definitions (must match server)
#[derive(Component, Serialize, Deserialize, Debug, Clone)]
struct Player {
    id: u64,
    position: Vec3,
    health: f32,
    current_floor: u32,
}

#[derive(Component, Serialize, Deserialize, Debug)]
struct Monster {
    monster_type: String,
    position: Vec3,
    health: f32,
    max_health: f32,
}

#[derive(Component, Clone, Serialize, Deserialize, Debug)]
struct FloorTile {
    tile_type: u8,
    grid_x: i32,
    grid_y: i32,
}

// ============================================================================
// Shared counters (lock-free, across all client threads)
// ============================================================================

struct SharedCounters {
    connects_ok: AtomicU64,
    connects_fail: AtomicU64,
    total_ticks: AtomicU64,
    max_players_seen: AtomicU64,
    clients_finished: AtomicU64,
    test_running: AtomicBool,
}

impl SharedCounters {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            connects_ok: AtomicU64::new(0),
            connects_fail: AtomicU64::new(0),
            total_ticks: AtomicU64::new(0),
            max_players_seen: AtomicU64::new(0),
            clients_finished: AtomicU64::new(0),
            test_running: AtomicBool::new(true),
        })
    }

    fn update_max_players(&self, count: u64) {
        let mut current = self.max_players_seen.load(Ordering::Relaxed);
        while count > current {
            match self.max_players_seen.compare_exchange_weak(
                current, count, Ordering::Relaxed, Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(c) => current = c,
            }
        }
    }
}

/// Per-client statistics (thread-local)
#[derive(Resource, Default)]
struct ClientStats {
    client_id: u64,
    connected: bool,
    connect_time_ms: u64,
    players_seen: u32,
    connection_time: f32,
    last_update: f32,
    ticks: u64,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let num_clients = parse_arg(&args, "--clients").unwrap_or(10);
    let test_duration = parse_arg(&args, "--duration").unwrap_or(60);
    let server_addr = parse_str_arg(&args, "--server")
        .unwrap_or_else(|| "127.0.0.1:5000".into());

    println!("=== Renet UDP Stress Test ===");
    println!("  Clients:  {}", num_clients);
    println!("  Duration: {}s", test_duration);
    println!("  Server:   {}", server_addr);
    println!();

    let counters = SharedCounters::new();
    let test_start = Instant::now();

    // Spawn client threads
    let mut handles = vec![];
    for i in 0..num_clients {
        let client_idx = i as usize;
        let counters = counters.clone();
        let server_addr = server_addr.clone();
        let handle = std::thread::spawn(move || {
            run_client(client_idx, test_duration, server_addr, counters);
        });
        handles.push(handle);
        std::thread::sleep(Duration::from_millis(100));
    }

    // Progress reporter (main thread)
    let report_counters = counters.clone();
    loop {
        std::thread::sleep(Duration::from_secs(5));
        let elapsed = test_start.elapsed().as_secs();
        let finished = report_counters.clients_finished.load(Ordering::Relaxed);
        let ok = report_counters.connects_ok.load(Ordering::Relaxed);
        let fail = report_counters.connects_fail.load(Ordering::Relaxed);
        let max_p = report_counters.max_players_seen.load(Ordering::Relaxed);

        println!(
            "  [{:>3}s] connected: {}/{}, failed: {}, max_players: {}, finished: {}",
            elapsed, ok, num_clients, fail, max_p, finished
        );

        if finished >= num_clients {
            break;
        }
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    let total_time = test_start.elapsed();
    counters.test_running.store(false, Ordering::Relaxed);

    // Print summary
    let ok = counters.connects_ok.load(Ordering::Relaxed);
    let fail = counters.connects_fail.load(Ordering::Relaxed);
    let ticks = counters.total_ticks.load(Ordering::Relaxed);
    let max_p = counters.max_players_seen.load(Ordering::Relaxed);

    println!("\n=== Stress Test Results ===");
    println!("  Duration:         {:.1}s", total_time.as_secs_f64());
    println!("  Clients:          {}", num_clients);
    println!("  Connected:        {} ({} failed)", ok, fail);
    println!("  Max players seen: {}", max_p);
    println!("  Total ticks:      {}", ticks);
    println!("  Avg ticks/client: {:.0}", ticks as f64 / num_clients.max(1) as f64);

    if fail > 0 {
        println!("\n  WARNING: {} clients failed to connect", fail);
    }

    // Write JSON results
    let results = serde_json::json!({
        "test_config": {
            "clients": num_clients,
            "duration_secs": test_duration,
            "server": server_addr,
        },
        "results": {
            "duration_secs": total_time.as_secs_f64(),
            "connects_ok": ok,
            "connects_fail": fail,
            "max_players_seen": max_p,
            "total_ticks": ticks,
        }
    });

    let path = "stress_test_results.json";
    if let Ok(_) = std::fs::write(path, serde_json::to_string_pretty(&results).unwrap()) {
        println!("\n  Results written to {}", path);
    }

    println!("\nDone.");

    if fail > num_clients / 2 {
        std::process::exit(1);
    }
}

fn parse_arg(args: &[String], flag: &str) -> Option<u64> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|i| args.get(i + 1))
        .and_then(|val| val.parse().ok())
}

fn parse_str_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn run_client(client_index: usize, duration: u64, server_addr: String, counters: Arc<SharedCounters>) {
    // Only first client sets global subscriber
    if client_index == 0 {
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::WARN)
            .with_thread_ids(true)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    }

    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(RepliconPlugins)
        .add_plugins(RepliconRenetPlugins)

        .replicate::<Player>()
        .replicate::<Monster>()
        .replicate::<FloorTile>()

        .insert_resource(TestConfig {
            client_index,
            duration,
            start_time: SystemTime::now(),
            connect_start: Instant::now(),
            server_addr,
        })
        .insert_resource(ClientStats::default())
        .insert_resource(SharedCountersRes(counters))

        .add_systems(Startup, setup_stress_client)
        .add_systems(Update, (
            track_stats,
            check_timeout,
        ))

        .run();
}

#[derive(Resource)]
struct SharedCountersRes(Arc<SharedCounters>);

#[derive(Resource)]
struct TestConfig {
    client_index: usize,
    duration: u64,
    start_time: SystemTime,
    connect_start: Instant,
    server_addr: String,
}

fn setup_stress_client(
    mut commands: Commands,
    config: Res<TestConfig>,
    mut stats: ResMut<ClientStats>,
) {
    let base_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let client_id = base_time + (config.client_index as u64 * 1000);
    stats.client_id = client_id;

    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind socket");
    let server_addr: SocketAddr = config.server_addr.parse().unwrap();

    let connection_config = bevy_replicon_renet::renet::ConnectionConfig::default();
    let client = RenetClient::new(connection_config);

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap();

    let authentication = ClientAuthentication::Unsecure {
        server_addr,
        client_id,
        user_data: None,
        protocol_id: 0,
    };

    let transport = NetcodeClientTransport::new(
        current_time,
        authentication,
        socket,
    ).expect("Failed to create transport");

    commands.insert_resource(client);
    commands.insert_resource(transport);

    info!("Client #{} (ID: {}) connecting...", config.client_index, client_id);
}

fn track_stats(
    mut stats: ResMut<ClientStats>,
    client: Option<Res<RenetClient>>,
    players: Query<&Player>,
    time: Res<Time>,
    config: Res<TestConfig>,
    shared: Res<SharedCountersRes>,
) {
    stats.ticks += 1;
    shared.0.total_ticks.fetch_add(1, Ordering::Relaxed);

    if let Some(client) = client {
        if client.is_connected() {
            if !stats.connected {
                stats.connected = true;
                stats.connect_time_ms = config.connect_start.elapsed().as_millis() as u64;
                shared.0.connects_ok.fetch_add(1, Ordering::Relaxed);
            }

            stats.connection_time = time.elapsed_secs();
            let player_count = players.iter().count() as u32;
            stats.players_seen = player_count;
            shared.0.update_max_players(player_count as u64);

            if stats.connection_time - stats.last_update >= 10.0 {
                info!(
                    "Client #{} stats: {}s, {} players, {} ticks",
                    config.client_index,
                    stats.connection_time as u32,
                    stats.players_seen,
                    stats.ticks,
                );
                stats.last_update = stats.connection_time;
            }
        }
    }
}

fn check_timeout(
    config: Res<TestConfig>,
    stats: Res<ClientStats>,
    shared: Res<SharedCountersRes>,
    mut exit: EventWriter<AppExit>,
) {
    let elapsed = SystemTime::now()
        .duration_since(config.start_time)
        .unwrap()
        .as_secs();

    if elapsed >= config.duration {
        println!(
            "  Client #{}: {}s, {} players, {} ticks, connect={}ms",
            config.client_index,
            stats.connection_time as u32,
            stats.players_seen,
            stats.ticks,
            stats.connect_time_ms,
        );
        shared.0.clients_finished.fetch_add(1, Ordering::Relaxed);
        exit.send(AppExit::Success);
    }
}
