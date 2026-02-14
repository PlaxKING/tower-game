#include "ProceduralFloorRenderer.h"
#include "Components/InstancedStaticMeshComponent.h"
#include "Components/PointLightComponent.h"
#include "Components/BoxComponent.h"
#include "Engine/StaticMesh.h"
#include "Materials/MaterialInterface.h"
#include "Materials/MaterialInstanceDynamic.h"
#include "UObject/ConstructorHelpers.h"
#include "NavigationSystem.h"

DEFINE_LOG_CATEGORY_STATIC(LogFloorRenderer, Log, All);

// ============================================================================
// Constructor & Lifecycle
// ============================================================================

ATowerProceduralFloorRenderer::ATowerProceduralFloorRenderer()
{
	PrimaryActorTick.bCanEverTick = false;

	SceneRoot = CreateDefaultSubobject<USceneComponent>(TEXT("SceneRoot"));
	RootComponent = SceneRoot;

	// Cache fallback cube mesh for tile types without assigned meshes
	static ConstructorHelpers::FObjectFinder<UStaticMesh> CubeFinder(
		TEXT("/Engine/BasicShapes/Cube.Cube"));
	if (CubeFinder.Succeeded())
	{
		FallbackCubeMesh = CubeFinder.Object;
	}
}

void ATowerProceduralFloorRenderer::BeginPlay()
{
	Super::BeginPlay();

	UE_LOG(LogFloorRenderer, Log, TEXT("ProceduralFloorRenderer ready. TileSize=%.0f WallHeight=%.0f Nanite=%s Lumen=%s"),
		RenderConfig.TileSize,
		RenderConfig.WallHeight,
		RenderConfig.bEnableNanite ? TEXT("ON") : TEXT("OFF"),
		RenderConfig.bEnableLumen ? TEXT("ON") : TEXT("OFF"));
}

void ATowerProceduralFloorRenderer::EndPlay(const EEndPlayReason::Type EndPlayReason)
{
	ClearFloor();
	Super::EndPlay(EndPlayReason);
}

// ============================================================================
// Primary API
// ============================================================================

void ATowerProceduralFloorRenderer::GenerateFloorFromData(
	const TArray<FTileRenderData>& Tiles,
	const TArray<FRoomRenderData>& Rooms)
{
	ClearFloor();

	if (Tiles.Num() == 0)
	{
		UE_LOG(LogFloorRenderer, Warning, TEXT("GenerateFloorFromData called with 0 tiles"));
		return;
	}

	UE_LOG(LogFloorRenderer, Log, TEXT("Generating floor: %d tiles, %d rooms"), Tiles.Num(), Rooms.Num());

	// Cache room data for later queries
	CachedRooms = Rooms;

	// ---- Phase 1: Group tiles by type for batched instanced rendering ----

	TMap<ETowerTileType, TArray<const FTileRenderData*>> TilesByType;
	for (const FTileRenderData& Tile : Tiles)
	{
		if (Tile.TileType == ETowerTileType::Empty)
		{
			continue; // Skip empty tiles — no geometry needed
		}

		TilesByType.FindOrAdd(Tile.TileType).Add(&Tile);

		// Record in tile grid for runtime queries and mutations
		int64 Key = PackGridKey(Tile.X, Tile.Y);
		TileGrid.Add(Key, Tile.TileType);
	}

	// ---- Phase 2: Create ISM components and add instances per type ----

	int32 InstanceCount = 0;

	for (auto& Pair : TilesByType)
	{
		ETowerTileType TileType = Pair.Key;
		const TArray<const FTileRenderData*>& TypeTiles = Pair.Value;

		UInstancedStaticMeshComponent* ISM = GetOrCreateISMForType(TileType);
		if (!ISM)
		{
			UE_LOG(LogFloorRenderer, Error, TEXT("Failed to create ISM for tile type %d"), static_cast<int32>(TileType));
			continue;
		}

		// Find which room each tile belongs to for biome material assignment
		// Use the first tile's room biome as the material for the entire ISM batch.
		// For per-room material variation, split ISMs by (type, biome) key.
		TArray<FString> PrimaryBiomeTags;
		if (Rooms.Num() > 0)
		{
			// Use the biome from the first tile's room
			const FTileRenderData* FirstTile = TypeTiles[0];
			FRoomRenderData FirstRoom;
			if (GetRoomAtGrid(FirstTile->X, FirstTile->Y, FirstRoom))
			{
				PrimaryBiomeTags = FirstRoom.BiomeTags;
			}
		}

		// Assign biome material if available
		UMaterialInterface* BiomeMat = ResolveBiomeMaterial(PrimaryBiomeTags);
		if (BiomeMat)
		{
			ISM->SetMaterial(0, BiomeMat);
		}
		else if (DefaultMaterial)
		{
			ISM->SetMaterial(0, DefaultMaterial);
		}

		// Batch-add all instances of this type
		int32 ISMIdx = TypeToISMIndex.FindChecked(TileType);

		for (const FTileRenderData* TilePtr : TypeTiles)
		{
			FTransform Transform = BuildTileTransform(TilePtr->X, TilePtr->Y, TileType);
			int32 InstIdx = ISM->AddInstance(Transform, /*bWorldSpace=*/false);

			int64 Key = PackGridKey(TilePtr->X, TilePtr->Y);
			GridToInstanceMap.Add(Key, TPair<int32, int32>(ISMIdx, InstIdx));

			InstanceCount++;
		}

		UE_LOG(LogFloorRenderer, Verbose, TEXT("  Type %d: %d instances"), static_cast<int32>(TileType), TypeTiles.Num());
	}

	TotalRenderedTiles = InstanceCount;

	// ---- Phase 3: Room lighting ----

	for (const FRoomRenderData& Room : Rooms)
	{
		UPointLightComponent* Light = SpawnRoomLight(Room);
		if (Light)
		{
			RoomLights.Add(Light);
		}
	}

	UE_LOG(LogFloorRenderer, Log, TEXT("Floor generated: %d tile instances, %d ISM components, %d room lights"),
		TotalRenderedTiles, TileInstances.Num(), RoomLights.Num());

	// Update navigation mesh
	UNavigationSystemV1* NavSys = FNavigationSystem::GetCurrent<UNavigationSystemV1>(GetWorld());
	if (NavSys)
	{
		NavSys->Build();
	}

	OnFloorGenerated.Broadcast(TotalRenderedTiles);
}

