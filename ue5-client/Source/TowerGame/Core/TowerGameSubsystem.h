#pragma once

#include "CoreMinimal.h"
#include "Subsystems/GameInstanceSubsystem.h"
#include "TowerGameSubsystem.generated.h"

class FProceduralCoreBridge;

/**
 * Game Instance Subsystem — owns the Rust DLL bridge.
 * Lives for the entire game session. All gameplay code accesses Rust
 * through this subsystem via GetGameInstance()->GetSubsystem<UTowerGameSubsystem>().
 */
UCLASS()
class TOWERGAME_API UTowerGameSubsystem : public UGameInstanceSubsystem
{
    GENERATED_BODY()

public:
    virtual void Initialize(FSubsystemCollectionBase& Collection) override;
    virtual void Deinitialize() override;

    /** Get the Rust bridge (null if DLL failed to load) */
    FProceduralCoreBridge* GetBridge() const { return Bridge.Get(); }

    /** Is the Rust core loaded and ready? */
    bool IsRustCoreReady() const;

    // ============ High-Level API ============

    /** Generate floor layout JSON for a given seed and floor number */
    UFUNCTION(BlueprintCallable, Category = "Tower|Generation")
    FString RequestFloorLayout(int64 Seed, int32 FloorId);

    /** Generate monsters for current floor */
    UFUNCTION(BlueprintCallable, Category = "Tower|Monster")
    FString RequestFloorMonsters(int64 Seed, int32 FloorId, int32 Count);

    /** Calculate combat damage */
    UFUNCTION(BlueprintCallable, Category = "Tower|Combat")
    float CalculateDamage(float BaseDamage, int32 AngleId, int32 ComboStep);

    /** Get semantic similarity between two tag sets */
    UFUNCTION(BlueprintCallable, Category = "Tower|Semantic")
    float GetSemanticSimilarity(const FString& TagsA, const FString& TagsB);

    /** Get current Breath of Tower state */
    UFUNCTION(BlueprintCallable, Category = "Tower|World")
    FString GetBreathState(float ElapsedSeconds);

    /** Get Rust core version */
    UFUNCTION(BlueprintCallable, Category = "Tower|Core")
    FString GetCoreVersion();

    // ============ State ============

    /** Current tower seed */
    UPROPERTY(BlueprintReadOnly, Category = "Tower|State")
    int64 TowerSeed = 42;

    /** Current floor number */
    UPROPERTY(BlueprintReadOnly, Category = "Tower|State")
    int32 CurrentFloor = 1;

    /** Highest floor reached */
    UPROPERTY(BlueprintReadOnly, Category = "Tower|State")
    int32 HighestFloor = 1;

    /** Game elapsed time in seconds */
    UPROPERTY(BlueprintReadOnly, Category = "Tower|State")
    float GameElapsedTime = 0.0f;

private:
    TUniquePtr<FProceduralCoreBridge> Bridge;

    FString FindDllPath() const;
};
