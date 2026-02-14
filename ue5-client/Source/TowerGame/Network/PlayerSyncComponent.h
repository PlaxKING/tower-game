#pragma once

#include "CoreMinimal.h"
#include "Components/ActorComponent.h"
#include "Network/MatchConnection.h"
#include "PlayerSyncComponent.generated.h"

class ARemotePlayer;
class ATowerPlayerCharacter;

/**
 * Manages synchronization between local player, remote players, and the match.
 *
 * Responsibilities:
 * - Periodically sends local player position to match (5Hz)
 * - Listens for match data events and routes them
 * - Spawns/despawns ARemotePlayer actors for other players
 * - Applies position updates to remote players with interpolation
 * - Broadcasts combat events (attacks, deaths) to remote player visuals
 *
 * Attach to the local player character (ATowerPlayerCharacter).
 */
UCLASS(ClassGroup = (Network), meta = (BlueprintSpawnableComponent))
class TOWERGAME_API UPlayerSyncComponent : public UActorComponent
{
    GENERATED_BODY()

public:
    UPlayerSyncComponent();

    virtual void BeginPlay() override;
    virtual void EndPlay(const EEndPlayReason::Type EndPlayReason) override;
    virtual void TickComponent(float DeltaTime, ELevelTick TickType, FActorComponentTickFunction* ThisTickFunction) override;

    // ============ Config ============

    /** How often to send position updates (Hz) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Sync")
    float SendRate = 5.0f;

    /** Class to spawn for remote players */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Sync")
    TSubclassOf<ARemotePlayer> RemotePlayerClass;

    // ============ Remote Players ============

    /** Get all currently connected remote players */
    UFUNCTION(BlueprintPure, Category = "Sync")
    TArray<ARemotePlayer*> GetRemotePlayers() const;

    /** Get remote player by user ID */
    UFUNCTION(BlueprintPure, Category = "Sync")
    ARemotePlayer* GetRemotePlayerById(const FString& UserId) const;

    /** Get number of players on this floor (including local) */
    UFUNCTION(BlueprintPure, Category = "Sync")
    int32 GetPlayerCount() const { return RemotePlayers.Num() + 1; }

    // ============ Controls ============

    /** Start syncing (called after match connection established) */
    UFUNCTION(BlueprintCallable, Category = "Sync")
    void StartSync();

    /** Stop syncing and cleanup remote players */
    UFUNCTION(BlueprintCallable, Category = "Sync")
    void StopSync();

    UFUNCTION(BlueprintPure, Category = "Sync")
    bool IsSyncing() const { return bSyncing; }

private:
    UPROPERTY()
    TMap<FString, ARemotePlayer*> RemotePlayers;

    float SendTimer = 0.0f;
    bool bSyncing = false;

    UMatchConnection* GetMatchConnection() const;

    // ============ Event Handlers ============

    UFUNCTION()
    void OnMatchDataReceived(EMatchOpCode OpCode, const FString& DataJson);

    void HandlePlayerPosition(const FString& UserId, const FString& DataJson);
    void HandlePlayerAttack(const FString& UserId, const FString& DataJson);
    void HandlePlayerDeath(const FString& UserId, const FString& DataJson);
    void HandlePlayerJoined(const FString& UserId, const FString& DataJson);
    void HandlePlayerLeft(const FString& UserId);
    void HandleChat(const FString& UserId, const FString& DataJson);

    /** Spawn a remote player actor */
    ARemotePlayer* SpawnRemotePlayer(const FString& UserId, const FString& DisplayName);

    /** Despawn a remote player actor */
    void DespawnRemotePlayer(const FString& UserId);

    /** Despawn all remote players */
    void DespawnAllRemotePlayers();

    /** Send local player position to match */
    void BroadcastLocalPosition();
};
