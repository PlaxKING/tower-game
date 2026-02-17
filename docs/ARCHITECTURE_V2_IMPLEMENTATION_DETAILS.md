# Architecture V2 - Implementation Details & Anti-Patterns

**Date**: 2026-02-16
**Parent Document**: ARCHITECTURE_V2_ANALYSIS.md
**Status**: 🚧 Critical Implementation Guidelines
**Type**: Technical Specification

---

## 🔴 Critical Issues & Solutions

### Issue 1: Синхронизация Схем Данных (Schema Sync Hell)

**Проблема:**
```
Rust изменил ChunkData:
struct ChunkData {
    seed: u64,
    tiles: Vec<TileType>,
    // ❌ ДОБАВИЛИ НОВОЕ ПОЛЕ
    biome_id: u32,
}

UE5 C++ все еще использует старую версию:
struct FChunkData {
    uint64 Seed;
    TArray<uint8> Tiles;
    // ❌ НЕТ biome_id - ДЕСЕРИАЛИЗАЦИЯ СЛОМАЕТСЯ
};
```

**Последствия:**
- 💥 Crashes при десериализации
- 💥 Невалидные данные (garbage values)
- 💥 Каждое изменение схемы = ручное обновление 2+ файлов

---

#### Solution A: Protocol Buffers (Recommended)

**Единый источник истины:**
```protobuf
// shared/proto/game_state.proto
syntax = "proto3";

message ChunkData {
  uint64 seed = 1;
  repeated uint32 tiles = 2;
  uint32 biome_id = 3;  // Новое поле
  bytes validation_hash = 4;
}
```

**Автоматическая генерация:**
```bash
# Генерация Rust кода
protoc --rust_out=bevy-server/src/proto game_state.proto

# Генерация UE5 C++ кода
protoc --cpp_out=ue5-client/Source/TowerGame/Proto game_state.proto
```

**Результат:**
```rust
// Rust (auto-generated)
pub struct ChunkData {
    pub seed: u64,
    pub tiles: Vec<u32>,
    pub biome_id: u32,
    pub validation_hash: Vec<u8>,
}
```

```cpp
// UE5 C++ (auto-generated)
class ChunkData {
  uint64_t seed() const;
  repeated_field<uint32_t> tiles() const;
  uint32_t biome_id() const;
  string validation_hash() const;
};
```

**Setup:**

```toml
# bevy-server/Cargo.toml
[dependencies]
prost = "0.13"  # Protobuf runtime
prost-types = "0.13"

[build-dependencies]
prost-build = "0.13"  # Code generation

# bevy-server/build.rs
fn main() {
    prost_build::compile_protos(
        &["../shared/proto/game_state.proto"],
        &["../shared/proto/"],
    ).unwrap();
}
```

**UE5 Plugin:**
```cpp
// ue5-client/Source/TowerGame/TowerGame.Build.cs
PublicDependencyModuleNames.AddRange(new string[] {
    "Protobuf",  // Add protobuf plugin
});
```

**Benefits:**
- ✅ Единый .proto файл = единственный источник истины
- ✅ Автоматическая кодогенерация для Rust + C++
- ✅ Backward compatibility (optional fields)
- ✅ Versioning support
- ✅ Меньше ошибок (type-safe)

---

#### Solution B: FlatBuffers (Alternative)

**Faster than Protobuf, но сложнее:**

```fbs
// shared/schemas/game_state.fbs
table ChunkData {
  seed: ulong;
  tiles: [uint];
  biome_id: uint;
  validation_hash: [ubyte];
}
```

**Pros:**
- ✅ Zero-copy deserialization (faster than Protobuf)
- ✅ Smaller binary size
- ✅ Better for large data (meshes, textures)

**Cons:**
- ❌ Более сложный API
- ❌ Меньше поддержки в Rust экосистеме
- ❌ Нужен build step

**Recommendation:** **Protobuf для Tower Game**
- Проще в использовании
- Лучшая интеграция с Bevy
- Достаточно быстро для наших нужд
- Versioning + backward compatibility

---

### Issue 2: CPU Нагрузка При Генерации

