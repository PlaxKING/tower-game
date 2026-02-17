use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    RepliconRenetPlugins,
    renet::RenetServer,
    netcode::{
        ServerAuthentication,
        ServerConfig as NetcodeServerConfig,
        NetcodeServerTransport,
    },
};
use std::sync::Arc;
use std::time::Duration;
use std::net::{SocketAddr, UdpSocket};
use tracing::{info, error};

// Shared modules from the library crate (ensures type compatibility with API layer)
use tower_bevy_server::{
    api, storage,
    ecs_bridge::{self, WorldSnapshotResource, ServerUptime},
    components::{Player, Monster, FloorTile},
    combat, destruction, monster_gen, physics, input,
};
use bevy_rapier3d::prelude::{RapierPhysicsPlugin, NoUserData};

// Binary-only modules (not shared with library)
#[allow(dead_code)]
mod dynamic_scaling;
#[allow(dead_code)]
mod hybrid_generation;
mod proto;  // Auto-generated Protobuf types
#[allow(dead_code)]
mod async_generation;  // Async floor generation with worker pool
#[allow(dead_code)]
mod lmdb_cache;  // LMDB embedded database caching (Tier 2)
#[allow(dead_code)]
mod semantic_tags;  // Semantic tag system
mod wfc;  // WFC floor generation (shared with library)

#[cfg(test)]
mod proto_test;  // Protobuf serialization tests

use dynamic_scaling::*;
use hybrid_generation::{
    generate_floor_with_validation, validate_client_floors,
    FloorValidationCache,
};
use destruction::FloorDestructionManager;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    println!("🚀 Starting Tower Bevy Server...");

    // ========================================================================
    // 1. Initialize LMDB template store (synchronous, embedded DB)
    // ========================================================================
    let lmdb_path = std::env::var("LMDB_PATH")
        .unwrap_or_else(|_| "data/templates".to_string());
    let lmdb_max_size: usize = {
        let raw = std::env::var("LMDB_MAX_SIZE")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(500_000_000);
        // LMDB requires map size to be a multiple of the OS page size (4096)
        let page_size = 4096_usize;
        (raw + page_size - 1) / page_size * page_size
    };

    let lmdb = Arc::new(
        storage::lmdb_templates::LmdbTemplateStore::new(&lmdb_path, lmdb_max_size)
            .expect("Failed to initialize LMDB template store")
    );

    // Seed initial game data (monsters, items, abilities, recipes, loot tables, quests, factions)
    storage::seed_data::seed_all(&lmdb)
        .expect("Failed to seed LMDB template data");
    info!("LMDB template store initialized at: {}", lmdb_path);

    // ========================================================================
    // 2. Create the API ↔ ECS bridge
    // ========================================================================
    let (cmd_sender, cmd_receiver, world_snapshot) = ecs_bridge::create_bridge();

    // ========================================================================
    // 3. Spawn HTTP API server on a separate tokio runtime
    //    PostgreSQL is initialized here (async) before starting the API server
    // ========================================================================
    let api_snapshot = world_snapshot.clone();
    let api_cmd_sender = cmd_sender.clone();
    let api_lmdb = lmdb.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async {
            // Initialize PostgreSQL (async connection pool + auto-run migrations)
            let database_url = std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:localdb@localhost:5433/tower_game".to_string());
            let pg_max_connections: u32 = std::env::var("PG_MAX_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10);

            info!("Connecting to PostgreSQL: {}...", database_url);

            let pg = match storage::postgres::PostgresStore::new(&database_url, pg_max_connections).await {
                Ok(store) => {
                    info!("PostgreSQL connected and migrations applied");
                    Arc::new(store)
                }
                Err(e) => {
                    error!("PostgreSQL connection failed: {}", e);
                    error!("Ensure PostgreSQL is running: docker compose up -d postgres");
                    error!("API server will NOT start. Bevy game server continues without HTTP API.");
                    // Keep the thread alive so Bevy server continues (game logic works without DB)
                    tokio::signal::ctrl_c().await.ok();
                    return;
                }
            };

            // Start HTTP API server (blocks until shutdown)
            let port: u16 = std::env::var("API_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(50051);

            if let Err(e) = api::start_api_server(
                api_lmdb, pg, api_cmd_sender, api_snapshot, port,
            ).await {
                error!("API server error: {}", e);
            }
        });
    });

    App::new()
        // Headless Bevy (no rendering)
        .add_plugins(MinimalPlugins)
        .add_plugins(TransformPlugin)   // Required for GlobalTransform sync (rapier reads GlobalTransform)
        .add_plugins(HierarchyPlugin)   // Required by TransformPlugin
        .add_plugins(bevy::asset::AssetPlugin::default()) // Required by rapier's async scene collider system
        .add_plugins(bevy::scene::ScenePlugin)            // Provides SceneSpawner for rapier
        .init_asset::<bevy::render::mesh::Mesh>()         // Register Mesh asset for rapier (no renderer needed)

        // Physics engine (headless — no debug rendering)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())

        // Networking plugins
        .add_plugins(RepliconPlugins)
        .add_plugins(RepliconRenetPlugins)

        // Register replicated components
        .replicate::<Player>()
        .replicate::<Monster>()
        .replicate::<FloorTile>()

        // Register client-to-server input event (bevy_replicon networking)
        .add_client_event::<input::PlayerInput>(ChannelKind::Ordered)

        // Server configuration (20 Hz for responsive combat)
        .insert_resource(ServerConfig {
            max_players_per_floor: 100,  // Dynamic scaling (see below)
            tick_rate: 20, // 20 ticks per second (50ms) - responsive!
            target_frame_time: Duration::from_millis(50),
        })

        // Resources
        .insert_resource(DynamicScaling::default())
        .insert_resource(FloorDestructionManager::new())
        .insert_resource(combat::WeaponMovesets::default())
        .insert_resource(FloorValidationCache::default())

        // ECS Bridge resources
        .insert_resource(cmd_receiver)
        .insert_resource(WorldSnapshotResource { snapshot: world_snapshot })
        .insert_resource(ServerUptime::default())

        // Server systems
        .add_systems(Startup, setup_server)
        .add_systems(Update, monitor_performance_system)
        .add_systems(Update, (
            generate_floor_with_validation,
            validate_client_floors,
        ))
        .add_systems(Update, (
            handle_player_connections,
            process_player_input,
            update_game_state,
        ))
        // Combat systems
        .add_systems(Update, combat::update_combat_timers)
        // Monster AI systems
        .add_systems(Update, monster_gen::update_monster_ai)
        // Destruction systems
        .add_systems(Update, (
            destruction::process_destruction_events,
            destruction::respawn_destructibles,
        ))
        // Physics knockback
        .add_systems(Update, physics::apply_knockback)
        // ECS Bridge systems (snapshot + command processing)
        .add_systems(Update, (
            ecs_bridge::update_uptime,
            ecs_bridge::update_world_snapshot,
            ecs_bridge::process_game_commands,
        ))

        .run();
}

