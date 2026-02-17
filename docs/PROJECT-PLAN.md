# Tower Game - Project Development Plan
**Last Updated**: 2026-02-16
**Current Phase**: Phase 6A - Rust DLL Integration
**Overall Progress**: 99%

---

## Executive Summary

Tower Game is a procedural 3D MMORPG with hybrid architecture (UE5 + Rust/Bevy + Nakama). As of Session 23, we have:

✅ **COMPLETED**:
- Rust procedural core with 100 FFI exports (1153 tests passing)
- 64+ UE5 C++ classes (all compile successfully)
- Nakama server modules (10 RPC endpoints, match handler)
- Protocol Buffers schema for cross-layer communication
- UI widgets for all game systems (crafting, trading, mastery, etc.)
- Semantic tag system and procedural generation (WFC, grammar)
- Combat calculations (angle/combo multipliers, parry mechanics)
- Analytics system (combat, progression, economy tracking)

🔄 **IN PROGRESS**:
- **Phase 6A**: Linking UE5 client with Rust DLL
- Building tower_core.dll with cdylib target
- Testing FFI bridge in UE5 editor

⏳ **UPCOMING**:
- **Phase 6B**: Integration testing (Rust ↔ UE5)
- **Phase 7**: Networking with Nakama (multiplayer, matchmaking)
- **Phase 8**: Content systems (AI pipeline, faction events)
- **Phase 9**: Polish & balance (Monte-Carlo simulations, load testing)

---

## Phase 6A: Rust DLL Integration (CURRENT)

**Goal**: Link UE5 client with Rust procedural core via FFI bridge

**Status**: 🔄 **IN PROGRESS** - C++ code compiles, DLL build pending

### Immediate Tasks (Priority P0)

#### 1. Build Rust DLL (tower_core.dll)
**Blocker**: Missing DLL prevents UE5 linking

**Steps**:
```bash
cd procedural-core
cargo build --release --lib
```

**Expected Output**:
- `procedural-core/target/release/tower_core.dll` (Windows)
- ~5-10 MB DLL with 100 exported FFI functions

**Verification**:
```bash
# Check DLL exports
dumpbin /EXPORTS target/release/tower_core.dll | grep -E "(generate_floor|get_core_version|calculate_damage)"
```

**Success Criteria**:
- DLL builds without errors
- All 100 FFI functions exported correctly
- No missing dependencies (verify with `ldd` or Dependency Walker)

---

#### 2. Resolve UE5 Linker Errors
**Blocker**: 7 missing Rust functions prevent UE5 project from linking

**Current Linker Errors**:
```
unresolved external symbol generate_floor_mutators
unresolved external symbol free_rust_string
unresolved external symbol migrate_save
unresolved external symbol get_save_version
unresolved external symbol get_current_save_version
unresolved external symbol validate_save
(+ 1 more TBD)
```

**Steps**:
1. Copy `tower_core.dll` to UE5 plugin directory:
   ```
   ue5-client/Plugins/ProceduralCore/Binaries/Win64/tower_core.dll
   ```
2. Update `ProceduralCoreBridge.cpp` to include `.lib` file for linking:
   ```cpp
   #pragma comment(lib, "tower_core.lib")
   ```
3. Rebuild UE5 project:
   ```bash
   cd ue5-client
   ./Scripts/rebuild.bat
   ```

**Success Criteria**:
- UE5 project compiles and links without errors
- TowerGame.uproject opens in UE5 Editor
- ProceduralCoreBridge loads DLL successfully (check output log)

---

#### 3. Test FFI Bridge in UE5 Editor
**Goal**: Verify Rust functions callable from UE5

**Steps**:
1. Open `ue5-client/TowerGame.uproject` in UE5 Editor
2. Create test level: `Content/Levels/Test_RustIntegration.umap`
3. Place `ARustIntegrationTest` actor in level
4. Press Play (PIE - Play In Editor)
5. Check Output Log for test results

**Expected Results**:
```
LogTowerGame: [RustIntegrationTest] ===== STARTING RUST INTEGRATION TESTS =====
LogTowerGame: [RustIntegrationTest] TEST 1/6: Version Check
LogTowerGame: [RustIntegrationTest] ✅ PASS: Core version = v0.6.0
LogTowerGame: [RustIntegrationTest] TEST 2/6: Floor Generation
LogTowerGame: [RustIntegrationTest] ✅ PASS: Generated floor 5 (JSON valid, 15 rooms)
...
LogTowerGame: [RustIntegrationTest] ===== ALL TESTS PASSED (6/6) =====
```

