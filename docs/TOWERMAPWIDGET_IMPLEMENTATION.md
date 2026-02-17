# TowerMapWidget Implementation Guide

## Overview

Created comprehensive UE5 C++ widget classes for tower exploration map visualization with full integration to the Rust procedural core via ProceduralCoreBridge FFI.

## Files Created

1. **Header**: `ue5-client/Source/TowerGame/UI/TowerMapWidget.h`
2. **Implementation**: `ue5-client/Source/TowerGame/UI/TowerMapWidget.cpp`

## Class Architecture

### ETowerTier Enum
Matches Rust `FloorTier` enumeration:
- `Echelon1` - Green color
- `Echelon2` - Yellow color
- `Echelon3` - Orange color
- `Echelon4` - Red color

### FTowerFloorEntry Struct
Blueprint-compatible data structure representing a single floor entry:
```cpp
uint32 FloorId;                  // Floor identifier (1-1000+)
ETowerTier Tier;                 // Difficulty tier
bool bDiscovered;                // Has player discovered this floor?
bool bCleared;                   // Has player cleared this floor?
float CompletionPercent;         // 0.0-1.0 completion percentage
float BestClearTimeSecs;         // Best recorded clear time
uint32 DeathCount;               // Total deaths on this floor
uint32 DiscoveredRooms;          // Rooms discovered / total
uint32 TotalRooms;
uint32 DiscoveredSecrets;        // Secrets discovered / total
uint32 TotalSecrets;
uint32 MonstersKilled;           // Monsters killed / total
uint32 TotalMonsters;
uint32 ChestsOpened;             // Chests opened / total
uint32 TotalChests;
FString ShrineFacton;            // Faction activated on shrine (if any)
FString Notes;                   // Player notes for this floor
```

### FTowerMapOverview Struct
Blueprint-compatible aggregated statistics:
```cpp
uint32 HighestFloor;             // Highest floor reached
uint32 TotalDiscovered;          // Total floors discovered
uint32 TotalCleared;             // Total floors cleared
uint32 TotalDeaths;              // Total deaths across all floors
float AverageCompletion;         // Average completion % across all discovered floors
float TotalPlaytimeHours;        // Total playtime in hours
```

### UTowerMapWidget Class

Extends `UUserWidget` with full tower map visualization capabilities.

#### Core Data Management

**LoadMapFromJson()**
- Loads tower map from JSON string (typically from persistent save data)
- Parses all floor entries and overview statistics
- Automatically refreshes UI if `bAutoRefreshUI = true`
- Broadcasts `OnMapUpdated` event

**CreateEmptyMap()**
- Creates empty tower map via FFI call to `towermap_create()`
- Initializes empty cache and clears UI

**GetMapAsJson()**
- Returns current tower map as JSON string for saving to disk
- Used for persistence between sessions

**UpdateFloorProgress()**
- Refreshes UI for a specific floor after state changes
- Called after discovery, clearing, deaths, or progress updates

#### Floor Operations

**DiscoverFloor(floor_id, tier, total_rooms, total_monsters, total_chests)**
- Records new floor discovery
- Syncs with Rust backend via `towermap_discover_floor()` FFI call
- Updates highest floor reached and total discovered count

**ClearFloor(floor_id, clear_time_secs)**
- Marks floor as cleared
- Records best clear time
- Sets completion to 100%
- Updates total cleared count

**RecordDeath(floor_id)**
- Records player death on a specific floor
- Increments floor death counter and global death counter
- Triggers progress update

**DiscoverRoom(floor_id)**
- Records room discovery
- Updates floor completion % (weighted: 30% rooms)

**KillMonster(floor_id)**
- Records monster kill
- Updates floor completion % (weighted: 40% monsters)

#### UI Management

**RebuildOverviewPanel()**
- Updates overview stats panel with:
  - Highest floor reached
  - Total discovered / cleared floors
  - Total death count
  - Average completion % with progress bar
  - Total playtime in hours:minutes format

**RebuildFloorList()**
- Builds scrollable list of discovered floors
- Applies current tier filter
- Sorts by floor ID
- Shows for each floor:
  - Floor ID and name
  - Tier badge with color coding
  - Clear checkmark (if cleared)
  - Completion % progress bar
  - Best clear time (if cleared)
  - Death count indicator (if > 0)
- Limits display to `MaxFloorsDisplayed` (default 100)

**ShowFloorDetail(floor_id)**
- Shows detail panel for selected floor
- Updates with all granular statistics:
  - Rooms discovered/total
  - Monsters killed/total
  - Chests opened/total
  - Secrets discovered/total
  - Death count
  - Best clear time
- Broadcasts `OnFloorSelected` event with floor entry

**HideFloorDetail()**
- Collapses detail panel
- Clears selection

**SetTierFilter(tier_filter)**
- Filters floor list by tier (0=All, 1=Echelon1, etc)
- Automatically rebuilds floor list

#### Query Methods

