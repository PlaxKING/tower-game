#include "TowerGameMode.h"
#include "TowerGameSubsystem.h"
#include "TowerGame/World/FloorBuilder.h"
#include "TowerGame/World/MonsterSpawner.h"
#include "Kismet/GameplayStatics.h"
#include "Dom/JsonObject.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

ATowerGameMode::ATowerGameMode()
{
}

void ATowerGameMode::InitGame(const FString& MapName, const FString& Options, FString& ErrorMessage)
{
    Super::InitGame(MapName, Options, ErrorMessage);

    UE_LOG(LogTemp, Log, TEXT("TowerGameMode::InitGame — Tower game initializing"));
}

void ATowerGameMode::StartPlay()
{
    Super::StartPlay();

    UTowerGameSubsystem* Sub = GetTowerSubsystem();
    if (Sub && Sub->IsRustCoreReady())
    {
        UE_LOG(LogTemp, Log, TEXT("Rust Core ready. Version: %s"), *Sub->GetCoreVersion());
        LoadFloor(CurrentFloorId);
    }
    else
    {
        UE_LOG(LogTemp, Error, TEXT("Rust Core not available — cannot generate floors"));
    }
}

UTowerGameSubsystem* ATowerGameMode::GetTowerSubsystem() const
{
    UGameInstance* GI = UGameplayStatics::GetGameInstance(this);
    if (!GI) return nullptr;
    return GI->GetSubsystem<UTowerGameSubsystem>();
}

void ATowerGameMode::LoadFloor(int32 FloorId)
{
    ClearCurrentFloor();

    UTowerGameSubsystem* Sub = GetTowerSubsystem();
    if (!Sub || !Sub->IsRustCoreReady())
    {
        UE_LOG(LogTemp, Error, TEXT("Cannot load floor %d — Rust core not ready"), FloorId);
        return;
    }

    CurrentFloorId = FloorId;
    Sub->CurrentFloor = FloorId;
    if (FloorId > Sub->HighestFloor)
    {
        Sub->HighestFloor = FloorId;
    }

    UE_LOG(LogTemp, Log, TEXT("=== Loading Floor %d ==="), FloorId);

    // 1. Generate floor layout from Rust
    FString LayoutJson = Sub->RequestFloorLayout(Sub->TowerSeed, FloorId);
    if (LayoutJson.IsEmpty())
    {
        UE_LOG(LogTemp, Error, TEXT("Failed to generate floor layout"));
        return;
    }

    // 2. Parse layout JSON
    TSharedPtr<FJsonObject> LayoutObj;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(LayoutJson);
    if (!FJsonSerializer::Deserialize(Reader, LayoutObj) || !LayoutObj.IsValid())
    {
        UE_LOG(LogTemp, Error, TEXT("Failed to parse floor layout JSON"));
        return;
    }

    // 3. Build floor geometry
    UWorld* World = GetWorld();
    int32 Width = LayoutObj->GetIntegerField(TEXT("width"));
    int32 Height = LayoutObj->GetIntegerField(TEXT("height"));

    // Tiles from Rust are a 2D array: tiles[y][x] (each element is a u8 tile type)
    const TArray<TSharedPtr<FJsonValue>>* TilesRows;
    if (LayoutObj->TryGetArrayField(TEXT("tiles"), TilesRows))
    {
        for (int32 y = 0; y < Height && y < TilesRows->Num(); y++)
        {
            const TArray<TSharedPtr<FJsonValue>>& Row = (*TilesRows)[y]->AsArray();
            for (int32 x = 0; x < Width && x < Row.Num(); x++)
            {
                int32 TileType = static_cast<int32>(Row[x]->AsNumber());
                FVector TileLocation(x * TileSize, y * TileSize, 0.0f);

                AActor* TileActor = AFloorBuilder::SpawnTile(World, TileType, TileLocation, TileSize, WallHeight);
                if (TileActor)
                {
                    SpawnedFloorActors.Add(TileActor);
                }
            }
        }
    }

    // 4. Parse rooms and find spawn points
    TArray<FVector> SpawnPoints;
    const TArray<TSharedPtr<FJsonValue>>* RoomsArray;
    if (LayoutObj->TryGetArrayField(TEXT("rooms"), RoomsArray))
    {
        for (const auto& RoomVal : *RoomsArray)
        {
            TSharedPtr<FJsonObject> Room = RoomVal->AsObject();
            if (!Room.IsValid()) continue;

            FString RoomType = Room->GetStringField(TEXT("room_type"));
            int32 RoomX = Room->GetIntegerField(TEXT("x"));
            int32 RoomY = Room->GetIntegerField(TEXT("y"));
            int32 RoomW = Room->GetIntegerField(TEXT("width"));
            int32 RoomH = Room->GetIntegerField(TEXT("height"));

            // Monsters spawn in Combat, Treasure, and Boss rooms
            if (RoomType == TEXT("Combat") || RoomType == TEXT("Boss"))
            {
                FVector Center(
                    (RoomX + RoomW * 0.5f) * TileSize,
                    (RoomY + RoomH * 0.5f) * TileSize,
                    50.0f
                );
                SpawnPoints.Add(Center);
            }
        }
    }

    // 5. Generate and spawn monsters
    int32 MonsterCount = BaseMonstersPerFloor + (FloorId / 5);
    FString MonstersJson = Sub->RequestFloorMonsters(Sub->TowerSeed, FloorId, MonsterCount);

    if (!MonstersJson.IsEmpty())
    {
        TArray<AActor*> MonsterActors = AMonsterSpawner::SpawnMonstersFromJson(
            World, MonstersJson, SpawnPoints, FloorId);

        for (AActor* M : MonsterActors)
        {
            SpawnedFloorActors.Add(M);
        }
        MonstersAlive = MonsterActors.Num();
    }

    bFloorLoaded = true;
    OnFloorLoaded.Broadcast(FloorId);
    UE_LOG(LogTemp, Log, TEXT("Floor %d loaded: %d tiles, %d monsters"), FloorId, SpawnedFloorActors.Num() - MonstersAlive, MonstersAlive);
}

void ATowerGameMode::GoToNextFloor()
{
    LoadFloor(CurrentFloorId + 1);
}

void ATowerGameMode::GoToPreviousFloor()
{
    if (CurrentFloorId > 1)
    {
        LoadFloor(CurrentFloorId - 1);
    }
}

void ATowerGameMode::ClearCurrentFloor()
{
    for (AActor* Actor : SpawnedFloorActors)
    {
        if (Actor && IsValid(Actor))
        {
            Actor->Destroy();
        }
    }
    SpawnedFloorActors.Empty();
    MonstersAlive = 0;
    bFloorLoaded = false;
    OnFloorCleared.Broadcast();
}
