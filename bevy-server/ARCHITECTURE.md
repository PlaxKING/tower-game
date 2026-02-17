# Bevy Server Architecture - Tower Game

## Overview

**Authoritative Bevy ECS server** for Tower Game multiplayer.

### Design Principles
1. **Server Authority**: All game logic runs on server
2. **Client Prediction**: UE5 predicts for responsiveness
3. **Hybrid Generation**: Client generates floors from seed, server validates with hash
4. **Dynamic Scaling**: Auto-adjust capacity based on performance

---

## Network Architecture

```
┌─────────────────────────────────────┐
│  UE5 Client (Presentation Only)    │
│  - Renders visuals                 │
│  - Predicts movement               │
│  - Sends input (20Hz)              │
│  - Receives state updates (20Hz)   │
└──────────────┬──────────────────────┘
               │ WebSocket (bevy_replicon)
               │ bincode serialization
               ▼
┌─────────────────────────────────────┐
│  Bevy Server (Authoritative)       │
│  - Headless (no rendering)         │
│  - 20 Hz tick rate (50ms)          │
│  - Server-side physics             │
│  - Combat validation               │
│  - Procedural generation           │
└──────────────┬──────────────────────┘
               │ gRPC (optional)
               ▼
┌─────────────────────────────────────┐
│  Nakama (Lobby & Persistence)      │
│  - Matchmaking                     │
│  - Player accounts                 │
│  - Leaderboards                    │
└─────────────────────────────────────┘
```

---

## Performance Characteristics

### Tick Rate: **20 Hz** (50ms per tick)
- **Why**: Skill-based combat requires responsiveness
- **Target**: < 50ms server tick time
- **Adaptive**: Reduces to 10 Hz if overloaded

### Player Capacity: **Dynamic Scaling**
- **Base**: 100 players/floor
- **Good performance** (< 35ms): Scale up to 150 players
- **Normal** (35-45ms): Maintain 100 players
- **Degraded** (45-60ms): Reduce to 80 players
- **Overload** (> 60ms): Emergency reduction to 60 players

### Bandwidth Estimation
Per player:
- **Position updates**: 24 bytes × 20 Hz = 480 B/s
- **Combat events**: ~50 events/s × 20 bytes = 1 KB/s
- **Floor generation**: 8 bytes seed + 32 bytes hash = 40 bytes (once)
- **Total**: ~2-3 KB/s per player (very low!)

100 players: **200-300 KB/s** total server bandwidth

---

## Hybrid Floor Generation

### Protocol

1. **Player joins floor**:
   ```rust
   Server → Client: FloorGenerationPacket {
       floor_id: 42,
       seed: 0x123456789ABCDEF,
       validation_hash: [SHA3-256 of canonical floor],
       tile_count: 1024,
       room_count: 8,
   }
   ```

2. **Client generates locally**:
   - WFC algorithm (deterministic from seed)
   - Same code as server (`procedural-core`)
   - Takes ~10-50ms

3. **Client submits for validation**:
   ```rust
   Client → Server: ClientFloorSubmission {
       layout: FloorLayout { ... },
       generation_time_ms: 25,
   }
   ```

4. **Server validates**:
   - Computes hash of client layout
   - Compares with validation_hash
   - ✅ **Match**: Client trusted
   - ❌ **Mismatch**: Force server floor (anti-cheat)

### Benefits
- **99% bandwidth savings**: 40 bytes vs ~500 KB
- **Anti-cheat**: Server validates integrity
- **Scalable**: Offloads generation to clients
- **Fast**: Parallel generation on all clients

---

## Component Replication Strategy

### Tier 1: Critical (20 Hz)
- `PlayerState`: position, velocity, health
- `CombatState`: current action, combo, effects

### Tier 2: Important (5 Hz)
- `Equipment`: weapon, armor, sockets
- `Monster`: type, position, health, AI state

### Tier 3: Static (once + delta)
- `FloorLayout`: seed, hash, metadata (40 bytes!)
- `FloorDelta`: only changes (chest opened, trap triggered)

---

## Systems Schedule

