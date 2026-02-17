// Copyright Epic Games, Inc. All Rights Reserved.

#include "TowerNetworkSubsystem.h"
#include "ReplicationManager.h"
#include "Engine/World.h"
#include "TimerManager.h"
#include "Engine/GameInstance.h"

void UTowerNetworkSubsystem::Initialize(FSubsystemCollectionBase& Collection)
{
    Super::Initialize(Collection);

    UE_LOG(LogTemp, Log, TEXT("TowerNetworkSubsystem: Initialized"));

    bIsConnected = false;
    LastPlayerCount = 0;
    LastPingTime = 0.0f;
    ClientId = 0;

    // Start tick timer (check network status every second)
    if (UWorld* World = GetWorld())
    {
        World->GetTimerManager().SetTimer(
            TickTimerHandle,
            this,
            &UTowerNetworkSubsystem::TickSubsystem,
            1.0f, // Every 1 second
            true  // Loop
        );
    }
}

void UTowerNetworkSubsystem::Deinitialize()
{
    DisconnectFromServer();

    if (UWorld* World = GetWorld())
    {
        World->GetTimerManager().ClearTimer(TickTimerHandle);
    }

    Super::Deinitialize();

    UE_LOG(LogTemp, Log, TEXT("TowerNetworkSubsystem: Deinitialized"));
}

bool UTowerNetworkSubsystem::ConnectToServer(const FString& InServerIP, int32 Port)
{
    if (bIsConnected)
    {
        UE_LOG(LogTemp, Warning, TEXT("TowerNetworkSubsystem: Already connected"));
        return false;
    }

    UE_LOG(LogTemp, Log, TEXT("TowerNetworkSubsystem: Connecting to %s:%d"), *InServerIP, Port);

    ServerIP = InServerIP;
    ServerPort = Port;

    // Spawn ReplicationManager if needed
    UWorld* World = GetWorld();
    if (!World)
    {
        UE_LOG(LogTemp, Error, TEXT("TowerNetworkSubsystem: No world!"));
        return false;
    }

    if (!ReplicationManager)
    {
        FActorSpawnParameters SpawnParams;
        SpawnParams.Name = FName("ReplicationManager");
        ReplicationManager = World->SpawnActor<AReplicationManager>(SpawnParams);

        if (!ReplicationManager)
        {
            UE_LOG(LogTemp, Error, TEXT("TowerNetworkSubsystem: Failed to spawn ReplicationManager"));
            return false;
        }

        // Bind events
        ReplicationManager->OnPlayerSpawned.AddDynamic(this, &UTowerNetworkSubsystem::HandlePlayerSpawned);
        ReplicationManager->OnPlayerUpdated.AddDynamic(this, &UTowerNetworkSubsystem::HandlePlayerUpdated);
    }

    // Connect
    ReplicationManager->ConnectToServer(ServerIP, Port);

    bIsConnected = true;
    ClientId = ReplicationManager->GetClientId();

    OnConnected.Broadcast();

    UE_LOG(LogTemp, Log, TEXT("TowerNetworkSubsystem: Connected! Client ID: %llu"), ClientId);

    return true;
}

void UTowerNetworkSubsystem::DisconnectFromServer()
{
    if (!bIsConnected)
    {
        return;
    }

    UE_LOG(LogTemp, Log, TEXT("TowerNetworkSubsystem: Disconnecting..."));

    if (ReplicationManager)
    {
        ReplicationManager->Disconnect();
        ReplicationManager->Destroy();
        ReplicationManager = nullptr;
    }

    bIsConnected = false;
    ClientId = 0;
    LastPlayerCount = 0;

    OnDisconnected.Broadcast();
}

bool UTowerNetworkSubsystem::IsConnected() const
{
    return bIsConnected && ReplicationManager && ReplicationManager->IsConnected();
}

FString UTowerNetworkSubsystem::GetConnectionStatus() const
{
    if (!bIsConnected)
    {
        return TEXT("Disconnected");
    }

    if (!ReplicationManager)
    {
        return TEXT("Error: No ReplicationManager");
    }

    if (!ReplicationManager->IsConnected())
    {
        return TEXT("Connecting...");
    }

    return FString::Printf(TEXT("Connected to %s:%d (Client ID: %llu)"),
        *ServerIP, ServerPort, ClientId);
}

int64 UTowerNetworkSubsystem::GetClientId() const
{
    return ClientId;
}

int32 UTowerNetworkSubsystem::GetPlayerCount() const
{
    if (!ReplicationManager)
    {
        return 0;
    }

    return ReplicationManager->ReplicatedPlayers.Num();
}

int32 UTowerNetworkSubsystem::GetMonsterCount() const
{
    if (!ReplicationManager)
    {
        return 0;
    }

    return ReplicationManager->ReplicatedMonsters.Num();
}

