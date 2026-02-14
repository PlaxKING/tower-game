#pragma once

#include "CoreMinimal.h"
#include "GameFramework/GameModeBase.h"
#include "TowerGameMode.generated.h"

class UTowerGameSubsystem;

/**
 * Tower Game Mode — manages floor lifecycle, monster spawning, and game state.
 * Uses UTowerGameSubsystem to call Rust procedural core for all generation.
 */
UCLASS()
class TOWERGAME_API ATowerGameMode : public AGameModeBase
{
    GENERATED_BODY()

public:
    ATowerGameMode();

    virtual void InitGame(const FString& MapName, const FString& Options, FString& ErrorMessage) override;
    virtual void StartPlay() override;

    // ============ Floor Management ============

    /** Load a specific floor — generates layout and spawns geometry + monsters */
    UFUNCTION(BlueprintCallable, Category = "Tower|Floor")
    void LoadFloor(int32 FloorId);

    /** Advance to next floor */
    UFUNCTION(BlueprintCallable, Category = "Tower|Floor")
    void GoToNextFloor();

    /** Return to previous floor */
    UFUNCTION(BlueprintCallable, Category = "Tower|Floor")
    void GoToPreviousFloor();

    /** Clear all spawned floor content */
    UFUNCTION(BlueprintCallable, Category = "Tower|Floor")
    void ClearCurrentFloor();

    // ============ State ============

    UPROPERTY(BlueprintReadOnly, Category = "Tower|State")
    int32 CurrentFloorId = 1;

    UPROPERTY(BlueprintReadOnly, Category = "Tower|State")
    int32 MonstersAlive = 0;

    UPROPERTY(BlueprintReadOnly, Category = "Tower|State")
    bool bFloorLoaded = false;

    // ============ Config ============

    /** Number of monsters per floor (base, scales with floor tier) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Config")
    int32 BaseMonstersPerFloor = 5;

    /** Tile size in Unreal units (1 Rust tile = this many UU) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Config")
    float TileSize = 300.0f;

    /** Wall height in Unreal units */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Tower|Config")
    float WallHeight = 400.0f;

    // ============ Delegates ============

    DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnFloorLoaded, int32, FloorId);
    DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnFloorCleared);
    DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnAllMonstersDefeated, int32, FloorId);

    UPROPERTY(BlueprintAssignable, Category = "Tower|Events")
    FOnFloorLoaded OnFloorLoaded;

    UPROPERTY(BlueprintAssignable, Category = "Tower|Events")
    FOnFloorCleared OnFloorCleared;

    UPROPERTY(BlueprintAssignable, Category = "Tower|Events")
    FOnAllMonstersDefeated OnAllMonstersDefeated;

private:
    UTowerGameSubsystem* GetTowerSubsystem() const;

    /** All actors spawned for the current floor (tiles, walls, monsters) */
    UPROPERTY()
    TArray<AActor*> SpawnedFloorActors;
};
