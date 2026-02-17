# Tower Game - Error Log & Known Patterns

## Active Errors

| ID | Priority | Status | Summary | Date |
|----|----------|--------|---------|------|
| ERROR-001 | P1 | Resolved | Git link.exe overrides MSVC linker | 2026-02-13 |
| ERROR-002 | P1 | Resolved | Windows SDK (kernel32.lib) not installed | 2026-02-13 |
| ERROR-003 | P1 | Resolved | dlltool.exe not found for GNU target | 2026-02-13 |
| ERROR-004 | P1 | Resolved | Wrong MasteryDomain enum variant names (41 errors) | 2026-02-14 |
| ERROR-005 | P1 | Resolved | Wrong method names on MasteryProfile/SpecializationProfile | 2026-02-14 |
| ERROR-006 | P1 | Resolved | Non-existent struct fields on Synergy | 2026-02-14 |
| ERROR-007 | P1 | Resolved | Importing non-existent types (AbilityDef, GearSlot) | 2026-02-14 |
| ERROR-022 | P0 | Resolved | PostgreSQL UTF-8 locale issue (sqlx on Windows) | 2026-02-17 |
| ERROR-023 | P1 | Resolved | PostgreSQL migration multi-statement error | 2026-02-17 |
| ERROR-024 | P1 | Resolved | Nakama Lua match handler registration | 2026-02-17 |
| ERROR-025 | P1 | Resolved | Docker build: Rust version too old for dependencies | 2026-02-17 |
| ERROR-026 | P1 | Resolved | Docker build: cargo cache stale (dummy binary shipped) | 2026-02-17 |

---

## Error Details

### ERROR-001: Git link.exe overrides MSVC linker
- **Date**: 2026-02-13
- **Priority**: P1 (high)
- **Status**: Resolved
- **Category**: Build
- **Component**: procedural-core
- **Error Message**:
  ```
  error: linking with `link.exe` failed: exit code: 1
  link: extra operand '...'
  ```
- **Context**: `cargo check` fails because `C:\Program Files\Git\usr\bin\link.exe` (GNU link) is found before MSVC `link.exe`
- **Root Cause**: Git Bash installs a `link.exe` in PATH that is NOT the MSVC linker
- **Fix**: Switched default toolchain to GNU: `rustup default stable-x86_64-pc-windows-gnu`
- **Prevention**: Use GNU toolchain on Windows with Git Bash. See KP-011.

### ERROR-002: Windows SDK (kernel32.lib) not installed
- **Date**: 2026-02-13
- **Priority**: P1 (high)
- **Status**: Resolved (bypassed via GNU toolchain)
- **Category**: Build
- **Component**: procedural-core
- **Error Message**:
  ```
  LINK : fatal error LNK1181: cannot open input file 'kernel32.lib'
  ```
- **Context**: Even with explicit MSVC linker path, linking fails because Windows SDK is not installed
- **Root Cause**: Visual Studio 2022 Community installed without "Desktop development with C++" workload (no Windows SDK)
- **Fix**: Bypassed by switching to GNU toolchain which doesn't need Windows SDK
- **Alternative Fix**: Install Windows SDK via Visual Studio Installer
- **Prevention**: See KP-012

### ERROR-003: dlltool.exe not found for GNU target
- **Date**: 2026-02-13
- **Priority**: P1 (high)
- **Status**: Resolved
- **Category**: Build
- **Component**: procedural-core
- **Error Message**:
  ```
  error: error calling dlltool 'dlltool.exe': program not found
  ```
- **Context**: `cargo test` on GNU target fails because MinGW is not installed
- **Root Cause**: `rustup` installs `rust-mingw` component with import libraries but not the full MinGW toolchain
- **Fix**: Installed WinLibs (MinGW) via `winget install BrechtSanders.WinLibs.POSIX.UCRT`
- **PATH**: MinGW bin at `C:/Users/Plax/AppData/Local/Microsoft/WinGet/Packages/BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe/mingw64/bin`
- **Note**: Must add MinGW to PATH before running cargo. Config in `.cargo/config.toml` sets linker/ar paths.
- **Prevention**: See KP-013

