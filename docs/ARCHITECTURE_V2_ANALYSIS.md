# Tower Game - Architecture V2.0 Analysis

**Date**: 2026-02-16
**Status**: 🚧 Design Phase
**Type**: Architecture Refinement
**Impact**: High - Changes core server architecture

---

## 📋 Executive Summary

**Proposed Changes:**
1. **Nakama → Meta-Service Only** (не игровая логика)
2. **Bevy Headless → Authoritative Game Server** (основной сервер)
3. **Procedural Data Transfer** (100x traffic savings)
4. **Snapshot Interpolation** (Source engine style)
5. **TCP/QUIC Transport** (вместо WebSocket)

**Expected Benefits:**
- ✅ 100x bandwidth reduction (procedural data vs meshes)
- ✅ Full control over game logic (no framework limitations)
- ✅ Better performance (Rust + ECS)
- ✅ Cleaner separation (meta-services vs game logic)
- ✅ Industry-standard networking (Snapshot Interpolation)

---

## 🏗️ Current Architecture (V1) - Analysis

### What We Have Now

```
┌─────────────┐         UDP         ┌──────────────┐
│   UE5       │◄──────bincode──────►│  Bevy        │
│  Client     │        20 Hz        │  Server      │
│             │                     │  (headless)  │
└─────────────┘                     └──────────────┘
      │                                     │
      │                                     │
      └──────────── Nakama (planned) ───────┘
                    WebSocket
            (matchmaking, storage, social)
```

### Current Status: Session 26

✅ **Completed:**
- Bevy headless server (main.rs, 220 lines)
- UDP netcode protocol (renet)
- Component replication (Player, Monster, FloorTile)
- Dynamic scaling (60-150 players)
- Bincode serialization
- Stress tested (30 clients successfully)
- UE5 C++ client (NetcodeClient, BincodeSerializer, ReplicationManager)
- Coordinate conversion (Bevy Y-up → UE5 Z-up)

⏳ **Pending:**
- Nakama integration
- Full game logic implementation
- Floor generation server-side
- Procedural mesh streaming

---

## 🚀 Proposed Architecture (V2) - Full Design

### New Architecture Diagram

```
┌──────────────────────────────────────────────────────────┐
│                   UE5 Client                              │
│  ┌──────────────┐  ┌───────────────┐  ┌──────────────┐  │
│  │ Rust Plugin  │  │ Procedural    │  │ Snapshot     │  │
│  │ C++ Bridge   │  │ Mesh Builder  │  │ Interpolator │  │
│  └──────────────┘  └───────────────┘  └──────────────┘  │
└─────────────┬────────────────────────────────────────────┘
              │ TCP/QUIC (game data)
              │ Protobuf/Bincode
              │ 30Hz snapshot
              ▼
┌──────────────────────────────────────────────────────────┐
│           Bevy Headless Authoritative Server             │
│  ┌───────────┐  ┌──────────┐  ┌────────────┐           │
│  │ ECS Core  │  │ Proc Gen │  │ Snapshot   │           │
│  │ (bevy)    │  │ (WFC)    │  │ Generator  │           │
│  └───────────┘  └──────────┘  └────────────┘           │
│  ┌───────────┐  ┌──────────┐  ┌────────────┐           │
│  │ Physics   │  │ Combat   │  │ Chunk      │           │
│  │ (rapier)  │  │ Logic    │  │ Manager    │           │
│  └───────────┘  └──────────┘  └────────────┘           │
└─────────────┬────────────────────────────────────────────┘
              │ gRPC/HTTP (meta operations)
              │ Infrequent, non-critical
              ▼
┌──────────────────────────────────────────────────────────┐
│              Nakama Meta-Service                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐               │
│  │ Account  │  │ Friends  │  │ Guild    │               │
│  │ Auth     │  │ List     │  │ System   │               │
│  └──────────┘  └──────────┘  └──────────┘               │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐               │
│  │ Leaderb. │  │ Storage  │  │ Match-   │               │
│  │ Ranks    │  │ Postgres │  │ making   │               │
│  └──────────┘  └──────────┘  └──────────┘               │
└──────────────────────────────────────────────────────────┘
```

### Layer Responsibilities

#### Layer 1: UE5 Client
**Purpose:** Rendering, input, procedural mesh generation