**Проблема:**
```rust
// ❌ BAD: Блокирует весь сервер
fn handle_chunk_request(floor_id: u32) -> ChunkData {
    let chunk = generate_floor_wfc(floor_id);  // 50-200ms!
    chunk
}
```

**Последствия:**
- 💥 Tick rate падает с 30 Hz до 5 Hz
- 💥 Все клиенты лагают
- 💥 10 одновременных запросов = 2-секундный freeze

---

#### Solution A: Асинхронные Воркеры (Tokio)

**Non-Blocking Generation:**
```rust
use tokio::sync::mpsc;
use tokio::task;

struct GenerationWorker {
    sender: mpsc::Sender<GenerationRequest>,
    receiver: mpsc::Receiver<GeneratedChunk>,
}

async fn generation_worker(
    mut requests: mpsc::Receiver<GenerationRequest>,
    results: mpsc::Sender<GeneratedChunk>,
) {
    while let Some(req) = requests.recv().await {
        // Генерация в отдельной задаче (не блокирует ECS)
        let chunk = task::spawn_blocking(move || {
            generate_floor_wfc(req.floor_id)
        }).await.unwrap();

        results.send(GeneratedChunk {
            floor_id: req.floor_id,
            data: chunk,
        }).await.unwrap();
    }
}

// Bevy System
fn request_chunk_generation(
    mut commands: Commands,
    worker: Res<GenerationWorker>,
    requests: Query<(Entity, &ChunkRequest), Added<ChunkRequest>>,
) {
    for (entity, request) in requests.iter() {
        // Отправляем запрос в воркер (не блокирует)
        worker.sender.try_send(GenerationRequest {
            floor_id: request.floor_id,
            requester: entity,
        }).ok();
    }
}

fn receive_generated_chunks(
    mut commands: Commands,
    worker: Res<GenerationWorker>,
) {
    // Получаем готовые чанки (если есть)
    while let Ok(chunk) = worker.receiver.try_recv() {
        commands.spawn((
            ChunkData(chunk.data),
            FloorId(chunk.floor_id),
        ));
    }
}
```

**Benefits:**
- ✅ Генерация не блокирует tick loop
- ✅ Используем все CPU cores (Tokio thread pool)
- ✅ Сервер остается responsive
- ✅ Parallel generation (10 чанков одновременно)

---

#### Solution B: Warm-Up (Предварительная Генерация)

**Pre-generate Popular Floors:**
```rust
async fn warmup_system(mut cache: ResMut<FloorCache>) {
    // Первые 10 этажей (самые посещаемые)
    for floor_id in 1..=10 {
        let chunk = task::spawn_blocking(move || {
            generate_floor_wfc(floor_id)
        }).await.unwrap();

        cache.insert(floor_id, chunk);
    }

    info!("Warm-up complete: 10 floors pre-generated");
}
```

**Start Server Faster:**
```rust
// Lazy warm-up (фоновая задача)
tokio::spawn(async {
    sleep(Duration::from_secs(5)).await;  // После запуска
    warmup_system().await;
});
```

---

#### Solution C: Redis Cache (Персистентный Кэш)

**Cache Generated Floors:**
```toml
[dependencies]
redis = { version = "0.27", features = ["tokio-comp", "connection-manager"] }
```

```rust
use redis::AsyncCommands;

struct FloorCacheRedis {
    client: redis::Client,
}

impl FloorCacheRedis {
    async fn get_or_generate(&self, floor_id: u32) -> ChunkData {
        let mut conn = self.client.get_multiplexed_async_connection().await.unwrap();

        // Пробуем получить из кэша
        let key = format!("floor:{}", floor_id);
        if let Ok(data) = conn.get::<_, Vec<u8>>(&key).await {
            return bincode::deserialize(&data).unwrap();
        }

        // Если нет в кэше - генерируем
        let chunk = task::spawn_blocking(move || {
            generate_floor_wfc(floor_id)
        }).await.unwrap();

        // Сохраняем в Redis (TTL = 1 hour)
        let serialized = bincode::serialize(&chunk).unwrap();
        conn.set_ex(&key, serialized, 3600).await.unwrap();

        chunk
    }
}
```

