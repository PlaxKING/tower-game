# Tower Game - Networking Architecture

**Version**: 2.0
**Last Updated**: 2026-02-17 (Session 30)
**Status**: Production-Ready (Core Systems Complete)

---

## Table of Contents
1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Port Map & Protocols](#port-map--protocols)
4. [Bevy Game Server (UDP)](#bevy-game-server-udp)
5. [HTTP API Server (TCP)](#http-api-server-tcp)
6. [ECS Bridge](#ecs-bridge)
7. [Replication & Components](#replication--components)
8. [Input Processing Pipeline](#input-processing-pipeline)
9. [Floor Generation & Caching](#floor-generation--caching)
10. [Protobuf Schema](#protobuf-schema)
11. [UE5 Client Integration](#ue5-client-integration)
12. [Authentication](#authentication)
13. [Performance](#performance)
14. [Docker Deployment](#docker-deployment)
15. [Testing](#testing)

---

## Overview

Tower Game uses a **hybrid authoritative server architecture** with three communication layers:

| Layer | Protocol | Purpose |
|-------|----------|---------|
| **Game Loop** | UDP (renet/netcode) | Real-time player replication, combat, movement |
| **HTTP API** | JSON-over-HTTP (Axum) | Procedural generation, ECS commands, game state queries |
| **Persistence** | PostgreSQL + LMDB | Player data, templates, floor cache |

### Design Principles

1. **Server Authority** - Bevy server is source of truth for all game state
2. **Client Prediction** - UE5 predicts movement, server reconciles
3. **Hybrid Generation** - Client generates floors from seed, server validates hash
4. **Dynamic Scaling** - Auto-adjust player capacity based on tick performance
5. **3-Tier Caching** - RAM (LRU) -> LMDB (disk) -> CPU (generation)

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│  UE5 Client (Presentation Only)                 │
│  - Renders visuals (Niagara, cel-shading)       │
│  - Predicts player movement locally             │
│  - Sends PlayerInput @ 20Hz (UDP)               │
│  - Queries HTTP API for generation/state        │
│  - Fallback: FFI DLL calls (tower_core.dll)     │
└───────────┬───────────────────┬─────────────────┘
            │ UDP :5000         │ HTTP :50051
            │ (renet/bincode)   │ (JSON)
            ▼                   ▼
┌─────────────────────────────────────────────────┐
│  Bevy Server (Authoritative)                    │
│  ┌──────────────┐  ┌─────────────────────────┐  │
│  │ Game Loop    │  │ Axum HTTP API           │  │
│  │ (20 Hz ECS)  │◄─┤ (tokio async)           │  │
│  │              │  │                         │  │
│  │ Physics      │  │ 6 Service modules:      │  │
│  │ Combat       │  │ Generation, GameState,  │  │
│  │ Input valid. │  │ Combat, Mastery,        │  │
│  │ Replication  │  │ Economy, Destruction    │  │
│  └──────┬───────┘  └────────┬────────────────┘  │
│         │  ECS Bridge       │                    │
│         │  (mpsc + RwLock)  │                    │
│  ┌──────┴───────────────────┴────────────────┐  │
│  │ Storage Layer                             │  │
│  │ LMDB (templates) + PostgreSQL (players)   │  │
│  └───────────────────────────────────────────┘  │
└──────────────────────┬──────────────────────────┘
                       │ PostgreSQL :5432
                       ▼
┌─────────────────────────────────────────────────┐
│  Nakama (Lobby & Persistence)                   │
│  - Matchmaking, player accounts                 │
│  - Leaderboards, social features                │
│  - Session management (JWT, 2h expiry)          │
└─────────────────────────────────────────────────┘
```

---

## Port Map & Protocols

| Service | Port | Protocol | Direction | Purpose |
|---------|------|----------|-----------|---------|
| **Bevy Game Server** | `5000` | UDP | Bidirectional | Renet netcode replication |
| **Bevy HTTP API** | `50051` | TCP/HTTP | Request-Response | JSON API for generation/state |
| **Nakama gRPC** | `7349` | gRPC | Request-Response | Server-to-server API |
| **Nakama HTTP** | `7350` | HTTP | Request-Response | Client REST API, matchmaking |
| **Nakama Console** | `7351` | HTTP | Browser | Admin web UI |
| **PostgreSQL** | `5432` | TCP | Internal | Player persistence, game data |

---

## Bevy Game Server (UDP)

### Technology Stack

```toml
bevy = "0.15.3"           # ECS game engine (headless)
bevy_replicon = "0.30"     # Entity replication framework
bevy_replicon_renet = "0.7" # UDP transport (renet netcode)
bevy_rapier3d = "0.28"     # Server-side physics (hitbox validation)
```

### Server Configuration

**File**: `bevy-server/src/main.rs`

```rust
// Tick rate: 20 Hz (50ms per tick)
tick_rate: 20,
target_frame_time: Duration::from_millis(50),

// Max clients: 100 (dynamic scaling adjusts this)
max_clients: 100,

// Protocol ID: 0 (version matching between client/server)
protocol_id: 0,

// Socket: 0.0.0.0:5000 (UDP)
ServerAuthentication::Unsecure,
```

### ECS Systems Schedule

```
Startup:
  setup_server              → Create UDP socket, bind port 5000
  setup_combat_resources    → Load weapon movesets, combo tables
  setup_destruction_manager → Initialize destructible templates

Update (every 50ms):
  monitor_performance       → Track tick time, adjust capacity
  generate_floor_with_validation → WFC floor generation on demand
  validate_client_floors    → SHA3-256 hash anti-cheat
  handle_player_connections → Spawn/despawn player entities
  process_player_input      → Validate movement/combat, apply to ECS
  process_game_commands     → Execute API commands (up to 64/tick)
  update_world_snapshot     → Write snapshot for HTTP API readers
  update_combat_timers      → Advance combo windows, cooldowns
  apply_knockback           → Physics-based knockback resolution
```

### Dynamic Scaling

```
Performance Ratio = avg_tick_time / target_tick_time (50ms)

< 0.7:  Increase capacity → 120 players  (good performance)
0.7-0.9: Maintain 100 players            (normal)
0.9-1.2: Reduce to 80 players            (warning)
> 1.2:   Emergency reduce to 60          (critical)

Evaluation interval: every 5 seconds
```

---

## HTTP API Server (TCP)

### Overview

JSON-over-HTTP API on port `50051`, mirroring gRPC service paths. Built with Axum.

**Base URL**: `http://127.0.0.1:50051`

### Endpoints

#### Health Check
```
GET /health → { "status": "ok", "version": "0.1.0" }
```

#### GenerationService (`/tower.GenerationService/*`)
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/GenerateFloor` | WFC floor layout from seed |
| POST | `/GenerateLoot` | Procedural loot tables |
| POST | `/SpawnMonsters` | Monster generation for floor |
| POST | `/QuerySemanticTags` | Semantic tag queries |
| POST | `/GenerateDestructibles` | Destructible environment objects |
| POST | `/GenerateMonsters` | Procedural monster blueprints |

#### GameStateService (`/tower.GameStateService/*`)
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/GetState` | Full player state + world |
| POST | `/GetWorldCycle` | Breath of the Tower phase |
| POST | `/GetPlayerProfile` | Player stats/profile |
| POST | `/GetLiveStatus` | Real-time player count (from ECS) |
| POST | `/GetLivePlayer` | Live player snapshot (from ECS) |

#### CombatService (`/tower.CombatService/*`)
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/CalculateDamage` | Damage preview (for UI) |
| POST | `/GetCombatState` | Entity combat state |
| POST | `/ProcessAction` | Process attack/parry/dodge |

#### MasteryService (`/tower.MasteryService/*`)
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/TrackProgress` | Update mastery XP |
| POST | `/GetMasteryProfile` | Full mastery tree state |
| POST | `/ChooseSpecialization` | Pick combat role |
| POST | `/UpdateAbilityLoadout` | Hotbar management |

#### EconomyService (`/tower.EconomyService/*`)
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/GetWallet` | Player gold/currency |
| POST | `/Craft` | Crafting system |
| POST | `/ListAuction` | Auction listing |
| POST | `/BuyAuction` | Auction purchase |
| POST | `/Trade` | Player-to-player trade |

#### DestructionService (`/tower.DestructionService/*`)
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/ApplyDamage` | Environmental destruction |
| POST | `/GetFloorState` | Destruction state snapshot |
| POST | `/Rebuild` | Repair destroyed objects |
| POST | `/GetTemplates` | Destructible templates |

---

## ECS Bridge

**File**: `bevy-server/src/ecs_bridge.rs`

The ECS Bridge connects the async HTTP API (tokio) with the synchronous Bevy ECS game loop using channels and shared state.

### Architecture

```
┌────────────────────────────┐
│  Axum HTTP Handler         │ (tokio async thread pool)
│  (receives JSON request)   │
└──────────┬─────────────────┘
           │ GameCommand (mpsc::unbounded)
           ▼
┌────────────────────────────┐
│  process_game_commands     │ (Bevy system, runs every tick)
│  (processes up to 64/tick) │ (prevents stalling game loop)
│  (executes on ECS world)   │
└──────────┬─────────────────┘
           │ oneshot::Sender (reply)
           ▼
┌────────────────────────────┐
│  update_world_snapshot     │ (Bevy system, runs every tick)
│  (queries all entities)    │ (writes to shared snapshot)
└──────────┬─────────────────┘
           │ Arc<RwLock<GameWorldSnapshot>>
           ▼
┌────────────────────────────┐
│  HTTP handler reads        │ (non-blocking RwLock read)
│  snapshot for response     │
└────────────────────────────┘
```

### GameCommand Types

```rust
pub enum GameCommand {
    MovePlayer     { player_id, position, reply },
    DealDamage     { attacker_id, target_id, damage, reply },
    SpawnMonster   { floor_id, monster_type, position, health, reply },
    DestroyObject  { entity_id, floor_id, impact_point, damage, radius, damage_type, reply },
    GetPlayerCount { reply },
    GetPlayer      { player_id, reply },
    CombatAction   { player_id, action, position, facing, reply },
}
```

### World Snapshot

```rust
pub struct GameWorldSnapshot {
    pub tick: u64,
    pub players: HashMap<u64, PlayerSnapshot>,
    pub monsters_per_floor: HashMap<u32, Vec<MonsterSnapshot>>,
    pub entity_count: usize,
    pub uptime_secs: f64,
    pub world_cycle_phase: u32,
    pub destruction_stats: HashMap<u32, (u32, u32, f32)>,  // floor → (total, destroyed, %)
}
```

---

## Replication & Components

### Replicated Components

Registered in `main.rs` with `bevy_replicon`:

```rust
.replicate::<Player>()
.replicate::<Monster>()
.replicate::<FloorTile>()
```

**File**: `bevy-server/src/components.rs`

```rust
#[derive(Component, Serialize, Deserialize)]
pub struct Player {
    pub id: u64,            // Unique client ID
    pub position: Vec3,     // World position (Y-up)
    pub health: f32,        // Current HP
    pub current_floor: u32, // Active floor number
}

#[derive(Component, Serialize, Deserialize)]
pub struct Monster {
    pub monster_type: String,  // Template name
    pub position: Vec3,
    pub health: f32,
    pub max_health: f32,
}

#[derive(Component, Serialize, Deserialize)]
pub struct FloorTile {
    pub tile_type: u8,   // WFC tile ID (0-11)
    pub grid_x: i32,
    pub grid_y: i32,
}
```

### Client-to-Server Event

```rust
.add_client_event::<PlayerInput>(ChannelKind::Ordered)
```

Input events are sent reliably and in-order from client to server.

---

## Input Processing Pipeline

**File**: `bevy-server/src/input.rs`

### Validation Constants

```rust
pub const MAX_MOVE_SPEED: f32 = 10.0;   // units/sec (anti-speed-hack)
pub const SPEED_TOLERANCE: f32 = 1.5;   // 50% over max allowed
pub const MAX_FACING_DELTA: f32 = TAU;  // anti-spinbot limit
```

### PlayerInput Event

```rust
#[derive(Event, Serialize, Deserialize)]
pub struct PlayerInput {
    pub movement: [f32; 3],           // World-space direction vector
    pub facing: f32,                  // Yaw in radians [0, TAU)
    pub action: Option<InputAction>,  // Combat action (if any)
    pub sequence: u32,                // For client-side prediction
}

pub enum InputAction {
    Attack, Block, BlockRelease,
    Parry, Dodge, HeavyAttack, Interact,
}
```

### Server-Side Processing

```
Client sends PlayerInput (UDP, ordered channel)
    ↓
FromClient<PlayerInput> received by Bevy
    ↓
validate_movement() → Clamp speed, reject NaN/Inf
validate_facing() → Normalize to [0, TAU)
    ↓
InputAction → CombatAction conversion
    ↓
Apply to Player component + Transform
Apply to CombatState (if combat action)
```

---

## Floor Generation & Caching

### 3-Tier Caching Architecture

**File**: `bevy-server/src/async_generation.rs`

```
[Request floor_id + seed]
    ↓
[Tier 1: LRU RAM Cache] → HIT? return (~4.7µs)
    ↓ MISS
[Tier 2: LMDB Disk Cache] → HIT? promote to Tier 1 + return (~330µs)
    ↓ MISS
[Tier 3: WFC CPU Generation] → Generate + store in Tiers 1 & 2 (~569µs)
```

| Tier | Storage | Latency | Capacity |
|------|---------|---------|----------|
| 1 | LRU (RAM) | ~4.7 µs | 100 floors |
| 2 | LMDB (disk) | ~330 µs | ~476 MB |
| 3 | WFC (CPU) | ~569 µs | Unlimited |

### WFC Floor Generator

**File**: `bevy-server/src/wfc.rs`

- 12 tile types (Floor, Wall, Door, Stairs, Chest, Trap, etc.)
- 7 room types (Combat, Treasure, Puzzle, Boss, Safe, Corridor, Secret)
- 4 echelon tiers by floor depth:

| Echelon | Floors | Grid Size | Room Count |
|---------|--------|-----------|------------|
| 1 | 1-250 | 16x16 | 3-6 |
| 2 | 251-500 | 24x24 | 5-8 |
| 3 | 501-750 | 32x32 | 7-12 |
| 4 | 751-1000 | 48x48 | 10-16 |

### Anti-Cheat Validation

```
Server generates floor → SHA3-256 hash of canonical tile array
Client generates floor locally → submits layout
Server computes hash of client layout
  ✅ Match → Client trusted (saves bandwidth)
  ❌ Mismatch → Force server floor, flag client
```

---

## Protobuf Schema

**Location**: `shared/proto/game_state.proto`

### Core Messages

```protobuf
package tower.game;

message Vec3 { float x, y, z; }
message Rotation { float x, y, z, w; }

message PlayerData {
    uint64 id = 1;
    Vec3 position = 2;
    float health = 5;
    float max_health = 6;
    uint32 current_floor = 7;
    bool in_combat = 10;
}

message ChunkData {
    uint64 seed = 1;
    uint32 floor_id = 2;
    repeated FloorTileData tiles = 3;
    bytes validation_hash = 4;
    uint32 width = 5;
    uint32 height = 6;
    Vec3 world_offset = 8;
    SemanticTags semantic_tags = 9;
}

message FloorTileData {
    uint32 tile_type = 1;
    int32 grid_x = 2;
    int32 grid_y = 3;
    uint32 biome_id = 4;
    bool is_walkable = 5;
    bool has_collision = 6;
}

message WorldSnapshot {
    uint64 tick = 1;
    repeated EntitySnapshot players = 4;
    repeated EntitySnapshot monsters = 5;
}
```

---

## UE5 Client Integration

### C++ Network Classes

**Location**: `ue5-client/Source/TowerGame/Network/`

| Class | Purpose |
|-------|---------|
| `UNetcodeClient` | UDP connection to Bevy server (renet) |
| `FBincodeSerializer` | Binary serialization (bincode format) |
| `AReplicationManager` | Entity spawn/update from server state |
| `UTowerNetworkSubsystem` | Game instance subsystem for networking |
| `UGRPCClientManager` | HTTP API client (JSON-over-HTTP) |
| `UStateSynchronizer` | Client prediction + server reconciliation |
| `UProtobufBridge` | Protobuf serialization bridge |

### Transport Modes (GRPCClientManager)

```cpp
enum class ETransportMode : uint8 {
    GRPC,  // JSON-over-HTTP (mirrors proto service paths)
    JSON,  // Plain JSON without gRPC framing
    FFI,   // Direct DLL calls to tower_core.dll
};

struct FGRPCConfig {
    FString Host = TEXT("127.0.0.1");
    int32 Port = 50051;
    float TimeoutSeconds = 10.0f;
    int32 MaxRetries = 3;
    FString FFIDllPath = TEXT("tower_core.dll");
};
```

### Coordinate Conversion

```
Rust (Bevy):  Y-up  →  (x, y, z)
UE5:          Z-up  →  (X, Y, Z)
Conversion:   UE5.X = Rust.x, UE5.Y = Rust.z, UE5.Z = Rust.y
```

---

## Authentication

### Current (Development)

```rust
// Server
ServerAuthentication::Unsecure

// Client
ClientAuthentication::Unsecure {
    server_addr: "127.0.0.1:5000",
    client_id: timestamp_ms as u64,
    user_data: None,
    protocol_id: 0,
}
```

### Planned (Production)

- Nakama session tokens (JWT, 2h expiry)
- `protocol_id` version matching (reject mismatched clients)
- `user_data` carries Nakama session for server-side validation
- Rate limiting on HTTP API endpoints

---

## Performance

### Measured Metrics (Integration Test, Session 30)

| Metric | Measured | Target |
|--------|----------|--------|
| Server startup | ~100ms | < 1s |
| LMDB init (50 templates) | ~10ms | < 100ms |
| Client connect | Instant | < 100ms |
| Replication latency | < 50ms | < 100ms |
| Tick performance | "good" (< 35ms) | < 50ms |
| Floor generation (Tier 3) | ~569µs | < 1ms |
| LRU cache hit (Tier 1) | ~4.7µs | < 10µs |

### Bandwidth Estimation

**Per Player**:
- Position updates: 24 bytes x 20 Hz = 480 B/s
- Combat events: ~50 events/s x 20 bytes = 1 KB/s
- Floor generation: 40 bytes (once per floor)
- **Total**: ~2-3 KB/s per player

**100 Players**:
- Server upload: 200-300 KB/s
- Server download: ~50 KB/s (input only)
- **Total**: ~350 KB/s bidirectional

---

## Docker Deployment

### Services

**File**: `docker-compose.yml`

```yaml
services:
  postgres:    # PostgreSQL 15 Alpine (:5432)
  nakama:      # Nakama 3.21.1 (:7349, :7350, :7351)
  bevy-server: # Custom Rust build (:5000/udp, :50051)
```

### Quick Start

```bash
# Start infrastructure
docker compose up -d postgres
# Wait for healthy, then:
docker compose up -d nakama

# Start Bevy server (local development)
cd bevy-server
DATABASE_URL="postgres://postgres:localdb@localhost:5432/tower_game" \
RUST_LOG=info \
cargo run --release

# Start test client
cd bevy-test-client
cargo run --release --bin bevy-test-client
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | (required) | PostgreSQL connection string |
| `LMDB_PATH` | `data/templates` | Embedded DB path |
| `LMDB_MAX_SIZE` | `500000000` | Max LMDB size (auto-aligned to 4096) |
| `API_PORT` | `50051` | HTTP API port |
| `RUST_LOG` | `info` | Log level (trace/debug/info/warn/error) |

---

## Testing

### Integration Test Results (Session 30)

**Full pipeline verified:**

1. PostgreSQL running in Docker (both `nakama` and `tower_game` databases)
2. Bevy server started: LMDB (50 templates), UDP :5000, physics running
3. Test client connected, player entity replicated
4. Movement input processed, position updating in real-time
5. All 221 tests passing (133 lib + 43 bin + 13 E2E + 9 semantic + 23 storage)

**Server log:**
```
LMDB template store initialized with 12 databases (476MB)
Seeded 50 total templates
Server listening on 0.0.0.0:5000 (UDP/renet)
HTTP API server running on port 50051
Player 1771321722400 connected (entity: 7v1#4294967303)
```

**Client log:**
```
Connected to server!
Replicated: 1 players, 0 monsters, 0 tiles
Player 1771321722400: pos=Vec3(-6.78, 0.0, 5.41) hp=100 floor=1
```

### Test Binaries

```bash
# Unit + integration tests
cargo test --manifest-path bevy-server/Cargo.toml

# Single client test
cargo run --release --manifest-path bevy-test-client/Cargo.toml --bin bevy-test-client

# Stress test (multiple clients)
cargo run --release --manifest-path bevy-test-client/Cargo.toml --bin stress_test -- --clients 10 --duration 60
```

---

**Document Version**: 2.0
**Author**: Claude + User (Sessions 26-30)
**Related**: `docs/ARCHITECTURE.md`, `docs/PROGRESS.md`, `docs/api-reference.md`
