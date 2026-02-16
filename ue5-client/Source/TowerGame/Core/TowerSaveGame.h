#pragma once

#include "CoreMinimal.h"
#include "GameFramework/SaveGame.h"
#include "TowerSaveGame.generated.h"

/// Saved player statistics
USTRUCT(BlueprintType)
struct FPlayerSaveStats
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) int32 HighestFloor = 0;
    UPROPERTY(BlueprintReadWrite) int32 TotalDeaths = 0;
    UPROPERTY(BlueprintReadWrite) int32 MonstersSlain = 0;
    UPROPERTY(BlueprintReadWrite) float TotalPlayTime = 0.0f;
    UPROPERTY(BlueprintReadWrite) int32 ChestsOpened = 0;
    UPROPERTY(BlueprintReadWrite) int32 QuestsCompleted = 0;
    UPROPERTY(BlueprintReadWrite) int32 ItemsCrafted = 0;
    UPROPERTY(BlueprintReadWrite) int32 EchoesEncountered = 0;
};

/// Saved settings (mirrors PauseMenuWidget settings)
USTRUCT(BlueprintType)
struct FPlayerSaveSettings
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) float MasterVolume = 1.0f;
    UPROPERTY(BlueprintReadWrite) float SFXVolume = 1.0f;
    UPROPERTY(BlueprintReadWrite) float MusicVolume = 0.7f;
    UPROPERTY(BlueprintReadWrite) float MouseSensitivity = 1.0f;
    UPROPERTY(BlueprintReadWrite) bool bInvertY = false;
    UPROPERTY(BlueprintReadWrite) bool bShowDamageNumbers = true;
    UPROPERTY(BlueprintReadWrite) bool bRotateMinimap = true;
    UPROPERTY(BlueprintReadWrite) bool bShowTimestamps = false;
};

/// Per-faction reputation snapshot
USTRUCT(BlueprintType)
struct FFactionRepSave
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString FactionName;
    UPROPERTY(BlueprintReadWrite) int32 Reputation = 0;
    UPROPERTY(BlueprintReadWrite) FString Tier; // Hostile, Unfriendly, Neutral, Friendly, Honored, Exalted
};

/// Saved inventory item
USTRUCT(BlueprintType)
struct FInventoryItemSave
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString ItemName;
    UPROPERTY(BlueprintReadWrite) FString Category;
    UPROPERTY(BlueprintReadWrite) FString Rarity;
    UPROPERTY(BlueprintReadWrite) int32 Quantity = 1;
    UPROPERTY(BlueprintReadWrite) FString LootJson; // Full loot data for tooltip
};

/**
 * Local save game. Persisted via UGameplayStatics::SaveGameToSlot.
 * Server (Nakama) is authoritative; this is for offline/fast-resume.
 */
UCLASS()
class TOWERGAME_API UTowerSaveGame : public USaveGame
{
    GENERATED_BODY()

public:
    UTowerSaveGame();

    // --- Identity ---
    UPROPERTY(BlueprintReadWrite) FString PlayerName;
    UPROPERTY(BlueprintReadWrite) FString NakamaUserId;
    UPROPERTY(BlueprintReadWrite) FString NakamaAuthToken;

    // --- Progress ---
    UPROPERTY(BlueprintReadWrite) int32 CurrentFloor = 1;
    UPROPERTY(BlueprintReadWrite) int64 TowerSeed = 0;
    UPROPERTY(BlueprintReadWrite) FPlayerSaveStats Stats;

    // --- Inventory ---
    UPROPERTY(BlueprintReadWrite) TArray<FInventoryItemSave> InventoryItems;
    UPROPERTY(BlueprintReadWrite) int64 TowerShards = 0;
    UPROPERTY(BlueprintReadWrite) int64 EchoFragments = 0;

    // --- Factions ---
    UPROPERTY(BlueprintReadWrite) TArray<FFactionRepSave> FactionReps;

    // --- Settings ---
    UPROPERTY(BlueprintReadWrite) FPlayerSaveSettings Settings;

    // --- Meta ---
    UPROPERTY(BlueprintReadWrite) FDateTime LastSaveTime;
    UPROPERTY(BlueprintReadWrite) int32 SaveVersion = 1;
    UPROPERTY(BlueprintReadWrite) FString GameVersion;
};

/**
 * Save game manager subsystem.
 * Handles auto-save, manual save/load, slot management.
 */
UCLASS()
class TOWERGAME_API UTowerSaveSubsystem : public UGameInstanceSubsystem
{
    GENERATED_BODY()

public:
    virtual void Initialize(FSubsystemCollectionBase& Collection) override;

    // --- Save/Load ---
    UFUNCTION(BlueprintCallable) bool SaveGame(int32 SlotIndex = 0);
    UFUNCTION(BlueprintCallable) bool LoadGame(int32 SlotIndex = 0);
    UFUNCTION(BlueprintCallable) bool HasSaveGame(int32 SlotIndex = 0) const;
    UFUNCTION(BlueprintCallable) bool DeleteSaveGame(int32 SlotIndex = 0);

    // --- Auto-save ---
    UFUNCTION(BlueprintCallable) void EnableAutoSave(float IntervalSeconds = 60.0f);
    UFUNCTION(BlueprintCallable) void DisableAutoSave();
    void TickAutoSave(float DeltaTime);

    // --- Data Access ---
    UFUNCTION(BlueprintCallable) UTowerSaveGame* GetCurrentSave() const { return CurrentSave; }

    // --- Stats ---
    UFUNCTION(BlueprintCallable) void IncrementStat(const FString& StatName, int32 Value = 1);
    UFUNCTION(BlueprintCallable) void UpdateHighestFloor(int32 Floor);
    UFUNCTION(BlueprintCallable) void AddPlayTime(float Seconds);

    // --- Settings shortcuts ---
    UFUNCTION(BlueprintCallable) FPlayerSaveSettings GetSettings() const;
    UFUNCTION(BlueprintCallable) void SaveSettings(const FPlayerSaveSettings& NewSettings);

    // --- Auth token cache ---
    UFUNCTION(BlueprintCallable) FString GetCachedAuthToken() const;
    UFUNCTION(BlueprintCallable) void CacheAuthToken(const FString& UserId, const FString& Token);

private:
    UPROPERTY() UTowerSaveGame* CurrentSave = nullptr;

    FString GetSlotName(int32 SlotIndex) const;

    bool bAutoSaveEnabled = false;
    float AutoSaveInterval = 60.0f;
    float AutoSaveTimer = 0.0f;
};