**Benefits:**
- ✅ Персистентный кэш (переживает рестарт сервера)
- ✅ Shared across multiple server instances
- ✅ Automatic eviction (TTL)
- ✅ Instant loading (0ms vs 50ms generation)

**Redis Memory:**
```
1 floor  = ~1 KB compressed
1000 floors = 1 MB (negligible)
```

---

#### Combined Solution (Best Practice)

```rust
async fn get_chunk(floor_id: u32, cache: &FloorCache) -> ChunkData {
    // 1. Check in-memory LRU cache (fastest)
    if let Some(chunk) = cache.get(floor_id) {
        return chunk.clone();  // 0ms
    }

    // 2. Check Redis (fast)
    if let Ok(chunk) = redis_cache.get(floor_id).await {
        cache.insert(floor_id, chunk.clone());  // Update LRU
        return chunk;  // ~1ms
    }

    // 3. Generate asynchronously (slow, but non-blocking)
    let chunk = generation_worker.request(floor_id).await;

    // 4. Update both caches
    cache.insert(floor_id, chunk.clone());
    redis_cache.set(floor_id, &chunk).await;

    chunk  // ~50ms first time, then cached
}
```

**Performance:**
- First request: 50ms (generation)
- Cached (LRU): 0ms
- Cached (Redis): 1ms
- **99% hit rate** after warm-up

---

### Issue 3: Анти-Чит (Client Can Predict Generation)

**Проблема:**
```
Клиент знает алгоритм WFC + seed → может предсказать будущие этажи
Клиент может сгенерировать этаж 100 локально → узнать, где сокровища
Клиент может "teleport" к известной позиции сокровища
```

**Последствия:**
- 💥 Читеры получают unfair advantage
- 💥 Экономика сломается (infinite loot)
- 💥 PvP unbalanced (wallhacks, ESP)

---

#### Solution A: Server-Only Seeds (Critical)

**❌ WRONG - Seed on Client:**
```rust
// DON'T DO THIS
struct FloorData {
    seed: u64,  // ❌ Client can use this to predict!
}
```

**✅ CORRECT - Hash Only:**
```rust
struct FloorDataClient {
    floor_id: u32,
    tiles: Vec<TileType>,  // Pre-generated on server
    validation_hash: [u8; 32],  // SHA3 of (seed + tiles)
    // ❌ NO SEED!
}
```

**Server-Side:**
```rust
fn generate_floor_for_client(floor_id: u32) -> FloorDataClient {
    // Seed хранится ТОЛЬКО на сервере
    let seed = get_server_seed(floor_id);  // Secret!

    // Генерируем на сервере
    let tiles = generate_floor_wfc(seed, floor_id);

    // Отправляем только результат + hash
    FloorDataClient {
        floor_id,
        tiles: tiles.clone(),
        validation_hash: compute_hash(&seed, &tiles),
    }
}
```

**Client CANNOT:**
- ❌ Predict future floors
- ❌ Generate floors locally
- ❌ Know where loot spawns before visiting

---

#### Solution B: Movement Validation (Anti-Teleport)

**Server validates ALL movement:**
```rust
fn validate_player_movement(
    mut players: Query<(&mut Transform, &PlayerInput, &MovementStats)>,
    time: Res<Time>,
) {
    for (mut transform, input, stats) in players.iter_mut() {
        let requested_pos = input.target_position;
        let current_pos = transform.translation;

        // Calculate max possible movement
        let max_distance = stats.max_speed * time.delta_secs();

        // Validate
        if current_pos.distance(requested_pos) > max_distance * 1.1 {
            // Client is trying to teleport!
            warn!("Player {:?} attempted invalid movement", player);

            // Reject + force correction
            transform.translation = current_pos;  // Stay in place
            // Send authoritative position to client
        } else {
            // Valid movement
            transform.translation = requested_pos;
        }
    }
}
```

**Client-Side Prediction (Optional):**
```cpp
// UE5 Client
void UPlayerMovement::PredictMovement(float DeltaTime)
{
    // Predict locally for smooth movement
    FVector PredictedPos = CurrentPos + Velocity * DeltaTime;
    SetActorLocation(PredictedPos);

    // Send input to server
    SendInput(Velocity);

    // Wait for server correction
    if (ServerPosition != PredictedPos) {
        // Server rejected - reconcile
        SetActorLocation(ServerPosition);
    }
}
```

