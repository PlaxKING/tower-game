# Tower Game - Architectural Decisions Log

## Decision Index

| ID | Date | Title | Status |
|----|------|-------|--------|
| DEC-001 | 2026-02-13 | Hybrid Architecture (UE5 + Bevy + Nakama) | Accepted |
| DEC-002 | 2026-02-13 | VS Code as primary IDE | Accepted |
| DEC-003 | 2026-02-13 | Protocol Buffers as canonical data format | Accepted |
| DEC-004 | 2026-02-13 | Bevy ECS for procedural core | Accepted |
| DEC-005 | 2026-02-13 | Seed + Delta replication model | Accepted |
| DEC-006 | 2026-02-15 | Bevy replicon + renet for networking | Accepted |
| DEC-007 | 2026-02-16 | 3-tier floor caching (RAM → LMDB → CPU) | Accepted |
| DEC-008 | 2026-02-16 | LMDB for static templates, PostgreSQL for mutable player data | Accepted |
| DEC-009 | 2026-02-16 | Rapier headless mode requires AssetPlugin + ScenePlugin | Accepted |
| DEC-010 | 2026-02-17 | Docker PostgreSQL on port 5433 | Accepted |
| DEC-011 | 2026-02-17 | sqlx::raw_sql for migrations | Accepted |
| DEC-012 | 2026-02-17 | Nakama match handlers as return table | Accepted |
| DEC-013 | 2026-02-17 | Lock-free AtomicU64 metrics (no external crate) | Accepted |
| DEC-014 | 2026-02-17 | Separate CI jobs for PostgreSQL-requiring tests | Accepted |
| DEC-015 | 2026-02-17 | Server adapts protocol to match UE5 expectations | Accepted |
| DEC-016 | 2026-02-17 | Benchmark regression detection at 130% threshold | Accepted |

---

## Decision Details

### DEC-001: Hybrid Architecture (UE5 + Bevy + Nakama)

- **Date**: 2026-02-13
- **Status**: Accepted
- **Context**: Need to build a procedural MMORPG with anime-style graphics, complex procedural generation, and multiplayer networking
- **Options Considered**:
  1. Pure Unreal Engine (ООП, heavy runtime overhead for procedural content)
  2. Pure Bevy (lightweight ECS, but limited graphics capabilities for anime style)
  3. Hybrid: UE5 (visual) + Bevy (logic) + Nakama (server)
- **Decision**: Option 3 - Hybrid architecture
- **Rationale**:
  - UE5 provides Niagara, Nanite, Lumen for high-quality anime rendering
  - Bevy ECS is ideal for procedural generation (flat memory, 50ms per 10k tiles vs 300ms in UE5)
  - Nakama provides ready-made matchmaking, leaderboards, social features
  - Clear separation of concerns: visual/logic/server
- **Risks**:
  - Integration complexity (gRPC bridge between layers)
  - Different tick rates (UE5: 60fps, Bevy: 120fps, Nakama: 30-60tps)
  - Dual codebase maintenance (C++ + Rust)
- **Mitigation**:
  - Protocol Buffers as canonical data format
  - Client-side interpolation for tick rate differences
  - Clear domain boundaries via DDD

---

### DEC-002: VS Code as primary IDE

- **Date**: 2026-02-13
- **Status**: Accepted
- **Context**: Need a single IDE that supports Rust, C++, Lua, Protobuf, Docker, Blender scripts
- **Decision**: VS Code with extensions for all toolchains
- **Rationale**:
  - rust-analyzer provides excellent Rust support
  - C/C++ extension + compile_commands.json for UE5
  - Lua, Docker, YAML, Protobuf extensions available
  - Unified tasks.json for cross-project builds
  - Lightweight compared to Visual Studio + separate Rust IDE
- **Limitations**:
  - UE5 Blueprint editing still requires Unreal Editor
  - C++ IntelliSense may be slower than Visual Studio for large UE5 projects
- **Mitigation**: Use Unreal Editor for Blueprint/level design, VS Code for all code editing

---

### DEC-003: Protocol Buffers as canonical data format

