# Tower Game - Nakama Server

Authoritative game server for Tower MMORPG multiplayer (v0.6.0).

## Features

- **Tower Seed Management**: Single source of truth for procedural generation
- **Player State**: Floor progress, inventory, faction standings, mastery
- **Matchmaking**: Up to 50 players per floor instance
- **Leaderboards**: Highest floor, fastest clears, mastery ranking
- **Echo System**: Player death creates helpers for others
- **Real-time Multiplayer**: WebSocket for live floor instances
- **Anti-cheat**: Server-side validation

## Architecture

```
Tower Game Server
├── PostgreSQL 15        # Player data, leaderboards, storage
├── Nakama 3.21.1        # Game server runtime
├── Lua Modules          # Game logic (tower_main.lua)
└── Docker Compose       # Orchestration
```

## Quick Start

### 1. Prerequisites

- Docker Desktop (Windows/Mac) or Docker Engine (Linux)
- 4GB RAM minimum
- Ports 5432, 7349, 7350, 7351 available

### 2. Setup Environment

```bash
cd nakama-server
cp .env.example .env
# Edit .env and change passwords
```

### 3. Start Server

```bash
docker-compose up -d
```

### 4. Verify Health

**Admin Console**: http://localhost:7351
- Username: `admin`
- Password: (from .env `CONSOLE_PASSWORD`)

**Health Check RPC**:
```bash
curl -X POST http://localhost:7350/v2/rpc/health_check \
  -H "Content-Type: application/json" \
  -d '{}'
```

Expected response:
```json
{
  "status": "healthy",
  "version": "0.6.0",
  "tower_seed": 42,
  "server_time": 1708094400
}
```

## API Endpoints (RPC)

All endpoints accessible via HTTP POST to `http://localhost:7350/v2/rpc/<endpoint>`

### Core Endpoints

**get_tower_seed** - Get global tower seed
```json
Request: {}
Response: {
  "seed": 42,
  "epoch": 1708094400,
  "server_time": 1708094400
}
```

**get_player_state** - Get player progress
```json
Request: {}
Response: {
  "status": "ok",
  "state": {
    "current_floor": 5,
    "highest_floor": 10,
    "total_deaths": 3,
    "total_kills": 127,
    "faction_standings": {...}
  }
}
```

**request_floor** - Enter a floor
```json
Request: { "floor_id": 5 }
Response: {
  "status": "ok",
  "seed": 42,
  "floor_id": 5,
  "highest_floor": 10
}
```

### Progression Endpoints

**report_floor_clear** - Report floor completion
```json
Request: {
  "floor_id": 5,
  "kills": 12,
  "clear_time_seconds": 347.5
}
Response: {
  "status": "ok",
  "new_highest": 6,
  "total_kills": 139
}
```

**report_death** - Create echo on death
```json
Request: {
  "floor_id": 5,
  "echo_type": "Lingering",
  "position": {"x": 10, "y": 2, "z": 15},
  "semantic_tags": ["fire:0.7", "exploration:0.5"]
}
Response: {
  "status": "ok",
  "total_deaths": 4,
  "echo_type": "Lingering"
}
```

### Multiplayer Endpoints

**join_floor_match** - Join/create floor instance
```json
Request: { "floor_id": 5 }
Response: {
  "status": "ok",
  "match_id": "abc123...",
  "floor_id": 5,
  "seed": 42
}
```

**list_active_matches** - List ongoing floors
```json
Request: {}
Response: {
  "status": "ok",
  "matches": [
    {
      "match_id": "abc123...",
      "floor_id": 5,
      "player_count": 3,
      "max_players": 50
    }
  ],
  "count": 1
}
```

### Social Endpoints

**update_faction** - Change faction standing
```json
Request: {
  "faction": "seekers",
  "delta": 10
}
Response: {
  "status": "ok",
  "faction": "seekers",
  "standing": 25,
  "all_standings": {...}
}
```

**get_floor_echoes** - Get echoes for floor
```json
Request: { "floor_id": 5 }
Response: {
  "status": "ok",
  "floor_id": 5,
  "echoes": [
    {
      "player_name": "Alice",
      "echo_type": "Lingering",
      "position": {...},
      "semantic_tags": [...]
    }
  ],
  "count": 1
}
```

## Leaderboards

