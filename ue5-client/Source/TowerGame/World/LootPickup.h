#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "LootPickup.generated.h"

class UStaticMeshComponent;
class USphereComponent;
class UPointLightComponent;

DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnLootCollected, AActor*, Collector, const FString&, LootJson);

/**
 * Loot pickup dropped by defeated monsters.
 *
 * Features:
 * - Color-coded by rarity (Common=white, Uncommon=green, Rare=blue, Epic=purple, Legendary=orange, Mythic=red)
 * - Bobbing animation + rotation
 * - Auto-magnet: pulls toward nearby player
 * - Despawn after timeout
 * - Overlap-based collection
 */
UENUM(BlueprintType)
enum class ELootRarity : uint8
{
    Common       UMETA(DisplayName = "Common"),
    Uncommon     UMETA(DisplayName = "Uncommon"),
    Rare         UMETA(DisplayName = "Rare"),
    Epic         UMETA(DisplayName = "Epic"),
    Legendary    UMETA(DisplayName = "Legendary"),
    Mythic       UMETA(DisplayName = "Mythic"),
};

UCLASS()
class TOWERGAME_API ALootPickup : public AActor
{
    GENERATED_BODY()

public:
    ALootPickup();

    virtual void BeginPlay() override;
    virtual void Tick(float DeltaTime) override;

    /** Initialize from Rust loot JSON */
    UFUNCTION(BlueprintCallable, Category = "Loot")
    void InitFromJson(const FString& LootJson);

    // ============ Loot Data ============

    UPROPERTY(BlueprintReadOnly, Category = "Loot")
    FString ItemName;

    UPROPERTY(BlueprintReadOnly, Category = "Loot")
    ELootRarity Rarity = ELootRarity::Common;

    UPROPERTY(BlueprintReadOnly, Category = "Loot")
    FString LootDataJson;

    // ============ Behavior ============

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Loot")
    float DespawnTime = 60.0f;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Loot")
    float MagnetRadius = 200.0f;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Loot")
    float MagnetSpeed = 500.0f;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Loot")
    float BobHeight = 15.0f;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Loot")
    float RotateSpeed = 90.0f;

    // ============ Events ============

    UPROPERTY(BlueprintAssignable, Category = "Loot")
    FOnLootCollected OnLootCollected;

    // ============ Components ============

    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Loot")
    UStaticMeshComponent* LootMesh;

    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Loot")
    USphereComponent* CollectionSphere;

    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Loot")
    UPointLightComponent* RarityGlow;

private:
    float TimeAlive = 0.0f;
    FVector SpawnPosition;
    bool bCollected = false;

    UFUNCTION()
    void OnOverlapBegin(UPrimitiveComponent* OverlappedComp, AActor* OtherActor,
        UPrimitiveComponent* OtherComp, int32 OtherBodyIndex,
        bool bFromSweep, const FHitResult& SweepResult);

    FLinearColor GetRarityColor() const;
    float GetRarityScale() const;
    float GetRarityGlowIntensity() const;
};