void ATowerProceduralFloorRenderer::ClearFloor()
{
	// Destroy all ISM components
	for (UInstancedStaticMeshComponent* ISM : TileInstances)
	{
		if (ISM)
		{
			ISM->ClearInstances();
			ISM->DestroyComponent();
		}
	}
	TileInstances.Empty();
	TypeToISMIndex.Empty();
	GridToInstanceMap.Empty();

	// Destroy all room lights
	for (UPointLightComponent* Light : RoomLights)
	{
		if (Light)
		{
			Light->DestroyComponent();
		}
	}
	RoomLights.Empty();

	// Clear state
	TileGrid.Empty();
	CachedRooms.Empty();
	TotalRenderedTiles = 0;

	UE_LOG(LogFloorRenderer, Log, TEXT("Floor cleared"));
}

void ATowerProceduralFloorRenderer::UpdateTileState(int32 X, int32 Y, ETowerTileType NewType)
{
	int64 Key = PackGridKey(X, Y);

	// Remove old instance if it exists
	if (GridToInstanceMap.Contains(Key))
	{
		TPair<int32, int32> OldMapping = GridToInstanceMap[Key];
		int32 OldISMIdx = OldMapping.Key;
		int32 OldInstIdx = OldMapping.Value;

		if (TileInstances.IsValidIndex(OldISMIdx) && TileInstances[OldISMIdx])
		{
			UInstancedStaticMeshComponent* OldISM = TileInstances[OldISMIdx];

			// Remove instance — note: this swaps the last instance into this slot
			if (OldInstIdx < OldISM->GetInstanceCount())
			{
				OldISM->RemoveInstance(OldInstIdx);
				TotalRenderedTiles--;

				// Fix up the mapping for the instance that was swapped in
				// After RemoveInstance(i), instance at (Count-1) is now at i
				int32 SwappedFromIdx = OldISM->GetInstanceCount(); // was Count-1 before removal, now at OldInstIdx
				if (OldInstIdx < OldISM->GetInstanceCount())
				{
					// Find which grid key pointed to SwappedFromIdx and update it
					for (auto& MapPair : GridToInstanceMap)
					{
						if (MapPair.Value.Key == OldISMIdx && MapPair.Value.Value == SwappedFromIdx)
						{
							MapPair.Value.Value = OldInstIdx;
							break;
						}
					}
				}
			}
		}

		GridToInstanceMap.Remove(Key);
	}

	// Update the grid
	if (NewType == ETowerTileType::Empty)
	{
		TileGrid.Remove(Key);
	}
	else
	{
		TileGrid.Add(Key, NewType);

		// Add new instance
		UInstancedStaticMeshComponent* ISM = GetOrCreateISMForType(NewType);
		if (ISM)
		{
			FTransform Transform = BuildTileTransform(X, Y, NewType);
			int32 ISMIdx = TypeToISMIndex.FindChecked(NewType);
			int32 InstIdx = ISM->AddInstance(Transform, /*bWorldSpace=*/false);

			GridToInstanceMap.Add(Key, TPair<int32, int32>(ISMIdx, InstIdx));
			TotalRenderedTiles++;
		}
	}

	OnTileMutated.Broadcast(X, Y, NewType);

	UE_LOG(LogFloorRenderer, Verbose, TEXT("Tile (%d,%d) mutated to %d"), X, Y, static_cast<int32>(NewType));
}

