#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "MonsterSpawner.generated.h"

/**
 * Static utility for spawning monsters from Rust JSON data.
 * Monsters are placed at spawn points derived from room locations.
 */
UCLASS()
class TOWERGAME_API AMonsterSpawner : public AActor
{
    GENERATED_BODY()

public:
    AMonsterSpawner();

    /**
     * Parse monster JSON array from Rust and spawn ATowerMonster actors.
     * Distributes monsters across provided spawn points.
     */
    static TArray<AActor*> SpawnMonstersFromJson(
        UWorld* World,
        const FString& MonstersJson,
        const TArray<FVector>& SpawnPoints,
        int32 FloorLevel);
};

/**
 * In-world monster actor with stats from Rust procedural core.
 * Visual representation is a scaled cube with element-based coloring.
 * In production, replace with skeletal mesh from DataTable lookup.
 */
UCLASS()
class TOWERGAME_API ATowerMonster : public AActor
{
    GENERATED_BODY()

public:
    ATowerMonster();

    /** Initialize from parsed JSON data */
    void InitFromData(
        const FString& InName,
        const FString& InSize,
        const FString& InElement,
        float InHp,
        float InAttack,
        float InDefense,
        float InSpeed,
        int32 InFloorLevel);

    // ============ Stats ============

    UPROPERTY(BlueprintReadOnly, Category = "Monster")
    FString MonsterName;

    UPROPERTY(BlueprintReadOnly, Category = "Monster")
    FString Size;

    UPROPERTY(BlueprintReadOnly, Category = "Monster")
    FString Element;

    UPROPERTY(BlueprintReadOnly, Category = "Monster")
    float MaxHp = 100.0f;

    UPROPERTY(BlueprintReadOnly, Category = "Monster")
    float CurrentHp = 100.0f;

    UPROPERTY(BlueprintReadOnly, Category = "Monster")
    float Attack = 10.0f;

    UPROPERTY(BlueprintReadOnly, Category = "Monster")
    float Defense = 5.0f;

    UPROPERTY(BlueprintReadOnly, Category = "Monster")
    float Speed = 3.0f;

    UPROPERTY(BlueprintReadOnly, Category = "Monster")
    int32 FloorLevel = 1;

    UPROPERTY(BlueprintReadOnly, Category = "Monster")
    bool bIsAlive = true;

    // ============ Components ============

    UPROPERTY(VisibleAnywhere, Category = "Monster")
    UStaticMeshComponent* MeshComponent;

    // ============ Gameplay ============

    UFUNCTION(BlueprintCallable, Category = "Monster")
    void TakeDamageFromPlayer(float DamageAmount);

    DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnMonsterDeath, ATowerMonster*, Monster);

    UPROPERTY(BlueprintAssignable, Category = "Monster")
    FOnMonsterDeath OnMonsterDeath;

private:
    /** Get color based on monster element */
    static FLinearColor GetElementColor(const FString& InElement);

    /** Get scale based on monster size */
    static float GetSizeScale(const FString& InSize);
};
