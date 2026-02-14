# Tower Game - Technology Stack & Tool Catalog

## Stack Overview

| Layer | Primary Tool | Language | Purpose |
|-------|-------------|----------|---------|
| Visual Client | Unreal Engine 5.3+ | C++ | Rendering, VFX, animations, UI |
| Procedural Core | Bevy 0.14+ | Rust | Game logic, procedural generation |
| Server | Nakama | Go/Lua | Matchmaking, auth, storage |
| 3D Modeling | Blender 4.0+ | Python | Asset creation, rigging |
| AI Generation | TripoSR, SD3, AudioCraft | Python | Procedural content |
| Integration | Protocol Buffers / gRPC | Multi | Cross-layer communication |
| IDE | VS Code | - | Unified development |

---

## Layer 1: Game Engine & Rendering

| Tool | Version | Repository | Purpose | Status |
|------|---------|------------|---------|--------|
| Unreal Engine 5 | 5.3+ | epicgames.com | Visual client, anime rendering | To Install |
| Niagara | UE5 built-in | - | Particle effects (fire, water, corruption) | To Configure |
| Control Rig | UE5 built-in | - | Procedural animations | To Configure |
| Nanite | UE5 built-in | - | Virtualized geometry | Evaluate |
| Lumen | UE5 built-in | - | Global illumination | Evaluate |

## Layer 2: Procedural Core (Rust/Bevy)

| Tool | Version | Crate/Repo | Purpose | Status |
|------|---------|------------|---------|--------|
| Bevy | 0.14+ | bevyengine/bevy | ECS game engine | To Install |
| bevy_rapier3d | 0.27+ | dimforge/bevy_rapier | Physics, angular hitboxes | To Install |
| bevy_replicon | latest | lightsoutgames/bevy_replicon | Network replication | To Install |
| parry | latest | dimforge/parry | Spatial partitioning for collisions | To Install |
| petgraph | latest | petgraph-rs/petgraph | Semantic graph for UPG | To Install |
| noise-rs | latest | Razaekel/noise-rs | Procedural noise (Perlin, etc.) | To Install |
| renet | latest | lucaspoffo/renet | UDP transport layer | To Install |
| leafwing-input-manager | latest | Leafwing-Studios | Input handling + prediction | To Install |
| serde | latest | serde-rs/serde | Serialization | To Install |
| prost | latest | tokio-rs/prost | Protocol Buffers for Rust | To Install |
| tonic | latest | hyperium/tonic | gRPC for Rust | To Install |
| tokio | latest | tokio-rs/tokio | Async runtime | To Install |
| thiserror | latest | dtolnay/thiserror | Error types | To Install |
| anyhow | latest | dtolnay/anyhow | Error handling | To Install |
| criterion | latest | bheisler/criterion.rs | Benchmarking | To Install |
| ron | latest | ron-rs/ron | Rusty Object Notation (config files) | To Install |
| tracing | latest | tokio-rs/tracing | Structured logging | To Install |
| rand_xoshiro | latest | rust-random | Deterministic RNG | To Install |

## Layer 3: Server & Networking

| Tool | Version | Repository | Purpose | Status |
|------|---------|------------|---------|--------|
| Nakama | 3.20+ | heroiclabs/nakama | Authoritative game server | To Install |
| FoundationDB | 7.1+ | apple/foundationdb | Distributed state storage | Evaluate |
| PostgreSQL | 15+ | - | Relational data (Nakama backend) | To Install |
| Docker | latest | - | Container orchestration | Installed |
| Docker Compose | latest | docker/compose | Multi-container setup | Installed |

## Layer 4: 3D Art & Assets

| Tool | Version | Repository | Purpose | Status |
|------|---------|------------|---------|--------|
| Blender | 4.0+ | blender/blender | 3D modeling, rigging, animation | To Install |
| ArmorPaint | latest | armory3d/armorpaint | 3D texture painting (anime style) | Evaluate |
| Cascadeur | Free version | cascadeur.com | AI-powered animation | Evaluate |
| Mixamo | Web service | mixamo.com | Base animations library | Available |
| Rigify | Blender addon | - | Character rigging | Available |

## Layer 5: AI Content Generation