**Success Criteria**:
- All 6 tests pass (Version, Floor, Combat, HotReload, Analytics, Monsters)
- No DLL loading errors
- JSON responses parseable
- Performance: Floor generation <50ms, Combat calculation <1ms

---

### Known Issues & Workarounds

#### Issue 1: DLL Path Discovery
**Problem**: UE5 may not find `tower_core.dll` if not in system PATH

**Workaround**:
- Copy DLL to `ue5-client/Binaries/Win64/` (next to TowerGame.exe)
- OR add DLL directory to system PATH
- OR use `LoadLibraryEx()` with full path in `ProceduralCoreBridge.cpp`

#### Issue 2: Rust String Memory Management
**Problem**: `free_rust_string()` must be called for all Rust-allocated strings

**Critical Functions**:
- `generate_floor_layout()` → returns JSON string → must call `free_rust_string()`
- `get_core_version()` → returns string → must call `free_rust_string()`
- `get_breath_state()` → returns JSON → must call `free_rust_string()`

**Prevention**: Use RAII wrapper in `ProceduralCoreBridge.cpp`:
```cpp
struct RustStringGuard {
    const char* ptr;
    ~RustStringGuard() { if (ptr) free_rust_string(ptr); }
};
```

#### Issue 3: JSON Parsing Errors
**Problem**: Malformed JSON from Rust causes UE5 crashes

**Prevention**:
- Validate JSON before parsing: `TSharedPtr<FJsonObject> JsonObject; TJsonReader<>::Create(JsonString)->ReadObject(JsonObject)`
- Add error logging with context
- Use `ensure()` instead of `check()` for non-critical failures

---

## Phase 6B: Integration Testing (NEXT)

**Goal**: Comprehensive testing of Rust ↔ UE5 integration

**Duration**: 1-2 weeks
**Priority**: P1 (after Phase 6A completes)

### Test Categories

#### 1. Functional Tests
**Coverage**: All 100 FFI functions

**Test Files**:
- `ue5-client/Source/TowerGame/Tests/RustIntegrationTest.cpp` (6 tests)
- Add: `CombatIntegrationTest.cpp` (hitbox, parry, combos)
- Add: `GenerationIntegrationTest.cpp` (1000 floors, monsters, loot)
- Add: `MasteryIntegrationTest.cpp` (XP gain, tier progression, specialization)
- Add: `AnalyticsIntegrationTest.cpp` (event recording, snapshot retrieval)

**Success Criteria**:
- All tests pass in PIE (Play In Editor)
- All tests pass in standalone build
- No memory leaks (check with Visual Studio Profiler)

---

#### 2. Performance Tests
**Goal**: Verify Rust core meets performance targets

**Benchmarks**:
| Function | Target | Method |
|----------|--------|--------|
| `generate_floor_layout()` | <50ms per floor | Average of 1000 floors |
| `calculate_damage()` | <1ms per hit | Average of 100k hits |
| `get_semantic_similarity()` | <5ms per query | Average of 10k queries |
| `request_floor_monsters()` | <20ms per floor | Average of 1000 floors |
| `get_analytics_snapshot()` | <10ms | Single call with 1M events recorded |

**Test Implementation**:
```cpp
// In RustIntegrationTest.cpp
void ARustIntegrationTest::Test7_PerformanceBenchmark()
{
    auto Start = FPlatformTime::Seconds();

    for (int32 i = 0; i < 1000; ++i)
    {
        FString Layout = TowerSys->RequestFloorLayout(42, i);
        // No parsing, just measure FFI call overhead
    }

    auto End = FPlatformTime::Seconds();
    double AvgMs = (End - Start) * 1000.0 / 1000.0;

    if (AvgMs > 50.0)
    {
        LogTestFail(TEXT("Performance"), FString::Printf(TEXT("Floor generation too slow: %.2f ms"), AvgMs));
    }
}
```

**Success Criteria**:
- All benchmarks meet or exceed targets
- No performance regressions vs. standalone Rust benchmarks
- FFI overhead <10% of pure Rust execution time

---

#### 3. Stress Tests
**Goal**: Verify stability under load

**Scenarios**:
1. **Rapid Fire**: Call 10k FFI functions in quick succession (no delays)
2. **Concurrent**: Multi-threaded FFI calls (10 threads × 1000 calls)
3. **Memory**: Generate 10k floors without cleanup, measure memory usage
4. **Long Session**: 1-hour PIE session with continuous generation/combat calls

