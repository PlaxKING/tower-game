// Copyright Epic Games, Inc. All Rights Reserved.

#include "ReplicationManager.h"
#include "Engine/World.h"
#include "Components/StaticMeshComponent.h"
#include "UObject/ConstructorHelpers.h"

AReplicationManager::AReplicationManager()
{
    PrimaryActorTick.bCanEverTick = true;
    bReplicates = false; // This is client-side only

    PacketsReceived = 0;
    BytesReceived = 0;
    LastUpdateTime = 0.0f;

    // Create netcode client
    NetcodeClient = CreateDefaultSubobject<UNetcodeClient>(TEXT("NetcodeClient"));
}

void AReplicationManager::BeginPlay()
{
    Super::BeginPlay();

    UE_LOG(LogTemp, Log, TEXT("ReplicationManager: Ready"));
}

void AReplicationManager::Tick(float DeltaTime)
{
    Super::Tick(DeltaTime);

    if (!NetcodeClient || !NetcodeClient->IsConnected())
    {
        return;
    }

    // Update netcode client
    NetcodeClient->Tick(DeltaTime);

    // Process received packets
    ProcessReceivedPackets();

    LastUpdateTime += DeltaTime;
}

void AReplicationManager::ConnectToServer(const FString& ServerIP, int32 Port)
{
    if (!NetcodeClient)
    {
        UE_LOG(LogTemp, Error, TEXT("ReplicationManager: NetcodeClient is null!"));
        return;
    }

    UE_LOG(LogTemp, Log, TEXT("ReplicationManager: Connecting to %s:%d"), *ServerIP, Port);

    if (NetcodeClient->Connect(ServerIP, Port))
    {
        UE_LOG(LogTemp, Log, TEXT("ReplicationManager: Connected! Client ID: %llu"), NetcodeClient->GetClientId());
    }
    else
    {
        UE_LOG(LogTemp, Error, TEXT("ReplicationManager: Failed to connect"));
    }
}

void AReplicationManager::Disconnect()
{
    if (NetcodeClient)
    {
        NetcodeClient->Disconnect();
    }

    // Clean up replicated actors
    for (auto& Pair : ReplicatedPlayers)
    {
        if (Pair.Value)
        {
            Pair.Value->Destroy();
        }
    }
    ReplicatedPlayers.Empty();

    for (auto& Pair : ReplicatedMonsters)
    {
        if (Pair.Value)
        {
            Pair.Value->Destroy();
        }
    }
    ReplicatedMonsters.Empty();

    for (AActor* Tile : ReplicatedTiles)
    {
        if (Tile)
        {
            Tile->Destroy();
        }
    }
    ReplicatedTiles.Empty();

    UE_LOG(LogTemp, Log, TEXT("ReplicationManager: Disconnected and cleaned up"));
}

bool AReplicationManager::IsConnected() const
{
    return NetcodeClient && NetcodeClient->IsConnected();
}

int64 AReplicationManager::GetClientId() const
{
    return NetcodeClient ? NetcodeClient->GetClientId() : 0;
}

void AReplicationManager::ProcessReceivedPackets()
{
    TArray<TArray<uint8>> Packets;
    if (!NetcodeClient->ReceivePackets(Packets))
    {
        return; // No packets
    }

    for (const TArray<uint8>& PacketData : Packets)
    {
        if (PacketData.Num() == 0)
        {
            continue;
        }

        PacketsReceived++;
        BytesReceived += PacketData.Num();

        // Read packet type
        FBincodeReader Reader(PacketData);
        uint8 PacketTypeByte = Reader.ReadU8();
        EPacketType PacketType = static_cast<EPacketType>(PacketTypeByte);

        // Process based on type
        switch (PacketType)
        {
            case EPacketType::Keepalive:
                // Ignore keepalive
                break;

            case EPacketType::PlayerUpdate:
            case EPacketType::PlayerSpawn:
                ProcessPlayerData(Reader);
                break;

            case EPacketType::MonsterUpdate:
                ProcessMonsterData(Reader);
                break;

            case EPacketType::FloorTileUpdate:
                ProcessFloorTileData(Reader);
                break;

            case EPacketType::PlayerDespawn:
            {
                int64 PlayerId = Reader.ReadU64();
                if (AActor** FoundActor = ReplicatedPlayers.Find(PlayerId))
                {
                    if (*FoundActor)
                    {
                        (*FoundActor)->Destroy();
                    }
                    ReplicatedPlayers.Remove(PlayerId);
                }
                break;
            }

            default:
                UE_LOG(LogTemp, Warning, TEXT("ReplicationManager: Unknown packet type: %d"), PacketTypeByte);
                break;
        }
    }

    // Log stats every 5 seconds
    if (LastUpdateTime >= 5.0f)
    {
        UE_LOG(LogTemp, Log, TEXT("ReplicationManager Stats: %d packets, %d bytes, %d players, %d monsters"),
            PacketsReceived, BytesReceived, ReplicatedPlayers.Num(), ReplicatedMonsters.Num());
        LastUpdateTime = 0.0f;
    }
}

