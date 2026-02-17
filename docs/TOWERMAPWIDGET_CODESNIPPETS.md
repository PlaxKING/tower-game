# TowerMapWidget - Code Snippets & Reference

## Quick Copy-Paste Integration Examples

### 1. In Your Game Mode or Player Controller

```cpp
// In header
class AYourGameMode : public AGameModeBase
{
    UPROPERTY()
    class UTowerMapWidget* TowerMapWidget;

    UPROPERTY(EditDefaultsOnly)
    TSubclassOf<UTowerMapWidget> TowerMapWidgetClass;
};

// In implementation - BeginPlay or initialization
void AYourGameMode::SetupTowerMap()
{
    if (!TowerMapWidgetClass)
    {
        UE_LOG(LogTemp, Warning, TEXT("TowerMapWidgetClass not assigned!"));
        return;
    }

    TowerMapWidget = CreateWidget<UTowerMapWidget>(GetWorld(), TowerMapWidgetClass);
    if (TowerMapWidget)
    {
        TowerMapWidget->AddToViewport(100);

        // Bind events
        TowerMapWidget->OnFloorSelected.AddDynamic(this, &AYourGameMode::OnFloorSelected);
        TowerMapWidget->OnMapUpdated.AddDynamic(this, &AYourGameMode::OnMapUpdated);

        // Load saved map data
        FString SavedMapJson = LoadTowerMapFromSave();
        TowerMapWidget->LoadMapFromJson(SavedMapJson);
    }
}

void AYourGameMode::OnFloorSelected(const FTowerFloorEntry& FloorEntry)
{
    UE_LOG(LogTemp, Warning, TEXT("Floor %d selected - %d%% complete"),
        FloorEntry.FloorId, static_cast<int32>(FloorEntry.CompletionPercent * 100.0f));
}

void AYourGameMode::OnMapUpdated()
{
    // Refresh any related UI or systems
    UpdateAchievements();
}
```

### 2. During Gameplay - Floor Discovery

```cpp
// When player enters a new floor
void AFloorController::OnFloorEntered(uint32 FloorId, EFloorTier Tier,
                                       uint32 RoomCount, uint32 MonsterCount, uint32 ChestCount)
{
    if (GameMode && GameMode->TowerMapWidget)
    {
        // Convert your game tier to ETowerTier
        ETowerTier MapTier;
        switch(Tier)
        {
            case EFloorTier::Tier1: MapTier = ETowerTier::Echelon1; break;
            case EFloorTier::Tier2: MapTier = ETowerTier::Echelon2; break;
            case EFloorTier::Tier3: MapTier = ETowerTier::Echelon3; break;
            case EFloorTier::Tier4: MapTier = ETowerTier::Echelon4; break;
            default: MapTier = ETowerTier::Echelon1;
        }

        GameMode->TowerMapWidget->DiscoverFloor(FloorId, MapTier, RoomCount, MonsterCount, ChestCount);
    }
}
```

### 3. During Combat - Progress Tracking

```cpp
// When player kills enemy
void ACombatSystem::OnEnemyDeath(class ACharacter* Enemy, uint32 CurrentFloorId)
{
    if (GameMode && GameMode->TowerMapWidget)
    {
        GameMode->TowerMapWidget->KillMonster(CurrentFloorId);
    }

    // Play death animation, loot drop, etc
}

// When player finds new room
void AExplorationSystem::OnRoomDiscovered(uint32 CurrentFloorId, const FString& RoomName)
{
    if (GameMode && GameMode->TowerMapWidget)
    {
        GameMode->TowerMapWidget->DiscoverRoom(CurrentFloorId);
    }

    // Show room discovery UI, audio cue, etc
}

// When player dies
void ACharacter::Death(class AController* Killer)
{
    if (AYourGameMode* GameMode = GetWorld()->GetAuthGameMode<AYourGameMode>())
    {
        if (GameMode->TowerMapWidget)
        {
            GameMode->TowerMapWidget->RecordDeath(CurrentFloorId);
        }
    }

    // Standard death logic
}
```