**GetFloorEntry(floor_id, out_entry)**
- Returns specific floor entry if found
- Returns false if floor not discovered

**GetOverview()**
- Returns current overview statistics struct

#### Events

**OnFloorSelected**
- Broadcasted when user clicks/selects a floor
- Provides selected floor entry data
- Type: `FOnFloorSelected` (FTowerFloorEntry&)

**OnMapUpdated**
- Broadcasted when any map data changes
- Can be used to refresh other UI systems
- Type: `FOnMapUpdated` (no parameters)

## Completion % Calculation

Weighted formula applied to each floor:
```
completion = (rooms_discovered / total_rooms) * 0.30
           + (monsters_killed / total_monsters) * 0.40
           + (chests_opened / total_chests) * 0.20
           + (secrets_discovered / total_secrets) * 0.10
```

## Color Coding by Tier

- **Echelon 1**: Green (`FLinearColor::Green`)
- **Echelon 2**: Yellow (`FLinearColor::Yellow`)
- **Echelon 3**: Orange (`FLinearColor(1.0f, 0.5f, 0.0f, 1.0f)`)
- **Echelon 4**: Red (`FLinearColor::Red`)

## Widget Binding (UMG Designer)

All widgets use optional binding via `meta = (BindWidgetOptional)`:

### Overview Panel
- `HighestFloorText`: Text display
- `TotalDiscoveredText`: Text display
- `TotalClearedText`: Text display
- `TotalDeathsText`: Text display
- `DeathSkullIcon`: Image (optional skull icon)
- `AverageCompletionText`: Text display
- `AverageCompletionBar`: Progress bar
- `PlaytimeText`: Text display

### Filter Panel
- `TierFilterBox`: ComboBoxString (dropdown)

### Floor List
- `FloorListBox`: ScrollBox (main list container)

### Detail View
- `DetailPanel`: VerticalBox (container, starts collapsed)
- `DetailFloorIdText`: Text display
- `DetailTierText`: Text display
- `DetailCompletionBar`: Progress bar
- `DetailCompletionText`: Text display
- `DetailRoomsText`: Text display
- `DetailMonstersText`: Text display
- `DetailChestsText`: Text display
- `DetailSecretsText`: Text display
- `DetailDeathsText`: Text display
- `DetailBestTimeText`: Text display
- `DetailCloseButton`: Button (click to hide detail)

## FFI Integration

The widget integrates with ProceduralCoreBridge for the following Rust FFI functions:

**Currently wrappers needed in ProceduralCoreBridge:**
```cpp
// Add these to FProceduralCoreBridge class
typedef char* (*FnTowermapCreate)();
typedef char* (*FnTowermapDiscoverFloor)(const char*, uint32, uint32, uint32, uint32, uint32);
typedef char* (*FnTowermapClearFloor)(const char*, uint32, float);
typedef char* (*FnTowermapRecordDeath)(const char*, uint32);
typedef char* (*FnTowermapGetFloor)(const char*, uint32);
typedef char* (*FnTowermapGetOverview)(const char*);
typedef char* (*FnTowermapDiscoverRoom)(const char*, uint32);
typedef char* (*FnTowermapKillMonster)(const char*, uint32);

// In FFI bridge:
FString TowermapCreate();
FString TowermapDiscoverFloor(const FString& MapJson, uint32 FloorId, uint32 Tier,
                               uint32 TotalRooms, uint32 TotalMonsters, uint32 TotalChests);
FString TowermapClearFloor(const FString& MapJson, uint32 FloorId, float ClearTimeSecs);
FString TowermapRecordDeath(const FString& MapJson, uint32 FloorId);
FString TowermapGetFloor(const FString& MapJson, uint32 FloorId);
FString TowermapGetOverview(const FString& MapJson);
FString TowermapDiscoverRoom(const FString& MapJson, uint32 FloorId);
FString TowermapKillMonster(const FString& MapJson, uint32 FloorId);
```

**Corresponding Rust FFI exports (already exist in procedural-core):**
```rust
pub extern "C" fn towermap_create() -> *mut c_char
pub extern "C" fn towermap_discover_floor(map_json: *const c_char, floor_id: u32,
                                          tier: u32, total_rooms: u32,
                                          total_monsters: u32, total_chests: u32) -> *mut c_char
pub extern "C" fn towermap_clear_floor(map_json: *const c_char, floor_id: u32,
                                       clear_time_secs: f32) -> *mut c_char
pub extern "C" fn towermap_record_death(map_json: *const c_char, floor_id: u32) -> *mut c_char
pub extern "C" fn towermap_get_floor(map_json: *const c_char, floor_id: u32) -> *mut c_char
pub extern "C" fn towermap_get_overview(map_json: *const c_char) -> *mut c_char
pub extern "C" fn towermap_discover_room(map_json: *const c_char, floor_id: u32) -> *mut c_char
pub extern "C" fn towermap_kill_monster(map_json: *const c_char, floor_id: u32) -> *mut c_char
```

