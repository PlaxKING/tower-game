#include "DestructibleComponent.h"
#include "GeometryCollection/GeometryCollectionComponent.h"
#include "Components/StaticMeshComponent.h"
#include "NiagaraFunctionLibrary.h"
#include "NiagaraComponent.h"
#include "Engine/StaticMesh.h"
#include "UObject/ConstructorHelpers.h"

// ============================================================================
// UTowerDestructibleComponent
// ============================================================================

UTowerDestructibleComponent::UTowerDestructibleComponent()
{
	PrimaryComponentTick.bCanEverTick = true;
	PrimaryComponentTick.TickInterval = 0.1f; // 10 Hz tick for LOD updates
	GeometryCollectionComp = nullptr;
}

void UTowerDestructibleComponent::BeginPlay()
{
	Super::BeginPlay();

	// Find GeometryCollectionComponent on parent actor
	if (AActor* Owner = GetOwner())
	{
		GeometryCollectionComp = Owner->FindComponentByClass<UGeometryCollectionComponent>();
	}
}

void UTowerDestructibleComponent::TickComponent(
	float DeltaTime,
	ELevelTick TickType,
	FActorComponentTickFunction* ThisTickFunction)
{
	Super::TickComponent(DeltaTime, TickType, ThisTickFunction);

	// Debris cleanup timer
	if (bCollapsed)
	{
		DebrisCleanupTimer += DeltaTime;
		if (DebrisCleanupTimer >= DebrisLifetime && GeometryCollectionComp)
		{
			// Fade out debris fragments after lifetime expires
			GeometryCollectionComp->SetVisibility(false);
			SetComponentTickEnabled(false);
		}
	}
}

void UTowerDestructibleComponent::InitFromServerState(
	uint64 InEntityID,
	const FString& InTemplateID,
	ETowerDestructionMaterial InMaterial,
	float InTotalHP,
	float InMaxTotalHP,
	bool bInCollapsed,
	const TArray<uint8>& InFragmentMask,
	int32 FragmentCount)
{
	ServerEntityID = InEntityID;
	TemplateID = InTemplateID;
	Material = InMaterial;
	TotalHP = InTotalHP;
	MaxTotalHP = InMaxTotalHP;
	bCollapsed = bInCollapsed;
	FragmentMask = InFragmentMask;

	// Initialize fragment array
	Fragments.Empty();
	Fragments.Reserve(FragmentCount);
	for (int32 i = 0; i < FragmentCount; i++)
	{
		FDestructionFragment Frag;
		Frag.ClusterID = static_cast<uint8>(i);
		Frag.MaxHP = InMaxTotalHP / FragmentCount;
		Frag.HP = Frag.MaxHP;
		Frag.bDestroyed = IsFragmentDestroyed(static_cast<uint8>(i));
		if (Frag.bDestroyed)
		{
			Frag.HP = 0.0f;
		}
		Fragments.Add(Frag);
	}

	// Apply visual state for already-destroyed fragments
	if (bInCollapsed || InFragmentMask.Num() > 0)
	{
		TArray<uint8> DestroyedClusters;
		for (int32 i = 0; i < FragmentCount; i++)
		{
			if (IsFragmentDestroyed(static_cast<uint8>(i)))
			{
				DestroyedClusters.Add(static_cast<uint8>(i));
			}
		}
		if (DestroyedClusters.Num() > 0)
		{
			ApplyVisualDestruction(DestroyedClusters, bInCollapsed, FVector::ZeroVector);
		}
	}
}

void UTowerDestructibleComponent::ApplyDestructionDelta(
	const TArray<uint8>& DestroyedClusters,
	const TArray<uint8>& NewFragmentMask,
	bool bStructuralCollapse,
	FVector CollapseImpulse)
{
	// Update fragment mask
	FragmentMask = NewFragmentMask;

	// Update individual fragment states
	for (uint8 ClusterID : DestroyedClusters)
	{
		if (ClusterID < Fragments.Num())
		{
			Fragments[ClusterID].bDestroyed = true;
			Fragments[ClusterID].HP = 0.0f;
		}
	}

	// Update collapse state
	if (bStructuralCollapse)
	{
		bCollapsed = true;
		DebrisCleanupTimer = 0.0f;
	}

	// Recalculate total HP
	TotalHP = 0.0f;
	for (const FDestructionFragment& Frag : Fragments)
	{
		TotalHP += Frag.HP;
	}

	// Apply visual destruction
	ApplyVisualDestruction(DestroyedClusters, bStructuralCollapse, CollapseImpulse);

	// Fire events
	OnDestructionStateChanged.Broadcast(DestroyedClusters, bStructuralCollapse);

	for (uint8 ClusterID : DestroyedClusters)
	{
		FVector FragWorldPos = GetOwner() ? GetOwner()->GetActorLocation() : FVector::ZeroVector;
		if (ClusterID < Fragments.Num())
		{
			FragWorldPos += Fragments[ClusterID].PositionOffset;
		}
		OnFragmentDestroyed.Broadcast(ClusterID, FragWorldPos, Material);
	}
}