void ATowerProceduralFloorRenderer::SpawnMonsterVisuals(const TArray<FMonsterSpawnData>& Spawns)
{
	for (const FMonsterSpawnData& SpawnData : Spawns)
	{
		FVector WorldPos = GridToWorld(SpawnData.X, SpawnData.Y);
		WorldPos.Z += RenderConfig.WallHeight * 0.25f; // Elevate slightly above floor

		// Use the Spawner ISM or create a dedicated visual marker
		UInstancedStaticMeshComponent* SpawnerISM = GetOrCreateISMForType(ETowerTileType::Spawner);
		if (SpawnerISM)
		{
			FTransform Transform;
			Transform.SetLocation(WorldPos);

			// Scale by monster size
			float Scale = 1.0f;
			if (SpawnData.Size == TEXT("Tiny"))         Scale = 0.4f;
			else if (SpawnData.Size == TEXT("Small"))    Scale = 0.6f;
			else if (SpawnData.Size == TEXT("Medium"))   Scale = 1.0f;
			else if (SpawnData.Size == TEXT("Large"))    Scale = 1.5f;
			else if (SpawnData.Size == TEXT("Huge"))     Scale = 2.0f;
			else if (SpawnData.Size == TEXT("Colossal")) Scale = 3.0f;

			Transform.SetScale3D(FVector(Scale * 0.5f)); // Smaller than full tile
			SpawnerISM->AddInstance(Transform, /*bWorldSpace=*/false);
		}

		UE_LOG(LogFloorRenderer, Verbose, TEXT("Monster spawn visual: %s [%s] at (%d,%d)"),
			*SpawnData.MonsterName, *SpawnData.Element, SpawnData.X, SpawnData.Y);
	}
}

void ATowerProceduralFloorRenderer::SetBiomeAtmosphere(const FString& BiomeType)
{
	FLinearColor BiomeColor = GetBiomeLightColor(BiomeType);

	// Adjust all existing room lights to blend with the biome atmosphere
	for (UPointLightComponent* Light : RoomLights)
	{
		if (!Light) continue;

		// Blend existing light color 70% room + 30% biome atmosphere
		FLinearColor CurrentColor = Light->GetLightColor();
		FLinearColor BlendedColor = FLinearColor::LerpUsingHSV(CurrentColor, BiomeColor, 0.3f);
		Light->SetLightColor(BlendedColor);
	}

	UE_LOG(LogFloorRenderer, Log, TEXT("Biome atmosphere set: %s (color: R=%.2f G=%.2f B=%.2f)"),
		*BiomeType, BiomeColor.R, BiomeColor.G, BiomeColor.B);
}

// ============================================================================
// Queries
// ============================================================================

ETowerTileType ATowerProceduralFloorRenderer::GetTileAt(int32 X, int32 Y) const
{
	int64 Key = PackGridKey(X, Y);
	const ETowerTileType* Found = TileGrid.Find(Key);
	return Found ? *Found : ETowerTileType::Empty;
}

