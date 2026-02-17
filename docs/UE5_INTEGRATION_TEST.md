# UE5 Integration Test Plan (v0.6.0)

## Prerequisites

✅ **DLL Requirements**:
- `tower_core.dll` with 100+ exported functions
- Built from latest commit with export fixes
- Located in: `ue5-client/Binaries/Win64/tower_core.dll`

✅ **CI Verification**:
Before testing, check GitHub Actions CI workflow:
- Export count: **100+** (not 7-12)
- Build status: **✅ Passed**
- Download artifact from: https://github.com/PlaxKING/tower-game/actions

---

## Test Setup

### 1. Install DLL

```bash
# Option A: Download from CI artifacts
# 1. Go to https://github.com/PlaxKING/tower-game/actions
# 2. Click latest successful "Rust CI" workflow
# 3. Download "tower_core-release" artifact
# 4. Extract tower_core.dll to:
cp tower_core.dll ue5-client/Binaries/Win64/

# Option B: Build locally (if Rust toolchain installed)
cd procedural-core
cargo build --release
cp target/release/tower_core.dll ../ue5-client/Binaries/Win64/
```

### 2. Open UE5 Project

```bash
cd ue5-client
# Double-click TowerGame.uproject
# Or launch from Epic Games Launcher
```

### 3. Compile C++ Code

In Unreal Editor:
1. **Build** → **Compile TowerGame**
2. Wait for compilation (should auto-detect DLL changes)
3. Check **Output Log** for:
   ```
   LogTemp: Tower Rust Core initialized. Version: 0.6.0
   ```

---

## Test Cases

### Test 1: Core Version ✅

**Blueprint Test**:
1. Create new **Blueprint Actor** (`BP_TestRustCore`)
2. Add **Event BeginPlay** node
3. Get **TowerGameSubsystem** → **GetCoreVersion**
4. **Print String** the result

**Expected Output**:
```
0.6.0
```

**C++ Test**:
```cpp
// In any Actor's BeginPlay:
UTowerGameSubsystem* TowerSys = GetGameInstance()->GetSubsystem<UTowerGameSubsystem>();
if (TowerSys && TowerSys->IsRustCoreReady())
{
    FString Version = TowerSys->GetCoreVersion();
    UE_LOG(LogTemp, Log, TEXT("Rust Core Version: %s"), *Version);
}
```

---

### Test 2: Procedural Generation 🎲

**Test Floor Layout**:
```cpp
UTowerGameSubsystem* TowerSys = GetGameInstance()->GetSubsystem<UTowerGameSubsystem>();

// Generate floor 5 with seed 42
FString LayoutJson = TowerSys->GenerateFloorLayout(5, 42);
UE_LOG(LogTemp, Log, TEXT("Floor Layout: %s"), *LayoutJson);
```

**Expected Output** (JSON):
```json
{
  "floor_id": 5,
  "width": 50,
  "height": 50,
  "tiles": [...], // Array of tile types
  "rooms": [...], // Room bounding boxes
  "spawn_points": [...],
  "boss_room": {"x": 25, "y": 25, "width": 10, "height": 10}
}
```

**Validation**:
- ✅ JSON is valid (parseable)
- ✅ `floor_id` matches input (5)
- ✅ Multiple runs with same seed = identical output
- ✅ Different seeds = different layouts

---

### Test 3: Combat System ⚔️

**Test Damage Calculation**:
```cpp
UTowerGameSubsystem* TowerSys = GetGameInstance()->GetSubsystem<UTowerGameSubsystem>();

// Simulate attack: 100 base damage, 45° angle, sword weapon
FString ResultJson = TowerSys->CalculateDamage(
    100.0f,   // BaseDamage
    45.0f,    // AttackAngle
    0,        // ComboStage
    TEXT("sword"),
    TEXT("slash")
);

UE_LOG(LogTemp, Log, TEXT("Damage Result: %s"), *ResultJson);
```

**Expected Output**:
```json
{
  "final_damage": 120,
  "angle_multiplier": 1.2,  // 45° = optimal angle
  "combo_multiplier": 1.0,
  "semantic_bonus": 0,
  "hit_type": "Normal"
}
```

---

### Test 4: Hot-Reload 🔥 (v0.6.0)

**Test Configuration Reload**:
```cpp
UTowerGameSubsystem* TowerSys = GetGameInstance()->GetSubsystem<UTowerGameSubsystem>();

// Check current status
FString Status = TowerSys->GetHotReloadStatus();
UE_LOG(LogTemp, Log, TEXT("Hot-Reload Status: %s"), *Status);

// Modify config/game_state.json (change a value)
// Then trigger reload:
int32 ReloadCount = TowerSys->TriggerConfigReload();
UE_LOG(LogTemp, Log, TEXT("Configs reloaded: %d"), ReloadCount);
```

**Expected Behavior**:
- ✅ Status shows `"enabled": true`
- ✅ Reload count > 0 (number of reloaded modules)
- ✅ Changes in `config/*.json` take effect without restarting UE5

---

### Test 5: Analytics 📊 (v0.6.0)

**Test Event Recording**:
```cpp
UTowerGameSubsystem* TowerSys = GetGameInstance()->GetSubsystem<UTowerGameSubsystem>();

// Record some events
TowerSys->RecordDamageDealt(TEXT("Longsword"), 150);
TowerSys->RecordDamageDealt(TEXT("Longsword"), 200);
TowerSys->RecordFloorCleared(5, 1, 347.5f); // Floor 5, Tier 1, 347.5 seconds
TowerSys->RecordGoldEarned(1250);

// Get snapshot
FString Snapshot = TowerSys->GetAnalyticsSnapshot();
UE_LOG(LogTemp, Log, TEXT("Analytics: %s"), *Snapshot);

// Get available event types
FString EventTypes = TowerSys->GetAnalyticsEventTypes();
UE_LOG(LogTemp, Log, TEXT("Event Types: %s"), *EventTypes);
```