void UTowerDestructibleComponent::RequestDamage(
	FVector ImpactPoint,
	float Damage,
	float Radius,
	ETowerDestructionDamageType DamageType,
	const FString& WeaponID,
	const FString& AbilityID)
{
	// Convert damage type to string for JSON API
	FString DamageTypeStr;
	switch (DamageType)
	{
	case ETowerDestructionDamageType::Kinetic:				DamageTypeStr = TEXT("kinetic"); break;
	case ETowerDestructionDamageType::Explosive:			DamageTypeStr = TEXT("explosive"); break;
	case ETowerDestructionDamageType::ElementalFire:		DamageTypeStr = TEXT("fire"); break;
	case ETowerDestructionDamageType::ElementalIce:			DamageTypeStr = TEXT("ice"); break;
	case ETowerDestructionDamageType::ElementalLightning:	DamageTypeStr = TEXT("lightning"); break;
	case ETowerDestructionDamageType::Semantic:				DamageTypeStr = TEXT("semantic"); break;
	}

	// Build JSON request body
	FVector EntityPos = GetOwner() ? GetOwner()->GetActorLocation() : FVector::ZeroVector;

	TSharedPtr<FJsonObject> RequestBody = MakeShareable(new FJsonObject());
	RequestBody->SetNumberField(TEXT("player_id"), 0); // TODO: Get from PlayerState
	RequestBody->SetNumberField(TEXT("target_entity_id"), static_cast<double>(ServerEntityID));
	RequestBody->SetNumberField(TEXT("floor_id"), 0); // TODO: Get from GameState

	TArray<TSharedPtr<FJsonValue>> ImpactArr;
	ImpactArr.Add(MakeShareable(new FJsonValueNumber(ImpactPoint.X)));
	ImpactArr.Add(MakeShareable(new FJsonValueNumber(ImpactPoint.Y)));
	ImpactArr.Add(MakeShareable(new FJsonValueNumber(ImpactPoint.Z)));
	RequestBody->SetArrayField(TEXT("impact_point"), ImpactArr);

	TArray<TSharedPtr<FJsonValue>> EntityPosArr;
	EntityPosArr.Add(MakeShareable(new FJsonValueNumber(EntityPos.X)));
	EntityPosArr.Add(MakeShareable(new FJsonValueNumber(EntityPos.Y)));
	EntityPosArr.Add(MakeShareable(new FJsonValueNumber(EntityPos.Z)));
	RequestBody->SetArrayField(TEXT("entity_position"), EntityPosArr);

	RequestBody->SetNumberField(TEXT("damage"), Damage);
	RequestBody->SetNumberField(TEXT("radius"), Radius);
	RequestBody->SetStringField(TEXT("damage_type"), DamageTypeStr);
	RequestBody->SetStringField(TEXT("weapon_id"), WeaponID);
	RequestBody->SetStringField(TEXT("ability_id"), AbilityID);

	// TODO: Send via GRPCClientManager to /tower.DestructionService/ApplyDamage
	// For now, log the request
	UE_LOG(LogTemp, Log, TEXT("DestructionService/ApplyDamage: entity=%llu damage=%.1f type=%s"),
		ServerEntityID, Damage, *DamageTypeStr);
}

void UTowerDestructibleComponent::RequestRebuild(const TArray<FString>& MaterialItems)
{
	// TODO: Send via GRPCClientManager to /tower.DestructionService/Rebuild
	UE_LOG(LogTemp, Log, TEXT("DestructionService/Rebuild: entity=%llu materials=%d"),
		ServerEntityID, MaterialItems.Num());
}

float UTowerDestructibleComponent::GetDestructionPercentage() const
{
	if (MaxTotalHP <= 0.0f) return 0.0f;
	return 1.0f - (TotalHP / MaxTotalHP);
}

