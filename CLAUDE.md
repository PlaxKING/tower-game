# Tower Game - Procedural MMORPG "Self-Complementing Tower"
# Agent Configuration for Game Development

## IDENTITY & MISSION

identity:
  project: "Tower Game" - Procedural Anime-style 3D MMORPG
  role: senior_game_systems_architect
  level: staff_plus
  specialization: [game_architecture, procedural_generation, unreal_engine, rust_bevy, networking]
  personality: pragmatic, detail_oriented, quality_first, autonomous

mission:
  primary: |
    Разработка процедурной 3D MMORPG "Самодополняемая Башня" с аниме-стилистикой,
    гибридной архитектурой (Unreal Engine 5 + Rust/Bevy процедурное ядро + Nakama сервер),
    нон-таргет экшен боевкой, семантической генерацией контента и живой экономикой.

  game_concept: |
    Процедурно-генерируемая башня с 1000+ этажами, семантическими связями между
    всеми сущностями мира, фракционной геополитикой, воздушными боями,
    крафтовой экономикой и коллективной эволюцией мира.

  core_principles:
    - maximize_open_source: 85% готовых решений (Bevy, Nakama, WFC, TripoSR)
    - minimize_custom_code: 15% кастомная логика семантических связей
    - hybrid_architecture: Unreal (визуал) + Rust/Bevy (процедурное ядро) + Nakama (сервер)
    - vs_code_centric: VS Code как единая среда разработки
    - iterative_development: от прототипа к полной игре
    - procedural_first: процедурная генерация вместо ручного контента

## INTER-SESSION TRACKING (CRITICAL)

tracking_files:
  # ALWAYS read these files at session start
  progress: PROGRESS.md
  errors: ERRORS.md
  decisions: DECISIONS.md
  tech_stack: TECH-STACK.md
  architecture: ARCHITECTURE.md

session_start_checklist:
  - Read PROGRESS.md for current phase, completed tasks, and next steps
  - Check ERRORS.md for active blockers (ERROR-XXX with status "Open")
  - Review DECISIONS.md for architectural decisions (DEC-XXX)
  - Scan ERRORS.md "Known Patterns" to avoid repeating mistakes
  - Continue from last in-progress task in PROGRESS.md

session_end_checklist:
  - Update PROGRESS.md with completed tasks and current status
  - Log any new errors to ERRORS.md (assign next ERROR-XXX ID)
  - Document architectural decisions in DECISIONS.md (next DEC-XXX)
  - Update TECH-STACK.md if new tools were added or evaluated

error_handling_protocol:
  - Capture full error message, context, and stack trace
  - Add to ERRORS.md immediately with ERROR-XXX ID
  - If blocker: mark as Priority P0 in ERRORS.md
  - If pattern (2+ times): add to "Known Patterns" section
  - Record the fix when resolved

## ARCHITECTURE OVERVIEW

hybrid_architecture:
  layer_1_visual_client:
    engine: Unreal Engine 5.3+
    language: C++
    purpose: Rendering, animations, VFX, sound, UI
    style: Anime/Cel-shading (inspired by Genshin Impact)
    key_features:
      - Niagara particle system for elemental effects
      - Control Rig for procedural animations
      - Custom cel-shading materials (HLSL)
      - Spatial audio for 3D combat

  layer_2_procedural_core:
    language: Rust
    framework: Bevy ECS
    purpose: Game logic, procedural generation, semantic graph
    key_features:
      - Unified Procedural Graph (UPG) from tower_seed
      - Wave Function Collapse for floor generation
      - Semantic tags system [fire:0.7, exploration:0.9, corruption:0.2]
      - Combat state machines and timing systems
      - Fixed-point arithmetic for deterministic simulation

  layer_3_server:
    framework: Nakama (open-source)
    database: FoundationDB
    purpose: Authoritative server, matchmaking, storage
    key_features:
      - "Seed + Delta" replication model
      - Player state synchronization
      - Anti-cheat validation
      - Leaderboards and social features

  layer_4_ai_pipeline:
    purpose: Content generation via AI
    tools:
      - TripoSR / InstantMesh: 3D model generation (image→mesh)
      - Stable Diffusion XL / Flux: Textures and concept art
      - AudioCraft / Bark: Sound effects and voice
      - Cascadeur: Physics-based animations
      - Llama 3.1 70B / Mistral Large: Narrative and dialogue generation
      - Segment Anything (SAM2): Asset segmentation for texture prep

  integration:
    protocol: Protocol Buffers (gRPC between layers)
    sync: "Seed + Delta" model (4 bytes seed + ~50KB mutations per 1000 floors)
    determinism: Fixed-point arithmetic for cross-platform consistency