FVector ATowerProceduralFloorRenderer::GridToWorld(int32 X, int32 Y) const
{
	return GetActorLocation() + FVector(
		static_cast<float>(X) * RenderConfig.TileSize,
		static_cast<float>(Y) * RenderConfig.TileSize,
		0.0f
	);
}

void ATowerProceduralFloorRenderer::WorldToGrid(const FVector& WorldPos, int32& OutX, int32& OutY) const
{
	FVector LocalPos = WorldPos - GetActorLocation();
	OutX = FMath::RoundToInt(LocalPos.X / RenderConfig.TileSize);
	OutY = FMath::RoundToInt(LocalPos.Y / RenderConfig.TileSize);
}

bool ATowerProceduralFloorRenderer::GetRoomAtGrid(int32 X, int32 Y, FRoomRenderData& OutRoom) const
{
	for (const FRoomRenderData& Room : CachedRooms)
	{
		if (X >= Room.X && X < Room.X + Room.Width &&
			Y >= Room.Y && Y < Room.Y + Room.Height)
		{
			OutRoom = Room;
			return true;
		}
	}
	return false;
}

// ============================================================================
// Internal Helpers
// ============================================================================

UInstancedStaticMeshComponent* ATowerProceduralFloorRenderer::GetOrCreateISMForType(ETowerTileType TileType)
{
	// Return existing ISM if already created for this type
	if (int32* ExistingIdx = TypeToISMIndex.Find(TileType))
	{
		if (TileInstances.IsValidIndex(*ExistingIdx))
		{
			return TileInstances[*ExistingIdx];
		}
	}

	// Resolve mesh: editor-assigned > fallback cube
	UStaticMesh* Mesh = nullptr;
	if (UStaticMesh** Found = TileMeshes.Find(TileType))
	{
		Mesh = *Found;
	}
	if (!Mesh)
	{
		Mesh = GetDefaultMeshForType(TileType);
	}
	if (!Mesh)
	{
		UE_LOG(LogFloorRenderer, Error, TEXT("No mesh available for tile type %d"), static_cast<int32>(TileType));
		return nullptr;
	}

	// Create new ISM component
	FName CompName = *FString::Printf(TEXT("ISM_TileType_%d"), static_cast<int32>(TileType));
	UInstancedStaticMeshComponent* ISM = NewObject<UInstancedStaticMeshComponent>(this, CompName);
	ISM->SetStaticMesh(Mesh);
	ISM->SetMobility(EComponentMobility::Static);
	ISM->AttachToComponent(SceneRoot, FAttachmentTransformRules::KeepRelativeTransform);
	ISM->RegisterComponent();

	// Nanite: ISM automatically uses Nanite if the mesh has Nanite data enabled.
	// No additional flag needed — Nanite is a property of the UStaticMesh asset.

	// LOD: set cull distance for large floors
	ISM->SetCullDistances(0.0f, RenderConfig.MaxLODDistance);

	// Collision and navigation
	ConfigureCollision(ISM, TileType);
	ConfigureNavigation(ISM, TileType);

	// Cast shadows (Lumen uses shadow maps for indirect bounces)
	ISM->SetCastShadow(true);

	int32 NewIdx = TileInstances.Add(ISM);
	TypeToISMIndex.Add(TileType, NewIdx);

	return ISM;
}