void AReplicationManager::ProcessPlayerData(FBincodeReader& Reader)
{
    FPlayerData PlayerData = FPlayerData::FromBincode(Reader);

    if (!Reader.IsValid())
    {
        UE_LOG(LogTemp, Error, TEXT("ReplicationManager: Failed to parse PlayerData"));
        return;
    }

    AActor* PlayerActor = SpawnOrUpdatePlayer(PlayerData);

    if (PlayerActor)
    {
        UE_LOG(LogTemp, Verbose, TEXT("ReplicationManager: Player %llu updated at %s"),
            PlayerData.Id, *PlayerData.Position.ToString());
    }
}

void AReplicationManager::ProcessMonsterData(FBincodeReader& Reader)
{
    FMonsterData MonsterData = FMonsterData::FromBincode(Reader);

    if (!Reader.IsValid())
    {
        UE_LOG(LogTemp, Error, TEXT("ReplicationManager: Failed to parse MonsterData"));
        return;
    }

    SpawnOrUpdateMonster(MonsterData);
}

void AReplicationManager::ProcessFloorTileData(FBincodeReader& Reader)
{
    FFloorTileData TileData = FFloorTileData::FromBincode(Reader);

    if (!Reader.IsValid())
    {
        UE_LOG(LogTemp, Error, TEXT("ReplicationManager: Failed to parse FloorTileData"));
        return;
    }

    SpawnFloorTile(TileData);
}

AActor* AReplicationManager::SpawnOrUpdatePlayer(const FPlayerData& PlayerData)
{
    // Check if player already exists
    if (AActor** ExistingActor = ReplicatedPlayers.Find(PlayerData.Id))
    {
        if (*ExistingActor)
        {
            UpdatePlayerActor(*ExistingActor, PlayerData);
            OnPlayerUpdated.Broadcast(*ExistingActor);
            return *ExistingActor;
        }
    }

    // Spawn new player
    UWorld* World = GetWorld();
    if (!World)
    {
        return nullptr;
    }

    FActorSpawnParameters SpawnParams;
    SpawnParams.SpawnCollisionHandlingOverride = ESpawnActorCollisionHandlingMethod::AlwaysSpawn;

    AActor* NewActor = nullptr;

    if (PlayerActorClass)
    {
        NewActor = World->SpawnActor<AActor>(PlayerActorClass, PlayerData.Position, FRotator::ZeroRotator, SpawnParams);
    }
    else
    {
        // Fallback to simple actor
        NewActor = World->SpawnActor<AReplicatedPlayerActor>(PlayerData.Position, FRotator::ZeroRotator, SpawnParams);
    }

    if (NewActor)
    {
        UpdatePlayerActor(NewActor, PlayerData);
        ReplicatedPlayers.Add(PlayerData.Id, NewActor);

        OnPlayerSpawned.Broadcast(NewActor);

        UE_LOG(LogTemp, Log, TEXT("ReplicationManager: Spawned player %llu at %s"),
            PlayerData.Id, *PlayerData.Position.ToString());
    }

    return NewActor;
}

AActor* AReplicationManager::SpawnOrUpdateMonster(const FMonsterData& MonsterData)
{
    // For monsters, use position as hash key (no unique ID from server yet)
    // TODO: Add monster IDs to server protocol
    int64 MonsterHash = static_cast<int64>(MonsterData.Position.X * 1000 + MonsterData.Position.Y * 100 + MonsterData.Position.Z * 10);

    if (AActor** ExistingActor = ReplicatedMonsters.Find(MonsterHash))
    {
        if (*ExistingActor)
        {
            UpdateMonsterActor(*ExistingActor, MonsterData);
            return *ExistingActor;
        }
    }

    // Spawn new monster
    UWorld* World = GetWorld();
    if (!World)
    {
        return nullptr;
    }

    FActorSpawnParameters SpawnParams;
    SpawnParams.SpawnCollisionHandlingOverride = ESpawnActorCollisionHandlingMethod::AlwaysSpawn;

    AActor* NewActor = World->SpawnActor<AReplicatedMonsterActor>(MonsterData.Position, FRotator::ZeroRotator, SpawnParams);

    if (NewActor)
    {
        UpdateMonsterActor(NewActor, MonsterData);
        ReplicatedMonsters.Add(MonsterHash, NewActor);

        UE_LOG(LogTemp, Log, TEXT("ReplicationManager: Spawned monster '%s' at %s"),
            *MonsterData.MonsterType, *MonsterData.Position.ToString());
    }

    return NewActor;
}

