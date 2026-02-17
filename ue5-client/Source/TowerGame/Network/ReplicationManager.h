// Copyright Epic Games, Inc. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "NetcodeClient.h"
#include "BincodeSerializer.h"
#include "ReplicationManager.generated.h"

// Forward declarations

/**
 * Manages replication of entities from Bevy server to UE5
 * Handles spawning, updating, and destroying replicated actors
 */
UCLASS(BlueprintType)
class TOWERGAME_API AReplicationManager : public AActor
{
    GENERATED_BODY()

public:
    AReplicationManager();

    virtual void BeginPlay() override;
    virtual void Tick(float DeltaTime) override;

    // Connection management
    UFUNCTION(BlueprintCallable, Category = "Replication")
    void ConnectToServer(const FString& ServerIP, int32 Port);

    UFUNCTION(BlueprintCallable, Category = "Replication")
    void Disconnect();

    UFUNCTION(BlueprintPure, Category = "Replication")
    bool IsConnected() const;

    UFUNCTION(BlueprintPure, Category = "Replication")
    int64 GetClientId() const;

    // Actor class configuration
    UPROPERTY(EditDefaultsOnly, Category = "Replication")
    TSubclassOf<AActor> PlayerActorClass;

    UPROPERTY(EditDefaultsOnly, Category = "Replication")
    TSubclassOf<AActor> MonsterActorClass;

    UPROPERTY(EditDefaultsOnly, Category = "Replication")
    TSubclassOf<AActor> FloorTileActorClass;

    // Replicated actors
    UPROPERTY(BlueprintReadOnly, Category = "Replication")
    TMap<int64, AActor*> ReplicatedPlayers;

    UPROPERTY(BlueprintReadOnly, Category = "Replication")
    TMap<int64, AActor*> ReplicatedMonsters;

    UPROPERTY(BlueprintReadOnly, Category = "Replication")
    TArray<AActor*> ReplicatedTiles;

    // Events
    DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnPlayerSpawned, AActor*, PlayerActor);
    UPROPERTY(BlueprintAssignable, Category = "Replication")
    FOnPlayerSpawned OnPlayerSpawned;

    DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnPlayerUpdated, AActor*, PlayerActor);
    UPROPERTY(BlueprintAssignable, Category = "Replication")
    FOnPlayerUpdated OnPlayerUpdated;

protected:
    // Netcode client
    UPROPERTY()
    UNetcodeClient* NetcodeClient;

    // Packet processing
    void ProcessReceivedPackets();
    void ProcessPlayerData(FBincodeReader& Reader);
    void ProcessMonsterData(FBincodeReader& Reader);
    void ProcessFloorTileData(FBincodeReader& Reader);

    // Entity management
    AActor* SpawnOrUpdatePlayer(const FPlayerData& PlayerData);
    AActor* SpawnOrUpdateMonster(const FMonsterData& MonsterData);
    AActor* SpawnFloorTile(const FFloorTileData& TileData);

    void UpdatePlayerActor(AActor* Actor, const FPlayerData& Data);
    void UpdateMonsterActor(AActor* Actor, const FMonsterData& Data);

private:
    // Protocol packet types (must match Bevy server)
    enum class EPacketType : uint8
    {
        Keepalive = 0x00,
        PlayerUpdate = 0x01,
        MonsterUpdate = 0x02,
        FloorTileUpdate = 0x03,
        PlayerSpawn = 0x04,
        PlayerDespawn = 0x05,
    };

    // Stats
    UPROPERTY()
    int32 PacketsReceived;

    UPROPERTY()
    int32 BytesReceived;

    UPROPERTY()
    float LastUpdateTime;
};

/**
 * Simple player actor for replication testing
 * Replace with your actual player character class
 */
UCLASS()
class TOWERGAME_API AReplicatedPlayerActor : public AActor
{
    GENERATED_BODY()

public:
    AReplicatedPlayerActor();

    UPROPERTY(BlueprintReadWrite, Category = "Player")
    int64 PlayerId;

    UPROPERTY(BlueprintReadWrite, Category = "Player")
    float Health;

    UPROPERTY(BlueprintReadWrite, Category = "Player")
    int32 CurrentFloor;

    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Components")
    UStaticMeshComponent* MeshComponent;

    void UpdateFromData(const FPlayerData& Data);
};

/**
 * Simple monster actor for replication testing
 */
UCLASS()
class TOWERGAME_API AReplicatedMonsterActor : public AActor
{
    GENERATED_BODY()

public:
    AReplicatedMonsterActor();

    UPROPERTY(BlueprintReadWrite, Category = "Monster")
    FString MonsterType;

    UPROPERTY(BlueprintReadWrite, Category = "Monster")
    float Health;

    UPROPERTY(BlueprintReadWrite, Category = "Monster")
    float MaxHealth;

    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Components")
    UStaticMeshComponent* MeshComponent;

    void UpdateFromData(const FMonsterData& Data);
};