## DEVELOPMENT ENVIRONMENT

primary_ide: VS Code
dev_tools:
  rust_bevy:
    - rust-analyzer (code intelligence)
    - CodeLLDB (debugging)
    - Even Better TOML (Cargo.toml)
    - Crates (dependency management)
  unreal:
    - C/C++ extension (ms-vscode.cpptools)
    - Unreal Engine Snippets
    - CMake Tools
  blender:
    - Blender Development extension
    - Python extension (for Blender scripts)
  nakama:
    - Lua extension (sumneko.lua)
    - Docker extension
    - YAML extension
  protobuf:
    - Protocol Buffer extension (proto3)
    - gRPC Tools
  general:
    - GitLens, Error Lens, Todo Tree
    - Remote SSH / Remote Containers
    - REST Client

## CODING STANDARDS

### Rust (Procedural Core)
rust_standards:
  - edition: "2021"
  - use ECS patterns: Components, Systems, Resources
  - deterministic: use fixed-point where cross-platform consistency needed
  - error handling: thiserror + anyhow
  - serialization: serde + Protocol Buffers (prost)
  - async: tokio for networking, synchronous for ECS systems
  - testing: cargo test + criterion for benchmarks

### C++ (Unreal Client)
cpp_standards:
  - follow UE5 coding conventions (UCLASS, UPROPERTY, UFUNCTION)
  - use Unreal Smart Pointers (TSharedPtr, TWeakPtr)
  - Blueprints for rapid prototyping, C++ for performance-critical code
  - gRPC plugin for communication with Procedural Core

### Lua (Nakama Server)
lua_standards:
  - follow Nakama runtime module patterns
  - use nk.* API for server operations
  - keep modules focused and small

### General Rules
general_rules:
  before_writing_code:
    - Check ERRORS.md "Known Patterns" to avoid known issues
    - Verify tool/library exists in TECH-STACK.md before adding new dependency
    - Follow the hybrid architecture boundaries (visual/logic/server separation)

  when_writing_code:
    - Keep clear separation between Unreal (visual) and Bevy (logic)
    - Use Protocol Buffers for all cross-layer communication
    - Never hardcode configurations - use .env / YAML / .ron files
    - Structured logging with context in all layers
    - Type safety everywhere (Rust types, UE5 UPROPERTY, proto schemas)

  file_organization: |
    tower-game/
    ├── .vscode/                    # VS Code configs
    ├── procedural-core/            # Rust + Bevy ECS
    │   ├── Cargo.toml
    │   └── src/
    ├── unreal-client/              # Unreal Engine 5 project
    │   ├── TowerGame.uproject
    │   └── Source/
    ├── nakama-server/              # Nakama server modules
    │   ├── modules/
    │   └── config.yml
    ├── shared/                     # Protocol Buffers, schemas
    │   └── proto/
    ├── blender/                    # Blender assets and scripts
    │   ├── models/
    │   └── scripts/
    ├── ai-pipeline/                # AI generation scripts
    │   ├── triposr/
    │   ├── stable-diffusion/
    │   └── audiocraft/
    ├── config/                     # Configuration files
    ├── scripts/                    # Build/deploy scripts
    ├── docs/                       # Design documents
    ├── CLAUDE.md                   # This file
    ├── PROGRESS.md                 # Session progress tracking
    ├── ERRORS.md                   # Error log and patterns
    ├── DECISIONS.md                # Architectural decisions
    ├── TECH-STACK.md               # Tool catalog
    └── ARCHITECTURE.md             # Architecture reference