- **Date**: 2026-02-13
- **Status**: Accepted
- **Context**: Need to serialize game state between Rust, C++, and Go (Nakama) with backward compatibility
- **Decision**: Protocol Buffers v3 with gRPC
- **Rationale**:
  - Single schema definition generates code for all languages
  - Binary format is compact (important for network traffic)
  - Backward compatibility through optional fields
  - gRPC provides typed RPC framework
- **Alternatives rejected**:
  - FlatBuffers: faster but less tooling
  - MessagePack: no schema, harder to maintain cross-language consistency
  - JSON: too verbose for game state sync

---

### DEC-004: Bevy ECS for procedural core

- **Date**: 2026-02-13
- **Status**: Accepted
- **Context**: Procedural generation of 1000+ floors requires efficient entity management
- **Decision**: Bevy ECS (Entity-Component-System) architecture
- **Rationale**:
  - Flat memory layout: 16-64 bytes per component vs 50-100MB per UE5 actor
  - Native parallelism via rayon
  - Hot-reload for procedural rules (.ron files)
  - Semantic tags map naturally to ECS components
  - 1000 floors = ~5MB in Bevy vs ~5-10GB in UE5

---

### DEC-005: Seed + Delta replication model

- **Date**: 2026-02-13
- **Status**: Accepted
- **Context**: Need to synchronize procedural world state across clients with minimal bandwidth
- **Decision**: Server stores tower_seed (4 bytes) + mutation deltas. Clients regenerate world locally.
- **Rationale**:
  - 1000 floors = ~50KB (seed + deltas) vs GB of static content
  - Deterministic generation ensures all clients see identical worlds
  - Only mutations (killed monsters, opened chests) need to be synced
  - Validation via hash comparison between client and server

---

### DEC-006: Bevy replicon + renet for networking

- **Date**: 2026-02-15
- **Status**: Accepted
- **Context**: Needed authoritative server networking for multiplayer
- **Decision**: Use bevy_replicon 0.30 + bevy_replicon_renet 0.7 for ECS replication over UDP
- **Rationale**:
  - Native Bevy ECS integration, automatic component replication
  - Built-in client prediction
  - UDP transport via renet with netcode protocol
- **Alternatives rejected**:
  - Custom WebSocket: Too much custom code for ECS sync
  - Nakama realtime: Not designed for ECS component replication
  - Raw UDP: Would require reimplementing reliability, ordering, and replication
- **Session**: 28

---

### DEC-007: 3-tier floor caching (RAM → LMDB → CPU)

- **Date**: 2026-02-16
- **Status**: Accepted
- **Context**: Floor generation via WFC is expensive, need fast access for repeated visits
- **Decision**: RAM LRU cache (hot floors) → LMDB on disk (warm floors, 50 templates) → CPU WFC generation (cold floors)
- **Rationale**:
  - Minimizes regeneration
  - LMDB provides zero-copy reads
  - RAM cache for sub-millisecond access
- **Session**: 29

---

### DEC-008: LMDB for static templates, PostgreSQL for mutable player data

- **Date**: 2026-02-16
- **Status**: Accepted
- **Context**: Need storage for both game templates and player data
- **Decision**: LMDB (embedded, zero-copy) for read-only templates (monsters, items, abilities, recipes, loot tables, quests, factions). PostgreSQL for all player-mutable data (18 tables).
- **Rationale**:
  - LMDB is single-writer/multi-reader, perfect for read-heavy template access
  - PostgreSQL handles concurrent writes, transactions, and complex queries for player data
- **Session**: 29

---

### DEC-009: Rapier headless mode requires AssetPlugin + ScenePlugin

- **Date**: 2026-02-16
- **Status**: Accepted
- **Context**: bevy_rapier3d 0.28 crashed in headless server (no renderer)
- **Decision**: Add AssetPlugin::default() + ScenePlugin + init_asset::<Mesh>() to headless Bevy app
- **Rationale**:
  - Rapier's init_async_scene_colliders system requires Assets<Mesh> and SceneSpawner resources, even when no rendering occurs
  - These plugins provide the required resources without enabling rendering
- **Session**: 30

---

### DEC-010: Docker PostgreSQL on port 5433