bool UTowerDestructibleComponent::IsFragmentDestroyed(uint8 ClusterID) const
{
	int32 ByteIndex = ClusterID / 8;
	int32 BitIndex = ClusterID % 8;

	if (ByteIndex < FragmentMask.Num())
	{
		return (FragmentMask[ByteIndex] & (1 << BitIndex)) != 0;
	}
	return false;
}

void UTowerDestructibleComponent::ApplyVisualDestruction(
	const TArray<uint8>& DestroyedClusters,
	bool bCollapse,
	FVector Impulse)
{
	if (!GeometryCollectionComp) return;

	// Apply physics break on Chaos Geometry Collection
	// Each destroyed cluster maps to a transform index in the Geometry Collection
	for (uint8 ClusterID : DestroyedClusters)
	{
		// Apply break strain to the specific cluster
		// This triggers Chaos Destruction's fracture simulation
		FVector FragImpulse = Impulse;
		if (FragImpulse.IsNearlyZero())
		{
			// Default outward impulse if no direction specified
			FragImpulse = FVector(
				FMath::RandRange(-100.0f, 100.0f),
				FMath::RandRange(-100.0f, 100.0f),
				FMath::RandRange(50.0f, 200.0f)
			);
		}

		// Apply force to geometry collection at cluster location
		FVector ClusterWorldPos = GetOwner()->GetActorLocation();
		if (ClusterID < Fragments.Num())
		{
			ClusterWorldPos += Fragments[ClusterID].PositionOffset;
		}

		// Apply internal strain to trigger Chaos break
		GeometryCollectionComp->ApplyExternalStrain(
			ClusterID,
			FVector(ClusterWorldPos),
			FVector(FragImpulse.GetSafeNormal()),
			FragImpulse.Size(),
			1, // num iterations
			1.0f // radius
		);

		// Spawn VFX per fragment
		SpawnDestructionVFX(ClusterID, ClusterWorldPos, Material);
	}

	// Full collapse: apply massive force to all remaining pieces
	if (bCollapse)
	{
		FVector CollapseDir = Impulse.IsNearlyZero() ? FVector(0, 0, -500.0f) : Impulse * 5.0f;

		GeometryCollectionComp->ApplyPhysicsField(
			true, // enabled
			EGeometryCollectionPhysicsTypeEnum::Chaos_Angular_Torque,
			nullptr, // field node (use default)
			nullptr  // command
		);
	}
}

void UTowerDestructibleComponent::SpawnDestructionVFX(
	uint8 ClusterID,
	FVector WorldPos,
	ETowerDestructionMaterial Mat)
{
	UNiagaraSystem* VFXSystem = GetDestructionVFXForMaterial(Mat);
	if (!VFXSystem) return;

	UNiagaraComponent* VFXComp = UNiagaraFunctionLibrary::SpawnSystemAtLocation(
		GetWorld(),
		VFXSystem,
		WorldPos,
		FRotator::ZeroRotator,
		FVector(1.0f),
		true, // bAutoDestroy
		true, // bAutoActivate
		ENCPoolMethod::AutoRelease
	);

	if (VFXComp)
	{
		// Set material-specific parameters
		switch (Mat)
		{
		case ETowerDestructionMaterial::Wood:
			VFXComp->SetColorParameter(TEXT("ParticleColor"), FLinearColor(0.6f, 0.4f, 0.2f));
			break;
		case ETowerDestructionMaterial::Stone:
			VFXComp->SetColorParameter(TEXT("ParticleColor"), FLinearColor(0.5f, 0.5f, 0.5f));
			break;
		case ETowerDestructionMaterial::Metal:
			VFXComp->SetColorParameter(TEXT("ParticleColor"), FLinearColor(0.7f, 0.7f, 0.8f));
			break;
		case ETowerDestructionMaterial::Crystal:
			VFXComp->SetColorParameter(TEXT("ParticleColor"), FLinearColor(0.3f, 0.6f, 1.0f));
			break;
		case ETowerDestructionMaterial::Ice:
			VFXComp->SetColorParameter(TEXT("ParticleColor"), FLinearColor(0.8f, 0.9f, 1.0f));
			break;
		case ETowerDestructionMaterial::Organic:
			VFXComp->SetColorParameter(TEXT("ParticleColor"), FLinearColor(0.3f, 0.5f, 0.2f));
			break;
		}
	}
}