### ERROR-004: Wrong MasteryDomain enum variant names (41 errors)
- **Date**: 2026-02-14
- **Priority**: P1 (high)
- **Status**: Resolved
- **Category**: Build
- **Component**: procedural-core (engine/mod.rs)
- **Error Message**:
  ```
  error[E0599]: no variant named `Sword` found for enum `MasteryDomain`
  ```
- **Context**: engine/mod.rs mapped string names like "sword" to `MasteryDomain::Sword`, but actual variants are `SwordMastery`, `GreatswordMastery`, etc.
- **Root Cause**: Did not read the MasteryDomain enum definition before using it. Assumed short names.
- **Fix**: Corrected all 21 domain mappings to use actual variant names (SwordMastery, not Sword)
- **Prevention**: See COMMON-ERRORS.md CE-001. ALWAYS read enum definitions before use.

### ERROR-005: Wrong method names on MasteryProfile / SpecializationProfile
- **Date**: 2026-02-14
- **Priority**: P1 (high)
- **Status**: Resolved
- **Category**: Build
- **Component**: procedural-core (engine/mod.rs)
- **Error Message**:
  ```
  error[E0599]: no method named `get_tier` found for struct `MasteryProfile`
  error[E0599]: no method named `add_xp` found for struct `MasteryProfile`
  error[E0599]: no method named `active_branches` found for struct `SpecializationProfile`
  ```
- **Context**: Used guessed method names instead of reading the actual impl blocks
- **Root Cause**: Wrote code referencing module APIs without reading the source
- **Fix**: `get_tier()` → `tier()`, `add_xp()` → `gain_xp()`, `sp.primary_role()` → `sp.primary_role` (field)
- **Prevention**: See COMMON-ERRORS.md CE-002. Read impl blocks before calling methods.

### ERROR-006: Non-existent struct fields on Synergy
- **Date**: 2026-02-14
- **Priority**: P1 (high)
- **Status**: Resolved
- **Category**: Build
- **Component**: procedural-core (engine/mod.rs)
- **Error Message**:
  ```
  error[E0609]: no field `required_branches` on type `Synergy`
  error[E0609]: no field `bonus_description` on type `Synergy`
  ```
- **Context**: Assumed Synergy had `required_branches` and `bonus_description` fields
- **Root Cause**: Did not read the Synergy struct definition
- **Fix**: Used actual fields: `branch_a`, `branch_b`, `description`
- **Prevention**: See COMMON-ERRORS.md CE-003. Read struct definitions before field access.

### ERROR-007: Importing non-existent types
- **Date**: 2026-02-14
- **Priority**: P1 (high)
- **Status**: Resolved
- **Category**: Build
- **Component**: procedural-core (engine/mod.rs)
- **Error Message**:
  ```
  error[E0432]: unresolved import `crate::abilities::AbilityDef`
  error[E0432]: unresolved import `crate::equipment::GearSlot`
  ```