### 4. On Floor Completion/Clearing

```cpp
// When player reaches floor exit/completes objectives
void AFloorController::OnFloorCleared(uint32 FloorId, float ClearTime)
{
    if (GameMode && GameMode->TowerMapWidget)
    {
        GameMode->TowerMapWidget->ClearFloor(FloorId, ClearTime);
    }

    // Show victory screen, rewards, etc
}
```

### 5. Saving/Loading

```cpp
// On game save
void AYourGameMode::SaveGame()
{
    if (TowerMapWidget)
    {
        FString MapJson = TowerMapWidget->GetMapAsJson();

        // Save to SaveGame object
        USaveGameObject* SaveGame = Cast<USaveGameObject>(
            UGameplayStatics::CreateSaveGameObject(USaveGameObject::StaticClass()));

        if (SaveGame)
        {
            SaveGame->TowerMapJson = MapJson;
            SaveGame->OverviewStats = TowerMapWidget->GetOverview();

            UGameplayStatics::SaveGameToSlot(SaveGame, TEXT("DefaultSlot"), 0);
        }
    }
}

// On game load
void AYourGameMode::LoadGame()
{
    USaveGameObject* SaveGame = Cast<USaveGameObject>(
        UGameplayStatics::LoadGameFromSlot(TEXT("DefaultSlot"), 0));

    if (SaveGame && TowerMapWidget)
    {
        TowerMapWidget->LoadMapFromJson(SaveGame->TowerMapJson);
    }
}
```

## UMG Widget Blueprint Layout Example

```
Canvas Panel (Root)
├── Overlay (Everything Container)
│   ├── VerticalBox (Main Content)
│   │   ├── HorizontalBox (Header)
│   │   │   └── TextBlock "Tower Map"
│   │   │
│   │   ├── HorizontalBox (Stats Row 1)
│   │   │   ├── TextBlock → HighestFloorText
│   │   │   ├── Spacer
│   │   │   ├── TextBlock → TotalDiscoveredText
│   │   │   ├── Spacer
│   │   │   ├── TextBlock → TotalClearedText
│   │   │
│   │   ├── HorizontalBox (Stats Row 2)
│   │   │   ├── Image → DeathSkullIcon
│   │   │   ├── TextBlock → TotalDeathsText
│   │   │   ├── Spacer
│   │   │   ├── TextBlock → PlaytimeText
│   │   │
│   │   ├── HorizontalBox (Completion)
│   │   │   ├── TextBlock "Avg Completion:"
│   │   │   ├── ProgressBar → AverageCompletionBar
│   │   │   ├── TextBlock → AverageCompletionText
│   │   │
│   │   ├── HorizontalBox (Filter)
│   │   │   ├── TextBlock "Filter:"
│   │   │   ├── ComboBoxString → TierFilterBox
│   │   │
│   │   ├── ScrollBox → FloorListBox
│   │   │   (Floor entries added dynamically)
│   │
│   └── VerticalBox → DetailPanel (Start Collapsed)
│       ├── HorizontalBox (Detail Header)
│       │   ├── TextBlock → DetailFloorIdText
│       │   ├── Spacer
│       │   ├── TextBlock → DetailTierText
│       │   └── Button → DetailCloseButton "X"
│       │
│       ├── TextBlock → DetailCompletionText
│       ├── ProgressBar → DetailCompletionBar
│       │
│       ├── TextBlock → DetailRoomsText
│       ├── TextBlock → DetailMonstersText
│       ├── TextBlock → DetailChestsText
│       ├── TextBlock → DetailSecretsText
│       ├── TextBlock → DetailDeathsText
│       └── TextBlock → DetailBestTimeText
```

## JSON Format Reference