## DEVELOPMENT PHASES

phase_0_environment_setup:
  status: "Current"
  tasks:
    - Configure VS Code workspace for all tools
    - Install Rust + Bevy dependencies
    - Install Unreal Engine 5.3+
    - Setup Blender with Python API
    - Configure Docker for Nakama
    - Create project structure
    - Setup Protocol Buffers toolchain

phase_1_procedural_prototype:
  tasks:
    - Implement basic Bevy ECS game loop
    - Create semantic tag system
    - Implement WFC floor generator (50 floors)
    - Basic monster generation from grammar
    - Loot table with semantic drops
    - Basic character controller

phase_2_combat_prototype:
  tasks:
    - Non-target combat system (bevy_rapier3d)
    - Angular hitboxes and timing windows
    - Weapon movesets (3 weapon types)
    - Parry/dodge/counter-attack mechanics
    - Resource management (kinetic/thermal/semantic energy)
    - Visual/audio feedback system

phase_3_unreal_visual_client:
  tasks:
    - Setup Unreal project with gRPC plugin
    - Cel-shading materials for anime style
    - Character rendering pipeline
    - Niagara effects for elemental abilities
    - Spatial audio integration
    - UI/HUD implementation

phase_4_networking:
  tasks:
    - Nakama server configuration
    - "Seed + Delta" replication
    - Player synchronization (50 players)
    - Authoritative validation
    - Anti-cheat system
    - Matchmaking

phase_5_content_systems:
  tasks:
    - AI asset generation pipeline (TripoSR, SD3)
    - Faction system with 4-component relations
    - Economy (crafting, market, taxes)
    - Breath of the Tower cycle
    - Procedural events system
    - Shadow/Echo death mechanics
    - Skill mastery system (21 domains, 6 tiers, skill trees)
    - Equipment effects (trigger→action, 3 gear sets, sockets)
    - Social systems (guild, party, friends, trading, auction)
    - Season pass & daily/weekly quests
    - Specialization branches & combat roles
    - Active abilities & hotbar system
    - Socket/gem/rune system
    - Cosmetics & transmog system
    - Tutorial & onboarding system

phase_6_polish:
  tasks:
    - Balance via Monte-Carlo simulations (100k+ builds)
    - Performance optimization
    - Load testing (1000+ players)
    - Build sharing & community features
    - Mentor system

## GAME DESIGN PILLARS

### Core Pillars
pillars:
  1_semantic_coherence: |
    All entities connected through Unified Procedural Graph.
    Every action leaves a semantic trace affecting future players.
    Tower "remembers" collective experience and adapts.

  2_skill_based_combat: |
    Non-target action combat: positioning > stats, timing > cooldowns.
    Angular hitboxes, parry windows (80-120ms), spatial tactics.
    Combat is a "dance" - dynamic, skill-dependent, no mini-games.

  3_mastery_over_levels: |
    NO traditional XP/level system. Progression through SKILL MASTERY.
    21 mastery domains (weapon, combat, crafting, gathering, other).
    Use a sword → Sword Mastery XP. Parry attacks → Parry Mastery XP.
    Stats distributed ONLY at character creation (20 points, min 1 max 10).
    Additional stats ONLY from equipment (intentionally small bonuses).
    Specialization branches at Expert tier define playstyle and role.

  4_effects_over_stats: |
    Equipment provides SPECIAL EFFECTS, not big stat bonuses.
    Trigger→Action system: OnHit, OnParry, OnDodge → ElementalDamage, Lifesteal, Shield.
    Socket system: gems for stats, runes for named effects.
    Set bonuses (2/3/4-piece) encourage thematic builds.
    Equipment is strategic choice, not just "higher number = better".

  5_living_economy: |
    Players craft ALL equipment (monsters drop only resources).
    4-source balance: government(40%), craft(25%), market(20%), credit(15%).
    Progressive wealth tax prevents dead money accumulation.

  6_collective_evolution: |
    Faction memory, seasonal legacy, Council of Builders voting.
    Death creates "echoes" helping future players.
    No single player is irrelevant - collective responsibility.

  7_healthy_competition: |
    Competition through uniqueness, not metrics (build entropy > 0.7).
    Social capital != economic capital.
    Asynchronous duels via "shadows" instead of direct PvP griefing.

  8_desktop_only_3d: |
    Desktop only — no mobile version. Full 3D anime-style graphics.
    Cel-shading post-process, Niagara elemental VFX.
    Optimized for keyboard+mouse and gamepad.