FTransform ATowerProceduralFloorRenderer::BuildTileTransform(int32 X, int32 Y, ETowerTileType TileType) const
{
	FVector Location(
		static_cast<float>(X) * RenderConfig.TileSize,
		static_cast<float>(Y) * RenderConfig.TileSize,
		0.0f
	);

	// Default scale: tile fills the grid cell. Engine cube is 100x100x100 UU.
	float BaseScale = RenderConfig.TileSize / 100.0f;
	float HeightScale = RenderConfig.FloorThickness / 100.0f;
	FRotator Rotation = FRotator::ZeroRotator;

	switch (TileType)
	{
	case ETowerTileType::Wall:
		HeightScale = RenderConfig.WallHeight / 100.0f;
		Location.Z = RenderConfig.WallHeight * 0.5f;
		break;

	case ETowerTileType::Door:
	{
		// Doors are narrower and slightly shorter than walls
		float DoorScale = RenderConfig.DoorWidth / RenderConfig.TileSize;
		BaseScale *= DoorScale;
		HeightScale = (RenderConfig.WallHeight * 0.8f) / 100.0f;
		Location.Z = (RenderConfig.WallHeight * 0.8f) * 0.5f;
		break;
	}

	case ETowerTileType::StairsUp:
		// Angled slab rising upward
		HeightScale = RenderConfig.FloorThickness / 100.0f * 2.0f;
		Rotation = FRotator(-15.0f, 0.0f, 0.0f);
		Location.Z = RenderConfig.TileSize * 0.25f;
		break;

	case ETowerTileType::StairsDown:
		// Angled slab descending
		HeightScale = RenderConfig.FloorThickness / 100.0f * 2.0f;
		Rotation = FRotator(15.0f, 0.0f, 0.0f);
		Location.Z = -RenderConfig.TileSize * 0.1f;
		break;

	case ETowerTileType::VoidPit:
		// Below ground level
		HeightScale = BaseScale * 0.5f;
		Location.Z = -RenderConfig.TileSize * 0.25f;
		break;

	case ETowerTileType::WindColumn:
		// Tall narrow column
		BaseScale *= 0.4f;
		HeightScale = (RenderConfig.WallHeight * 1.5f) / 100.0f;
		Location.Z = RenderConfig.WallHeight * 0.75f;
		break;

	case ETowerTileType::Chest:
	case ETowerTileType::Shrine:
		// Placed on top of floor, half-tile scale
		BaseScale *= 0.6f;
		HeightScale = BaseScale * 0.6f;
		Location.Z = RenderConfig.FloorThickness + (BaseScale * 0.6f * 100.0f * 0.5f);
		break;

	case ETowerTileType::Trap:
		// Flush with floor, slightly recessed
		HeightScale = RenderConfig.FloorThickness / 100.0f * 0.5f;
		Location.Z = -RenderConfig.FloorThickness * 0.25f;
		break;

	case ETowerTileType::Spawner:
		// Marker on floor surface
		BaseScale *= 0.5f;
		HeightScale = RenderConfig.FloorThickness / 100.0f;
		Location.Z = RenderConfig.FloorThickness;
		break;

	case ETowerTileType::Floor:
	default:
		// Standard floor slab
		Location.Z = 0.0f;
		break;
	}

	FTransform Transform;
	Transform.SetLocation(Location);
	Transform.SetRotation(Rotation.Quaternion());
	Transform.SetScale3D(FVector(BaseScale, BaseScale, HeightScale));

	return Transform;
}

UMaterialInterface* ATowerProceduralFloorRenderer::ResolveBiomeMaterial(const TArray<FString>& BiomeTags) const
{
	// Try each tag in priority order — first match wins
	for (const FString& Tag : BiomeTags)
	{
		if (UMaterialInterface* const* Found = BiomeMaterials.Find(Tag))
		{
			return *Found;
		}
	}

	// Try compound keys (e.g. "stone_moss")
	if (BiomeTags.Num() >= 2)
	{
		FString CompoundKey = BiomeTags[0] + TEXT("_") + BiomeTags[1];
		if (UMaterialInterface* const* Found = BiomeMaterials.Find(CompoundKey))
		{
			return *Found;
		}
	}

	return nullptr;
}

