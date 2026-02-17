# Tower Game - Architecture Reference

## System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                    PLAYER (Input / Display)                          │
└────────────────────────────┬────────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────────┐
│              UNREAL ENGINE 5 CLIENT (Visual Layer)                   │
│                                                                      │
│  Rendering      Animations       VFX/Particles    Sound/Music       │
│  (Cel-shading)  (Control Rig)   (Niagara)        (Spatial Audio)    │
│  UI/HUD         Camera System   Post-Processing  Input Handling     │
│                                                                      │
└────────────────────────────┬────────────────────────────────────────┘
                             │ gRPC (Protocol Buffers)
┌────────────────────────────▼────────────────────────────────────────┐
│              PROCEDURAL CORE (Rust + Bevy ECS)                       │
│                                                                      │
│  ┌──────────────┐ ┌───────────────┐ ┌────────────────────────────┐  │
│  │ Semantic      │ │ Procedural    │ │ Combat System              │  │
│  │ Graph (UPG)   │ │ Generator     │ │ - Angular hitboxes         │  │
│  │ - Tags        │ │ - WFC floors  │ │ - Timing windows           │  │
│  │ - Relations   │ │ - Monsters    │ │ - Movesets FSM             │  │
│  │ - Cosine sim  │ │ - Loot tables │ │ - Resource management      │  │
│  └──────────────┘ └───────────────┘ └────────────────────────────┘  │
│  ┌──────────────┐ ┌───────────────┐ ┌────────────────────────────┐  │
│  │ Economy       │ │ Faction       │ │ World Systems              │  │
│  │ - Crafting    │ │ - Relations   │ │ - Breath of Tower          │  │
│  │ - Order book  │ │ - Diplomacy   │ │ - Semantic contagion       │  │
│  │ - Taxes       │ │ - Wars        │ │ - Time layers              │  │
│  └──────────────┘ └───────────────┘ └────────────────────────────┘  │
│                                                                      │
└────────────────────────────┬────────────────────────────────────────┘
                             │ Nakama API (WebSocket)
┌────────────────────────────▼────────────────────────────────────────┐
│              NAKAMA SERVER (Authoritative)                            │
│                                                                      │
│  Matchmaking    Auth         Storage      Leaderboards              │
│  Anti-cheat     State Sync   Replays      Social (friends, guilds)  │
│                                                                      │
└────────────────────────────┬────────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────────┐
│              DATABASE LAYER                                          │
│                                                                      │
│  PostgreSQL (Nakama data)    FoundationDB (World state)              │
│  MinIO (Assets)              ChromaDB (Semantic vectors)             │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Core Game Systems

### 1. Unified Procedural Graph (UPG)

The foundation of the entire game world. All entities are generated from a single `tower_seed`.

```
tower_seed (u64, 8 bytes)
    │
    ├── floor_hash = SHA3(tower_seed XOR floor_id)
    │       ├── layout (WFC with semantic constraints)
    │       ├── monster_pool = sample(floor_hash)
    │       │       └── loot_table = sample(monster_hash)
    │       ├── events (triggers based on semantic density)
    │       └── environmental_tags [biome, corruption, element]
    │
    └── mutations (deltas from base generation)
            ├── killed_monsters
            ├── opened_chests
            ├── player_echoes
            └── architectural_interventions
```

**Storage**: 1000 floors = tower_seed (8 bytes) + ~50KB mutations

### 2. 3-Tier Caching Architecture (LMDB + LRU)

Optimized floor generation caching with 3 tiers for maximum performance.