float UTowerNetworkSubsystem::GetPing() const
{
    // TODO: Implement actual ping measurement
    return LastPingTime;
}

void UTowerNetworkSubsystem::TickSubsystem()
{
    if (!bIsConnected || !ReplicationManager)
    {
        return;
    }

    // Check player count changes
    int32 CurrentPlayerCount = GetPlayerCount();
    if (CurrentPlayerCount != LastPlayerCount)
    {
        OnPlayerCountChanged.Broadcast(CurrentPlayerCount);
        LastPlayerCount = CurrentPlayerCount;
    }

    // Update ping (placeholder)
    LastPingTime = 25.0f; // TODO: Real ping measurement
}

void UTowerNetworkSubsystem::HandlePlayerSpawned(AActor* PlayerActor)
{
    UE_LOG(LogTemp, Log, TEXT("TowerNetworkSubsystem: Player spawned"));
}

void UTowerNetworkSubsystem::HandlePlayerUpdated(AActor* PlayerActor)
{
    // Silent - happens frequently
}

// ============================================================================
// UNetworkBlueprintLibrary
// ============================================================================

UTowerNetworkSubsystem* UNetworkBlueprintLibrary::GetTowerNetworkSubsystem(const UObject* WorldContextObject)
{
    if (!WorldContextObject)
    {
        return nullptr;
    }

    UWorld* World = WorldContextObject->GetWorld();
    if (!World)
    {
        return nullptr;
    }

    UGameInstance* GameInstance = World->GetGameInstance();
    if (!GameInstance)
    {
        return nullptr;
    }

    return GameInstance->GetSubsystem<UTowerNetworkSubsystem>();
}

bool UNetworkBlueprintLibrary::QuickConnect(const UObject* WorldContextObject, const FString& ServerIP)
{
    UTowerNetworkSubsystem* NetSubsystem = GetTowerNetworkSubsystem(WorldContextObject);
    if (!NetSubsystem)
    {
        UE_LOG(LogTemp, Error, TEXT("NetworkBlueprintLibrary: TowerNetworkSubsystem not found"));
        return false;
    }

    return NetSubsystem->ConnectToServer(ServerIP, 5000);
}

void UNetworkBlueprintLibrary::QuickDisconnect(const UObject* WorldContextObject)
{
    UTowerNetworkSubsystem* NetSubsystem = GetTowerNetworkSubsystem(WorldContextObject);
    if (NetSubsystem)
    {
        NetSubsystem->DisconnectFromServer();
    }
}

bool UNetworkBlueprintLibrary::IsConnectedToServer(const UObject* WorldContextObject)
{
    UTowerNetworkSubsystem* NetSubsystem = GetTowerNetworkSubsystem(WorldContextObject);
    return NetSubsystem && NetSubsystem->IsConnected();
}

FString UNetworkBlueprintLibrary::GetConnectionInfo(const UObject* WorldContextObject)
{
    UTowerNetworkSubsystem* NetSubsystem = GetTowerNetworkSubsystem(WorldContextObject);
    if (!NetSubsystem)
    {
        return TEXT("No TowerNetworkSubsystem");
    }

    return NetSubsystem->GetConnectionStatus();
}

void UNetworkBlueprintLibrary::LogNetworkStats(const FString& Message)
{
    UE_LOG(LogTemp, Log, TEXT("[Network Stats] %s"), *Message);
}

FString UNetworkBlueprintLibrary::FormatBytes(int64 Bytes)
{
    if (Bytes < 1024)
    {
        return FString::Printf(TEXT("%lld B"), Bytes);
    }
    else if (Bytes < 1024 * 1024)
    {
        return FString::Printf(TEXT("%.2f KB"), Bytes / 1024.0);
    }
    else if (Bytes < 1024 * 1024 * 1024)
    {
        return FString::Printf(TEXT("%.2f MB"), Bytes / (1024.0 * 1024.0));
    }
    else
    {
        return FString::Printf(TEXT("%.2f GB"), Bytes / (1024.0 * 1024.0 * 1024.0));
    }
}

FString UNetworkBlueprintLibrary::FormatLatency(float Milliseconds)
{
    if (Milliseconds < 30.0f)
    {
        return FString::Printf(TEXT("%.1fms (Excellent)"), Milliseconds);
    }
    else if (Milliseconds < 60.0f)
    {
        return FString::Printf(TEXT("%.1fms (Good)"), Milliseconds);
    }
    else if (Milliseconds < 100.0f)
    {
        return FString::Printf(TEXT("%.1fms (Fair)"), Milliseconds);
    }
    else
    {
        return FString::Printf(TEXT("%.1fms (Poor)"), Milliseconds);
    }
}
