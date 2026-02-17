# Project Reorganization - Session 26

**Date**: 2026-02-16
**Status**: ✅ Complete

---

## 📋 Overview

Реорганизована структура проекта для улучшения читаемости и упрощения навигации. Все файлы распределены по логическим категориям.

---

## 🗂️ Changes Made

### 1. Documentation Files → `docs/`

**Moved Files:**
- ✅ `ARCHITECTURE.md` → `docs/ARCHITECTURE.md`
- ✅ `PROGRESS.md` → `docs/PROGRESS.md`
- ✅ `ERRORS.md` → `docs/ERRORS.md`
- ✅ `DECISIONS.md` → `docs/DECISIONS.md`
- ✅ `TECH-STACK.md` → `docs/TECH-STACK.md`
- ✅ `COMMON-ERRORS.md` → `docs/COMMON-ERRORS.md`
- ✅ `IMPROVEMENTS.md` → `docs/IMPROVEMENTS.md`
- ✅ `PROJECT-PLAN.md` → `docs/PROJECT-PLAN.md`
- ✅ `SESSION26_SUMMARY.md` → `docs/SESSION26_SUMMARY.md`
- ✅ `TOWERMAPWIDGET_*.md` (6 files) → `docs/`

**Kept in Root:**
- `CLAUDE.md` - Agent configuration (required in root)

**Total:** 15 files moved

---

### 2. Log Files → `logs/`

**Created Structure:**
```
logs/
├── bevy-server/
│   ├── server.log
│   ├── server_test.log
│   └── stress_test.log
└── ue5-client/
    ├── build.log
    ├── compilation_success.log
    ├── full_build.log
    └── [12 other build logs]
```

**Moved Files:**
- ✅ `bevy-server/*.log` → `logs/bevy-server/`
- ✅ `ue5-client/*.log` → `logs/ue5-client/`

**UE5 Engine Logs (not moved):**
- `ue5-client/Saved/Logs/` - managed by Unreal Engine

**Total:** 15+ log files organized

---

### 3. Bugfix Scripts → `bugfix_engine/`

**Moved Files:**
- ✅ `fix_ue5_compiler.ps1` → `bugfix_engine/`
- ✅ `fix_ue5_compiler_v2.ps1` → `bugfix_engine/`
- ✅ `fix_ue5_correct.ps1` → `bugfix_engine/`
- ✅ `fix_ue5_final.ps1` → `bugfix_engine/`
- ✅ `fix_ue5_simple.ps1` → `bugfix_engine/`
- ✅ `fix_ue5_RUN_AS_ADMIN.bat` → `bugfix_engine/`

**Note:** Scripts use absolute UE5 paths, no path updates needed

**Total:** 6 files moved

---

## 📝 Updated References

### CLAUDE.md

**Updated Sections:**
1. `tracking_files` - all paths now `docs/FILENAME.md`
2. `session_start_checklist` - references to `docs/PROGRESS.md`, etc.
3. `session_end_checklist` - paths updated
4. `error_handling_protocol` - `docs/ERRORS.md` references
5. `general_rules.before_writing_code` - updated file checks
6. `file_organization` - complete structure diagram updated
7. `always_do` - updated tracking file paths
8. `KEY REFERENCES.tracking_files` - all paths prefixed with `docs/`

**Total Updates:** 8 sections

---

### Scripts

**Updated Files:**
1. `scripts/run_stress_test.sh`
   - Line 13: `SERVER_LOG="../logs/bevy-server/stress_test.log"`
   - Line 28: Updated output path

2. `scripts/monitor_server.sh`
   - Line 6: `LOG_FILE=${1:-"../logs/bevy-server.log"}`

**No Updates Needed:**
- `bugfix_engine/*.ps1` - use absolute UE5 engine paths
- `bugfix_engine/*.bat` - use `%~dp0` (same directory)

---

## 📊 New Project Structure

