#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "FloorBuilder.generated.h"

/**
 * Static utility actor for spawning floor tile geometry.
 *
 * Tile types from Rust tile_to_u8 (bridge/mod.rs):
 *   0 = Empty, 1 = Floor, 2 = Wall, 3 = Door,
 *   4 = StairsUp, 5 = StairsDown, 6 = Chest, 7 = Trap,
 *   8 = Spawner, 9 = Shrine, 10 = WindColumn, 11 = VoidPit
 */
UCLASS()
class TOWERGAME_API AFloorBuilder : public AActor
{
    GENERATED_BODY()

public:
    AFloorBuilder();

    /**
     * Spawn a single tile actor at the given location.
     * TileType maps to the u8 from Rust bridge.
     */
    static AActor* SpawnTile(UWorld* World, int32 TileType, FVector Location, float TileSize, float WallHeight);

    /** Get the display color for a tile type (used for debug/placeholder meshes) */
    static FLinearColor GetTileColor(int32 TileType);

    /** Get human-readable name for tile type */
    static FString GetTileName(int32 TileType);
};

/**
 * Single floor tile — spawned as a cube with appropriate color/scale.
 * In production, replace with proper static meshes via data table.
 */
UCLASS()
class TOWERGAME_API ATowerTile : public AActor
{
    GENERATED_BODY()

public:
    ATowerTile();

    void InitTile(int32 InTileType, float InTileSize, float InWallHeight);

    UPROPERTY(VisibleAnywhere, Category = "Tower|Tile")
    int32 TileType = 0;

    UPROPERTY(VisibleAnywhere, Category = "Tower|Tile")
    UStaticMeshComponent* MeshComponent;
};