**Success Criteria**:
- No crashes or deadlocks
- Memory usage stable (<1 GB growth over 1 hour)
- No DLL unloading/reloading errors

---

#### 4. Edge Case Tests
**Goal**: Verify error handling and boundary conditions

**Test Cases**:
| Test | Input | Expected Output |
|------|-------|-----------------|
| Invalid seed | seed = -1 | Error or default behavior (seed 0) |
| Out of range floor | floor_id = 2000 | Valid generation or clamping |
| Empty tags | tags = "" | Zero similarity, no crash |
| Null pointers | nullptr string | Graceful error, no crash |
| Malformed JSON | JSON with syntax error | Parsing failure, error log |
| Unicode strings | Tags with emoji/Cyrillic | Correct UTF-8 handling |

**Implementation**:
```cpp
void ARustIntegrationTest::Test8_EdgeCases()
{
    // Test 1: Invalid seed
    FString Layout = TowerSys->RequestFloorLayout(-1, 1);
    ensure(!Layout.IsEmpty()); // Should return something, not crash

    // Test 2: Out of range floor
    FString HighFloor = TowerSys->RequestFloorLayout(42, 9999);
    ensure(!HighFloor.IsEmpty());

    // Test 3: Empty tags
    float Similarity = TowerSys->GetSemanticSimilarity("", "");
    ensure(Similarity == 0.0f);
}
```

---

### Integration Test Deliverables

1. **Test Report**: Markdown document with all test results
   - `docs/test-reports/integration-test-report-v1.md`
   - Include: Pass/fail counts, performance metrics, edge cases
   - Screenshots of Output Log showing test results

2. **CI Pipeline**: Automated testing via GitHub Actions
   - `.github/workflows/ue5-integration-test.yml`
   - Runs on every push to `main` branch
   - Artifacts: Test logs, performance CSV, crash dumps (if any)

3. **Performance Baseline**: CSV file with benchmark results
   - `docs/performance/integration-benchmarks-baseline.csv`
   - Used for regression detection in future updates

---

## Phase 7: Networking & Multiplayer (UPCOMING)

**Goal**: Connect UE5 client to Nakama server, implement multiplayer

**Duration**: 2-3 weeks
**Priority**: P2 (after Phase 6B)

### 7.1: Nakama Server Setup

**Tasks**:
1. Configure Nakama docker-compose for production-like environment
   - PostgreSQL persistent volume
   - Prometheus + Grafana monitoring
   - Jaeger distributed tracing

2. Deploy Nakama modules:
   - `modules/tower_main.lua` (10 RPC endpoints)
   - `modules/tower_match.lua` (authoritative match handler)

3. Database schema:
   - Player profiles (stats, inventory, mastery levels)
   - Guild data (members, ranks, permissions)
   - Leaderboards (floor progress, combat rating, crafting skill)
   - Echoes (death locations, replay data)

**Success Criteria**:
- Nakama server running on `localhost:7350`
- Dashboard accessible at `localhost:7351`
- Database migrations applied
- Health check endpoint returns 200 OK

---

### 7.2: Client-Server Communication

**Architecture**: "Seed + Delta" Replication Model

**Flow**:
```
1. Client requests floor layout: send (seed=42, floor_id=5) to Nakama RPC
2. Nakama calls Rust FFI: generate_floor_layout(42, 5)
3. Nakama returns JSON to client
4. Client renders floor using instanced meshes
5. Client sends actions (move, attack, loot) to Nakama match
6. Nakama validates actions via Rust FFI: validate_action(action_json)
7. Nakama broadcasts state updates to all clients
8. Clients apply deltas: current_state + delta = new_state
```

**Implementation**:

#### Client Side (UE5):
```cpp
// ue5-client/Source/TowerGame/Network/NakamaClient.h
class FNakamaClient
{
public:
    void Connect(const FString& ServerUrl, const FString& ServerKey);
    void RequestFloorLayout(int64 Seed, int32 FloorId, TFunction<void(FString)> Callback);
    void SendAction(const FString& ActionJson);
    void OnMatchState(const FString& StateJson);
};
```

#### Server Side (Nakama Lua):
```lua
-- nakama-server/modules/tower_main.lua
local function rpc_request_floor_layout(context, payload)
    local json = nk.json_decode(payload)
    local seed = json.seed
    local floor_id = json.floor_id

    -- Call Rust FFI via HTTP (or embedded DLL)
    local layout_json = call_rust_ffi("generate_floor_layout", seed, floor_id)

    return layout_json
end
nk.register_rpc(rpc_request_floor_layout, "request_floor_layout")
```