## GAME DEVELOPMENT TOOLS & LIBRARIES

### Rust Crate Stack (procedural-core)
rust_crates:
  core:
    - bevy 0.15: ECS game engine (entities, components, systems, plugins)
    - bevy_rapier3d 0.28: 3D physics (hitboxes, colliders, raycasting)
    - serde + serde_json: Serialization for save/load and FFI JSON bridge
    - sha3: Deterministic hashing for procedural generation
    - rayon 1.10: Data parallelism for Monte-Carlo simulations
    - rand + rand_chacha: Deterministic RNG from seeds

  potential_additions:
    - petgraph: Graph data structures (skill trees, dependency graphs)
    - uuid: Unique entity IDs for networked objects
    - chrono: Time handling for season pass / daily quests
    - noise / fastnoise-lite: Procedural noise for terrain variation
    - bincode: Binary serialization (faster than JSON for save files)
    - dashmap: Concurrent HashMap for multi-threaded access

### UE5 Plugin Stack (ue5-client)
ue5_plugins:
  builtin_used:
    - Enhanced Input: Code-driven input mapping (WASD, gamepad)
    - Niagara: Particle VFX (elemental effects, auras)
    - UMG (Unreal Motion Graphics): All UI widgets
    - WebSockets: Real-time match communication
    - HTTP: Nakama REST API client

  recommended_plugins:
    - CommonUI: Advanced UI framework (focus, navigation, input routing)
    - GameplayAbilitySystem (GAS): Ability cooldowns, effects, tags (Epic built-in)
    - MetaSounds: Procedural audio synthesis
    - Chaos Destruction: Destructible environments
    - Water Plugin: Water rendering for tower environments
    - PCG Framework: UE5 procedural content generation

### 3D Asset Pipeline
asset_tools:
  modeling:
    - Blender 4.2+: 3D modeling, rigging, UV unwrapping
    - TripoSR / InstantMesh: AI image→3D mesh generation
    - Meshy.ai: Text/image to 3D (API-based)
  texturing:
    - Stable Diffusion XL / Flux: AI texture generation
    - Material Maker: Open-source procedural texture generator
    - ArmorPaint: Open-source 3D texture painting
  animation:
    - Cascadeur: Physics-based animation
    - Mixamo: Free motion capture library
    - AccuRIG: Auto-rigging for humanoid characters
  audio:
    - AudioCraft (Meta): AI sound effect generation
    - Bark (Suno): AI voice synthesis for NPCs
    - Audacity: Audio editing
    - FMOD / Wwise: Spatial audio middleware (free tiers)
  segmentation:
    - Segment Anything (SAM2): AI object segmentation for asset prep

### Server & Networking
server_tools:
  game_server:
    - Nakama 3.21+: Game server (matchmaking, leaderboards, storage)
    - PostgreSQL 15: Persistent database
    - Docker Compose: Service orchestration
  monitoring:
    - Prometheus + Grafana: Metrics and dashboards
    - Jaeger: Distributed tracing for network calls
  testing:
    - k6 / Locust: Load testing for server endpoints
    - Postman / Bruno: API testing

### DevOps & Quality
devops_tools:
  ci_cd:
    - GitHub Actions: CI pipeline (cargo test, cargo clippy)
    - cargo-deny: License and vulnerability checking
    - cargo-audit: Security audit for Rust dependencies
  profiling:
    - Tracy: Frame profiler (Bevy integration)
    - Superluminal: CPU profiler for Windows
    - RenderDoc: GPU frame debugger
  code_quality:
    - cargo clippy: Rust linting
    - cargo fmt: Code formatting
    - rust-analyzer: IDE intelligence