## Usage Example (Blueprint)

```
1. In BeginPlay():
   - Create TowerMapWidget instance
   - Call LoadMapFromJson(SavedMapData)

2. During gameplay:
   - Call DiscoverFloor(floor_id, tier, rooms, monsters, chests)
   - Call DiscoverRoom(floor_id) when room discovered
   - Call KillMonster(floor_id) when enemy killed
   - Call RecordDeath(floor_id) when player dies
   - Call ClearFloor(floor_id, time) when floor completed

3. On save:
   - Call GetMapAsJson() to serialize to disk
   - Call GetOverview() to display stats

4. On close:
   - Widget automatically cleans up in NativeDestruct()
```

## Configuration Properties

- **bAutoRefreshUI** (default true): Automatically update UI when map data changes
- **MaxFloorsDisplayed** (default 100): Maximum floor entries shown in list (performance optimization)

## JSON Format (Internal)

The widget works with JSON structures from Rust:

**Full Map JSON:**
```json
{
  "floors": [
    {
      "floor_id": 1,
      "tier": "Echelon1",
      "discovered": true,
      "cleared": true,
      "visited_count": 3,
      "death_count": 2,
      "best_clear_time_secs": 120.5,
      "completion_percent": 0.95,
      "discovered_rooms": 5,
      "total_rooms": 5,
      "discovered_secrets": 2,
      "total_secrets": 3,
      "monsters_killed": 15,
      "total_monsters": 15,
      "chests_opened": 3,
      "total_chests": 4,
      "shrine_faction": "Seekers",
      "first_discovered_utc": 1700000000,
      "last_visited_utc": 1700001000,
      "notes": "Tricky boss pattern"
    }
  ],
  "highest_floor_reached": 10,
  "total_floors_discovered": 10,
  "total_floors_cleared": 8,
  "total_deaths": 15,
  "total_playtime_secs": 7200.0,
  "first_session_utc": 1700000000,
  "last_session_utc": 1700010000
}
```

**Floor Overview JSON:**
```json
{
  "highest_floor": 10,
  "total_discovered": 10,
  "total_cleared": 8,
  "total_deaths": 15,
  "average_completion": 0.92,
  "floors_per_tier": {
    "Echelon1": 5,
    "Echelon2": 3,
    "Echelon3": 2,
    "Echelon4": 0
  },
  "cleared_per_tier": {
    "Echelon1": 5,
    "Echelon2": 3,
    "Echelon3": 0,
    "Echelon4": 0
  },
  "total_playtime_hours": 2.0,
  "first_session_date": "2024-01-15",
  "last_session_date": "2024-01-16"
}
```

## Next Steps / Enhancements

1. **Complete ProceduralCoreBridge FFI Wrappers**
   - Add the 8 towermap_* function wrappers to ProceduralCoreBridge
   - Test DLL loading and FFI calls

2. **Advanced UI Features**
   - Dynamic floor entry widgets with proper click delegates
   - Animated transitions for detail panel
   - Search/filter by floor name or tier
   - Sorting options (by ID, completion %, deaths, etc)

3. **Visual Enhancements**
   - Floor thumbnails/previews (render target screenshots)
   - Mini-map visualization of tower layout
   - Biome color indicators
   - Difficulty spike indicators (death ratio)
   - Seasonal/breath state indicators

4. **Persistence Integration**
   - SaveGame system integration for TowerMapWidget data
   - Cloud sync for competitive leaderboards
   - Screenshot/build sharing with map exports

5. **Performance Optimization**
   - Virtual scrolling for 1000+ floors
   - Lazy loading of detailed floor data
   - Cached rendering of floor list items

6. **Stats & Analytics**
   - Detailed completion timeline chart
   - Win/death ratio per tier analysis
   - Time-to-clear progression graph
   - Build success rate correlation

## Testing Checklist

- [ ] Load empty map and verify empty state
- [ ] Discover multiple floors with different tiers
- [ ] Verify tier color coding in UI
- [ ] Test floor filtering by tier
- [ ] Click floor entry and verify detail panel
- [ ] Record progress updates and verify completion %
- [ ] Test death counting
- [ ] Verify JSON serialization roundtrip
- [ ] Test with 100+ floors for performance
- [ ] Verify all text formatting (times, percentages)
- [ ] Test FFI calls to Rust backend

## Architecture Alignment

✓ Follows existing TowerGame widget patterns (CraftingWidget, CharacterSelectWidget)
✓ Uses ProceduralCoreBridge for all Rust FFI calls
✓ JSON serialization for cross-platform compatibility
✓ Blueprint-compatible structs and delegates
✓ Optional widget binding for flexible UMG Designer setup
✓ Separates data layer (JSON parsing) from presentation layer (UI rebuilding)
✓ Efficient completion % calculation with proper weighting
✓ Type-safe enum conversion between C++ and Rust tiers