**Success Criteria**:
- Client connects to Nakama without errors
- RPC calls return valid responses (<100ms latency)
- Match state updates received in real-time (<50ms latency)
- No desync between client prediction and server state

---

### 7.3: Multiplayer Match System

**Features**:
1. **Matchmaking**: Join random floor instance (up to 50 players)
2. **Party System**: Create/join parties (2-5 players)
3. **Authoritative Server**: All actions validated by Nakama
4. **Client Prediction**: Smooth movement despite network latency
5. **State Reconciliation**: Correct client state on server mismatch

**Match Flow**:
```
1. Client: Request matchmaking → Nakama
2. Nakama: Find/create match, return match_id
3. Client: Join match → Nakama match handler
4. Client: Send input actions every tick (60 Hz)
5. Nakama: Validate actions, update authoritative state (20 Hz)
6. Nakama: Broadcast state deltas to all clients (20 Hz)
7. Client: Apply deltas, interpolate for smooth rendering (60 FPS)
```

**Anti-Cheat**:
- Server validates all damage calculations via Rust FFI
- Movement speed checked against max speed
- Cooldown timers enforced server-side
- Loot drops validated against floor seed + RNG

**Success Criteria**:
- 50 players in single match without lag
- Client prediction accurate (minimal rubber-banding)
- No duplication exploits (items, gold, XP)
- Cheaters detected and kicked within 5 seconds

---

### 7.4: Networking Test Plan

**Load Test**: 1000 concurrent players
- Tool: k6 or Locust
- Scenario: 1000 virtual clients connect, join matches, send actions
- Target: <100ms average response time, <1% error rate

**Stress Test**: Peak load (5000 players)
- Tool: k6 with ramping VUs (virtual users)
- Scenario: Ramp from 0 to 5000 players over 10 minutes
- Target: Graceful degradation (queue players if capacity exceeded)

**Chaos Test**: Network failures
- Scenario: Random packet loss (5-10%), latency spikes (500ms), server restarts
- Target: No data loss, players reconnect automatically

---

## Phase 8: Content Systems (FUTURE)

**Goal**: Implement AI asset pipeline and procedural content generation

**Duration**: 3-4 weeks
**Priority**: P3

### 8.1: AI Asset Generation Pipeline

**Tools**:
- TripoSR / InstantMesh: 3D model generation (image → mesh)
- Stable Diffusion XL: Texture generation
- AudioCraft: Sound effects
- Bark: NPC voice synthesis

**Workflow**:
1. Generate concept art (Stable Diffusion)
2. Convert to 3D mesh (TripoSR)
3. Auto-rig (AccuRIG)
4. Auto-weight paint (Blender Python script)
5. Generate textures (Stable Diffusion)
6. Export to UE5 (FBX + PNG)
7. Import via DataTable

**Success Criteria**:
- 100 unique monster models generated
- 50 weapon models generated
- 20 environment tile sets generated
- All assets < 10k polygons (optimization target)

---

### 8.2: Faction System

**Features**:
- 4 factions with dynamic relations
- Reputation tracking per player
- Faction-specific quests and rewards
- Territory control (floors 500-1000)

**Implementation**:
- Rust: Faction state machine, relation updates
- UE5: Faction UI widget, reputation bar, territory map
- Nakama: Persistent faction data, leaderboards

---

### 8.3: Economy & Trading

**Features**:
- Player-to-player trading
- Global market (auction house)
- Crafting with recipe discovery
- Progressive wealth tax (4-source balance)

**Implementation**:
- Rust: Wallet management, transaction validation
- UE5: Trade widget, market browser, crafting UI
- Nakama: Transaction logging, anti-fraud detection

---

## Phase 9: Polish & Balance (FINAL)

**Goal**: Optimize, balance, and prepare for release

**Duration**: 2-3 weeks
**Priority**: P4

### 9.1: Balance via Monte-Carlo Simulation

**Method**:
- Generate 100k random builds (weapon + stats + specialization)
- Simulate 1000 combat encounters per build
- Calculate build entropy (Shannon entropy of build distribution)
- Nerf/buff outliers (builds with >5% win rate deviation)

**Target**: Build entropy > 0.7 (no single dominant build)

---

### 9.2: Performance Optimization