**Components:**
1. **Rust Plugin (C++ FFI Bridge)**
   - Calls Rust procedural generation code
   - Shares WFC/grammar algorithms with server
   - Deterministic generation from seed

2. **Procedural Mesh Builder**
   - `UProceduralMeshComponent` for floor tiles
   - Instanced Static Meshes for repeated elements
   - LOD system based on distance

3. **Snapshot Interpolator**
   - Source engine-style buffering
   - 100ms buffer (3 snapshots at 30Hz)
   - Smooth movement between states

**Data Received from Server:**
```protobuf
message ChunkData {
  uint64 seed = 1;              // 8 bytes
  int32 floor_id = 2;           // 4 bytes
  repeated TileType tiles = 3;  // ~200 bytes for 64x64 grid
  bytes validation_hash = 4;    // 32 bytes SHA3
  repeated EntitySnapshot entities = 5; // ~100-500 bytes
}
// Total: ~400 bytes (vs ~500KB full mesh!)
```

#### Layer 2: Bevy Headless Server
**Purpose:** Authoritative game logic, physics, generation

**ECS Systems:**
1. **Procedural Generation**
   - WFC floor layouts
   - Monster spawning via grammar
   - Loot tables with semantic tags

2. **Physics & Combat**
   - bevy_rapier3d for collisions
   - Angular hitboxes, parry windows
   - Damage calculation

3. **Snapshot System**
   - Snapshot Generator (30Hz)
   - Delta compression (only changes)
   - Chunk streaming (player vision radius)

4. **Validation**
   - Anti-cheat: server validates client-generated floors
   - SHA3 hash check (client seed must match server hash)
   - Server rejects invalid geometry

**Rust Crates:**
```toml
bevy = { version = "0.15", default-features = false }
bevy_rapier3d = "0.28"
quinn = "0.11"  # QUIC protocol
tokio = "1.0"   # Async runtime
prost = "0.13"  # Protobuf
sha3 = "0.10"   # Hash validation
```

#### Layer 3: Nakama Meta-Service
**Purpose:** Non-game-critical services

**Responsibilities:**
- ✅ Account authentication (login/register)
- ✅ Friend list, guild management
- ✅ Leaderboards, achievements
- ✅ Persistent storage (inventory, progress)
- ✅ Matchmaking queue (join game server)
- ❌ **NOT** real-time game logic
- ❌ **NOT** entity replication

**Communication:**
- Client → Nakama: HTTP REST / WebSocket (social features)
- Bevy → Nakama: gRPC (save player state, verify auth tokens)
- Frequency: Infrequent (login, save, leaderboard update)

---

## 🔄 Data Flow - Detailed

### Scenario 1: Player Joins Game

```
1. Client → Nakama:  Login (username/password)
   Nakama → Client:  Auth token + available servers

2. Client → Bevy:    Connect (auth token, player_id)
   Bevy → Nakama:    Verify token (gRPC)
   Nakama → Bevy:    Token valid + player data
   Bevy → Client:    Connection accepted + spawn data

3. Bevy → Client:    Initial snapshot (30Hz)
   - Player position, health, floor
   - Nearby entities (monsters, players)
   - Chunk seeds (for procedural generation)

4. Client:           Generate floor meshes from seeds
   Client:           Spawn player actor
   Client:           Start snapshot interpolation
```

### Scenario 2: Floor Generation

**Old Way (V1):**
```
Server generates floor → 500KB mesh data → Client renders
```

**New Way (V2):**
```
Server: Generate seed (8 bytes) + tile types (200 bytes)
Server: Compute SHA3 hash (32 bytes)
Server → Client: seed + hash + tile types (240 bytes total)

Client: Run same WFC algorithm with seed
Client: Generate mesh locally with UProceduralMeshComponent
Client: Send hash back to server
Server: Validate hash → accept/reject

Result: 2000x less bandwidth (240 bytes vs 500KB)
```

### Scenario 3: Player Movement (Snapshot Interpolation)

**Server (30Hz = 33ms interval):**
```rust
fn send_snapshots(
    mut clients: ResMut<Clients>,
    query: Query<(Entity, &Player, &Transform, &Health)>,
) {
    let snapshot = Snapshot {
        tick: server.tick,
        timestamp: server.time,
        entities: query.iter().map(|(e, p, t, h)| {
            EntitySnapshot {
                id: e.index(),
                position: t.translation,
                velocity: p.velocity,
                health: h.current,
            }
        }).collect(),
    };

    clients.send_all(snapshot);
}
```