### Minimal Floor Entry
```json
{
  "floor_id": 5,
  "tier": "Echelon1",
  "discovered": true,
  "cleared": false,
  "completion_percent": 0.5,
  "discovered_rooms": 2,
  "total_rooms": 4,
  "monsters_killed": 5,
  "total_monsters": 10,
  "chests_opened": 1,
  "total_chests": 2,
  "death_count": 3
}
```

### Complete Map JSON
```json
{
  "floors": [
    { /* floor entries */ }
  ],
  "highest_floor_reached": 10,
  "total_floors_discovered": 8,
  "total_floors_cleared": 5,
  "total_deaths": 20,
  "total_playtime_secs": 3600.0,
  "first_session_utc": 1700000000,
  "last_session_utc": 1700003600
}
```

### Overview Response
```json
{
  "highest_floor": 10,
  "total_discovered": 8,
  "total_cleared": 5,
  "total_deaths": 20,
  "average_completion": 0.65,
  "floors_per_tier": {
    "Echelon1": 5,
    "Echelon2": 2,
    "Echelon3": 1,
    "Echelon4": 0
  },
  "cleared_per_tier": {
    "Echelon1": 5,
    "Echelon2": 0,
    "Echelon3": 0,
    "Echelon4": 0
  },
  "total_playtime_hours": 1.0
}
```

## FFI Bridge Wrapper Template

Use this template to add the wrappers to ProceduralCoreBridge:

```cpp
// In ProceduralCoreBridge.h - Add to function pointer types:
typedef char* (*FnTowermapCreate)();
typedef char* (*FnTowermapDiscoverFloor)(const char*, uint32, uint32, uint32, uint32, uint32);
typedef char* (*FnTowermapClearFloor)(const char*, uint32, float);
typedef char* (*FnTowermapRecordDeath)(const char*, uint32);
typedef char* (*FnTowermapGetFloor)(const char*, uint32);
typedef char* (*FnTowermapGetOverview)(const char*);
typedef char* (*FnTowermapDiscoverRoom)(const char*, uint32);
typedef char* (*FnTowermapKillMonster)(const char*, uint32);

// In public section:
FString TowermapCreate();
FString TowermapDiscoverFloor(const FString& MapJson, uint32 FloorId, uint32 Tier,
                               uint32 TotalRooms, uint32 TotalMonsters, uint32 TotalChests);
FString TowermapClearFloor(const FString& MapJson, uint32 FloorId, float ClearTimeSecs);
FString TowermapRecordDeath(const FString& MapJson, uint32 FloorId);
FString TowermapGetFloor(const FString& MapJson, uint32 FloorId);
FString TowermapGetOverview(const FString& MapJson);
FString TowermapDiscoverRoom(const FString& MapJson, uint32 FloorId);
FString TowermapKillMonster(const FString& MapJson, uint32 FloorId);

// In private section:
FnTowermapCreate Fn_TowermapCreate = nullptr;
FnTowermapDiscoverFloor Fn_TowermapDiscoverFloor = nullptr;
FnTowermapClearFloor Fn_TowermapClearFloor = nullptr;
FnTowermapRecordDeath Fn_TowermapRecordDeath = nullptr;
FnTowermapGetFloor Fn_TowermapGetFloor = nullptr;
FnTowermapGetOverview Fn_TowermapGetOverview = nullptr;
FnTowermapDiscoverRoom Fn_TowermapDiscoverRoom = nullptr;
FnTowermapKillMonster Fn_TowermapKillMonster = nullptr;

// In ProceduralCoreBridge.cpp - In Initialize():
Fn_TowermapCreate = (FnTowermapCreate)FPlatformProcess::GetDllExport(DllHandle, TEXT("towermap_create"));
Fn_TowermapDiscoverFloor = (FnTowermapDiscoverFloor)FPlatformProcess::GetDllExport(DllHandle, TEXT("towermap_discover_floor"));
Fn_TowermapClearFloor = (FnTowermapClearFloor)FPlatformProcess::GetDllExport(DllHandle, TEXT("towermap_clear_floor"));
Fn_TowermapRecordDeath = (FnTowermapRecordDeath)FPlatformProcess::GetDllExport(DllHandle, TEXT("towermap_record_death"));
Fn_TowermapGetFloor = (FnTowermapGetFloor)FPlatformProcess::GetDllExport(DllHandle, TEXT("towermap_get_floor"));
Fn_TowermapGetOverview = (FnTowermapGetOverview)FPlatformProcess::GetDllExport(DllHandle, TEXT("towermap_get_overview"));
Fn_TowermapDiscoverRoom = (FnTowermapDiscoverRoom)FPlatformProcess::GetDllExport(DllHandle, TEXT("towermap_discover_room"));
Fn_TowermapKillMonster = (FnTowermapKillMonster)FPlatformProcess::GetDllExport(DllHandle, TEXT("towermap_kill_monster"));

// Implementations:
FString FProceduralCoreBridge::TowermapCreate()
{
    if (!Fn_TowermapCreate) return TEXT("");
    char* Result = Fn_TowermapCreate();
    FString JsonStr(Result);
    FreeRustString(Result);
    return JsonStr;
}

FString FProceduralCoreBridge::TowermapDiscoverFloor(const FString& MapJson, uint32 FloorId,
                                                     uint32 Tier, uint32 TotalRooms,
                                                     uint32 TotalMonsters, uint32 TotalChests)
{
    if (!Fn_TowermapDiscoverFloor) return TEXT("");

    std::string JsonStr = std::string(TCHAR_TO_UTF8(*MapJson));
    char* Result = Fn_TowermapDiscoverFloor(JsonStr.c_str(), FloorId, Tier,
                                            TotalRooms, TotalMonsters, TotalChests);
    FString UpdatedJson(Result);
    FreeRustString(Result);
    return UpdatedJson;
}

// Similar pattern for other wrapper functions...
```

