// Copyright Epic Games, Inc. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "Subsystems/GameInstanceSubsystem.h"
#include "TowerNetworkSubsystem.generated.h"

class AReplicationManager;

/**
 * Game Instance Subsystem for network management
 * Provides Blueprint-friendly interface to networking
 */
UCLASS(BlueprintType)
class TOWERGAME_API UTowerNetworkSubsystem : public UGameInstanceSubsystem
{
    GENERATED_BODY()

public:
    // Subsystem lifecycle
    virtual void Initialize(FSubsystemCollectionBase& Collection) override;
    virtual void Deinitialize() override;

    // Connection management
    UFUNCTION(BlueprintCallable, Category = "Network")
    bool ConnectToServer(const FString& ServerIP = TEXT("127.0.0.1"), int32 Port = 5000);

    UFUNCTION(BlueprintCallable, Category = "Network")
    void DisconnectFromServer();

    UFUNCTION(BlueprintPure, Category = "Network")
    bool IsConnected() const;

    UFUNCTION(BlueprintPure, Category = "Network")
    FString GetConnectionStatus() const;

    // Network info
    UFUNCTION(BlueprintPure, Category = "Network")
    int64 GetClientId() const;

    UFUNCTION(BlueprintPure, Category = "Network")
    int32 GetPlayerCount() const;

    UFUNCTION(BlueprintPure, Category = "Network")
    int32 GetMonsterCount() const;

    UFUNCTION(BlueprintPure, Category = "Network")
    float GetPing() const;

    // Events
    DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnConnected);
    UPROPERTY(BlueprintAssignable, Category = "Network|Events")
    FOnConnected OnConnected;

    DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnDisconnected);
    UPROPERTY(BlueprintAssignable, Category = "Network|Events")
    FOnDisconnected OnDisconnected;

    DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnPlayerCountChanged, int32, NewCount);
    UPROPERTY(BlueprintAssignable, Category = "Network|Events")
    FOnPlayerCountChanged OnPlayerCountChanged;

    // Replication manager access
    UFUNCTION(BlueprintPure, Category = "Network")
    AReplicationManager* GetReplicationManager() const { return ReplicationManager; }

protected:
    UPROPERTY()
    AReplicationManager* ReplicationManager;

    // Connection state
    bool bIsConnected;
    FString ServerIP;
    int32 ServerPort;
    int64 ClientId;

    // Stats
    int32 LastPlayerCount;
    float LastPingTime;

    // Tick for monitoring
    void TickSubsystem();
    FTimerHandle TickTimerHandle;

private:
    void HandlePlayerSpawned(AActor* PlayerActor);
    void HandlePlayerUpdated(AActor* PlayerActor);
};

/**
 * Blueprint Function Library for network utilities
 */
UCLASS()
class TOWERGAME_API UNetworkBlueprintLibrary : public UBlueprintFunctionLibrary
{
    GENERATED_BODY()

public:
    // Quick access to TowerNetworkSubsystem
    UFUNCTION(BlueprintPure, Category = "Network", meta = (WorldContext = "WorldContextObject"))
    static UTowerNetworkSubsystem* GetTowerNetworkSubsystem(const UObject* WorldContextObject);

    // Connection helpers
    UFUNCTION(BlueprintCallable, Category = "Network", meta = (WorldContext = "WorldContextObject"))
    static bool QuickConnect(const UObject* WorldContextObject, const FString& ServerIP = TEXT("127.0.0.1"));

    UFUNCTION(BlueprintCallable, Category = "Network", meta = (WorldContext = "WorldContextObject"))
    static void QuickDisconnect(const UObject* WorldContextObject);

    // Info getters
    UFUNCTION(BlueprintPure, Category = "Network", meta = (WorldContext = "WorldContextObject"))
    static bool IsConnectedToServer(const UObject* WorldContextObject);

    UFUNCTION(BlueprintPure, Category = "Network", meta = (WorldContext = "WorldContextObject"))
    static FString GetConnectionInfo(const UObject* WorldContextObject);

    // Debug helpers
    UFUNCTION(BlueprintCallable, Category = "Network|Debug")
    static void LogNetworkStats(const FString& Message);

    UFUNCTION(BlueprintPure, Category = "Network|Debug")
    static FString FormatBytes(int64 Bytes);

    UFUNCTION(BlueprintPure, Category = "Network|Debug")
    static FString FormatLatency(float Milliseconds);
};

/**
 * Struct for Blueprint-friendly network stats display
 */
USTRUCT(BlueprintType)
struct FNetworkStats
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadOnly, Category = "Stats")
    bool bConnected = false;

    UPROPERTY(BlueprintReadOnly, Category = "Stats")
    int64 ClientId = 0;

    UPROPERTY(BlueprintReadOnly, Category = "Stats")
    int32 PlayerCount = 0;

    UPROPERTY(BlueprintReadOnly, Category = "Stats")
    int32 MonsterCount = 0;

    UPROPERTY(BlueprintReadOnly, Category = "Stats")
    float Ping = 0.0f;

    UPROPERTY(BlueprintReadOnly, Category = "Stats")
    int32 PacketsReceived = 0;

    UPROPERTY(BlueprintReadOnly, Category = "Stats")
    int64 BytesReceived = 0;

    UPROPERTY(BlueprintReadOnly, Category = "Stats")
    FString ServerAddress;
};