Access via Nakama API: `GET /v2/leaderboard/<leaderboard_id>`

**Available Leaderboards**:
- `highest_floor` - Highest floor reached (all-time)
- `floor_1_speed` to `floor_50_speed` - Speedrun rankings (per floor)

## Development

### Logs

```bash
# View all logs
docker-compose logs

# Follow Nakama logs
docker-compose logs -f nakama

# Follow PostgreSQL logs
docker-compose logs -f postgres
```

### Restart Server

```bash
docker-compose restart nakama
```

### Stop Server

```bash
docker-compose down
```

### Reset Database (CAUTION)

```bash
docker-compose down -v  # Deletes all player data!
docker-compose up -d
```

### Update Lua Modules

Modules in `./modules/` are hot-reloaded:
1. Edit `tower_main.lua`
2. Restart Nakama: `docker-compose restart nakama`
3. Changes applied immediately

## UE5 Integration

Example C++ code for calling Nakama from Unreal Engine 5:

```cpp
// Get NakamaSubsystem
UNakamaSubsystem* Nakama = GetGameInstance()->GetSubsystem<UNakamaSubsystem>();

// Health check
Nakama->CallRPC("health_check", "{}", [](FString Response) {
    UE_LOG(LogTemp, Log, TEXT("Nakama: %s"), *Response);
});

// Get tower seed
Nakama->CallRPC("get_tower_seed", "{}", [](FString Response) {
    // Parse JSON and use seed for Rust procedural core
});

// Report floor clear
FString Payload = FString::Printf(TEXT(
    "{\"floor_id\": %d, \"kills\": %d, \"clear_time_seconds\": %.1f}"
), FloorId, Kills, ClearTime);
Nakama->CallRPC("report_floor_clear", Payload, OnComplete);
```

## Production Deployment

**Security Checklist**:
- ✅ Change all passwords in `.env`
- ✅ Use HTTPS/TLS (reverse proxy)
- ✅ Enable Prometheus metrics
- ✅ Setup backup for PostgreSQL
- ✅ Configure firewall (close port 5432 externally)
- ✅ Set `NAKAMA_LOG_LEVEL=WARN` for performance

**Scaling**:
- Horizontal: Run multiple Nakama instances behind load balancer
- Database: PostgreSQL read replicas for leaderboards
- Monitoring: Prometheus + Grafana for metrics

## Troubleshooting

**Port already in use**:
```bash
# Find process using port 7350
lsof -i :7350  # Mac/Linux
netstat -ano | findstr :7350  # Windows

# Change port in docker-compose.yml
ports:
  - "7360:7350"  # Map to different external port
```

**Database connection failed**:
```bash
# Check PostgreSQL is healthy
docker-compose ps
docker-compose logs postgres

# Run diagnostic script
chmod +x check-connection.sh
./check-connection.sh

# Verify password in docker-compose.yml matches local.yml
```

**Nakama tries to connect with user=root (CockroachDB defaults)**:
This is a known issue where Nakama defaults to CockroachDB settings.

**Fix Applied** (v0.6.0):
- Triple-layer configuration: command args + env vars + config file
- Explicitly sets PostgreSQL credentials in all three places
- Environment variables: `NAKAMA_DATABASE_USER=nakama`
- Command flags: `--database.user nakama`
- Config file: `database.user: nakama`

**If still failing**:
```bash
# Full reset (WARNING: deletes all data)
docker-compose down -v
docker-compose up -d

# Check Nakama is reading correct config
docker exec tower-nakama env | grep DATABASE
# Should show: NAKAMA_DATABASE_USER=nakama (not root)

# Manually test PostgreSQL connection
docker exec tower-postgres psql -U nakama -d nakama -c "SELECT version();"
# Should succeed without password prompt
```

**Lua module not loading**:
```bash
# Check for syntax errors
docker-compose logs nakama | grep ERROR

# Verify module path
docker exec -it tower-nakama ls /nakama/data/modules
```

## References

- Nakama Docs: https://heroiclabs.com/docs/nakama/
- Nakama Lua API: https://heroiclabs.com/docs/nakama/server-framework/lua-runtime/
- Tower Game Design Doc: `../docs/game-design.md`

---

**Version**: 0.6.0
**Last Updated**: Session 23 (2026-02-16)