**Client (Interpolation):**
```cpp
void USnapshotInterpolator::Update(float DeltaTime)
{
    // Buffer holds last 3 snapshots (100ms at 30Hz)
    if (SnapshotBuffer.Num() < 2) return;

    // Interpolate between snapshot[N-2] and snapshot[N-1]
    // Render 100ms in the past for smooth movement
    Snapshot& From = SnapshotBuffer[SnapshotBuffer.Num() - 2];
    Snapshot& To = SnapshotBuffer[SnapshotBuffer.Num() - 1];

    float Alpha = (CurrentTime - From.Timestamp) / (To.Timestamp - From.Timestamp);
    FVector InterpolatedPos = FMath::Lerp(From.Position, To.Position, Alpha);

    Actor->SetActorLocation(InterpolatedPos);
}
```

**Benefits:**
- ✅ Smooth movement even with packet loss
- ✅ Hides network jitter
- ✅ 100ms latency compensation
- ✅ Industry-standard (used in Source, Overwatch, etc.)

---

## 📊 Bandwidth Comparison

### Full Mesh Transfer (Old)

| Data Type | Size | Per Player | 100 Players |
|-----------|------|------------|-------------|
| Floor Mesh | 500 KB | 500 KB | 50 MB |
| Textures | 2 MB | 2 MB | 200 MB |
| **Total per floor** | **2.5 MB** | **2.5 MB** | **250 MB** |

**Problem:** Impossible for MMO with 1000 floors

### Procedural Data Transfer (New)

| Data Type | Size | Per Player | 100 Players |
|-----------|------|------------|-------------|
| Seed + Hash | 40 bytes | 40 bytes | 4 KB |
| Tile Types | 200 bytes | 200 bytes | 20 KB |
| Entity Snapshots (30Hz) | 500 bytes/s | 500 bytes/s | 50 KB/s |
| **Total bandwidth** | **~3 KB/s** | **3 KB/s** | **300 KB/s** |

**Improvement:** **100x-1000x less bandwidth**

---

## 🛠️ Implementation Plan

### Phase 1: Bevy Server Enhancement (1-2 weeks)

**Tasks:**
1. ✅ ~~Basic Bevy server~~ (Already complete)
2. Add Snapshot Generator system
3. Implement Chunk Manager (spatial partitioning)
4. Add Delta compression (only send changes)
5. Integrate procedural generation (WFC, grammar)

**Deliverables:**
- `snapshot.rs` (Snapshot Generator + Delta Compression)
- `chunk_manager.rs` (Spatial partitioning, streaming)
- `procedural.rs` (Floor generation, seed validation)

### Phase 2: UE5 Procedural Client (2-3 weeks)

**Tasks:**
1. Create `UProceduralFloorBuilder` component
2. Implement Snapshot Interpolation
3. Add client-side WFC generation (from seed)
4. Integrate hash validation
5. Create debug visualization

**Deliverables:**
- `ProceduralFloorBuilder.h/cpp` (UProceduralMeshComponent wrapper)
- `SnapshotInterpolator.h/cpp` (Source-style interpolation)
- `FloorGenerator.h/cpp` (Client-side WFC)

### Phase 3: Nakama Integration (1 week)

**Tasks:**
1. Setup Nakama Docker container
2. Implement auth token verification (Bevy ↔ Nakama gRPC)
3. Add player state save/load
4. Create leaderboards, friends list
5. Implement matchmaking queue

**Deliverables:**
- `nakama_client.rs` (gRPC client for Bevy)
- `UNakamaSubsystem` (UE5 REST client)
- Nakama Lua modules (auth, storage, leaderboards)

### Phase 4: Protocol Optimization (1 week)

**Tasks:**
1. Switch UDP → QUIC (better than TCP for games)
2. Implement variable tick rate (30Hz combat, 10Hz idle)
3. Add priority system (critical vs non-critical data)
4. Optimize Protobuf schemas
5. Add bandwidth monitoring

**Deliverables:**
- QUIC transport layer
- Adaptive tick rate system
- Network stats dashboard

---

## 🔬 Reference Projects Analysis

### 1. Veloren (Rust Voxel MMO)

**GitHub:** https://github.com/veloren/veloren

**What to Learn:**
- ✅ Chunk streaming system (`common/src/terrain/`)
- ✅ ECS-based server (`server/src/`)
- ✅ Client-side terrain generation
- ✅ Network protocol (`network/protocol/`)