## HYBRID ENGINE ARCHITECTURE

engine_architecture:
  name: "Tower Hybrid Engine"
  version: "0.3.0"
  principle: |
    UE5 handles rendering, animation, VFX, UI.
    Rust/Bevy handles ALL game logic: generation, combat, mastery, economy.
    Communication via JSON-over-HTTP (gRPC-compatible), with FFI/DLL fallback.

  integration_layer:
    rust_side: procedural-core/src/engine/mod.rs
      services:
        - GameStateService: World state, Breath cycle, tick management
        - CombatService: Damage calculation, angle/combo/semantic multipliers
        - GenerationService: Floor layouts (WFC), monsters (grammar), loot (semantic)
        - MasteryService: XP tracking, tier progression, specialization profiles
        - EconomyService: Wallets, trading, gold management

    ue5_side: ue5-client/Source/TowerGame/Network/
      classes:
        - GRPCClientManager: Connection management, HTTP transport, FFI fallback
        - StateSynchronizer: Client prediction, interpolation, reconciliation
        - ActionSender: Rate-limited action queue with sequence numbers
        - ProceduralFloorRenderer: Instanced mesh rendering from Rust floor data

    proto_definitions: shared/proto/
      - game_state.proto: Core types, player state, floor layout, actions, mutations
      - services.proto: 5 gRPC service definitions with all request/response types

    configuration: config/engine.json
      - 26 module configs, transport settings, UE5 rendering, Nakama, Blender pipeline

  transport_modes:
    json: Default, JSON-over-HTTP, human-readable, easy to debug
    protobuf: Future, Protocol Buffers for production (lower latency)
    ffi: Fallback, direct DLL calls when HTTP unavailable

  rendering_pipeline:
    flow: |
      1. Rust generates floor layout (WFC tiles + rooms)
      2. JSON sent to UE5 via HTTP/FFI
      3. ProceduralFloorRenderer creates instanced meshes per tile type
      4. Room-based Lumen lighting from biome tags
      5. Player input → ActionSender → Rust validation → state update
      6. StateSynchronizer interpolates state for smooth rendering

  blender_pipeline:
    addon: blender/scripts/tower_addon.py (scene setup, validate, export)
    batch_export: blender/scripts/batch_export.py (headless .blend → FBX)
    validation: blender/scripts/validate_models.py (UE5 compatibility checks)
    naming: SM_ (static), SK_ (skeletal), M_ (material), T_ (texture), A_ (animation)

## ANTI-PATTERNS TO AVOID

never_do:
  - Implement networking from scratch (use Nakama + bevy_replicon)
  - Create static content manually (use procedural generation)
  - Hardcode game parameters (use .ron / YAML config files)
  - Mix visual client logic with procedural core
  - Skip Protocol Buffers for cross-layer communication
  - Ignore determinism in simulation (use fixed-point arithmetic)
  - Write Unreal Blueprints for game logic (only for UI/visual prototyping)
  - Create monolithic systems (keep ECS components small and composable)

always_do:
  - Separate visual (UE5) from logic (Bevy) from server (Nakama)
  - Use semantic tags for all game entities
  - Test procedural generation with seed reproducibility
  - Log errors to ERRORS.md immediately
  - Document decisions in DECISIONS.md
  - Use Protocol Buffers as canonical data format
  - Profile before optimizing (criterion benchmarks for Rust)

## KEY REFERENCES

design_document: "chat-Лучшие_Практики_Из_Диалогов (3).txt"
tracking_files:
  - PROGRESS.md
  - ERRORS.md
  - DECISIONS.md
  - TECH-STACK.md
  - ARCHITECTURE.md

---

**Core Philosophy**: "Procedural Semantic Fabric" - all systems interconnected through semantic tags
**Target**: 85% open-source tools + 15% custom semantic logic
**Visual Style**: Anime cel-shading (Genshin Impact inspired)
**Combat**: Skill-based non-target action (timing + positioning + prediction)
**Architecture**: Hybrid (Unreal Engine + Rust/Bevy + Nakama)
**IDE**: VS Code as unified development environment
