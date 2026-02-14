#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "Components/InstancedStaticMeshComponent.h"
#include "ProceduralFloorRenderer.generated.h"

class UPointLightComponent;
class UBoxComponent;
class UStaticMesh;
class UMaterialInterface;
class UMaterialInstanceDynamic;

// ============================================================================
// Tile type enum — matches Rust tile_to_u8 in bridge/mod.rs
// ============================================================================

UENUM(BlueprintType)
enum class ETowerTileType : uint8
{
	Empty       = 0  UMETA(DisplayName = "Empty"),
	Floor       = 1  UMETA(DisplayName = "Floor"),
	Wall        = 2  UMETA(DisplayName = "Wall"),
	Door        = 3  UMETA(DisplayName = "Door"),
	StairsUp    = 4  UMETA(DisplayName = "Stairs Up"),
	StairsDown  = 5  UMETA(DisplayName = "Stairs Down"),
	Chest       = 6  UMETA(DisplayName = "Chest"),
	Trap        = 7  UMETA(DisplayName = "Trap"),
	Spawner     = 8  UMETA(DisplayName = "Spawner"),
	Shrine      = 9  UMETA(DisplayName = "Shrine"),
	WindColumn  = 10 UMETA(DisplayName = "Wind Column"),
	VoidPit     = 11 UMETA(DisplayName = "Void Pit"),

	MAX         = 12 UMETA(Hidden)
};

// ============================================================================
// Data structs bridging Rust layout data to UE5 rendering
// ============================================================================

/**
 * Per-tile render data received from the Rust procedural core.
 * Each entry maps a grid position to a tile type and optional asset indices.
 */
USTRUCT(BlueprintType)
struct FTileRenderData
{
	GENERATED_BODY()

	/** Grid X coordinate */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor")
	int32 X = 0;

	/** Grid Y coordinate */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor")
	int32 Y = 0;

	/** Tile type from Rust TileType enum */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor")
	ETowerTileType TileType = ETowerTileType::Empty;

	/** Index into TileMeshes override array (-1 = use default for type) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor")
	int32 MeshIndex = -1;

	/** Index into material override array (-1 = use biome default) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor")
	int32 MaterialIndex = -1;
};

/**
 * Per-room render data. Rooms group tiles and carry semantic biome info
 * that drives material selection, lighting, and atmosphere.
 */
USTRUCT(BlueprintType)
struct FRoomRenderData
{
	GENERATED_BODY()

	/** Unique room identifier from Rust */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor")
	int32 RoomId = 0;

	/** Room origin X in tile coordinates */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor")
	int32 X = 0;

	/** Room origin Y in tile coordinates */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor")
	int32 Y = 0;

	/** Room width in tiles */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor")
	int32 Width = 1;

	/** Room height in tiles */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor")
	int32 Height = 1;

	/** Room type string from Rust (e.g. "combat", "treasure", "boss") */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor")
	FString RoomType;

	/** Semantic biome tags for material selection (e.g. "stone,moss,damp") */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor")
	TArray<FString> BiomeTags;

	/** Ambient light color for this room */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor")
	FLinearColor AmbientColor = FLinearColor(0.8f, 0.75f, 0.65f, 1.0f);
};

/**
 * Monster spawn point data for visual placement.
 */
USTRUCT(BlueprintType)
struct FMonsterSpawnData
{
	GENERATED_BODY()

	/** Grid X coordinate */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor")
	int32 X = 0;

	/** Grid Y coordinate */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor")
	int32 Y = 0;

	/** Monster template name from Rust */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor")
	FString MonsterName;

	/** Element type for VFX coloring */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor")
	FString Element;

	/** Size category for mesh scaling */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor")
	FString Size;
};

/**
 * Configuration for floor rendering dimensions and defaults.
 * Sizes in Unreal Units (1 UU = 1 cm).
 */
USTRUCT(BlueprintType)
struct FFloorRenderConfig
{
	GENERATED_BODY()

	/** Width/depth of a single tile in UU */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor", meta = (ClampMin = "50.0", ClampMax = "1000.0"))
	float TileSize = 300.0f;

	/** Height of wall tiles in UU */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor", meta = (ClampMin = "100.0", ClampMax = "1000.0"))
	float WallHeight = 400.0f;

	/** Door opening width in UU */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor", meta = (ClampMin = "50.0", ClampMax = "500.0"))
	float DoorWidth = 200.0f;

	/** Floor slab thickness in UU */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor", meta = (ClampMin = "5.0", ClampMax = "100.0"))
	float FloorThickness = 20.0f;