void ATowerProceduralFloorRenderer::ConfigureCollision(UInstancedStaticMeshComponent* ISM, ETowerTileType TileType)
{
	if (!ISM) return;

	switch (TileType)
	{
	case ETowerTileType::Wall:
		// Walls block everything — players, projectiles, AI
		ISM->SetCollisionEnabled(ECollisionEnabled::QueryAndPhysics);
		ISM->SetCollisionObjectType(ECC_WorldStatic);
		ISM->SetCollisionResponseToAllChannels(ECR_Block);
		break;

	case ETowerTileType::Floor:
		// Floors block physics (walking) but allow overlap queries
		ISM->SetCollisionEnabled(ECollisionEnabled::QueryAndPhysics);
		ISM->SetCollisionObjectType(ECC_WorldStatic);
		ISM->SetCollisionResponseToAllChannels(ECR_Block);
		break;

	case ETowerTileType::Door:
		// Doors block by default; gameplay code toggles to overlap when opened
		ISM->SetCollisionEnabled(ECollisionEnabled::QueryAndPhysics);
		ISM->SetCollisionObjectType(ECC_WorldDynamic);
		ISM->SetCollisionResponseToAllChannels(ECR_Block);
		break;

	case ETowerTileType::Trap:
	case ETowerTileType::Chest:
	case ETowerTileType::Shrine:
	case ETowerTileType::Spawner:
		// Interactive objects — overlap only for trigger detection
		ISM->SetCollisionEnabled(ECollisionEnabled::QueryOnly);
		ISM->SetCollisionObjectType(ECC_WorldDynamic);
		ISM->SetCollisionResponseToAllChannels(ECR_Overlap);
		break;

	case ETowerTileType::VoidPit:
		// Void pits — overlap trigger for falling/damage
		ISM->SetCollisionEnabled(ECollisionEnabled::QueryOnly);
		ISM->SetCollisionObjectType(ECC_WorldStatic);
		ISM->SetCollisionResponseToAllChannels(ECR_Overlap);
		break;

	case ETowerTileType::WindColumn:
		// Wind columns — overlap for force application
		ISM->SetCollisionEnabled(ECollisionEnabled::QueryOnly);
		ISM->SetCollisionObjectType(ECC_WorldDynamic);
		ISM->SetCollisionResponseToAllChannels(ECR_Overlap);
		break;

	case ETowerTileType::StairsUp:
	case ETowerTileType::StairsDown:
		// Stairs — solid for walking, block all
		ISM->SetCollisionEnabled(ECollisionEnabled::QueryAndPhysics);
		ISM->SetCollisionObjectType(ECC_WorldStatic);
		ISM->SetCollisionResponseToAllChannels(ECR_Block);
		break;

	default:
		ISM->SetCollisionEnabled(ECollisionEnabled::NoCollision);
		break;
	}
}

void ATowerProceduralFloorRenderer::ConfigureNavigation(UInstancedStaticMeshComponent* ISM, ETowerTileType TileType)
{
	if (!ISM) return;

	switch (TileType)
	{
	case ETowerTileType::Floor:
	case ETowerTileType::Door:
	case ETowerTileType::StairsUp:
	case ETowerTileType::StairsDown:
	case ETowerTileType::Trap:       // AI can walk over traps
	case ETowerTileType::Spawner:    // Walkable spawn area
		// Walkable surfaces — affect navigation mesh generation
		ISM->SetCanEverAffectNavigation(true);
		break;

	case ETowerTileType::Wall:
		// Walls carve out navigation as obstacles
		ISM->SetCanEverAffectNavigation(true);
		break;

	case ETowerTileType::VoidPit:
	case ETowerTileType::WindColumn:
		// Mark as navigation modifiers — AI should avoid but may path through
		ISM->SetCanEverAffectNavigation(true);
		break;

	default:
		ISM->SetCanEverAffectNavigation(false);
		break;
	}
}

UPointLightComponent* ATowerProceduralFloorRenderer::SpawnRoomLight(const FRoomRenderData& Room)
{
	// Place light at room center, elevated above wall height
	FVector RoomCenter(
		(static_cast<float>(Room.X) + static_cast<float>(Room.Width) * 0.5f) * RenderConfig.TileSize,
		(static_cast<float>(Room.Y) + static_cast<float>(Room.Height) * 0.5f) * RenderConfig.TileSize,
		RenderConfig.WallHeight * 0.85f
	);

	UPointLightComponent* Light = NewObject<UPointLightComponent>(this);
	Light->AttachToComponent(SceneRoot, FAttachmentTransformRules::KeepRelativeTransform);
	Light->SetRelativeLocation(RoomCenter);
	Light->RegisterComponent();

	// Color from room ambient color
	Light->SetLightColor(Room.AmbientColor);

	// Intensity scaled by room size and type
	float RoomArea = static_cast<float>(Room.Width * Room.Height);
	float BaseIntensity = RenderConfig.DefaultLightIntensity * GetRoomLightIntensity(Room.RoomType);
	float AreaScale = FMath::Sqrt(RoomArea) / 4.0f; // Normalize around a 4x4 room
	Light->SetIntensity(BaseIntensity * FMath::Max(0.5f, AreaScale));

	// Attenuation radius covers the room
	float RoomDiagonal = FMath::Sqrt(
		FMath::Square(static_cast<float>(Room.Width) * RenderConfig.TileSize) +
		FMath::Square(static_cast<float>(Room.Height) * RenderConfig.TileSize)
	);
	Light->SetAttenuationRadius(FMath::Max(RenderConfig.DefaultLightRadius, RoomDiagonal * 0.75f));

	// Soft shadows for Lumen indirect lighting
	Light->SetCastShadows(true);
	Light->SetSoftSourceRadius(50.0f);

	// Lumen: use inverse squared falloff for physically correct lighting
	Light->SetIntensityUnits(ELightUnits::Candelas);

	UE_LOG(LogFloorRenderer, Verbose, TEXT("Room %d light at (%.0f, %.0f, %.0f) color=(%.2f,%.2f,%.2f) intensity=%.0f"),
		Room.RoomId, RoomCenter.X, RoomCenter.Y, RoomCenter.Z,
		Room.AmbientColor.R, Room.AmbientColor.G, Room.AmbientColor.B,
		BaseIntensity * AreaScale);

	return Light;
}