- **Context**: Imported `AbilityDef` (actual type is `Ability`) and `GearSlot` (doesn't exist)
- **Root Cause**: Assumed type names without checking module exports
- **Fix**: Removed unused imports, removed fields using those types
- **Prevention**: See COMMON-ERRORS.md CE-004. Check pub exports before importing.

### ERROR-022: PostgreSQL UTF-8 locale issue
- **Date**: 2026-02-17
- **Priority**: P0 (blocker)
- **Status**: Resolved
- **Category**: Config
- **Component**: bevy-server
- **Error Message**:
  ```
  Postgres returned a non-UTF-8 string for its error message. This is most likely due to an error that occurred during authentication and the default lc_messages locale is not binary-compatible with UTF-8.
  ```
- **Context**: sqlx 0.8 on Windows connecting to PostgreSQL in Docker
- **Root Cause**: Local Windows PostgreSQL (postgres.exe) was running on port 5432 alongside Docker container, intercepting connections. The local PG had a non-UTF-8 locale (Windows code page).
- **Fix**: Changed Docker postgres port mapping from 5432:5432 to 5433:5432 to avoid conflict. Updated default DATABASE_URL in main.rs and tasks.json to use port 5433.
- **Prevention**: See KP-024. Always check for local services competing on standard ports before using Docker port mappings.
- **Session**: 31

### ERROR-023: PostgreSQL migration multi-statement error
- **Date**: 2026-02-17
- **Priority**: P1 (high)
- **Status**: Resolved
- **Category**: Runtime
- **Component**: bevy-server
- **Error Message**:
  ```
  cannot insert multiple commands into a prepared statement
  ```
- **Context**: Running database migrations via sqlx
- **Root Cause**: sqlx::query().execute() uses prepared statements which only support a single SQL command. The v1_initial_schema migration contained 30+ CREATE TABLE/INDEX/FUNCTION/TRIGGER statements.
- **Fix**: Changed sqlx::query(sql) to sqlx::raw_sql(sql) in run_migrations() - raw_sql sends statements without prepared statement wrapping.
- **Prevention**: Always use sqlx::raw_sql() for migration scripts that contain multiple SQL statements. Reserve sqlx::query() for single-statement operations.
- **Session**: 31

### ERROR-024: Nakama Lua match handler registration
- **Date**: 2026-02-17
- **Priority**: P1 (high)
- **Status**: Resolved
- **Category**: Runtime
- **Component**: nakama-server
- **Error Message**:
  ```
  /nakama/data/modules/tower_match.lua:521: attempt to call a non-function object
  ```
- **Context**: Nakama 3.21.1 Lua runtime loading tower_match.lua
- **Root Cause**: Used nk.register_match() which doesn't exist in Nakama Lua API. Match handlers should return a table of callbacks, not register them.
- **Fix**: Replaced nk.register_match("tower_match", {...}) with return { match_init = ..., match_join_attempt = ..., etc. }
- **Prevention**: Always check Nakama Lua runtime documentation for correct API patterns. Match modules return callback tables, they do not self-register.
- **Session**: 31

### ERROR-025: Docker build: Rust version too old for dependencies
- **Date**: 2026-02-17
- **Priority**: P1 (high)
- **Status**: Resolved
- **Category**: Build
- **Component**: bevy-server
- **Error Message**:
  ```
  error: rustc 1.85.1 is not supported by the following packages:
    home@0.5.12 requires rustc 1.88
  ```
- **Context**: Building bevy-server Docker image. Started with rust:1.82-bookworm, bumped to 1.85 (needed for edition2024/base64ct), then still failed because home@0.5.12 needs 1.88.
- **Root Cause**: Transitive dependencies (base64ct, home) have progressively higher MSRV requirements. The Dockerfile's base image was too far behind.
- **Fix**: Updated Dockerfile FROM rust:1.85-bookworm to rust:1.93-bookworm (matching local toolchain).
- **Prevention**: Keep Docker Rust version in sync with local toolchain. Use `rust:latest-bookworm` or pin to the same version as local `rustc --version`.
- **Session**: 31

### ERROR-026: Docker build: cargo cache produces dummy binary
- **Date**: 2026-02-17
- **Priority**: P1 (high)
- **Status**: Resolved
- **Category**: Build
- **Component**: bevy-server
- **Error Message**: No error — binary exits silently with code 0 (360KB dummy binary instead of 17MB real binary)
- **Context**: Docker multi-stage build uses a dummy main.rs to pre-cache dependencies, then replaces with real source. Cargo didn't detect the source change because Docker COPY preserves timestamps from build context.
- **Root Cause**: Cargo uses file modification times to detect changes. Docker COPY preserves the original timestamps, so after replacing dummy sources with real ones, cargo saw no changes and skipped recompilation.
- **Fix**: Added `RUN find bevy-server/src -name '*.rs' -exec touch {} +` after COPY to force fresh timestamps and trigger recompilation.
- **Prevention**: Always touch source files after replacing dummy sources in Docker dependency caching patterns. Alternatively, remove the cargo fingerprint cache before the final build.
- **Session**: 31

### Template for new errors:
```
### ERROR-XXX: [Short description]
- **Date**: YYYY-MM-DD
- **Priority**: P0 (blocker) / P1 (high) / P2 (medium) / P3 (low)
- **Status**: Open / In Progress / Resolved
- **Category**: Build / Runtime / Config / Integration / Network
- **Component**: procedural-core / unreal-client / nakama-server / blender / vs-code
- **Error Message**:
  ```
  [paste error here]
  ```
- **Context**: What were you doing when this happened?
- **Root Cause**: (fill after investigation)
- **Fix**: (fill after resolution)
- **Prevention**: How to avoid this in the future
```

---

## Known Patterns (Anti-patterns to avoid)

### KP-001: Bevy version mismatch with plugins
- **Description**: Bevy plugins must match the exact Bevy version (e.g., bevy_rapier for bevy 0.14 != bevy 0.13)
- **Prevention**: Always check plugin compatibility before adding to Cargo.toml. Pin exact versions.

### KP-002: Unreal Engine build from VS Code
- **Description**: UE5 builds triggered from VS Code may fail if UnrealBuildTool path is not correctly set
- **Prevention**: Set UE_EDITOR_CMD environment variable. Use UnrealBuildTool.exe, not UnrealEditor-Cmd.exe for builds.

### KP-003: Protocol Buffers code generation order
- **Description**: Proto files must be compiled before Rust/C++ compilation
- **Prevention**: Add proto generation as pre-build task in tasks.json. Use build.rs for Rust proto generation.

### KP-004: Docker on Windows - line endings
- **Description**: Windows CRLF line endings break shell scripts in Docker containers
- **Prevention**: Use .gitattributes with `* text=auto` and `*.sh text eol=lf`

### KP-005: Nakama module hot-reload
- **Description**: Nakama Lua modules don't hot-reload automatically
- **Prevention**: Use `docker-compose restart nakama` after module changes

### KP-006: Rust determinism across platforms
- **Description**: f32 arithmetic may produce different results on different CPU architectures
- **Prevention**: Use fixed-point arithmetic (i32 with 16-bit fractional) for game simulation. Use f32 only for rendering.

### KP-007: UE5 + VS Code IntelliSense slowness
- **Description**: Unreal Engine generates massive compilation databases, making IntelliSense slow
- **Prevention**: Limit includePath in c_cpp_properties.json. Use compile_commands.json from UBT.

### KP-008: Bevy asset loading on Windows
- **Description**: Bevy asset paths use forward slashes, Windows uses backslashes
- **Prevention**: Always use forward slashes in asset paths. Use `AssetServer` with relative paths.

### KP-009: gRPC connection between Rust and UE5
- **Description**: gRPC channel may fail silently if server is not ready
- **Prevention**: Implement health check endpoint. Use retry logic with exponential backoff.

### KP-010: Blender Python API version
- **Description**: Blender Python API changes between major versions
- **Prevention**: Pin Blender version in project docs. Test scripts after Blender updates.

### KP-011: Git link.exe overrides MSVC linker on Windows
- **Description**: Git Bash installs `link.exe` in PATH that shadows MSVC linker. Rust MSVC target fails to link.
- **Prevention**: Use GNU toolchain: `rustup default stable-x86_64-pc-windows-gnu`. Avoids MSVC linker entirely.

### KP-012: Windows SDK not installed with Visual Studio
- **Description**: VS 2022 Community may be installed without Windows SDK, causing MSVC linker to fail on kernel32.lib
- **Prevention**: Either install "Desktop development with C++" workload, or use GNU toolchain.

### KP-013: MinGW PATH for GNU target on Windows
- **Description**: Rust GNU target needs `dlltool.exe`, `gcc.exe` from MinGW, but `rust-mingw` component doesn't include them
- **Prevention**: Install WinLibs via winget. Set linker/ar in `.cargo/config.toml`. Add MinGW bin to PATH or run with `export PATH="/c/Users/Plax/.../mingw64/bin:$PATH"` before cargo commands.
- **Critical**: Shell PATH from one bash call does NOT persist to the next. Always prepend PATH in the same command.

### KP-014: Bevy EventWriter API changes
- **Description**: Bevy 0.15 uses `EventWriter::send()`, not `EventWriter::write()`
- **Prevention**: Check Bevy migration guide when upgrading. Test compilation after changes.

### KP-015: UE5 TUniquePtr incomplete type
- **Description**: Forward declaration insufficient for TUniquePtr<T> - destructor requires complete type definition
- **Prevention**: Use `#include` instead of forward declaration when TUniquePtr/TSharedPtr holds the type
- **Example**: `TUniquePtr<FProceduralCoreBridge>` in header requires `#include "Bridge/ProceduralCoreBridge.h"`, not `class FProceduralCoreBridge;`

### KP-016: UWidget::Slot naming conflicts
- **Description**: Local variables named "Slot" shadow inherited UWidget::Slot member (C4458)
- **Prevention**: Never name local variables "Slot" in UWidget-derived classes. Use descriptive names like "MatSlot", "SlotData", "SlotStr"
- **Critical**: Affects all UI widgets - check for shadowing warnings in compilation output

### KP-017: Blueprint type requirements
- **Description**: UPROPERTY/UFUNCTION exposed to Blueprint must use signed integers (int32/int64), not unsigned (uint32/uint64)
- **Prevention**: Use int32/int64 for all Blueprint-exposed properties and function parameters. Reserve uint types for internal implementation only
- **Example**: `UFUNCTION(BlueprintCallable)` with `uint32` parameter will cause type conversion errors

### KP-018: Uninitialized pointer variables
- **Description**: MSVC C4703 - pointer declared without initialization, compiler cannot prove all paths initialize it
- **Prevention**: Always initialize pointer variables to nullptr at declaration: `const TArray<T>* Ptr = nullptr;`
- **Critical**: Applies to all pointer types, especially when used with conditional initialization (FindObject, TryGetJsonObject, etc.)

### KP-019: UButton delegate signature constraints
- **Description**: UButton::OnClicked delegate expects void() with no parameters - cannot bind to parameterized functions
- **Prevention**: Store parameters in button tags/userData, or use lambda capture. Do not attempt to bind functions with parameters directly
- **Workaround**: `Button->OnClicked.AddDynamic(this, &Class::OnClickHandler);` where OnClickHandler() is void with no params

### KP-020: MSVC vs GNU toolchain for Windows DLL linking
- **Description**: GNU toolchain produces .dll.a (GNU-style import library), incompatible with MSVC linker used by UE5. MSVC toolchain produces .dll.lib (required for linking)
- **Prevention**: Always use `x86_64-pc-windows-msvc` target for Rust when building DLLs for UE5/MSVC projects
- **Benefit**: MSVC builds are also significantly smaller (1.9 MB vs 57 MB for GNU - 97% reduction)
- **Command**: `cargo build --release --lib --target x86_64-pc-windows-msvc`

### KP-021: Rust FFI function naming must match exactly
- **Description**: C++ code must use exact function name from Rust `pub extern "C" fn` declaration. #[no_mangle] ensures C-compatible naming
- **Prevention**: Always check Rust source for actual function names. Do not guess or use similar names
- **Example**: Rust declares `free_string`, C++ must call `free_string` (not `free_rust_string`)

### KP-022: UE5 delegate names must be unique across headers
- **Description**: UnrealHeaderTool (UHT) generates code for each delegate. Same delegate name in multiple headers → duplicate definitions in .gen.cpp → LNK2005 linker error
- **Prevention**: Use unique, descriptive names for all delegates. Prefix with context (e.g., `FOnFloorGenerated` vs `FOnFloorGeneratedResponse`)
- **Critical**: Unity build compiles multiple .gen.cpp together, making conflicts more likely

### KP-023: Check all DLL search paths for stale versions
- **Description**: TowerGameSubsystem searches multiple paths for tower_core.dll. If old DLL exists in higher-priority path (e.g., ThirdParty/), it will be loaded instead of new version
- **Prevention**: When updating DLL, verify all search paths and remove/update stale copies. Search order:
  1. Binaries/Win64/tower_core.dll
  2. ThirdParty/TowerCore/lib/tower_core.dll ← check this!
  3. ../../procedural-core/target/release/tower_core.dll
  4. ../../procedural-core/target/debug/tower_core.dll
- **Detection**: "Failed to load ProceduralCore DLL" despite DLL existing → wrong version loaded
- **Fix**: Copy latest DLL to all search paths or remove old copies

### KP-024: Port conflicts with local services
- **Description**: When Docker containers map to standard service ports (5432, 3306, etc.), local installations of the same service can intercept connections. Always check netstat for competing listeners, or use non-standard host ports (e.g., 5433 for PG).
- **Prevention**: Before using standard ports in docker-compose.yml, run `netstat -ano | findstr :<port>` (Windows) or `ss -tlnp | grep :<port>` (Linux) to check for competing listeners. Use non-standard host ports (e.g., 5433:5432, 3307:3306) to avoid conflicts with local installations.
- **Example**: Local postgres.exe on port 5432 intercepted connections meant for Docker PostgreSQL → changed to 5433:5432

---

## Resolved Errors

| ID | Summary | Resolution Date | Fix |
|----|---------|----------------|-----|
| ERROR-001 | Git link.exe overrides MSVC linker | 2026-02-13 | Switched to GNU toolchain |
| ERROR-002 | Windows SDK not installed | 2026-02-13 | Bypassed via GNU toolchain |
| ERROR-003 | dlltool.exe not found | 2026-02-13 | Installed WinLibs MinGW |
| ERROR-004 | Wrong MasteryDomain enum variants | 2026-02-14 | Corrected all 21 mappings |
| ERROR-005 | Wrong method names | 2026-02-14 | Used actual API: tier(), gain_xp() |
| ERROR-006 | Non-existent Synergy fields | 2026-02-14 | Used branch_a, branch_b, description |
| ERROR-007 | Non-existent type imports | 2026-02-14 | Removed unused imports |
| ERROR-008 | TUniquePtr incomplete type in TowerGameSubsystem | 2026-02-16 | Changed forward declaration to #include |
| ERROR-009 | RustIntegrationTest calling wrong function names | 2026-02-16 | Updated to RequestFloorLayout, RequestFloorMonsters |
| ERROR-010 | CraftingWidget Slot variable shadowing | 2026-02-16 | Renamed all "Slot" to "MatSlot" |
| ERROR-011 | TowerMapWidget type mismatches (uint32 vs int32) | 2026-02-16 | Changed to int32 for Blueprint compatibility |
| ERROR-012 | TransmogWidget uninitialized pointers | 2026-02-16 | Added nullptr initializers |
| ERROR-013 | SocketWidget Slot shadowing + uninitialized pointers | 2026-02-16 | Renamed Slot, added nullptr |
| ERROR-014 | SaveMigrationWidget uint64 vs int64 | 2026-02-16 | Changed to int64 |
| ERROR-015 | ActionSender uint64 vs int64 | 2026-02-16 | Changed to int64 |
| ERROR-016 | LoggingConfigWidget delegate signature mismatch | 2026-02-16 | Commented out, added TODO |
| ERROR-017 | ProceduralFloorRenderer NavigationSystem include | 2026-02-16 | Changed to #include "NavigationSystem.h" + module |
| ERROR-018 | Missing .lib file for MSVC linking | 2026-02-16 | Rebuilt with x86_64-pc-windows-msvc target |
| ERROR-019 | free_rust_string function not found | 2026-02-16 | Renamed to free_string (actual Rust name) |
| ERROR-020 | Delegate name conflict FOnFloorGenerated | 2026-02-16 | Renamed in GRPCClientManager → FOnFloorGeneratedResponse |
| ERROR-021 | Old DLL loaded instead of new one (ThirdParty) | 2026-02-16 | Copied new MSVC DLL to ThirdParty/TowerCore/lib/ |
| ERROR-022 | PostgreSQL UTF-8 locale issue (sqlx on Windows) | 2026-02-17 | Changed Docker PG port to 5433:5432 |
| ERROR-023 | PostgreSQL migration multi-statement error | 2026-02-17 | Changed sqlx::query() to sqlx::raw_sql() |
| ERROR-024 | Nakama Lua match handler registration | 2026-02-17 | Return callback table instead of nk.register_match() |
| ERROR-025 | Docker build: Rust version too old | 2026-02-17 | Updated Dockerfile to rust:1.93-bookworm |
| ERROR-026 | Docker build: cargo cache dummy binary | 2026-02-17 | Added `touch` after COPY to force recompilation |

---

## Error Statistics
- Total errors logged: 26
- Open: 0
- Resolved: 26
- Known patterns: 26
- Common error patterns: See COMMON-ERRORS.md (16 CE + 5 BP)