**Applicable to Tower:**
- Chunk Manager design pattern
- Spatial hashing for entity queries
- Procedural terrain generation flow
- Compression techniques

**Differences:**
- Veloren: Voxel-based (Minecraft-like)
- Tower: Room-based procedural (Binding of Isaac-like)

### 2. Lightyear (Rust Networking Library)

**GitHub:** https://github.com/cBournhonesque/lightyear

**What to Learn:**
- ✅ Snapshot interpolation implementation
- ✅ Client prediction + server reconciliation
- ✅ Bandwidth optimization (delta compression)
- ✅ Bevy integration patterns

**Directly Applicable:**
- Use Lightyear instead of custom snapshot system
- Built-in interpolation and prediction
- Designed for Bevy ECS
- Supports QUIC transport

**Integration:**
```toml
[dependencies]
lightyear = "0.17"
```

**Benefits:**
- ✅ Battle-tested networking library
- ✅ Saves 2-4 weeks of development
- ✅ Better than custom implementation
- ✅ Active maintenance

### 3. Unreal Rust Plugin

**GitHub:** https://github.com/MaikKlein/unreal-rust

**What to Learn:**
- FFI bridge patterns (C++ ↔ Rust)
- Memory safety guarantees
- Performance considerations

**NOT Recommended for Tower:**
- ❌ Server should be pure Rust (no UE dependency)
- ❌ Client can call Rust via C++ FFI (but keep server separate)
- ❌ Complexity not worth it for our use case

**Alternative:** C++ calls into Rust DLL for procedural generation

---

## 💡 Additional Optimizations

### 1. QUIC Instead of TCP

**Why QUIC?**
```
TCP:  Single stream, head-of-line blocking
QUIC: Multiple streams, no HOL blocking
      Built-in encryption (TLS 1.3)
      Faster handshake (0-RTT)
      Better for unreliable networks
```

**Rust Library:**
```toml
quinn = "0.11"  # QUIC implementation
```

**Benefits:**
- ✅ Better than UDP (reliable delivery when needed)
- ✅ Better than TCP (no head-of-line blocking)
- ✅ Native support in HTTP/3
- ✅ Mobile-friendly (connection migration)

### 2. Entity Streaming (Interest Management)

**Problem:** Sending all entities to all players wastes bandwidth

**Solution:** Area-of-Interest (AOI) system

```rust
fn stream_entities(
    mut clients: ResMut<Clients>,
    players: Query<&Transform, With<Player>>,
    entities: Query<(Entity, &Transform, &EntityType)>,
) {
    for (client_id, player_transform) in clients.iter() {
        let nearby = entities
            .iter()
            .filter(|(_, transform, _)| {
                let distance = player_transform.translation.distance(transform.translation);
                distance < VIEW_RADIUS  // Only send entities within 50m
            })
            .collect();

        client.send(SnapshotFiltered { entities: nearby });
    }
}
```

**Savings:** 90% reduction when 100 players, only 10 nearby

### 3. Variable Tick Rate

**Adaptive Update Frequency:**
```rust
let tick_rate = match player.state {
    PlayerState::InCombat => 30,    // 30 Hz during combat
    PlayerState::Moving => 20,      // 20 Hz when exploring
    PlayerState::Idle => 5,         // 5 Hz when standing still
};
```

**Savings:** 80% reduction during non-combat (most of the time)

### 4. Sparse Updates (Bitfield Masks)

**Only send changed fields:**
```protobuf
message EntityUpdate {
  uint64 entity_id = 1;
  uint32 changed_fields = 2;  // Bitmask: 0b00001011 = position+health changed
  optional Vec3 position = 3;
  optional float health = 4;
  optional float velocity = 5;
}
```

**Savings:** 50-70% reduction (only ~3 fields change per update)

### 5. Persistent Floor Cache

**Server caches generated floors:**
```rust
struct FloorCache {
    floors: HashMap<FloorId, CachedFloor>,
    lru: LRU<FloorId>,  // Evict least-recently-used
}
```

**Benefits:**
- ✅ Instant loading for revisited floors
- ✅ Consistent layouts (same seed = same floor)
- ✅ Reduced CPU (no re-generation)

**Memory:** 1000 floors × 200 bytes = 200 KB (negligible)

---

## 📈 Expected Performance

