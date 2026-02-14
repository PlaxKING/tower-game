# Tower Game - Architectural Decisions Log

## Decision Index

| ID | Date | Title | Status |
|----|------|-------|--------|
| DEC-001 | 2026-02-13 | Hybrid Architecture (UE5 + Bevy + Nakama) | Accepted |
| DEC-002 | 2026-02-13 | VS Code as primary IDE | Accepted |
| DEC-003 | 2026-02-13 | Protocol Buffers as canonical data format | Accepted |
| DEC-004 | 2026-02-13 | Bevy ECS for procedural core | Accepted |
| DEC-005 | 2026-02-13 | Seed + Delta replication model | Accepted |

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
