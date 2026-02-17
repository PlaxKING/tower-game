#pragma once

#include "CoreMinimal.h"
#include "Components/ActorComponent.h"
#include "DestructibleComponent.generated.h"

/**
 * Material types for destruction (maps to Rust DestructionMaterial enum).
 * Affects damage resistance, fracture pattern, and VFX.
 */
UENUM(BlueprintType)
enum class ETowerDestructionMaterial : uint8
{
	Wood		UMETA(DisplayName = "Wood"),
	Stone		UMETA(DisplayName = "Stone"),
	Metal		UMETA(DisplayName = "Metal"),
	Crystal		UMETA(DisplayName = "Crystal"),
	Ice			UMETA(DisplayName = "Ice"),
	Organic		UMETA(DisplayName = "Organic")
};

/**
 * Damage type for destruction interactions (maps to Rust DestructionDamageType).
 */
UENUM(BlueprintType)
enum class ETowerDestructionDamageType : uint8
{
	Kinetic				UMETA(DisplayName = "Kinetic"),
	Explosive			UMETA(DisplayName = "Explosive"),
	ElementalFire		UMETA(DisplayName = "Fire"),
	ElementalIce		UMETA(DisplayName = "Ice"),
	ElementalLightning	UMETA(DisplayName = "Lightning"),
	Semantic			UMETA(DisplayName = "Semantic")
};

/**
 * Category of destructible object (maps to Rust DestructibleCategory).
 */
UENUM(BlueprintType)
enum class ETowerDestructibleCategory : uint8
{
	Wall		UMETA(DisplayName = "Wall"),
	Pillar		UMETA(DisplayName = "Pillar"),
	Tree		UMETA(DisplayName = "Tree"),
	Container	UMETA(DisplayName = "Container"),
	Crystal		UMETA(DisplayName = "Crystal"),
	Bridge		UMETA(DisplayName = "Bridge"),
	Corruption	UMETA(DisplayName = "Corruption")
};

/**
 * State of a single fragment within the destructible.
 * Synced from Bevy server via DestructionDelta.
 */
USTRUCT(BlueprintType)
struct FDestructionFragment
{
	GENERATED_BODY()

	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Destruction")
	uint8 ClusterID = 0;

	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Destruction")
	float HP = 0.0f;

	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Destruction")
	float MaxHP = 0.0f;

	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Destruction")
	bool bDestroyed = false;

	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Destruction")
	FVector PositionOffset = FVector::ZeroVector;
};

/** Delegate fired when destruction state changes */
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(
	FOnDestructionStateChanged,
	const TArray<uint8>&, DestroyedClusters,
	bool, bFullCollapse
);

/** Delegate fired when a fragment is destroyed (for VFX/SFX) */
DECLARE_DYNAMIC_MULTICAST_DELEGATE_ThreeParams(
	FOnFragmentDestroyed,
	uint8, ClusterID,
	FVector, WorldPosition,
	ETowerDestructionMaterial, Material
);

/**
 * UTowerDestructibleComponent — Client-side destruction state manager.
 *
 * Receives authoritative destruction deltas from the Bevy server and
 * drives UE5 Chaos Destruction / visual fracture accordingly.
 *
 * Architecture:
 *   Bevy server (authoritative) → DestructionDelta (JSON) → this component
 *     → UGeometryCollectionComponent (Chaos Destruction) visual fracture
 *     → Niagara VFX + SFX per material type
 *
 * Attach to any actor with a UGeometryCollectionComponent.
 * In production, the GeometryCollection is pre-fractured per template.
 */
UCLASS(ClassGroup=(Tower), meta=(BlueprintSpawnableComponent))
class TOWERGAME_API UTowerDestructibleComponent : public UActorComponent
{
	GENERATED_BODY()

public:
	UTowerDestructibleComponent();

	// ========== Server-synced State ==========

	/** Server entity ID (matches Rust entity_id) */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Destruction")
	uint64 ServerEntityID = 0;

	/** Template ID from destruction system */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Destruction")
	FString TemplateID;

	/** Material type (affects VFX and damage resistance display) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Destruction")
	ETowerDestructionMaterial Material = ETowerDestructionMaterial::Stone;

	/** Category (affects fracture pattern) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Destruction")
	ETowerDestructibleCategory Category = ETowerDestructibleCategory::Wall;

	/** Current total HP (synced from server) */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Destruction")
	float TotalHP = 0.0f;

	/** Maximum total HP */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Destruction")
	float MaxTotalHP = 0.0f;

	/** Is the object fully collapsed? */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Destruction")
	bool bCollapsed = false;

	/** Can this object be rebuilt by players? */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Destruction")
	bool bSupportsRebuild = false;

	/** Current rebuild progress (0.0-1.0) */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Destruction")
	float RebuildProgress = 0.0f;

	/** Fragment states */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Destruction")
	TArray<FDestructionFragment> Fragments;

	/** Bitmask of destroyed fragments (compact representation) */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Destruction")
	TArray<uint8> FragmentMask;

	// ========== LOD Settings ==========