```
CLIENT REQUEST (floor_id)
    ↓
┌──────────────────────────────────────────────────────────────┐
│ TIER 1: LRU RAM Cache                         4.74µs         │
│ - Capacity: 100 floors (hot cache)                           │
│ - Hit rate: ~90%                                             │
│ - Implementation: lru = "0.12" crate                         │
└────────────────────┬─────────────────────────────────────────┘
                     │ MISS (10%)
                     ▼
┌──────────────────────────────────────────────────────────────┐
│ TIER 2: LMDB Persistent Cache                 339µs          │
│ - Embedded database (heed = "0.20")                         │
│ - Capacity: Unlimited (disk-bound)                          │
│ - Hit rate: ~9% (90% of misses)                             │
│ - ACID transactions, memory-mapped I/O                      │
│ - Performance: 1.68x faster than generation                 │
└────────────────────┬─────────────────────────────────────────┘
                     │ MISS (1%)
                     ▼
┌──────────────────────────────────────────────────────────────┐
│ TIER 3: CPU Generation (Baseline)             569µs          │
│ - Procedural generation from tower_seed                     │
│ - WFC + semantic tags + deterministic RNG                   │
│ - Protobuf serialization (ChunkData)                        │
│ - Auto-populate Tier 1 & Tier 2 on generation              │
└──────────────────────────────────────────────────────────────┘

Average Latency (with 90/9/1 hit distribution):
  = (0.90 × 4.74µs) + (0.09 × 339µs) + (0.01 × 569µs)
  = 4.27µs + 30.5µs + 5.69µs
  = 40.5µs per request

Improvement vs 2-tier (no LMDB): 37% faster, 51% higher throughput
```

**Why LMDB over Redis?**
- **3.7x faster**: 339µs vs 1.27ms (Redis tested and rejected)
- **Embedded**: No separate server process (simpler deployment)
- **Zero-copy**: Memory-mapped file access (no network serialization)
- **ACID**: Crash-safe persistence with transactions

**Benchmark Results** (Session 28, 2026-02-16):
| Cache | Time | vs Generation | Status |
|-------|------|---------------|--------|
| LRU RAM | 4.74µs | 120x faster | ✅ Tier 1 |
| LMDB | 339µs | 1.68x faster | ✅ Tier 2 |
| Redis | 1.27ms | 2.23x slower | ❌ Removed |
| Generation | 569µs | 1.00x | Baseline |

**Implementation**: `bevy-server/src/lmdb_cache.rs`, `async_generation.rs`

#### Performance Metrics (Real-time Monitoring)

The FloorGenerator tracks comprehensive metrics for all 3 cache tiers using atomic counters:

```rust
pub struct CacheStats {
    pub tier1_hits: u64,      // LRU cache hits
    pub tier2_hits: u64,      // LMDB cache hits
    pub tier3_gens: u64,      // CPU generations (misses)
    pub total_requests: u64,  // tier1 + tier2 + tier3
    pub size: usize,          // Current LRU size
    pub capacity: usize,      // LRU capacity
    pub lmdb_enabled: bool,   // LMDB availability
}
```

**Available Metrics**:
- `tier1_hit_rate()`: Percentage of requests served from LRU RAM (target: ~90%)
- `tier2_hit_rate()`: Percentage of requests served from LMDB (target: ~9%)
- `overall_hit_rate()`: Combined Tier 1 + Tier 2 hit rate (target: ~99%)
- `miss_rate()`: Percentage requiring CPU generation (target: ~1%)
- `fill_percent()`: LRU cache utilization

**Usage**:
```rust
let stats = generator.cache_stats();
println!("{}", stats.summary());  // Human-readable report

// Output example:
// Cache Stats:
// - LRU: 98/100 (98.0% filled)
// - Tier 1 (LRU):    450 hits (90.0%)
// - Tier 2 (LMDB):   45 hits (9.0%)
// - Tier 3 (Gen):    5 misses (1.0%)
// - Overall:         99.0% hit rate
// - Total requests:  500
```

**Monitoring Integration**: Metrics can be exported to Prometheus/Grafana via the stats API for production monitoring.

**Implementation**: `bevy-server/src/async_generation.rs` (lines 75-83, 314-376)

### 3. Semantic Tag System

**"Procedural Semantic Fabric"** - the core interconnection layer for all game content.

Every game entity (floor, monster, item, ability) has a semantic tag vector:

```rust
pub struct SemanticTags {
    pub tags: Vec<(String, f32)>,  // Tag name → weight (0.0-1.0)
}

// Example: Fire floor
SemanticTags::from_pairs(vec![
    ("fire", 0.9),
    ("heat", 0.8),
    ("danger", 0.7),
]);
```

**Core Operations**:
- **Cosine Similarity**: Measure alignment between tag sets (-1.0 to 1.0)
- **Blending**: Weighted average for emergent effects (fire + water = steam)
- **Normalization**: Convert to probability distribution
- **Domain Mapping**: 21 mastery domains → semantic profiles

**Interactions**: Computed via cosine similarity between tag vectors.
- Fire ability in fire floor: `similarity > 0.8` → +50% damage bonus
- Water ability in fire floor: `similarity ≈ 0.0` → Normal damage
- Fire ability in ice floor: `similarity < 0.0` → -40% penalty

**Floor Tagging**: Floors automatically tagged based on:
- **Biome** (plains, forest, desert, mountains, ice, volcano, void)
- **Progression** (difficulty 0.3→1.0, corruption 0.0→0.8)
- **Random flavor** (treasure, combat, puzzle - 20% each)

**Mastery Domains** (21 total):
- Weapon (7): Sword, Axe, Spear, Bow, Staff, Fist, Dual Wield
- Combat (5): Parry, Dodge, Counter, Combo, Positioning
- Crafting (3): Smithing, Alchemy, Cooking
- Gathering (3): Mining, Herbalism, Logging
- Other (3): Exploration, Corruption Resistance, Social

**Implementation**: `bevy-server/src/semantic_tags.rs` (630 lines)
**Documentation**: [SEMANTIC_TAG_SYSTEM.md](SEMANTIC_TAG_SYSTEM.md)
**Tests**: 12 unit tests + 10 integration tests

### 4. Combat Architecture

```
INPUT PROCESSING
    │
    ▼
┌─────────────────┐    ┌──────────────────┐
│ Timing System   │───▶│ Quality Score    │
│ (80-120ms parry)│    │ (0.0 - 1.0)     │
└────────┬────────┘    └────────┬─────────┘
         │                      │
         ▼                      ▼
┌─────────────────┐    ┌──────────────────┐
│ Physics Engine  │    │ Damage Calc      │
│ (bevy_rapier3d) │    │ Angle multiplier │
│ Angular hitboxes│    │ Quality bonus    │
└────────┬────────┘    └────────┬─────────┘
         │                      │
         ▼                      ▼
┌─────────────────┐    ┌──────────────────┐
│ State Machine   │    │ Resource Manager │
│ (FSM per weapon)│    │ Kinetic/Thermal  │
│ Combo chains    │    │ Semantic/Rage    │
└────────┬────────┘    └────────┬─────────┘
         │                      │
         └───────────┬──────────┘
                     ▼
            ┌────────────────┐
            │ Feedback System│
            │ Visual + Audio │
            │ + Haptic       │
            └────────────────┘
```

**Angle Multipliers**: Front 1.0x, Side 0.7x, Back 1.5x
**Parry Window**: 80ms (perfect) / 120ms (good) / miss
**Combo**: Light → Light → Heavy (interrupt = 1.2s vulnerability)

### 5. Breath of the Tower (World Cycle)

| Phase | Duration | Monster Behavior | Ability Effect | Tactical Value |
|-------|----------|-----------------|----------------|----------------|
| Inhale | 6h | Passive, avoid players | +20% recovery | Exploration & crafting |
| Hold | 4h | Swarm key points | +30% damage, -40% recovery | Mass boss attacks |
| Exhale | 6h | Aggressive, hunt players | -30% recovery | Defensive tactics |
| Pause | 2h | Reality cracks appear | Portals to hidden floors | High-risk operations |

### 6. Faction Relations (4-Component Model)

