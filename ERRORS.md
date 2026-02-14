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

---

## Error Statistics
- Total errors logged: 7
- Open: 0
- Resolved: 7
- Known patterns: 14
- Common error patterns: See COMMON-ERRORS.md (16 CE + 5 BP)