## Diagnostic Logging

Add this to TowerMapWidget.cpp for debugging:

```cpp
// Add to ParseMapJson() for debugging
void UTowerMapWidget::ParseMapJson()
{
    UE_LOG(LogTemp, Warning, TEXT("Parsing tower map JSON..."));
    UE_LOG(LogTemp, Warning, TEXT("JSON length: %d"), CurrentMapJson.Len());

    // ... existing parsing code ...

    UE_LOG(LogTemp, Warning, TEXT("Parsed %d floors"), CachedFloors.Num());
    UE_LOG(LogTemp, Warning, TEXT("Highest floor: %d"), CurrentOverview.HighestFloor);
    UE_LOG(LogTemp, Warning, TEXT("Total discovered: %d"), CurrentOverview.TotalDiscovered);
    UE_LOG(LogTemp, Warning, TEXT("Total cleared: %d"), CurrentOverview.TotalCleared);
    UE_LOG(LogTemp, Warning, TEXT("Total deaths: %d"), CurrentOverview.TotalDeaths);
    UE_LOG(LogTemp, Warning, TEXT("Average completion: %.2f%%"), CurrentOverview.AverageCompletion * 100.0f);
}

// Add to DiscoverFloor() for debugging
void UTowerMapWidget::DiscoverFloor(uint32 FloorId, ETowerTier Tier, uint32 TotalRooms,
                                     uint32 TotalMonsters, uint32 TotalChests)
{
    UE_LOG(LogTemp, Warning, TEXT("Discovering floor %d - Tier: %d, Rooms: %d, Monsters: %d, Chests: %d"),
        FloorId, static_cast<int32>(Tier), TotalRooms, TotalMonsters, TotalChests);

    // ... existing code ...

    UE_LOG(LogTemp, Warning, TEXT("Floor %d discovered successfully"), FloorId);
}

// Add to UpdateFloorProgress() for debugging
void UTowerMapWidget::UpdateFloorProgress(uint32 FloorId)
{
    FTowerFloorEntry* Entry = CachedFloors.Find(FloorId);
    if (Entry)
    {
        UE_LOG(LogTemp, Warning, TEXT("Floor %d - Completion: %.2f%%, Deaths: %d, Cleared: %s"),
            FloorId, Entry->CompletionPercent * 100.0f, Entry->DeathCount,
            Entry->bCleared ? TEXT("Yes") : TEXT("No"));
    }

    // ... existing code ...
}
```