| Tool | Version | Repository | Purpose | Status |
|------|---------|------------|---------|--------|
| TripoSR | latest | VAST-AI-Research/TripoSR | 3D model from text/image | Evaluate |
| InstantMesh | latest | TencentARC/InstantMesh | Retopology for animations | Evaluate |
| Stable Diffusion 3 | Medium | stabilityai/sd3-medium | Concept art, textures | Evaluate |
| ComfyUI | latest | comfyanonymous/ComfyUI | SD pipeline editor | Evaluate |
| AudioCraft | latest | facebookresearch/audiocraft | Sound effects generation | Evaluate |
| Magenta Studio | latest | magenta/magenta-studio | Procedural music | Evaluate |
| Tortoise-TTS | latest | neonbjb/tortoise-tts | NPC voice generation | Evaluate |
| Llama 3 8B | quantized | meta-llama | Narrative/dialogue generation | Evaluate |
| ChromaDB | latest | chroma-core/chroma | Vector storage for semantic validation | Evaluate |

## Layer 6: Protocol & Integration

| Tool | Version | Purpose | Status |
|------|---------|---------|--------|
| Protocol Buffers | v3 | Canonical data format | To Install |
| gRPC | latest | RPC between layers | To Install |
| protoc | latest | Proto compiler | To Install |
| prost-build | latest | Rust proto codegen | To Install |

## Layer 7: Analytics & Monitoring

| Tool | Version | Repository | Purpose | Status |
|------|---------|------------|---------|--------|
| Prometheus | latest | prometheus/prometheus | Metrics collection | Evaluate |
| Grafana | latest | grafana/grafana | Metrics dashboards | Evaluate |
| Loki | latest | grafana/loki | Log aggregation | Evaluate |
| Apache DataFusion | latest | apache/datafusion | OLAP analytics | Evaluate |
| linfa | latest | rust-ml/linfa | ML for build clustering | Evaluate |

## Layer 8: Security & Anti-Cheat

| Tool | Version | Repository | Purpose | Status |
|------|---------|------------|---------|--------|
| Perspective API | - | perspectiveapi.com | Toxic chat filtering | Evaluate |
| Custom anti-cheat | - | Internal | Speed/bot/exploit detection | To Build |

## Layer 9: Infrastructure

| Tool | Version | Purpose | Status |
|------|---------|---------|--------|
| Docker | latest | Containerization | Installed |
| Docker Compose | latest | Service orchestration | Installed |
| MinIO | latest | S3-compatible asset storage | Evaluate |
| GitHub Actions | - | CI/CD | Evaluate |
| minikube | latest | Local Kubernetes | Evaluate |

---

## VS Code Extensions

### Required
| Extension | ID | Purpose |
|-----------|----|---------|
| rust-analyzer | rust-lang.rust-analyzer | Rust language server |
| CodeLLDB | vadimcn.vscode-lldb | Rust debugger |
| C/C++ | ms-vscode.cpptools | C++ for Unreal |
| Even Better TOML | tamasfe.even-better-toml | Cargo.toml support |
| Protocol Buffers | zxh404.vscode-proto3 | .proto file support |
| Docker | ms-azuretools.vscode-docker | Docker management |
| YAML | redhat.vscode-yaml | Config file support |
| GitLens | eamodio.gitlens | Git history & blame |
| Error Lens | usernamehw.errorlens | Inline error display |

### Recommended
| Extension | ID | Purpose |
|-----------|----|---------|
| Lua | sumneko.lua | Nakama Lua modules |
| Crates | serayuzgur.crates | Cargo dependency management |
| Todo Tree | gruntfuggly.todo-tree | TODO tracking |
| REST Client | humao.rest-client | API testing |
| Remote SSH | ms-vscode-remote.remote-ssh | Remote development |
| CMake Tools | ms-vscode.cmake-tools | UE5 build system |
| Python | ms-python.python | Blender/AI scripts |
| Blender Development | JacquesLucke.blender-development | Blender integration |

---

## Tool Evaluation Criteria

When adding new tools to the stack:
1. **Is it open-source?** (prefer MIT/Apache 2.0)
2. **Is it actively maintained?** (last commit < 6 months)
3. **Does it integrate with existing stack?** (Rust/C++/Lua compatibility)
4. **Does it replace custom code?** (prefer library over custom implementation)
5. **What's the learning curve?** (documentation quality)

Log evaluation results in DECISIONS.md as DEC-XXX entries.
