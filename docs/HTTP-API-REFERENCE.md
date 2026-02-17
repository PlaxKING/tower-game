# Tower Game — HTTP API Reference

**Base URL:** `http://localhost:50051`
**Protocol:** JSON-over-HTTP (gRPC-compatible path conventions)
**Content-Type:** `application/json`

All game endpoints use `POST` method with gRPC-style paths: `/tower.<Service>/<Method>`

---

## System Endpoints

### GET /health

Health check for monitoring and load balancers.

**Response:**
```json
{ "status": "ok", "version": "0.1.0" }
```

### GET /metrics

Prometheus text exposition format for Grafana/Prometheus scraping.

### GET /metrics/json

JSON metrics for programmatic consumption (stress test clients).

**Response:**
```json
{
  "uptime_secs": 3600.0,
  "player_count": 12,
  "entity_count": 47,
  "tick": 50000,
  "avg_tick_time_ms": 49.8,
  "total_requests": 1234,
  "total_errors": 5,
  "rps": 41.13,
  "avg_request_duration_ms": 1.234
}
```

---

## GenerationService (6 endpoints)

| Endpoint | Description |
|---|---|
| `POST /tower.GenerationService/GenerateFloor` | WFC floor layout generation |
| `POST /tower.GenerationService/GenerateLoot` | Semantic loot drops |
| `POST /tower.GenerationService/SpawnMonsters` | Template-based monster spawning |
| `POST /tower.GenerationService/GenerateMonsters` | Grammar-based monster generation |
| `POST /tower.GenerationService/GenerateDestructibles` | Environmental object spawning |
| `POST /tower.GenerationService/QuerySemanticTags` | Semantic similarity search |

### GenerateFloor

**Request:** `{ "tower_seed": 42, "floor_id": 1 }`

**Response:** `{ "floor_id", "seed", "biome_id", "width", "height", "tiles[]", "semantic_tags[]" }`

### GenerateLoot

**Request:** `{ "source_entity_id", "player_id", "source_tags[]", "luck_modifier" }`

**Response:** `{ "items": [{ "item_template_id", "item_name", "quantity", "rarity", "socket_count", "tags[]" }] }`

### SpawnMonsters

**Request:** `{ "tower_seed", "floor_id", "room_id", "biome_tags[]" }`

**Response:** `{ "monsters": [{ "template_id", "position[3]", "health", "tier" }] }`

### GenerateMonsters

**Request:** `{ "tower_seed", "floor_id", "room_id", "biome_tags[]", "count?" }`

**Response:** `{ "monsters": [{ "variant_id", "name", "size", "element", "corruption", "body_type", "max_health", "base_damage", "move_speed", "ai_behavior", "position[3]", "semantic_tags[]", "loot_tier" }], "total_count" }`

### GenerateDestructibles

**Request:** `{ "tower_seed", "floor_id", "biome_tags[]" }`

**Response:** `{ "destructibles": [{ "entity_id", "template_id", "position[3]", "rotation_yaw", "material", "fragment_count", "total_hp", "category", "semantic_tags[]" }], "total_count" }`

### QuerySemanticTags

**Request:** `{ "query_tags[]", "similarity_threshold", "max_results" }`

**Response:** `{ "matches": [{ "entity_id", "entity_type", "similarity", "tags[]" }] }`

---

## GameStateService (5 endpoints)

| Endpoint | Description |
|---|---|
| `POST /tower.GameStateService/GetState` | Combined player + world state |
| `POST /tower.GameStateService/GetWorldCycle` | Breath of the Tower cycle |
| `POST /tower.GameStateService/GetPlayerProfile` | Full player profile with stats |
| `POST /tower.GameStateService/GetLiveStatus` | Live ECS snapshot |
| `POST /tower.GameStateService/GetLivePlayer` | Live player via ECS bridge |

### GetWorldCycle

**Request:** `{ "tower_seed": 12345 }`

**Response:** `{ "cycle_name", "phase", "phase_progress", "corruption_level", "active_events[]" }`

### GetLiveStatus

**Request:** `{}`

**Response:** `{ "server_tick", "uptime_secs", "player_count", "entity_count", "players[]", "destruction_stats{}", "world_cycle" }`

---

## CombatService (3 endpoints)

| Endpoint | Description |
|---|---|
| `POST /tower.CombatService/CalculateDamage` | Damage calculation with multipliers |
| `POST /tower.CombatService/GetCombatState` | Entity combat state |
| `POST /tower.CombatService/ProcessAction` | Combat action via ECS bridge |

### CalculateDamage

**Request:** `{ "attacker_id", "defender_id", "weapon_id", "ability_id", "attack_angle?", "combo_step?", "semantic_affinity?" }`

**Response:** `{ "base_damage", "modified_damage", "crit_chance", "crit_damage", "angle_mult", "combo_mult", "semantic_mult", "was_blocked", "was_parried", "modifiers[]" }`

---

## MasteryService (4 endpoints)

| Endpoint | Description |
|---|---|
| `POST /tower.MasteryService/TrackProgress` | Add mastery XP |
| `POST /tower.MasteryService/GetMasteryProfile` | All domains profile |
| `POST /tower.MasteryService/ChooseSpecialization` | Pick specialization branch |
| `POST /tower.MasteryService/UpdateAbilityLoadout` | Assign hotbar abilities |

### TrackProgress

**Request:** `{ "player_id", "domain", "action_type", "xp_amount" }`

**Response:** `{ "domain", "new_tier", "new_xp", "xp_to_next", "tier_up", "newly_unlocked[]" }`

---

## EconomyService (5 endpoints)

| Endpoint | Description |
|---|---|
| `POST /tower.EconomyService/GetWallet` | Player currency balances |
| `POST /tower.EconomyService/Craft` | Craft item from recipe |
| `POST /tower.EconomyService/ListAuction` | Browse auction house |
| `POST /tower.EconomyService/BuyAuction` | Buy auction listing |
| `POST /tower.EconomyService/Trade` | Gold trade between players |

### GetWallet

**Request:** `{ "player_id": 1 }`

**Response:** `{ "gold", "premium_currency", "honor_points", "seasonal_currency" }`

> `seasonal_currency` is an alias for `honor_points` (UE5 compatibility).

---

## DestructionService (4 endpoints)

| Endpoint | Description |
|---|---|
| `POST /tower.DestructionService/ApplyDamage` | Damage environmental object |
| `POST /tower.DestructionService/GetFloorState` | Floor destruction state |
| `POST /tower.DestructionService/Rebuild` | Rebuild destroyed object |
| `POST /tower.DestructionService/GetTemplates` | List destructible templates |

---

## UE5 Integration

### Endpoints called by GRPCClientManager.cpp

| UE5 Method | Server Endpoint | Status |
|---|---|---|
| `RequestFloor()` | `GenerateFloor` | OK |
| `RequestDamageCalculation()` | `CalculateDamage` | OK |
| `RequestTrackProgress()` | `TrackProgress` | OK |
| `RequestWallet()` | `GetWallet` | OK (added `seasonal_currency`) |
| `RequestLootGeneration()` | `GenerateLoot` | OK (added `item_name`, `socket_count`, `tags`) |

### Protocol Fixes (Session 32)

1. **Wallet:** Added `seasonal_currency` field (alias of `honor_points`)
2. **Loot:** Added `item_name` (alias of `item_template_id`), `socket_count`, and `tags[]`

**Total endpoints:** 27 POST + 3 GET = 30
