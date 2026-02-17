use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    RepliconRenetPlugins,
    renet::RenetClient,
    netcode::{
        ClientAuthentication,
        NetcodeClientTransport,
    },
};
use std::net::{SocketAddr, UdpSocket};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use tracing::info;

fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    println!("Tower Test Client Starting...");

    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(RepliconPlugins)
        .add_plugins(RepliconRenetPlugins)

        // Register replicated components (must match server)
        .replicate::<Player>()
        .replicate::<Monster>()
        .replicate::<FloorTile>()

        // Register client-to-server input event (must match server)
        .add_client_event::<PlayerInput>(ChannelKind::Ordered)

        // Track connection state
        .insert_resource(ConnectionState::default())

        // Systems
        .add_systems(Startup, setup_client)
        .add_systems(Update, (
            track_connection,
            send_movement_input,
            log_replicated_entities,
        ))

        .run();
}

// ============================================================================
// Components (must match server exactly for replication)
// ============================================================================

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
// Input event (must match server's input::PlayerInput)
// ============================================================================

#[derive(Event, Serialize, Deserialize, Debug, Clone)]
struct PlayerInput {
    movement: [f32; 3],
    facing: f32,
    action: Option<InputAction>,
    sequence: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
enum InputAction {
    Attack,
    Block,
    BlockRelease,
    Parry,
    Dodge,
    HeavyAttack,
    Interact,
}

// ============================================================================
// Resources
// ============================================================================

#[derive(Resource, Default)]
struct ConnectionState {
    connected: bool,
    logged_connect: bool,
    logged_disconnect: bool,
    input_sequence: u32,
    last_entity_log: f32,
}

// ============================================================================
// Systems
// ============================================================================

fn setup_client(mut commands: Commands) {
    info!("Connecting to server at 127.0.0.1:5000...");

    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind client socket");
    let server_addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();

    let connection_config = bevy_replicon_renet::renet::ConnectionConfig::default();
    let client = RenetClient::new(connection_config);

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap();
    let client_id = current_time.as_millis() as u64;

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
    ).expect("Failed to create client transport");

    commands.insert_resource(client);
    commands.insert_resource(transport);

    info!("Client initialized (ID: {})", client_id);
}

fn track_connection(
    client: Option<Res<RenetClient>>,
    mut state: ResMut<ConnectionState>,
) {
    if let Some(client) = client {
        if client.is_connected() && !state.logged_connect {
            info!("Connected to server!");
            state.connected = true;
            state.logged_connect = true;
        } else if !client.is_connected() && state.connected && !state.logged_disconnect {
            info!("Disconnected from server");
            state.connected = false;
            state.logged_disconnect = true;
        }
    }
}

/// Send simulated movement input (walk in a circle)
fn send_movement_input(
    mut state: ResMut<ConnectionState>,
    mut input_events: EventWriter<PlayerInput>,
    time: Res<Time>,
    client: Option<Res<RenetClient>>,
) {
    let Some(client) = client else { return };
    if !client.is_connected() { return; }

    // Walk in a circle (1 unit/s)
    let t = time.elapsed_secs();
    let speed = 3.0;
    let mx = t.cos() * speed;
    let mz = t.sin() * speed;
    let facing = t.sin().atan2(t.cos());

    state.input_sequence += 1;

    input_events.send(PlayerInput {
        movement: [mx, 0.0, mz],
        facing,
        action: None,
        sequence: state.input_sequence,
    });
}

fn log_replicated_entities(
    players: Query<&Player>,
    monsters: Query<&Monster>,
    tiles: Query<&FloorTile>,
    mut state: ResMut<ConnectionState>,
    time: Res<Time>,
) {
    let elapsed = time.elapsed_secs();
    // Log entity counts every 5 seconds
    if elapsed - state.last_entity_log >= 5.0 {
        let player_count = players.iter().count();
        let monster_count = monsters.iter().count();
        let tile_count = tiles.iter().count();

        if player_count > 0 || monster_count > 0 || tile_count > 0 {
            info!(
                "Replicated: {} players, {} monsters, {} tiles",
                player_count, monster_count, tile_count
            );

            for player in &players {
                info!("  Player {}: pos={:?} hp={} floor={}",
                    player.id, player.position, player.health, player.current_floor);
            }
        }

        state.last_entity_log = elapsed;
    }
}