// ============================================================================
// Resources
// ============================================================================

#[derive(Resource)]
#[allow(dead_code)]
struct ServerConfig {
    max_players_per_floor: usize,
    tick_rate: u32,
    target_frame_time: Duration,
}

// ============================================================================
// Systems  (Floor types now in crate::wfc)
// ============================================================================

fn setup_server(mut commands: Commands) {
    info!("🚀 Tower Bevy Server starting...");

    // Create UDP socket
    let socket = UdpSocket::bind("0.0.0.0:5000").expect("Failed to bind socket");
    let addr: SocketAddr = "0.0.0.0:5000".parse().unwrap();

    // Server configuration for renet
    let connection_config = bevy_replicon_renet::renet::ConnectionConfig::default();
    let server = RenetServer::new(connection_config);

    // Transport (netcode) configuration
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();

    // Create server config for netcode transport
    let protocol_id = 0; // Protocol ID for version matching
    let max_clients = 100;
    let public_addresses = vec![addr];

    let netcode_config = NetcodeServerConfig {
        current_time,
        max_clients,
        protocol_id,
        public_addresses,
        authentication: ServerAuthentication::Unsecure,
    };

    let transport = NetcodeServerTransport::new(
        netcode_config,
        socket,
    ).expect("Failed to create transport");

    commands.insert_resource(server);
    commands.insert_resource(transport);

    info!("✅ Server listening on 0.0.0.0:5000 (UDP/renet)");
    info!("✅ HTTP API server running on port 50051 (LMDB + PostgreSQL)");
}

fn handle_player_connections(
    mut commands: Commands,
    server: Res<RenetServer>,
    existing_players: Query<&Player>,
) {
    // Handle new player connections
    for client_id in server.clients_id() {
        if !server.is_connected(client_id) {
            continue;
        }

        // Check if player already exists
        let player_exists = existing_players.iter().any(|p| p.id == client_id);
        if player_exists {
            continue; // Skip if already connected
        }

        // Spawn new player entity with physics + combat components
        let player_entity = commands.spawn((
            Player {
                id: client_id,
                position: Vec3::ZERO,
                health: 100.0,
                current_floor: 1,
            },
            Transform::from_translation(Vec3::ZERO),
            physics::player_physics_bundle(),
            combat::CombatState::default(),
            combat::EquippedWeapon {
                weapon_type: combat::WeaponType::Sword,
                weapon_id: format!("starter_sword"),
                base_damage: 25.0,
                attack_speed: 1.0,
                range: 2.0,
            },
            combat::CombatEnergy::default(),
            Replicated, // Mark for replication
        )).id();

        info!("👤 Player {} connected (entity: {:?})", client_id, player_entity);
    }
}

fn process_player_input(
    mut input_events: EventReader<FromClient<input::PlayerInput>>,
    mut players: Query<(Entity, &mut Player, &mut Transform)>,
    mut combat_states: Query<&mut combat::CombatState>,
    weapons: Query<&combat::EquippedWeapon>,
    movesets: Res<combat::WeaponMovesets>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for event in input_events.read() {
        let client_id = event.client_id;
        let player_input = &event.event;

        // Find this client's player entity
        let Some((entity, mut player, mut transform)) = players.iter_mut()
            .find(|(_, p, _)| p.id == client_id.get())
        else {
            continue;
        };

        // Validate and apply movement
        if let Some(movement) = input::validate_movement(player_input.movement) {
            transform.translation += movement * dt;
            player.position = transform.translation;
        }

        // Process combat action
        if let Some(action) = player_input.action {
            if let Some(combat_action) = action.to_combat_action() {
                if let Ok(mut cs) = combat_states.get_mut(entity) {
                    cs.facing = input::validate_facing(player_input.facing);
                    if let Ok(weapon) = weapons.get(entity) {
                        let _ = combat::try_combat_action(
                            &mut cs, combat_action, weapon, &movesets,
                        );
                    }
                }
            }
        }
    }
}

fn update_game_state(
    mut players: Query<(&mut Player, &Transform)>,
) {
    // Sync Player.position with Transform (after input, physics, and knockback updates)
    for (mut player, transform) in &mut players {
        player.position = transform.translation;
    }
}