AActor* AReplicationManager::SpawnFloorTile(const FFloorTileData& TileData)
{
    UWorld* World = GetWorld();
    if (!World || !FloorTileActorClass)
    {
        return nullptr;
    }

    // Calculate world position from grid coordinates
    FVector WorldPosition(TileData.GridX * 100.0f, TileData.GridY * 100.0f, 0.0f);

    FActorSpawnParameters SpawnParams;
    AActor* NewTile = World->SpawnActor<AActor>(FloorTileActorClass, WorldPosition, FRotator::ZeroRotator, SpawnParams);

    if (NewTile)
    {
        ReplicatedTiles.Add(NewTile);
    }

    return NewTile;
}

void AReplicationManager::UpdatePlayerActor(AActor* Actor, const FPlayerData& Data)
{
    if (!Actor)
    {
        return;
    }

    // Update position
    Actor->SetActorLocation(Data.Position);

    // Update custom properties if actor is AReplicatedPlayerActor
    if (AReplicatedPlayerActor* PlayerActor = Cast<AReplicatedPlayerActor>(Actor))
    {
        PlayerActor->UpdateFromData(Data);
    }
}

void AReplicationManager::UpdateMonsterActor(AActor* Actor, const FMonsterData& Data)
{
    if (!Actor)
    {
        return;
    }

    Actor->SetActorLocation(Data.Position);

    if (AReplicatedMonsterActor* MonsterActor = Cast<AReplicatedMonsterActor>(Actor))
    {
        MonsterActor->UpdateFromData(Data);
    }
}

// ============================================================================
// AReplicatedPlayerActor
// ============================================================================

AReplicatedPlayerActor::AReplicatedPlayerActor()
{
    PrimaryActorTick.bCanEverTick = false;

    MeshComponent = CreateDefaultSubobject<UStaticMeshComponent>(TEXT("Mesh"));
    RootComponent = MeshComponent;

    // Load a simple cube mesh for visualization
    static ConstructorHelpers::FObjectFinder<UStaticMesh> CubeMesh(TEXT("/Engine/BasicShapes/Cube"));
    if (CubeMesh.Succeeded())
    {
        MeshComponent->SetStaticMesh(CubeMesh.Object);
        MeshComponent->SetRelativeScale3D(FVector(0.5f, 0.5f, 1.0f)); // Human-sized
    }

    PlayerId = 0;
    Health = 100.0f;
    CurrentFloor = 1;
}

void AReplicatedPlayerActor::UpdateFromData(const FPlayerData& Data)
{
    PlayerId = Data.Id;
    Health = Data.Health;
    CurrentFloor = Data.CurrentFloor;

    // Change color based on health
    if (MeshComponent)
    {
        UMaterialInstanceDynamic* DynMat = MeshComponent->CreateDynamicMaterialInstance(0);
        if (DynMat)
        {
            float HealthPercent = Health / 100.0f;
            FLinearColor Color = FLinearColor::LerpUsingHSV(FLinearColor::Red, FLinearColor::Green, HealthPercent);
            DynMat->SetVectorParameterValue(TEXT("Color"), Color);
        }
    }
}

// ============================================================================
// AReplicatedMonsterActor
// ============================================================================

AReplicatedMonsterActor::AReplicatedMonsterActor()
{
    PrimaryActorTick.bCanEverTick = false;

    MeshComponent = CreateDefaultSubobject<UStaticMeshComponent>(TEXT("Mesh"));
    RootComponent = MeshComponent;

    static ConstructorHelpers::FObjectFinder<UStaticMesh> SphereMesh(TEXT("/Engine/BasicShapes/Sphere"));
    if (SphereMesh.Succeeded())
    {
        MeshComponent->SetStaticMesh(SphereMesh.Object);
        MeshComponent->SetRelativeScale3D(FVector(0.75f));
    }

    Health = 0.0f;
    MaxHealth = 100.0f;
}

void AReplicatedMonsterActor::UpdateFromData(const FMonsterData& Data)
{
    MonsterType = Data.MonsterType;
    Health = Data.Health;
    MaxHealth = Data.MaxHealth;

    // Color monsters red
    if (MeshComponent)
    {
        UMaterialInstanceDynamic* DynMat = MeshComponent->CreateDynamicMaterialInstance(0);
        if (DynMat)
        {
            DynMat->SetVectorParameterValue(TEXT("Color"), FLinearColor::Red);
        }
    }
}