UStaticMesh* ATowerProceduralFloorRenderer::GetDefaultMeshForType(ETowerTileType TileType) const
{
	// All tile types fall back to the cube primitive.
	// In production, assign Nanite meshes per type via TileMeshes map in the editor.
	return FallbackCubeMesh;
}

FLinearColor ATowerProceduralFloorRenderer::GetBiomeLightColor(const FString& BiomeType)
{
	if (BiomeType == TEXT("crystal_cave"))    return FLinearColor(0.5f, 0.6f, 1.0f);
	if (BiomeType == TEXT("void_rift"))       return FLinearColor(0.3f, 0.1f, 0.5f);
	if (BiomeType == TEXT("moss_grove"))      return FLinearColor(0.4f, 0.8f, 0.3f);
	if (BiomeType == TEXT("fire_pit"))        return FLinearColor(1.0f, 0.5f, 0.2f);
	if (BiomeType == TEXT("ice_cavern"))      return FLinearColor(0.6f, 0.85f, 1.0f);
	if (BiomeType == TEXT("corruption"))      return FLinearColor(0.6f, 0.2f, 0.4f);
	if (BiomeType == TEXT("wind_spire"))      return FLinearColor(0.7f, 0.9f, 0.7f);
	if (BiomeType == TEXT("stone_hall"))      return FLinearColor(0.8f, 0.75f, 0.6f);
	if (BiomeType == TEXT("ancient_library")) return FLinearColor(0.9f, 0.8f, 0.5f);
	if (BiomeType == TEXT("boss_chamber"))    return FLinearColor(0.9f, 0.2f, 0.2f);

	// Default warm dungeon ambient
	return FLinearColor(0.8f, 0.75f, 0.65f);
}

float ATowerProceduralFloorRenderer::GetRoomLightIntensity(const FString& RoomType)
{
	if (RoomType == TEXT("boss"))        return 1.5f;   // Dramatic lighting
	if (RoomType == TEXT("combat"))      return 1.0f;   // Standard
	if (RoomType == TEXT("treasure"))    return 1.3f;   // Inviting glow
	if (RoomType == TEXT("shrine"))      return 1.4f;   // Sacred light
	if (RoomType == TEXT("corridor"))    return 0.6f;   // Dim passageways
	if (RoomType == TEXT("entrance"))    return 1.2f;   // Welcoming
	if (RoomType == TEXT("trap"))        return 0.5f;   // Ominously dark
	if (RoomType == TEXT("secret"))      return 0.4f;   // Hidden, barely lit
	if (RoomType == TEXT("void"))        return 0.3f;   // Eerie faint glow

	return 1.0f;
}

int64 ATowerProceduralFloorRenderer::PackGridKey(int32 X, int32 Y)
{
	// Pack two int32 values into a single int64 for TMap key.
	// Upper 32 bits = X, lower 32 bits = Y.
	return (static_cast<int64>(X) << 32) | (static_cast<int64>(Y) & 0xFFFFFFFF);
}

void ATowerProceduralFloorRenderer::UnpackGridKey(int64 Key, int32& OutX, int32& OutY)
{
	OutX = static_cast<int32>(Key >> 32);
	OutY = static_cast<int32>(Key & 0xFFFFFFFF);
}