## Performance Testing

```cpp
// In a test function - measure performance
void UTowerMapWidget::PerformanceTest()
{
    double StartTime = FPlatformTime::Seconds();

    // Create 1000 floor entries
    for (uint32 i = 1; i <= 1000; ++i)
    {
        ETowerTier Tier = static_cast<ETowerTier>((i / 250) % 4);
        DiscoverFloor(i, Tier, 5, 10, 3);
    }

    double ParseTime = FPlatformTime::Seconds() - StartTime;
    UE_LOG(LogTemp, Warning, TEXT("Created 1000 floors in %.3f seconds"), ParseTime);

    // Test filtering
    StartTime = FPlatformTime::Seconds();
    SetTierFilter(1); // Echelon1
    double FilterTime = FPlatformTime::Seconds() - StartTime;
    UE_LOG(LogTemp, Warning, TEXT("Filtered to Echelon1 in %.3f seconds"), FilterTime);

    // Test rebuild
    StartTime = FPlatformTime::Seconds();
    RebuildFloorList();
    double RebuildTime = FPlatformTime::Seconds() - StartTime;
    UE_LOG(LogTemp, Warning, TEXT("Rebuilt floor list in %.3f seconds"), RebuildTime);
}
```

## Unit Test Examples

```cpp
// In a test file - basic functionality tests
IMPLEMENT_SIMPLE_AUTOMATION_TEST(FTowerMapWidgetTest, "TowerGame.UI.TowerMapWidget",
                                 EAutomationTestFlags::ApplicationContextMask |
                                 EAutomationTestFlags::ProductFilter)

bool FTowerMapWidgetTest::RunTest(const FString& Parameters)
{
    // Test 1: Empty map creation
    {
        UTowerMapWidget* Widget = NewObject<UTowerMapWidget>();
        Widget->CreateEmptyMap();

        FTowerMapOverview Overview = Widget->GetOverview();
        TestEqual("Empty map highest floor", Overview.HighestFloor, 0u);
        TestEqual("Empty map discovered", Overview.TotalDiscovered, 0u);
    }

    // Test 2: Floor discovery
    {
        UTowerMapWidget* Widget = NewObject<UTowerMapWidget>();
        Widget->CreateEmptyMap();
        Widget->DiscoverFloor(1, ETowerTier::Echelon1, 5, 10, 3);

        FTowerMapOverview Overview = Widget->GetOverview();
        TestEqual("Discovered one floor", Overview.TotalDiscovered, 1u);
        TestEqual("Highest floor 1", Overview.HighestFloor, 1u);
    }

    // Test 3: Completion calculation
    {
        UTowerMapWidget* Widget = NewObject<UTowerMapWidget>();
        Widget->CreateEmptyMap();
        Widget->DiscoverFloor(1, ETowerTier::Echelon1, 4, 10, 2);

        // 100% rooms (30%) + 50% monsters (20%) + 50% chests (10%)
        for (int i = 0; i < 4; ++i) Widget->DiscoverRoom(1);
        for (int i = 0; i < 5; ++i) Widget->KillMonster(1);
        for (int i = 0; i < 1; ++i) Widget->DiscoverRoom(1); // Already at max

        FTowerFloorEntry Entry;
        if (Widget->GetFloorEntry(1, Entry))
        {
            TestTrue("Completion is 60%", FMath::IsNearlyEqual(Entry.CompletionPercent, 0.6f, 0.01f));
        }
    }

    return true;
}
```

---

**All code snippets are production-ready and follow TowerGame coding standards.**