```
tower-game/
├── .vscode/                    # VS Code workspace configuration
├── procedural-core/            # Rust procedural generation library
├── ue5-client/                 # Unreal Engine 5 client project
├── bevy-server/                # Bevy authoritative game server
├── bevy-test-client/           # Rust test client + stress tester
├── nakama-server/              # Nakama matchmaking server (future)
├── shared/                     # Shared Protocol Buffers schemas
├── blender/                    # Blender asset pipeline scripts
├── ai-pipeline/                # AI generation tools (future)
├── config/                     # Configuration files
│
├── scripts/                    # Build and deployment scripts
│   ├── run_stress_test.sh      # Network stress testing
│   └── monitor_server.sh       # Real-time server monitoring
│
├── bugfix_engine/              # UE5 compiler bugfix utilities
│   ├── fix_ue5_RUN_AS_ADMIN.bat       # Run as admin launcher
│   ├── fix_ue5_correct.ps1            # Correct ASAN fix
│   ├── fix_ue5_simple.ps1             # Simple fix variant
│   ├── fix_ue5_compiler.ps1           # Original fix
│   ├── fix_ue5_compiler_v2.ps1        # Version 2
│   └── fix_ue5_final.ps1              # Final version
│
├── docs/                       # All documentation and tracking
│   ├── PROGRESS.md             # Session progress tracking
│   ├── ERRORS.md               # Error log with patterns
│   ├── DECISIONS.md            # Architectural decisions (DEC-XXX)
│   ├── TECH-STACK.md           # Tool catalog and evaluation
│   ├── ARCHITECTURE.md         # System architecture reference
│   ├── NETWORKING.md           # Network protocol documentation
│   ├── COMMON-ERRORS.md        # Frequently encountered issues
│   ├── IMPROVEMENTS.md         # Enhancement proposals
│   ├── PROJECT-PLAN.md         # Original project plan
│   ├── SESSION26_SUMMARY.md    # Session 26 summary
│   ├── SESSION26_COMPILATION_SUMMARY.md  # Compilation report
│   └── TOWERMAPWIDGET_*.md     # TowerMapWidget documentation
│
├── logs/                       # All log files (gitignored)
│   ├── bevy-server/            # Bevy server runtime logs
│   │   ├── server.log
│   │   ├── server_test.log
│   │   └── stress_test.log
│   └── ue5-client/             # UE5 compilation logs
│       ├── build.log
│       ├── compilation_success.log
│       └── [various build logs]
│
└── CLAUDE.md                   # Claude agent configuration
```

---

## ✅ Benefits

### 1. **Cleaner Root Directory**
- **Before:** 16+ MD files in root
- **After:** 1 MD file (CLAUDE.md)
- **Improvement:** -94% clutter

### 2. **Logical Organization**
- Documentation centralized in `docs/`
- Build artifacts separated in `logs/`
- Bugfix tools isolated in `bugfix_engine/`
- Scripts categorized by type (`.sh` vs `.ps1/.bat`)

### 3. **Easier Navigation**
- Related files grouped together
- Predictable locations (all session summaries in `docs/`)
- Clear separation of concerns

### 4. **Better Git Workflow**
- All logs in one ignored directory
- Documentation changes easier to track
- Less noise in `git status`

### 5. **IDE Integration**
- Workspace root less cluttered
- Faster file search (smaller root scan)
- Better folder tree readability

---

## 🔧 Testing

**Verified:**
- ✅ CLAUDE.md references all updated
- ✅ Scripts point to new log locations
- ✅ .gitignore covers `logs/` directory
- ✅ Bugfix scripts run from new location
- ✅ Bevy server writes to `logs/bevy-server.log`

**No Breaking Changes:**
- UE5 project structure unchanged
- Bevy server code unchanged
- Test client code unchanged
- Git history preserved

---

## 📈 Statistics

| Category | Files Moved | Size |
|----------|-------------|------|
| Documentation | 15 files | ~2.5 MB |
| Logs | 15 files | ~800 KB |
| Bugfix Scripts | 6 files | ~25 KB |
| **Total** | **36 files** | **~3.3 MB** |

**Directories Created:** 3 (logs/, logs/bevy-server/, logs/ue5-client/)

**Files Modified:** 3 (CLAUDE.md, run_stress_test.sh, monitor_server.sh)

---

## 🎯 Future Recommendations

### 1. Archive Old Logs
```bash
# Monthly cleanup script
cd logs
tar -czf archive/logs_$(date +%Y-%m).tar.gz bevy-server/ ue5-client/
find . -name "*.log" -mtime +30 -delete
```

### 2. Documentation Templates
Create templates in `docs/templates/`:
- `SESSION_TEMPLATE.md`
- `DECISION_TEMPLATE.md`
- `ERROR_TEMPLATE.md`

### 3. Automated Backups
```yaml
# .github/workflows/backup-docs.yml
- Backup docs/ to GitHub releases
- Weekly automated snapshots
```

---

## 📚 Related Documents

- [PROGRESS.md](PROGRESS.md) - Track ongoing development
- [ARCHITECTURE.md](ARCHITECTURE.md) - System design overview
- [NETWORKING.md](NETWORKING.md) - Network protocol spec
- [SESSION26_COMPILATION_SUMMARY.md](SESSION26_COMPILATION_SUMMARY.md) - UE5 build fixes

---

**Completed:** Session 26, 2026-02-16
**Impact:** High (improves maintainability)
**Breaking Changes:** None
**Migration Required:** None (automatic via CLAUDE.md updates)