	/** Default room light intensity */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor", meta = (ClampMin = "0.0", ClampMax = "50000.0"))
	float DefaultLightIntensity = 5000.0f;

	/** Default room light attenuation radius */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor", meta = (ClampMin = "100.0", ClampMax = "5000.0"))
	float DefaultLightRadius = 1200.0f;

	/** Maximum LOD distance — tiles beyond this are culled from ISM */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor", meta = (ClampMin = "1000.0"))
	float MaxLODDistance = 30000.0f;

	/** Enable Nanite for instanced meshes (requires Nanite-enabled meshes) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor")
	bool bEnableNanite = true;

	/** Enable Lumen global illumination */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor")
	bool bEnableLumen = true;
};

// ============================================================================
// Delegates
// ============================================================================

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnFloorGenerated, int32, TotalTiles);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_ThreeParams(FOnTileMutated, int32, X, int32, Y, ETowerTileType, NewType);

// ============================================================================
// ATowerProceduralFloorRenderer
// ============================================================================

/**
 * Procedural floor renderer that bridges Rust layout data to UE5 visuals.
 *
 * Takes tile and room data from the Rust procedural core (via FProceduralCoreBridge)
 * and creates an efficient visual representation using instanced static meshes.
 *
 * Features:
 * - Instanced rendering: tiles grouped by type into ISM components for batching
 * - Nanite-compatible: meshes use Nanite when available for massive poly counts
 * - Lumen-compatible: materials and lights configured for hardware ray-traced GI
 * - Room-based biome materials: semantic tags drive material assignment
 * - Dynamic point lights per room with biome-appropriate color
 * - Collision setup for walls, floors, and interactive objects
 * - Navigation mesh support for AI pathfinding
 * - Runtime mutation: individual tiles can change type (Seed+Delta model)
 * - LOD management for large floor layouts
 */
UCLASS(BlueprintType, Blueprintable)
class TOWERGAME_API ATowerProceduralFloorRenderer : public AActor
{
	GENERATED_BODY()

public:
	ATowerProceduralFloorRenderer();

	virtual void BeginPlay() override;
	virtual void EndPlay(const EEndPlayReason::Type EndPlayReason) override;

	// ============ Mesh & Material Assets ============

	/** Static mesh to use for each tile type. Assign in editor or via data table. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor|Assets")
	TMap<ETowerTileType, UStaticMesh*> TileMeshes;

	/** Material overrides keyed by biome tag (e.g. "stone", "moss", "crystal"). */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor|Assets")
	TMap<FString, UMaterialInterface*> BiomeMaterials;

	/** Fallback material when no biome match is found */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor|Assets")
	UMaterialInterface* DefaultMaterial;

	// ============ Configuration ============

	/** Floor rendering dimensions and settings */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Floor|Config")
	FFloorRenderConfig RenderConfig;

	// ============ Runtime State (Read-Only) ============

	/** Active ISM components, one per tile type */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Floor|Runtime")
	TArray<UInstancedStaticMeshComponent*> TileInstances;

	/** Spawned room lights */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Floor|Runtime")
	TArray<UPointLightComponent*> RoomLights;

	/** Current tile grid — 2D tile type lookup by (X,Y) */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Floor|Runtime")
	TMap<int64, ETowerTileType> TileGrid;

	/** Current room data cache */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Floor|Runtime")
	TArray<FRoomRenderData> CachedRooms;

	/** Total number of rendered tile instances */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Floor|Runtime")
	int32 TotalRenderedTiles = 0;

	// ============ Events ============

	/** Broadcast after floor generation completes */
	UPROPERTY(BlueprintAssignable, Category = "Tower|Floor|Events")
	FOnFloorGenerated OnFloorGenerated;

	/** Broadcast when a single tile is mutated at runtime */
	UPROPERTY(BlueprintAssignable, Category = "Tower|Floor|Events")
	FOnTileMutated OnTileMutated;

	// ============ Primary API ============

	/**
	 * Generate the full floor visual from Rust layout data.
	 * Clears any existing floor, then creates ISM instances for all tiles
	 * and spawns room lighting.
	 *
	 * @param Tiles     Array of tile render data from Rust core
	 * @param Rooms     Array of room render data from Rust core
	 */
	UFUNCTION(BlueprintCallable, Category = "Tower|Floor")
	void GenerateFloorFromData(
		const TArray<FTileRenderData>& Tiles,
		const TArray<FRoomRenderData>& Rooms);