---

#### Solution C: Loot Spawn Validation

**Server controls loot:**
```rust
fn spawn_loot(
    floor_id: u32,
    player_pos: Vec3,
) -> Option<LootDrop> {
    // Loot seed = server_seed + player_id + timestamp
    let loot_seed = hash(SERVER_SEED, floor_id, player.id, time.now());

    // Client CANNOT predict this (needs SERVER_SEED)
    let loot = generate_loot(loot_seed, player.luck);

    Some(loot)
}
```

**Client Receives:**
```rust
struct LootDrop {
    item_id: u32,
    position: Vec3,
    // ❌ NO SEED - Client cannot predict next loot
}
```

---

#### Solution D: Encryption + Obfuscation (Layer 2)

**Optional: Encrypt seeds in memory:**
```rust
use aes_gcm::{Aes256Gcm, Key, Nonce};

struct SecretSeedManager {
    cipher: Aes256Gcm,
    encrypted_seeds: HashMap<u32, Vec<u8>>,
}

impl SecretSeedManager {
    fn get_seed(&self, floor_id: u32) -> u64 {
        let encrypted = self.encrypted_seeds.get(&floor_id).unwrap();
        let decrypted = self.cipher.decrypt(&nonce, encrypted.as_ref()).unwrap();
        u64::from_le_bytes(decrypted.try_into().unwrap())
    }
}
```

**Benefits:**
- ✅ Even if someone dumps server memory, seeds are encrypted
- ✅ Harder to reverse-engineer

**Cons:**
- ❌ Overkill for most games
- ❌ Performance overhead
- ❌ Complexity

**Recommendation:** Not needed if server-side validation is strong

---

## 📊 Implementation Checklist

### Schema Synchronization

- [ ] Create `shared/proto/` directory
- [ ] Define `game_state.proto` (ChunkData, EntitySnapshot, etc.)
- [ ] Add `prost-build` to Rust build.rs
- [ ] Add Protobuf plugin to UE5 .Build.cs
- [ ] Test: Change schema → regenerate → verify both sides compile

### CPU Load Optimization

- [ ] Implement Tokio async workers
- [ ] Add LRU cache (lru crate)
- [ ] Setup Redis (Docker Compose)
- [ ] Implement warm-up system (first 10 floors)
- [ ] Benchmark: 10 simultaneous requests < 50ms

### Anti-Cheat

- [ ] Remove seeds from client messages
- [ ] Implement server-side movement validation
- [ ] Add loot spawn validation
- [ ] Test: Client mod cannot predict floors
- [ ] Test: Teleport hacks are rejected

---

## 🔬 Testing Strategy

### Schema Sync Test

```rust
#[test]
fn test_protobuf_roundtrip() {
    let chunk = ChunkData {
        seed: 12345,
        tiles: vec![1, 2, 3],
        biome_id: 5,
        validation_hash: vec![0xAB; 32],
    };

    // Serialize
    let bytes = chunk.encode_to_vec();

    // Deserialize
    let decoded = ChunkData::decode(&bytes[..]).unwrap();

    assert_eq!(chunk, decoded);
}
```

### CPU Load Test

```rust
#[tokio::test]
async fn test_parallel_generation() {
    let start = Instant::now();

    // Request 10 floors simultaneously
    let handles: Vec<_> = (1..=10)
        .map(|floor_id| {
            tokio::spawn(async move {
                get_chunk(floor_id).await
            })
        })
        .collect();

    // Wait for all
    for handle in handles {
        handle.await.unwrap();
    }

    let duration = start.elapsed();
    assert!(duration < Duration::from_millis(100));  // Should be <100ms total
}
```

### Anti-Cheat Test