UNiagaraSystem* UTowerDestructibleComponent::GetDestructionVFXForMaterial(ETowerDestructionMaterial Mat) const
{
	// Load VFX asset based on material type
	// These paths reference Niagara systems that should be created in Content
	FString AssetPath;
	switch (Mat)
	{
	case ETowerDestructionMaterial::Wood:
		AssetPath = TEXT("/Game/VFX/Destruction/NS_WoodDestruction.NS_WoodDestruction");
		break;
	case ETowerDestructionMaterial::Stone:
		AssetPath = TEXT("/Game/VFX/Destruction/NS_StoneDestruction.NS_StoneDestruction");
		break;
	case ETowerDestructionMaterial::Metal:
		AssetPath = TEXT("/Game/VFX/Destruction/NS_MetalDestruction.NS_MetalDestruction");
		break;
	case ETowerDestructionMaterial::Crystal:
		AssetPath = TEXT("/Game/VFX/Destruction/NS_CrystalDestruction.NS_CrystalDestruction");
		break;
	case ETowerDestructionMaterial::Ice:
		AssetPath = TEXT("/Game/VFX/Destruction/NS_IceDestruction.NS_IceDestruction");
		break;
	case ETowerDestructionMaterial::Organic:
		AssetPath = TEXT("/Game/VFX/Destruction/NS_OrganicDestruction.NS_OrganicDestruction");
		break;
	}

	if (!AssetPath.IsEmpty())
	{
		return Cast<UNiagaraSystem>(StaticLoadObject(
			UNiagaraSystem::StaticClass(),
			nullptr,
			*AssetPath
		));
	}

	return nullptr;
}

// ============================================================================
// ATowerDestructibleActor
// ============================================================================

ATowerDestructibleActor::ATowerDestructibleActor()
{
	PrimaryActorTick.bCanEverTick = false;

	// Create intact mesh
	IntactMesh = CreateDefaultSubobject<UStaticMeshComponent>(TEXT("IntactMesh"));
	RootComponent = IntactMesh;

	// Use engine cube as placeholder
	static ConstructorHelpers::FObjectFinder<UStaticMesh> CubeMesh(
		TEXT("/Engine/BasicShapes/Cube.Cube"));
	if (CubeMesh.Succeeded())
	{
		IntactMesh->SetStaticMesh(CubeMesh.Object);
	}
	IntactMesh->SetCollisionEnabled(ECollisionEnabled::QueryAndPhysics);
	IntactMesh->SetCollisionResponseToAllChannels(ECR_Block);

	// Create destruction component
	DestructibleComp = CreateDefaultSubobject<UTowerDestructibleComponent>(TEXT("DestructibleComp"));
}

void ATowerDestructibleActor::InitFromSpawnData(
	uint64 EntityID,
	const FString& InTemplateID,
	ETowerDestructionMaterial InMaterial,
	float InTotalHP,
	float InMaxTotalHP,
	int32 FragmentCount)
{
	TArray<uint8> EmptyMask;
	DestructibleComp->InitFromServerState(
		EntityID,
		InTemplateID,
		InMaterial,
		InTotalHP,
		InMaxTotalHP,
		false, // not collapsed
		EmptyMask,
		FragmentCount
	);

	// Set mesh color based on material for placeholder visual
	UMaterialInterface* BaseMat = IntactMesh->GetMaterial(0);
	if (BaseMat)
	{
		UMaterialInstanceDynamic* DynMat = UMaterialInstanceDynamic::Create(BaseMat, this);
		FLinearColor Color;
		switch (InMaterial)
		{
		case ETowerDestructionMaterial::Wood:    Color = FLinearColor(0.6f, 0.4f, 0.2f); break;
		case ETowerDestructionMaterial::Stone:   Color = FLinearColor(0.5f, 0.5f, 0.5f); break;
		case ETowerDestructionMaterial::Metal:   Color = FLinearColor(0.7f, 0.7f, 0.8f); break;
		case ETowerDestructionMaterial::Crystal:	Color = FLinearColor(0.3f, 0.6f, 1.0f); break;
		case ETowerDestructionMaterial::Ice:     Color = FLinearColor(0.8f, 0.9f, 1.0f); break;
		case ETowerDestructionMaterial::Organic: Color = FLinearColor(0.3f, 0.6f, 0.2f); break;
		default:                                Color = FLinearColor(1.0f, 1.0f, 1.0f); break;
		}
		DynMat->SetVectorParameterValue(TEXT("BaseColor"), Color);
		IntactMesh->SetMaterial(0, DynMat);
	}

#if WITH_EDITOR
	SetActorLabel(FString::Printf(TEXT("Destructible_%s_%llu"), *InTemplateID, EntityID));
#endif
}