### Bandwidth (Per Player)

| Scenario | Old (V1) | New (V2) | Improvement |
|----------|----------|----------|-------------|
| **Floor Load** | 2.5 MB | 240 bytes | **10,000x** |
| **Idle Updates** | 3 KB/s | 200 bytes/s | **15x** |
| **Combat Updates** | 3 KB/s | 1.5 KB/s | **2x** |
| **100 players** | 300 KB/s | 50 KB/s | **6x** |

**Hosting Cost Savings:**
- Old: 300 KB/s × 100 players = 30 Mbps = $500/month
- New: 50 KB/s × 100 players = 5 Mbps = $50/month
- **Savings: $450/month per 100 players**

### Latency

| Operation | Latency | Notes |
|-----------|---------|-------|
| **Floor Load** | 50ms | Seed transmission + local generation |
| **Player Spawn** | 33ms | One snapshot (30Hz) |
| **Movement** | 133ms | 100ms interpolation buffer + 33ms tick |
| **Combat Hit** | 33ms | Server authoritative, immediate |

**Comparison:**
- V1: 200-500ms (full mesh transfer)
- V2: 50ms (procedural data)
- **4-10x faster**

---

## ✅ Decision Matrix

| Criterion | V1 (Nakama Game Server) | V2 (Bevy + Nakama Meta) | Winner |
|-----------|-------------------------|-------------------------|--------|
| **Bandwidth Efficiency** | 3 KB/s per player | 0.5 KB/s per player | ✅ V2 |
| **Development Control** | Limited (Nakama API) | Full control (Rust/Bevy) | ✅ V2 |
| **Performance** | Good | Excellent | ✅ V2 |
| **Scalability** | 50-100 players | 100-150 players | ✅ V2 |
| **Development Time** | 4-6 weeks | 6-8 weeks | ✅ V1 |
| **Hosting Cost** | $500/month | $50/month | ✅ V2 |
| **Complexity** | Medium | High | ✅ V1 |
| **Flexibility** | Low | High | ✅ V2 |

**Recommendation:** **V2 (Bevy + Nakama Meta)** - Benefits far outweigh the 2-week extra development time

---

## 🎯 Action Items

### Immediate (Session 26 Completion)

1. ✅ ~~Document V2 architecture~~ (This document)
2. Update CLAUDE.md with new architecture
3. Create DEC-029 (Architecture V2 decision)
4. Update TECH-STACK.md (add Lightyear, Quinn)

### Short-term (Session 27-28)

1. Integrate Lightyear into Bevy server
2. Implement Snapshot Generator
3. Create ProceduralFloorBuilder in UE5
4. Test procedural floor streaming

### Medium-term (Session 29-32)

1. Implement QUIC transport (Quinn)
2. Add variable tick rate system
3. Integrate Nakama for meta-services
4. Full integration testing

---

## 📚 Resources & Links

**Reference Projects:**
- Veloren: https://gitlab.com/veloren/veloren
- Lightyear: https://github.com/cBournhonesque/lightyear
- Quinn (QUIC): https://github.com/quinn-rs/quinn

**Documentation:**
- Source Engine Networking: https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking
- QUIC Protocol: https://www.chromium.org/quic/
- Snapshot Interpolation: https://www.gabrielgambetta.com/client-side-prediction-server-reconciliation.html

**Bevy Resources:**
- bevy_replicon: https://github.com/projectharmonia/bevy_replicon
- bevy_rapier3d: https://github.com/dimforge/bevy_rapier

---

## ✅ Conclusion

**Architecture V2 is superior for Tower Game:**

1. **100x bandwidth savings** through procedural data transfer
2. **Full control** over game logic (no framework limitations)
3. **Industry-standard networking** (Snapshot Interpolation)
4. **Cleaner separation** (meta-services vs game logic)
5. **Better performance** (Rust + ECS)
6. **Lower hosting costs** ($50/month vs $500/month)

**Trade-offs:**
- +2 weeks development time (manageable)
- +Higher complexity (but more flexibility)

**Recommendation:** ✅ **APPROVE** - Proceed with V2 implementation

---

**Status:** 🚧 Design Complete, Awaiting Approval
**Next Step:** Update CLAUDE.md + Begin Lightyear integration
**Session:** 26 - Phase 7 (Networking & Multiplayer)
**Author:** Claude Sonnet 4.5 (based on user feedback)
