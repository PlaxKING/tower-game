#include "FloorBuilder.h"
#include "Components/StaticMeshComponent.h"
#include "Engine/StaticMesh.h"
#include "Materials/MaterialInstanceDynamic.h"
#include "UObject/ConstructorHelpers.h"

// ============ AFloorBuilder ============

AFloorBuilder::AFloorBuilder()
{
    PrimaryActorTick.bCanEverTick = false;
}

FLinearColor AFloorBuilder::GetTileColor(int32 TileType)
{
    // Matches Rust tile_to_u8 in bridge/mod.rs
    switch (TileType)
    {
    case 0:  return FLinearColor(0.1f, 0.1f, 0.1f);    // Empty — dark
    case 1:  return FLinearColor(0.6f, 0.6f, 0.55f);   // Floor — stone gray
    case 2:  return FLinearColor(0.35f, 0.3f, 0.25f);   // Wall — dark brown
    case 3:  return FLinearColor(0.7f, 0.5f, 0.2f);     // Door — wood brown
    case 4:  return FLinearColor(0.2f, 0.8f, 0.3f);     // StairsUp — green
    case 5:  return FLinearColor(0.8f, 0.3f, 0.2f);     // StairsDown — red
    case 6:  return FLinearColor(1.0f, 0.85f, 0.0f);    // Chest — gold
    case 7:  return FLinearColor(0.9f, 0.2f, 0.6f);     // Trap — magenta
    case 8:  return FLinearColor(0.9f, 0.4f, 0.1f);     // Spawner — orange
    case 9:  return FLinearColor(0.4f, 0.6f, 1.0f);     // Shrine — blue
    case 10: return FLinearColor(0.7f, 0.9f, 0.7f);     // WindColumn — light green
    case 11: return FLinearColor(0.05f, 0.0f, 0.15f);   // VoidPit — deep purple
    default: return FLinearColor(1.0f, 0.0f, 1.0f);     // Unknown — magenta
    }
}

FString AFloorBuilder::GetTileName(int32 TileType)
{
    // Matches Rust tile_to_u8 in bridge/mod.rs
    switch (TileType)
    {
    case 0:  return TEXT("Empty");
    case 1:  return TEXT("Floor");
    case 2:  return TEXT("Wall");
    case 3:  return TEXT("Door");
    case 4:  return TEXT("StairsUp");
    case 5:  return TEXT("StairsDown");
    case 6:  return TEXT("Chest");
    case 7:  return TEXT("Trap");
    case 8:  return TEXT("Spawner");
    case 9:  return TEXT("Shrine");
    case 10: return TEXT("WindColumn");
    case 11: return TEXT("VoidPit");
    default: return TEXT("Unknown");
    }
}

AActor* AFloorBuilder::SpawnTile(UWorld* World, int32 TileType, FVector Location, float TileSize, float WallHeight)
{
    if (!World) return nullptr;

    // Skip empty tiles
    if (TileType == 0) return nullptr;

    FActorSpawnParameters SpawnParams;
    SpawnParams.SpawnCollisionHandlingOverride = ESpawnActorCollisionHandlingMethod::AlwaysSpawn;

    ATowerTile* Tile = World->SpawnActor<ATowerTile>(ATowerTile::StaticClass(), Location, FRotator::ZeroRotator, SpawnParams);
    if (Tile)
    {
        Tile->InitTile(TileType, TileSize, WallHeight);
    }
    return Tile;
}

// ============ ATowerTile ============

ATowerTile::ATowerTile()
{
    PrimaryActorTick.bCanEverTick = false;

    MeshComponent = CreateDefaultSubobject<UStaticMeshComponent>(TEXT("TileMesh"));
    RootComponent = MeshComponent;

    // Use engine's default cube mesh
    static ConstructorHelpers::FObjectFinder<UStaticMesh> CubeMesh(
        TEXT("/Engine/BasicShapes/Cube.Cube"));
    if (CubeMesh.Succeeded())
    {
        MeshComponent->SetStaticMesh(CubeMesh.Object);
    }

    MeshComponent->SetCollisionEnabled(ECollisionEnabled::QueryAndPhysics);
    MeshComponent->SetCollisionResponseToAllChannels(ECR_Block);
}

void ATowerTile::InitTile(int32 InTileType, float InTileSize, float InWallHeight)
{
    TileType = InTileType;

    // Scale: cube default is 100x100x100 UU (1m), so divide tile size by 100
    float BaseScale = InTileSize / 100.0f;
    float HeightScale = BaseScale * 0.1f; // Floor is thin

    // Walls are tall (tile 2 = Wall)
    if (InTileType == 2)
    {
        HeightScale = InWallHeight / 100.0f;
        FVector Loc = GetActorLocation();
        Loc.Z = InWallHeight * 0.5f;
        SetActorLocation(Loc);
    }
    // VoidPit is below ground (tile 11 = VoidPit)
    else if (InTileType == 11)
    {
        HeightScale = BaseScale * 0.5f;
        FVector Loc = GetActorLocation();
        Loc.Z = -InTileSize * 0.25f;
        SetActorLocation(Loc);
    }

    MeshComponent->SetWorldScale3D(FVector(BaseScale, BaseScale, HeightScale));

    // Create dynamic material with tile color
    UMaterialInterface* BaseMat = MeshComponent->GetMaterial(0);
    if (BaseMat)
    {
        UMaterialInstanceDynamic* DynMat = UMaterialInstanceDynamic::Create(BaseMat, this);
        FLinearColor Color = AFloorBuilder::GetTileColor(InTileType);
        DynMat->SetVectorParameterValue(TEXT("BaseColor"), Color);
        MeshComponent->SetMaterial(0, DynMat);
    }

#if WITH_EDITOR
    SetActorLabel(FString::Printf(TEXT("Tile_%s"), *AFloorBuilder::GetTileName(InTileType)));
#endif
}