	/** Distance at which full Chaos physics is used (close range) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Destruction|LOD")
	float FullPhysicsDistance = 2000.0f;

	/** Distance at which simplified animation is used (medium range) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Destruction|LOD")
	float SimplifiedAnimDistance = 5000.0f;

	/** Maximum active physics fragments on screen */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Destruction|LOD")
	int32 MaxActivePhysicsFragments = 200;

	// ========== Events ==========

	/** Fired when destruction state changes (from server delta) */
	UPROPERTY(BlueprintAssignable, Category = "Tower|Destruction")
	FOnDestructionStateChanged OnDestructionStateChanged;

	/** Fired when a specific fragment is destroyed (for VFX spawning) */
	UPROPERTY(BlueprintAssignable, Category = "Tower|Destruction")
	FOnFragmentDestroyed OnFragmentDestroyed;

	// ========== Public API ==========

	/**
	 * Apply a destruction delta received from the server.
	 * Updates fragment states and triggers Chaos Destruction visuals.
	 *
	 * @param DestroyedClusters  Cluster IDs newly destroyed this update
	 * @param NewFragmentMask    Updated bitmask of all destroyed fragments
	 * @param bStructuralCollapse True if the entire object collapsed
	 * @param CollapseImpulse    Direction of collapse (for physics impulse)
	 */
	UFUNCTION(BlueprintCallable, Category = "Tower|Destruction")
	void ApplyDestructionDelta(
		const TArray<uint8>& DestroyedClusters,
		const TArray<uint8>& NewFragmentMask,
		bool bStructuralCollapse,
		FVector CollapseImpulse
	);

	/**
	 * Initialize from server state (called when floor is first loaded).
	 * Sets up all fragment states from the server snapshot.
	 */
	UFUNCTION(BlueprintCallable, Category = "Tower|Destruction")
	void InitFromServerState(
		uint64 InEntityID,
		const FString& InTemplateID,
		ETowerDestructionMaterial InMaterial,
		float InTotalHP,
		float InMaxTotalHP,
		bool bInCollapsed,
		const TArray<uint8>& InFragmentMask,
		int32 FragmentCount
	);

	/** Send a damage request to the server for this destructible */
	UFUNCTION(BlueprintCallable, Category = "Tower|Destruction")
	void RequestDamage(
		FVector ImpactPoint,
		float Damage,
		float Radius,
		ETowerDestructionDamageType DamageType,
		const FString& WeaponID,
		const FString& AbilityID
	);

	/** Send a rebuild request to the server */
	UFUNCTION(BlueprintCallable, Category = "Tower|Destruction")
	void RequestRebuild(const TArray<FString>& MaterialItems);

	/** Get destruction percentage (0.0 = intact, 1.0 = fully destroyed) */
	UFUNCTION(BlueprintPure, Category = "Tower|Destruction")
	float GetDestructionPercentage() const;

	/** Is this fragment destroyed? */
	UFUNCTION(BlueprintPure, Category = "Tower|Destruction")
	bool IsFragmentDestroyed(uint8 ClusterID) const;

protected:
	virtual void BeginPlay() override;
	virtual void TickComponent(float DeltaTime, ELevelTick TickType, FActorComponentTickFunction* ThisTickFunction) override;

private:
	/** Apply visual destruction to the Geometry Collection */
	void ApplyVisualDestruction(const TArray<uint8>& DestroyedClusters, bool bCollapse, FVector Impulse);

	/** Spawn destruction VFX based on material type */
	void SpawnDestructionVFX(uint8 ClusterID, FVector WorldPos, ETowerDestructionMaterial Mat);

	/** Get the appropriate Niagara system for this material's destruction */
	class UNiagaraSystem* GetDestructionVFXForMaterial(ETowerDestructionMaterial Mat) const;

	/** Cached reference to GeometryCollection on parent actor */
	UPROPERTY()
	class UGeometryCollectionComponent* GeometryCollectionComp;

	/** Timer for debris cleanup (fading out old fragments) */
	float DebrisCleanupTimer = 0.0f;
	static constexpr float DebrisLifetime = 10.0f;
};

// ============================================================================
// ATowerDestructibleActor — Pre-configured destructible actor
// ============================================================================

/**
 * Standalone destructible actor that can be placed in levels or spawned
 * by the procedural generation system.
 *
 * Contains a static mesh for the intact state and a Geometry Collection
 * for the fractured state. The component handles the visual transition.
 */
UCLASS()
class TOWERGAME_API ATowerDestructibleActor : public AActor
{
	GENERATED_BODY()

public:
	ATowerDestructibleActor();

	/** The static mesh shown when intact */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Destruction")
	UStaticMeshComponent* IntactMesh;

	/** The destruction component managing server sync and visuals */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Tower|Destruction")
	UTowerDestructibleComponent* DestructibleComp;

	/** Initialize from a server-provided destructible spawn */
	UFUNCTION(BlueprintCallable, Category = "Tower|Destruction")
	void InitFromSpawnData(
		uint64 EntityID,
		const FString& TemplateID,
		ETowerDestructionMaterial Material,
		float TotalHP,
		float MaxTotalHP,
		int32 FragmentCount
	);
};
