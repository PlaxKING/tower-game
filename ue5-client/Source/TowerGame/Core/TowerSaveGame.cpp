#include "TowerSaveGame.h"
#include "Kismet/GameplayStatics.h"

UTowerSaveGame::UTowerSaveGame()
{
    GameVersion = TEXT("0.3.0");
    LastSaveTime = FDateTime::Now();
}

// ===========================
// UTowerSaveSubsystem
// ===========================

void UTowerSaveSubsystem::Initialize(FSubsystemCollectionBase& Collection)
{
    Super::Initialize(Collection);

    // Try loading default slot
    if (HasSaveGame(0))
    {
        LoadGame(0);
        UE_LOG(LogTemp, Log, TEXT("Loaded save game from slot 0"));
    }
    else
    {
        CurrentSave = Cast<UTowerSaveGame>(
            UGameplayStatics::CreateSaveGameObject(UTowerSaveGame::StaticClass()));
        UE_LOG(LogTemp, Log, TEXT("Created new save game"));
    }
}

bool UTowerSaveSubsystem::SaveGame(int32 SlotIndex)
{
    if (!CurrentSave) return false;

    CurrentSave->LastSaveTime = FDateTime::Now();
    CurrentSave->SaveVersion = 1;

    FString SlotName = GetSlotName(SlotIndex);
    bool bSuccess = UGameplayStatics::SaveGameToSlot(CurrentSave, SlotName, 0);

    if (bSuccess)
    {
        UE_LOG(LogTemp, Log, TEXT("Saved game to slot %d (%s)"), SlotIndex, *SlotName);
    }
    else
    {
        UE_LOG(LogTemp, Error, TEXT("Failed to save game to slot %d"), SlotIndex);
    }

    return bSuccess;
}

bool UTowerSaveSubsystem::LoadGame(int32 SlotIndex)
{
    FString SlotName = GetSlotName(SlotIndex);

    USaveGame* Loaded = UGameplayStatics::LoadGameFromSlot(SlotName, 0);
    if (!Loaded) return false;

    UTowerSaveGame* TowerSave = Cast<UTowerSaveGame>(Loaded);
    if (!TowerSave) return false;

    CurrentSave = TowerSave;
    UE_LOG(LogTemp, Log, TEXT("Loaded game from slot %d: floor %d, %d items"),
        SlotIndex, CurrentSave->CurrentFloor, CurrentSave->InventoryItems.Num());

    return true;
}

bool UTowerSaveSubsystem::HasSaveGame(int32 SlotIndex) const
{
    return UGameplayStatics::DoesSaveGameExist(GetSlotName(SlotIndex), 0);
}

bool UTowerSaveSubsystem::DeleteSaveGame(int32 SlotIndex)
{
    FString SlotName = GetSlotName(SlotIndex);
    if (!UGameplayStatics::DoesSaveGameExist(SlotName, 0)) return false;

    bool bDeleted = UGameplayStatics::DeleteGameInSlot(SlotName, 0);
    if (bDeleted)
    {
        UE_LOG(LogTemp, Log, TEXT("Deleted save slot %d"), SlotIndex);
    }
    return bDeleted;
}

void UTowerSaveSubsystem::EnableAutoSave(float IntervalSeconds)
{
    bAutoSaveEnabled = true;
    AutoSaveInterval = IntervalSeconds;
    AutoSaveTimer = 0.0f;
    UE_LOG(LogTemp, Log, TEXT("Auto-save enabled: every %.0fs"), IntervalSeconds);
}

void UTowerSaveSubsystem::DisableAutoSave()
{
    bAutoSaveEnabled = false;
    UE_LOG(LogTemp, Log, TEXT("Auto-save disabled"));
}

void UTowerSaveSubsystem::TickAutoSave(float DeltaTime)
{
    if (!bAutoSaveEnabled) return;

    AutoSaveTimer += DeltaTime;
    if (AutoSaveTimer >= AutoSaveInterval)
    {
        AutoSaveTimer = 0.0f;
        SaveGame(0);
    }
}

void UTowerSaveSubsystem::IncrementStat(const FString& StatName, int32 Value)
{
    if (!CurrentSave) return;

    if (StatName == TEXT("Deaths")) CurrentSave->Stats.TotalDeaths += Value;
    else if (StatName == TEXT("Monsters")) CurrentSave->Stats.MonstersSlain += Value;
    else if (StatName == TEXT("Chests")) CurrentSave->Stats.ChestsOpened += Value;
    else if (StatName == TEXT("Quests")) CurrentSave->Stats.QuestsCompleted += Value;
    else if (StatName == TEXT("Crafts")) CurrentSave->Stats.ItemsCrafted += Value;
    else if (StatName == TEXT("Echoes")) CurrentSave->Stats.EchoesEncountered += Value;
}

void UTowerSaveSubsystem::UpdateHighestFloor(int32 Floor)
{
    if (!CurrentSave) return;

    if (Floor > CurrentSave->Stats.HighestFloor)
    {
        CurrentSave->Stats.HighestFloor = Floor;
        CurrentSave->CurrentFloor = Floor;
    }
}

void UTowerSaveSubsystem::AddPlayTime(float Seconds)
{
    if (!CurrentSave) return;
    CurrentSave->Stats.TotalPlayTime += Seconds;
}

FPlayerSaveSettings UTowerSaveSubsystem::GetSettings() const
{
    if (!CurrentSave) return FPlayerSaveSettings();
    return CurrentSave->Settings;
}

void UTowerSaveSubsystem::SaveSettings(const FPlayerSaveSettings& NewSettings)
{
    if (!CurrentSave) return;
    CurrentSave->Settings = NewSettings;
    SaveGame(0); // Immediately persist settings
}

FString UTowerSaveSubsystem::GetCachedAuthToken() const
{
    if (!CurrentSave) return TEXT("");
    return CurrentSave->NakamaAuthToken;
}

void UTowerSaveSubsystem::CacheAuthToken(const FString& UserId, const FString& Token)
{
    if (!CurrentSave) return;
    CurrentSave->NakamaUserId = UserId;
    CurrentSave->NakamaAuthToken = Token;
    SaveGame(0); // Persist auth immediately
}

FString UTowerSaveSubsystem::GetSlotName(int32 SlotIndex) const
{
    return FString::Printf(TEXT("TowerSave_%d"), SlotIndex);
}