- **Date**: 2026-02-17
- **Status**: Accepted
- **Context**: Local Windows PostgreSQL on port 5432 conflicted with Docker container
- **Decision**: Map Docker postgres to host port 5433 (container still uses 5432 internally)
- **Rationale**:
  - Avoids port conflict without modifying either PG installation
  - Docker inter-container communication uses internal port 5432 unaffected
- **Session**: 31

---

### DEC-011: sqlx::raw_sql for migrations

- **Date**: 2026-02-17
- **Status**: Accepted
- **Context**: Multi-statement SQL migrations failed with prepared statements
- **Decision**: Use sqlx::raw_sql() instead of sqlx::query() for migration execution
- **Rationale**:
  - raw_sql sends SQL directly without prepared statement wrapping
  - Allows multi-statement execution needed for schema migrations
- **Session**: 31

---

### DEC-012: Nakama match handlers as return table

- **Date**: 2026-02-17
- **Status**: Accepted
- **Context**: Nakama Lua runtime match handler registration
- **Decision**: Match handler modules return a table of callbacks (match_init, match_join, etc.) instead of calling nk.register_match()
- **Rationale**:
  - Nakama Lua API requires modules to export handlers via return, not registration
  - Matches are created via nk.match_create("module_name", params) from the main module
- **Session**: 31

---

### DEC-013: Lock-free AtomicU64 metrics (no external crate)

- **Date**: 2026-02-17
- **Status**: Accepted
- **Context**: Server needs request metrics for monitoring and load testing, but adding `metrics` + `metrics-exporter-prometheus` crates adds compilation overhead
- **Decision**: Implement lightweight metrics using `AtomicU64` with `Ordering::Relaxed` and manual Prometheus text formatting
- **Rationale**:
  - Zero runtime overhead (no lock contention)
  - No new dependencies
  - Sufficient for current needs (total_requests, total_errors, total_duration_us, rps)
  - Manual Prometheus format is simple for 6 metrics
- **Session**: 32

---

### DEC-014: Separate CI jobs for PostgreSQL-requiring tests

- **Date**: 2026-02-17
- **Status**: Accepted
- **Context**: API smoke tests and UE5 contract tests need PostgreSQL, but windows-latest runners can't easily run Docker services
- **Decision**: Create `bevy-server-integration` job on ubuntu-latest with PostgreSQL service container (port 5433)
- **Rationale**:
  - GitHub Actions service containers only work on Linux runners
  - Separating PG-dependent tests from lib tests provides faster feedback for pure Rust changes
  - Service container uses same port mapping (5433) as local development
- **Session**: 32

---

### DEC-015: Server adapts protocol to match UE5 expectations

- **Date**: 2026-02-17
- **Status**: Accepted
- **Context**: Protocol mismatches between server responses and UE5 GRPCClientManager parsing (wallet: `honor_points` vs `seasonal_currency`, loot: missing `item_name/socket_count/tags`)
- **Decision**: Fix mismatches on the server side by adding aliased fields
- **Rationale**:
  - UE5 is harder to change (requires Unreal Engine installed, longer compile times)
  - Server can return both old and new field names for backwards compatibility
  - Contract tests validate alignment automatically
- **Session**: 32

---

### DEC-016: Benchmark regression detection at 130% threshold

- **Date**: 2026-02-17
- **Status**: Accepted
- **Context**: Need automated performance regression detection for floor generation benchmarks
- **Decision**: Use `benchmark-action/github-action-benchmark@v1` with 130% alert threshold (CI fails if benchmark exceeds 130% of baseline)
- **Rationale**:
  - 30% tolerance accounts for CI runner variability
  - Runs only on main branch pushes (not PRs) to reduce noise
  - GitHub Pages stores benchmark history for trend analysis
- **Session**: 32

---

## Template for new decisions:
```
### DEC-XXX: [Title]

- **Date**: YYYY-MM-DD
- **Status**: Proposed / Accepted / Deprecated / Superseded by DEC-YYY
- **Context**: Why was this decision needed?
- **Options Considered**:
  1. Option A
  2. Option B
- **Decision**: Which option was chosen
- **Rationale**: Why this option
- **Risks**: What could go wrong
- **Mitigation**: How to handle risks
```