```rust
#[test]
fn test_movement_validation() {
    let mut player = Player {
        position: Vec3::ZERO,
        max_speed: 10.0,  // 10 m/s
    };

    // Valid movement (5m in 0.5s = 10 m/s)
    let input = PlayerInput {
        target_position: Vec3::new(5.0, 0.0, 0.0),
        delta_time: 0.5,
    };
    assert!(validate_movement(&player, &input).is_ok());

    // Invalid movement (teleport 100m in 0.5s = 200 m/s)
    let invalid = PlayerInput {
        target_position: Vec3::new(100.0, 0.0, 0.0),
        delta_time: 0.5,
    };
    assert!(validate_movement(&player, &invalid).is_err());
}
```

---

## 📈 Performance Benchmarks

### Expected Results

| Operation | Target | Acceptable | Warning |
|-----------|--------|------------|---------|
| **Schema codegen time** | <1s | <5s | >10s |
| **Single floor generation** | <50ms | <100ms | >200ms |
| **10 parallel generations** | <100ms | <200ms | >500ms |
| **Cache hit latency** | <1ms | <5ms | >10ms |
| **Redis round-trip** | <2ms | <10ms | >20ms |
| **Movement validation** | <0.1ms | <1ms | >5ms |

---

## ✅ Recommended Tech Stack

```toml
[dependencies]
# Networking
prost = "0.13"           # Protobuf runtime
quinn = "0.11"           # QUIC protocol

# Async
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"

# Caching
lru = "0.12"             # LRU cache
redis = "0.27"           # Redis client

# Validation
sha3 = "0.10"            # Hashing
aes-gcm = "0.10"         # Encryption (optional)

# ECS
bevy = "0.15"
lightyear = "0.17"       # Snapshot interpolation
```

---

## 📝 Architecture Diagram (Updated)

```
┌─────────────────────────────────────────┐
│           UE5 Client                    │
│  ┌──────────────────────────────────┐  │
│  │ Protobuf Auto-Generated Structs  │  │
│  │ (from shared/proto/*.proto)      │  │
│  └──────────────────────────────────┘  │
│  ┌──────────────────────────────────┐  │
│  │ Procedural Mesh Builder          │  │
│  │ (uses server tile data)          │  │
│  └──────────────────────────────────┘  │
└──────────────┬──────────────────────────┘
               │ QUIC/Protobuf
               │ (ChunkData, EntitySnapshot)
               ▼
┌─────────────────────────────────────────┐
│      Bevy Headless Server               │
│  ┌──────────────────────────────────┐  │
│  │ Tokio Async Workers (8 threads)  │  │
│  │ - Floor generation (non-blocking)│  │
│  │ - Physics simulation             │  │
│  └──────────────────────────────────┘  │
│  ┌──────────────────────────────────┐  │
│  │ LRU Cache (100 floors in RAM)    │  │
│  └──────────────────────────────────┘  │
│  ┌──────────────────────────────────┐  │
│  │ Movement Validator (anti-cheat)  │  │
│  └──────────────────────────────────┘  │
│  ┌──────────────────────────────────┐  │
│  │ Protobuf Auto-Generated Structs  │  │
│  │ (from shared/proto/*.proto)      │  │
│  └──────────────────────────────────┘  │
└──────────────┬──────────────────────────┘
               │ gRPC
               │ (save state, verify auth)
               ▼
┌─────────────────────────────────────────┐
│          Redis Cache                    │
│  - Generated floors (TTL=1h)            │
│  - Player session data                  │
└─────────────────────────────────────────┘

┌─────────────────────────────────────────┐
│       Nakama Meta-Service               │
│  - Authentication                        │
│  - Leaderboards                          │
│  - Friends/Guilds                        │
└─────────────────────────────────────────┘
```

---

## 🎯 Next Steps

1. ✅ ~~Document implementation details~~ (This file)
2. Update CLAUDE.md with Protobuf requirement
3. Create shared/proto/ directory structure
4. Define initial game_state.proto schema
5. Setup Redis in docker-compose.yml
6. Implement async generation workers
7. Add movement validation system
8. Write unit tests for all systems

---

**Status:** 🚧 Design Complete
**Critical Issues Addressed:** 3/3
**Ready for Implementation:** Yes
**Estimated Time:** 2-3 weeks

---

**Document Created:** Session 26
**Author:** Claude Sonnet 4.5
**Based on User Feedback:** Yes