**Expected Output**:
```json
{
  "total_events": 4,
  "events": [
    {"type": "DamageDealt", "weapon": "Longsword", "amount": 150},
    {"type": "DamageDealt", "weapon": "Longsword", "amount": 200},
    {"type": "FloorCleared", "floor_id": 5, "tier": 1, "time": 347.5},
    {"type": "GoldEarned", "amount": 1250}
  ]
}
```

---

### Test 6: Monster Generation 👹

**Test Monster Spawning**:
```cpp
UTowerGameSubsystem* TowerSys = GetGameInstance()->GetSubsystem<UTowerGameSubsystem>();

// Generate 5 monsters for floor 10
FString MonstersJson = TowerSys->GenerateMonsters(10, 5, 42);
UE_LOG(LogTemp, Log, TEXT("Monsters: %s"), *MonstersJson);
```

**Expected Output**:
```json
{
  "monsters": [
    {
      "id": "monster_10_0",
      "type": "skeleton_warrior",
      "level": 10,
      "health": 500,
      "damage": 25,
      "semantic_tags": ["undead:1.0", "physical:0.8"],
      "loot_table": {...}
    },
    // ... 4 more monsters
  ]
}
```

---

### Test 7: Loot System 💎

**Test Loot Generation**:
```cpp
UTowerGameSubsystem* TowerSys = GetGameInstance()->GetSubsystem<UTowerGameSubsystem>();

// Generate loot for floor 5, player mastery tier 2
FString LootJson = TowerSys->GenerateLoot(5, 2, 42);
UE_LOG(LogTemp, Log, TEXT("Loot: %s"), *LootJson);
```

**Expected Output**:
```json
{
  "items": [
    {
      "id": "sword_of_flames",
      "type": "weapon",
      "rarity": "rare",
      "stats": {"damage": 50, "fire_bonus": 15},
      "semantic_tags": ["fire:0.7", "melee:1.0"],
      "sockets": 2
    }
  ]
}
```

---

## Performance Tests

### Test 8: Batch Generation 🚀

**Stress Test**:
```cpp
UTowerGameSubsystem* TowerSys = GetGameInstance()->GetSubsystem<UTowerGameSubsystem>();

double StartTime = FPlatformTime::Seconds();

// Generate 100 floors
for (int32 i = 1; i <= 100; ++i)
{
    FString Layout = TowerSys->GenerateFloorLayout(i, 42);
}

double ElapsedMs = (FPlatformTime::Seconds() - StartTime) * 1000.0;
UE_LOG(LogTemp, Log, TEXT("Generated 100 floors in %.2f ms (avg: %.2f ms/floor)"),
    ElapsedMs, ElapsedMs / 100.0);
```

**Expected Performance**:
- ✅ Average: **< 10ms per floor** (release build)
- ✅ No crashes or memory leaks
- ✅ Deterministic (same seed = same results)

---

## Troubleshooting

### ❌ "Failed to initialize Tower Rust Core"

**Cause**: DLL not found or missing exports

**Fix**:
1. Verify DLL location:
   ```
   ue5-client/Binaries/Win64/tower_core.dll
   ```
2. Check DLL exports:
   ```bash
   objdump -p ue5-client/Binaries/Win64/tower_core.dll | grep "Name:" | wc -l
   # Should output: 100+ (not 7-12)
   ```
3. Rebuild UE5 project (Ctrl+Alt+F11)

---

### ❌ "LNK2019: unresolved external symbol"

**Cause**: Mismatch between Bridge function declarations and DLL exports

**Fix**:
1. Check `ProceduralCoreBridge.h` function typedefs match Rust signatures
2. Verify `LOAD_DLL_FUNC` calls in `ProceduralCoreBridge.cpp::Initialize()`
3. Ensure all 100 functions from `procedural-core/src/bridge/mod.rs` are declared

---

### ❌ JSON parse errors

**Cause**: Rust returning invalid JSON

**Fix**:
1. Test Rust function directly in unit test:
   ```rust
   #[test]
   fn test_generate_floor_json() {
       let json = generate_floor_layout(5, 42);
       let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
       assert_eq!(parsed["floor_id"], 5);
   }
   ```
2. Check Rust logs for serialization errors
3. Validate JSON with: https://jsonlint.com

---

### ❌ Different results with same seed

**Cause**: Non-deterministic RNG or floating-point precision

**Fix**:
1. Verify Rust uses `ChaCha8Rng::seed_from_u64(seed)`
2. Use fixed-point arithmetic for cross-platform consistency
3. Check for system time dependencies in generation code

---

## Next Steps After Successful Tests

Once all tests pass:

1. ✅ **Update PROGRESS.md**: Mark "UE5 Integration v0.6.0" as complete
2. ✅ **Commit test results**: Add screenshots/logs to `docs/test-results/`
3. ✅ **Begin floor rendering**: Implement `ProceduralFloorRenderer` to visualize layouts
4. ✅ **Setup Nakama**: Fix database connection for multiplayer testing
5. ✅ **Create sample level**: Blueprint level demonstrating all features

---

**Version**: 0.6.0
**Last Updated**: 2026-02-16
**Status**: Ready for testing (waiting for CI DLL artifact)