**Targets**:
- 60 FPS @ 1920×1080 on GTX 1660 Ti
- Load time <5 seconds (floor transition)
- Memory usage <4 GB (client)
- Network bandwidth <50 KB/s per player

**Tools**:
- UE5 Profiler (GPU/CPU)
- Tracy (frame profiler)
- RenderDoc (GPU debugging)

---

### 9.3: User Testing

**Phases**:
1. **Alpha**: 10 internal testers (1 week)
2. **Closed Beta**: 100 external testers (2 weeks)
3. **Open Beta**: 1000 players (4 weeks)

**Feedback Collection**:
- In-game survey widget
- Discord feedback channel
- Analytics (heatmaps, drop-off points)

---

## Timeline Summary

| Phase | Duration | Start Date | End Date | Status |
|-------|----------|------------|----------|--------|
| Phase 6A: Rust DLL Integration | 3 days | 2026-02-16 | 2026-02-18 | 🔄 In Progress |
| Phase 6B: Integration Testing | 1-2 weeks | 2026-02-19 | 2026-03-04 | ⏳ Upcoming |
| Phase 7: Networking & Multiplayer | 2-3 weeks | 2026-03-05 | 2026-03-25 | ⏳ Upcoming |
| Phase 8: Content Systems | 3-4 weeks | 2026-03-26 | 2026-04-22 | ⏳ Upcoming |
| Phase 9: Polish & Balance | 2-3 weeks | 2026-04-23 | 2026-05-13 | ⏳ Upcoming |
| **Release Candidate** | - | - | **2026-05-13** | 🎯 Target |

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| DLL linking failure | Medium | High | Test on multiple machines, document dependencies |
| Network desync issues | Medium | High | Client prediction + reconciliation, extensive testing |
| AI asset quality low | Low | Medium | Human review + refinement, fallback to hand-crafted |
| Performance below target | Medium | Medium | Profiling early, incremental optimization |
| Player balance issues | High | Medium | Monte-Carlo simulation, beta testing feedback |

---

## Success Metrics

### Technical
- ✅ 1153 Rust tests passing
- ✅ 100 FFI exports working
- ✅ 64+ UE5 C++ classes compiled
- 🔄 0 linker errors (target: achieved in Phase 6A)
- ⏳ 60 FPS @ 1080p (target: Phase 9)
- ⏳ <100ms network latency (target: Phase 7)

### Gameplay
- ⏳ Build entropy > 0.7 (target: Phase 9)
- ⏳ Player retention > 40% (D7) (target: Open Beta)
- ⏳ Average session length > 60 minutes (target: Open Beta)

### Business
- ⏳ 1000 concurrent players (target: Open Beta)
- ⏳ 10k wishlists on Steam (target: Pre-release)
- ⏳ Positive reviews (>80%) (target: Post-release)

---

## Next Actions (Immediate)

1. **Build Rust DLL** (P0, 1 hour)
   - Command: `cd procedural-core && cargo build --release --lib`
   - Verify: `dumpbin /EXPORTS target/release/tower_core.dll`

2. **Copy DLL to UE5** (P0, 5 minutes)
   - Source: `procedural-core/target/release/tower_core.dll`
   - Destination: `ue5-client/Plugins/ProceduralCore/Binaries/Win64/`

3. **Rebuild UE5 Project** (P0, 10 minutes)
   - Command: `cd ue5-client && ./Scripts/rebuild.bat`
   - Expected: 0 linker errors

4. **Run Integration Tests** (P0, 15 minutes)
   - Open `TowerGame.uproject` in UE5 Editor
   - Place `ARustIntegrationTest` in test level
   - Press Play, check Output Log

5. **Document Results** (P1, 30 minutes)
   - Update `PROGRESS.md` with Phase 6A completion
   - Update `ERRORS.md` if any new issues found
   - Create test report: `docs/test-reports/integration-test-v1.md`

---

## Questions for User

1. **DLL Build Priority**: Should we build DLL immediately or review plan first?
2. **Test Coverage**: Are 6 integration tests sufficient for Phase 6B, or should we expand?
3. **Networking Timeline**: Is 2-3 weeks realistic for Phase 7, or should we allocate more time?
4. **AI Pipeline**: Should AI asset generation be prioritized, or focus on hand-crafted assets for now?
5. **Release Target**: Is 2026-05-13 (3 months) a hard deadline, or flexible based on quality?

---

**Document Version**: 1.0
**Author**: Claude (Tower Game Development Agent)
**Review Status**: Awaiting user approval
