# Tower Game - Deployment Guide

**Version**: 1.0
**Last Updated**: 2026-02-17 (Session 30)

---

## Table of Contents
1. [Prerequisites](#prerequisites)
2. [Local Development](#local-development)
3. [Docker Deployment](#docker-deployment)
4. [Production Deployment](#production-deployment)
5. [Configuration Reference](#configuration-reference)
6. [Monitoring & Health Checks](#monitoring--health-checks)
7. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Required Software

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.82+ | Bevy server compilation |
| Docker | 24+ | PostgreSQL, Nakama, containerized server |
| Docker Compose | v2+ | Service orchestration |
| Git | 2.40+ | Source control |

### Optional Software

| Tool | Version | Purpose |
|------|---------|---------|
| Unreal Engine | 5.3+ | UE5 client (Windows only) |
| Visual Studio 2022 | 17.x | UE5 C++ compilation |
| protoc | 3.25+ | Protocol Buffer compilation |

### System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| CPU | 4 cores | 8+ cores |
| RAM | 4 GB | 16 GB |
| Disk | 10 GB | 50 GB (with UE5) |
| OS | Windows 10 / Linux | Windows 10/11 / Ubuntu 22.04 |
| Network | UDP port 5000 open | Low-latency connection |

---

## Local Development

### 1. Clone and Setup

```bash
git clone <repo-url> tower_game
cd tower_game
```

### 2. Start Infrastructure (PostgreSQL)

```bash
# Start PostgreSQL (creates 'nakama' and 'tower_game' databases)
docker compose up -d postgres

# Verify it's healthy
docker ps --format "table {{.Names}}\t{{.Status}}"
# Expected: tower-postgres   Up X minutes (healthy)

# Verify databases
docker exec tower-postgres psql -U postgres -c "\l" | grep -E "nakama|tower_game"
```

### 3. Start Nakama (Optional)

```bash
docker compose up -d nakama

# Access admin console: http://localhost:7351
# Default credentials: admin / password
```

### 4. Build Bevy Server

```bash
cd bevy-server

# Debug build (faster compilation, slower runtime)
cargo build

# Release build (slower compilation, optimized runtime)
cargo build --release

# Run tests first
cargo test
```

### 5. Start Bevy Server

```bash
# From project root
DATABASE_URL="postgres://postgres:localdb@localhost:5432/tower_game" \
RUST_LOG=info \
cargo run --release --manifest-path bevy-server/Cargo.toml --bin tower-bevy-server
```

Expected output:
```
Starting Tower Bevy Server...
LMDB template store initialized with 12 databases (476MB)
Seeded 50 total templates
Server listening on 0.0.0.0:5000 (UDP/renet)
HTTP API server running on port 50051
```

### 6. Test Client Connection

```bash
# In another terminal — start test client
cargo run --release --manifest-path bevy-test-client/Cargo.toml --bin bevy-test-client
```

Expected output:
```
Connecting to server at 127.0.0.1:5000...
Connected to server!
Replicated: 1 players, 0 monsters, 0 tiles
```

### 7. Verify HTTP API

```bash
curl http://localhost:50051/health
# Expected: {"status":"ok","version":"0.1.0"}
```

---

## Docker Deployment

### Full Stack (All Services)

```bash
# Start everything
docker compose up -d

# Check status
docker compose ps

# View logs
docker compose logs -f bevy-server
```

### Service Dependencies

```
postgres (must be healthy first)
    ├── nakama (depends on postgres)
    └── bevy-server (depends on postgres)
```

### Individual Services

```bash
# Start only PostgreSQL
docker compose up -d postgres

# Start Nakama (requires postgres)
docker compose up -d nakama

# Build and start Bevy server (requires postgres)
docker compose up -d --build bevy-server
```

### Docker Compose File

```yaml
# docker-compose.yml
services:
  postgres:
    image: postgres:15-alpine
    ports: ["5432:5432"]
    environment:
      POSTGRES_DB: nakama
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: localdb
    volumes:
      - postgres-data:/var/lib/postgresql/data
      - ./scripts/init-db.sh:/docker-entrypoint-initdb.d/10-create-tower-db.sh
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]

  nakama:
    image: heroiclabs/nakama:3.21.1
    depends_on:
      postgres: { condition: service_healthy }
    ports:
      - "7349:7349"   # gRPC
      - "7350:7350"   # HTTP API
      - "7351:7351"   # Admin Console

  bevy-server:
    build: { context: ., dockerfile: bevy-server/Dockerfile }
    depends_on:
      postgres: { condition: service_healthy }
    ports:
      - "5000:5000/udp"  # Game server (renet)
      - "50051:50051"     # HTTP API (JSON)
    volumes:
      - bevy-data:/app/data
```

### Rebuild After Code Changes

```bash
# Rebuild and restart bevy-server only
docker compose up -d --build bevy-server

# Force full rebuild (no cache)
docker compose build --no-cache bevy-server
docker compose up -d bevy-server
```

---

## Production Deployment

### Bevy Server Docker Image

The Dockerfile uses multi-stage builds for minimal image size:

```
Stage 1 (builder): rust:1.82-bookworm
  - Installs: liblmdb-dev, libssl-dev, protobuf-compiler
  - Builds release binary with LTO

Stage 2 (runtime): debian:bookworm-slim (~80MB)
  - Installs: liblmdb0, libssl3, ca-certificates, curl
  - Copies binary from builder
  - Health check via curl
```

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | Yes | - | PostgreSQL connection string |
| `LMDB_PATH` | No | `/app/data/templates` | LMDB database directory |
| `LMDB_MAX_SIZE` | No | `500000000` | Max LMDB size in bytes (auto-aligned to 4096) |
| `API_PORT` | No | `50051` | HTTP API listen port |
| `RUST_LOG` | No | `info` | Log verbosity (trace/debug/info/warn/error) |

### Port Requirements

| Port | Protocol | Direction | Firewall Rule |
|------|----------|-----------|---------------|
| `5000` | UDP | Inbound | Required for game clients |
| `50051` | TCP | Inbound | Required for HTTP API / UE5 client |
| `5432` | TCP | Internal | PostgreSQL (not exposed externally) |
| `7350` | TCP | Inbound | Nakama HTTP API (for matchmaking) |
| `7351` | TCP | Admin only | Nakama Console (restrict access) |

### Security Checklist

- [ ] Change PostgreSQL password from `localdb`
- [ ] Enable Nakama authentication (disable Unsecure mode)
- [ ] Set `protocol_id` to match client version
- [ ] Restrict Nakama Console (port 7351) to admin IPs
- [ ] Enable TLS for HTTP API (port 50051)
- [ ] Set `RUST_LOG=warn` for production (reduce log volume)
- [ ] Configure CORS for HTTP API (currently allows all origins)

### Scaling Considerations

- **Vertical**: Single server handles ~100 players per floor
- **Horizontal**: Multiple bevy-server instances, each managing different floors
- **Database**: PostgreSQL connection pool (default 10, configurable via `PG_MAX_CONNECTIONS`)
- **LMDB**: Embedded per-server instance (no shared state)
- **Nakama**: Handles cross-server matchmaking and persistence

---

## Configuration Reference

### LMDB Template Store

The LMDB embedded database stores game templates (monsters, items, abilities, recipes, loot tables, quests, factions). It's seeded on first startup with 50 templates across 12 databases.

```
data/templates/
├── monsters    (10 templates)
├── items       (13 templates)
├── abilities   (7 templates)
├── recipes     (3 templates)
├── loot_tables (10 templates)
├── quests      (3 templates)
└── factions    (4 templates)
```

**Note**: LMDB map size must be a multiple of the OS page size (4096 bytes). The server automatically rounds up any configured value.

### Game Server Configuration

Configured in `bevy-server/src/main.rs`:

| Parameter | Value | Notes |
|-----------|-------|-------|
| Tick rate | 20 Hz (50ms) | Responsive combat timing |
| Max clients | 100 | Dynamic scaling adjusts this |
| Protocol ID | 0 | Must match client |
| UDP bind | 0.0.0.0:5000 | All interfaces |
| HTTP bind | 0.0.0.0:50051 | All interfaces |
| Command batch | 64/tick | Max API commands per game tick |

---

## Monitoring & Health Checks

### HTTP Health Endpoint

```bash
curl http://localhost:50051/health
# {"status":"ok","version":"0.1.0"}
```

### Docker Health Checks

All services have built-in health checks:

```bash
# Check all services
docker compose ps

# Service-specific health
docker inspect --format='{{.State.Health.Status}}' tower-postgres
docker inspect --format='{{.State.Health.Status}}' tower-nakama
docker inspect --format='{{.State.Health.Status}}' tower-bevy-server
```

### Server Logs

```bash
# Bevy server (Docker)
docker compose logs -f bevy-server

# Bevy server (local)
RUST_LOG=debug cargo run --release  # More verbose logging

# PostgreSQL
docker compose logs -f postgres

# Nakama
docker compose logs -f nakama
```

### Key Log Messages

| Log | Meaning |
|-----|---------|
| `Server listening on 0.0.0.0:5000` | UDP game server ready |
| `HTTP API server running on port 50051` | HTTP API ready |
| `Player X connected` | Client connected successfully |
| `Performance good, increasing capacity` | Server running well |
| `PostgreSQL connection failed` | DB unavailable (API won't start, game loop continues) |

---

## Troubleshooting

### Common Issues

#### LMDB: "map size must be a multiple of the system page size"
The LMDB_MAX_SIZE value must be a multiple of 4096. The server auto-aligns this, but if you see this error on older builds, update to the latest version.

#### PostgreSQL: "non-UTF-8 string for its error message"
This is a known issue with sqlx on Windows connecting to PostgreSQL in Docker Alpine. The game server (UDP) continues working without the HTTP API. Workaround: use PostgreSQL with a UTF-8 locale or run the server on Linux.

#### Docker init-db.sh: "bad interpreter"
Windows CRLF line endings in shell scripts. Fix:
```bash
sed -i 's/\r$//' scripts/init-db.sh
docker compose down -v  # Clean volumes
docker compose up -d postgres  # Recreate
```

#### "bevy_rapier3d could not access Assets<Mesh>"
The server needs `AssetPlugin`, `ScenePlugin`, and `init_asset::<Mesh>()` for rapier3d compatibility in headless mode. These are already configured in the current codebase.

#### Port 5000 already in use
```bash
# Find what's using the port (Windows)
netstat -ano | findstr :5000
# Kill the process
taskkill /PID <pid> /F

# Or change the server port in main.rs
```

#### Test client can't connect
1. Verify server is running: check for "Server listening on 0.0.0.0:5000"
2. Check firewall: UDP port 5000 must be open
3. Verify protocol_id matches (both default to 0)
4. Check if another server instance is already running

### Running Tests

```bash
# Full test suite (221 tests)
cargo test --manifest-path bevy-server/Cargo.toml

# Specific test module
cargo test --manifest-path bevy-server/Cargo.toml combat::tests
cargo test --manifest-path bevy-server/Cargo.toml wfc::tests
cargo test --manifest-path bevy-server/Cargo.toml storage

# With output
cargo test --manifest-path bevy-server/Cargo.toml -- --nocapture
```

---

**Document Version**: 1.0
**Author**: Claude + User (Session 30)
**Related**: `docs/NETWORKING.md`, `docs/ARCHITECTURE.md`, `docs/PROGRESS.md`