### Server Loop (50ms)
```rust
1. Input Processing (5ms)
   - receive_player_inputs
   - validate_inputs (anti-cheat)

2. Simulation (30ms)
   - update_player_movement
   - update_monster_ai
   - process_combat_hitboxes
   - update_status_effects

3. Generation (on-demand, 10ms)
   - generate_new_floors
   - spawn_monsters

4. Replication (5ms)
   - replicate_changed_components
   - send_combat_events
   - spatial_partitioning (only nearby entities)

5. Performance Monitoring (< 1ms)
   - monitor_tick_time
   - adjust_dynamic_scaling
```

---

## Spatial Partitioning

**Problem**: Don't send all 100 players to every player

**Solution**: Grid-based visibility culling
```rust
Grid: 50m × 50m cells
Visibility radius: 3×3 cells around player

Player sees:
- Self (always)
- Nearby players (< 100m)
- Monsters in 3×3 grid
- Floor tiles in chunk

Result: Each player receives ~20-50 entities, not 1000s
```

---

## Integration with `procedural-core`

### Shared Code
```rust
use tower_procedural_core::{
    generation::FloorGenerator,  // WFC
    monsters::MonsterGrammar,     // Monster gen
    combat::CombatCalculator,     // Damage formulas
    semantic::SemanticGraph,      // Tags
};
```

### Zero Duplication
- Server and client use **same generation code**
- Deterministic from seed
- Easy to test and debug

---

## Anti-Cheat Measures

1. **Input Validation**
   - Speed limits (max 20 units/tick)
   - Cooldown enforcement
   - Resource checks (energy costs)

2. **Floor Validation**
   - SHA3-256 hash of canonical floor
   - Detect modified floors (wallhacks)

3. **Combat Validation**
   - Server-side hitbox checks
   - Damage caps (max 200/hit)
   - Timing window validation (parry 80-120ms)

4. **State Authority**
   - Server is source of truth
   - Client predictions reconciled
   - Large discrepancies = disconnect

---

## Future Optimizations

1. **Interest Management**
   - Only replicate entities player can see
   - Reduce updates for distant players (10 Hz → 5 Hz)

2. **Delta Compression**
   - Send only changed fields
   - Binary diff for large structs

3. **Snapshot Interpolation**
   - Client smoothly interpolates between server snapshots
   - Hides network jitter

4. **Lag Compensation**
   - Server rewinds time for hit detection
   - Fair combat despite ping differences

---

## Performance Targets

| Metric | Target | Acceptable | Critical |
|--------|--------|-----------|----------|
| Tick time | < 40ms | < 50ms | < 60ms |
| Players/floor | 100 | 80 | 60 |
| CPU usage | < 60% | < 80% | < 95% |
| Memory | < 2 GB | < 4 GB | < 6 GB |
| Network | < 300 KB/s | < 500 KB/s | < 1 MB/s |

---

## Deployment

### Development
```bash
cd bevy-server
cargo run --release
# Listens on 0.0.0.0:5000
```

### Production
```bash
# Headless server (no graphics)
cargo build --release
./target/release/tower-bevy-server --port 5000 --max-players 100
```

### Docker (future)
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/tower-bevy-server /usr/local/bin/
CMD ["tower-bevy-server"]
```

---

## Monitoring & Metrics

### Key Metrics to Track
- Average tick time (should be < 50ms)
- Players per floor distribution
- Network bandwidth usage
- Floor generation success rate
- Combat event rate

### Logging
```rust
info!("✅ Player {} joined floor {}", id, floor);
warn!("⚠️ Tick time {} ms (target 50ms)", time);
error!("❌ Floor validation failed for player {}", id);
```

---

## Next Steps

- [ ] Implement combat system in ECS
- [ ] Setup WebSocket server (bevy_replicon_renet)
- [ ] Create UE5 client connector
- [ ] Test 2-player connection
- [ ] Benchmark 100-player stress test
- [ ] Deploy to Nakama integration

---

**Version**: 0.1.0
**Last Updated**: 2026-02-16
**Architecture Design**: Session 26, Phase 7