```
Relations(A, B) = {
    military_tension:    0.0 → 1.0,  // Griefers increase this
    economic_dependency: 0.0 → 1.0,  // Trade increases this
    cultural_influence:  0.0 → 1.0,  // Art/lore sharing
    ideological_proximity: 0.0 → 1.0 // Shared values
}

War triggers when: military_tension > 0.85 AND 60% faction vote approves
```

### 6. Economy Architecture

```
MONEY SOURCES                    MONEY SINKS
┌─────────────────┐             ┌─────────────────┐
│ Government (40%)│             │ Progressive Tax  │
│ Craft Sales(25%)│             │ (<1000: 0.5%    │
│ Market Fees(20%)│             │  >10000: 3.0%)  │
│ Credit    (15%) │             │ Credit Fee (5%)  │
└─────────────────┘             │ Item Decay       │
                                │ Tower Absorption │
                                └─────────────────┘

KEY RULE: Monsters drop ONLY resources. Equipment crafted ONLY by players.
```

### 7. Death & Echo System

```
Player Death
    │
    ├── Option A: Quick Respawn (free, lose 30% progress)
    ├── Option B: Ritual Respawn (cost, keep progress)
    └── Option C: Become Echo (60s replay for other players)
            │
            ├── Ghost trail of last 60 seconds
            ├── Used abilities visible
            ├── Tactical hints for future players
            └── Decays after 24 hours
```

### 8. Aerial Combat

```
ALTITUDE LAYERS
    1000m ┌──────────────────┐ Tower Portals (unique challenges)
         │ Eshelon Layer    │ +unique abilities
    600m  ├──────────────────┤
         │ Strategic Layer  │ +25% maneuverability
    200m  ├──────────────────┤
         │ Ground Layer     │ Basic movement
      0m  └──────────────────┘

Height advantage: +30% damage attacking downward
Cylinder sharding: each layer is a separate shard for networking
```

---

## Data Flow

### Player Action Flow
```
1. Player Input (keyboard/mouse/gamepad)
2. → Input Buffer (bevy_input, timing check)
3. → Combat System (quality score 0.0-1.0)
4. → Physics Engine (collision detection, angular hitboxes)
5. → ECS State Update (damage, effects, resources)
6. → Animation/VFX trigger (Bevy → gRPC → Unreal)
7. → Network Sync (Bevy → Nakama → other clients)
8. → Server Validation (Nakama authoritative check)
9. → State Reconciliation (if desync detected)
```

### Procedural Generation Flow
```
1. tower_seed loaded from server
2. → Floor Generator (WFC + semantic constraints)
3. → Monster Spawner (grammar: size × element × corruption × faction)
4. → Loot Table Generator (semantic drops from monster tags)
5. → Event Trigger Setup (7 semantic trigger types)
6. → Hash Computation (for client-server validation)
7. → Delta Application (mutations from server)
8. → Visual Representation sent to UE5 client
```

---

## Key Technical Constraints

| Constraint | Value | Reason |
|-----------|-------|--------|
| Parry timing precision | 1ms | Skill-based combat requires frame-perfect input |
| Floor generation time | < 50ms | Real-time generation as player explores |
| Network latency (p95) | < 50ms | Action combat needs low latency |
| Max entities per floor | 10,000 | Bevy ECS handles this in 16-64 bytes each |
| Seed + Delta per 1000 floors | ~50KB | Bandwidth-efficient replication |
| Fixed-point precision | 16.16 | Deterministic cross-platform simulation |
| Max concurrent players per shard | 500 | Nakama + sharding architecture |

---

## File Naming Conventions

| Component | Convention | Example |
|-----------|-----------|---------|
| Rust source | snake_case.rs | semantic_graph.rs |
| Bevy components | PascalCase struct | SemanticTags |
| Bevy systems | snake_case_system | combat_timing_system |
| UE5 classes | A/U prefix PascalCase | ATowerCharacter |
| Proto files | snake_case.proto | game_state.proto |
| Config files | kebab-case.ron/.yaml | tower-config.ron |
| Blender files | PascalCase.blend | FireMonster.blend |