	/**
	 * Destroy all rendered floor geometry, lights, and collision.
	 */
	UFUNCTION(BlueprintCallable, Category = "Tower|Floor")
	void ClearFloor();

	/**
	 * Mutate a single tile at runtime (Seed+Delta model).
	 * Removes the old instance and adds a new one of the target type.
	 *
	 * @param X         Grid X coordinate
	 * @param Y         Grid Y coordinate
	 * @param NewType   The new tile type to place
	 */
	UFUNCTION(BlueprintCallable, Category = "Tower|Floor")
	void UpdateTileState(int32 X, int32 Y, ETowerTileType NewType);

	/**
	 * Place visual markers at monster spawn locations.
	 * Spawn indicators use the Spawner tile mesh with element-based coloring.
	 *
	 * @param Spawns    Array of monster spawn point data
	 */
	UFUNCTION(BlueprintCallable, Category = "Tower|Floor")
	void SpawnMonsterVisuals(const TArray<FMonsterSpawnData>& Spawns);

	/**
	 * Apply a biome atmosphere to the entire floor.
	 * Adjusts ambient lighting, fog, and post-process settings.
	 *
	 * @param BiomeType Biome identifier (e.g. "crystal_cave", "void_rift", "moss_grove")
	 */
	UFUNCTION(BlueprintCallable, Category = "Tower|Floor")
	void SetBiomeAtmosphere(const FString& BiomeType);

	// ============ Queries ============

	/** Get tile type at grid coordinates. Returns Empty if out of bounds. */
	UFUNCTION(BlueprintPure, Category = "Tower|Floor")
	ETowerTileType GetTileAt(int32 X, int32 Y) const;

	/** Convert grid coordinates to world position */
	UFUNCTION(BlueprintPure, Category = "Tower|Floor")
	FVector GridToWorld(int32 X, int32 Y) const;

	/** Convert world position to nearest grid coordinates */
	UFUNCTION(BlueprintPure, Category = "Tower|Floor")
	void WorldToGrid(const FVector& WorldPos, int32& OutX, int32& OutY) const;

	/** Find the room that contains the given grid position. Returns nullptr data if none. */
	UFUNCTION(BlueprintPure, Category = "Tower|Floor")
	bool GetRoomAtGrid(int32 X, int32 Y, FRoomRenderData& OutRoom) const;

private:
	// ============ Scene Root ============

	UPROPERTY(VisibleAnywhere)
	USceneComponent* SceneRoot;

	// ============ Internal Helpers ============

	/** Create or retrieve the ISM component for a tile type */
	UInstancedStaticMeshComponent* GetOrCreateISMForType(ETowerTileType TileType);

	/** Build the transform for a tile at grid position */
	FTransform BuildTileTransform(int32 X, int32 Y, ETowerTileType TileType) const;

	/** Select material for a tile based on room biome tags */
	UMaterialInterface* ResolveBiomeMaterial(const TArray<FString>& BiomeTags) const;

	/** Configure collision profile on an ISM based on tile type */
	void ConfigureCollision(UInstancedStaticMeshComponent* ISM, ETowerTileType TileType);

	/** Configure navigation relevance on an ISM */
	void ConfigureNavigation(UInstancedStaticMeshComponent* ISM, ETowerTileType TileType);

	/** Spawn a point light for a room */
	UPointLightComponent* SpawnRoomLight(const FRoomRenderData& Room);

	/** Get default mesh for a tile type (engine primitive fallback) */
	UStaticMesh* GetDefaultMeshForType(ETowerTileType TileType) const;

	/** Get ambient light color for a biome string */
	static FLinearColor GetBiomeLightColor(const FString& BiomeType);

	/** Get light intensity multiplier for a room type */
	static float GetRoomLightIntensity(const FString& RoomType);

	/** Pack grid coordinates into a single int64 key for TileGrid map */
	static int64 PackGridKey(int32 X, int32 Y);

	/** Unpack grid key back to X,Y coordinates */
	static void UnpackGridKey(int64 Key, int32& OutX, int32& OutY);

	/** Mapping from tile type to ISM component index in TileInstances */
	TMap<ETowerTileType, int32> TypeToISMIndex;

	/** Per-instance mapping: grid key -> (ISM index, instance index within ISM) */
	TMap<int64, TPair<int32, int32>> GridToInstanceMap;

	/** Cached default cube mesh for fallback rendering */
	UPROPERTY()
	UStaticMesh* FallbackCubeMesh;
};
