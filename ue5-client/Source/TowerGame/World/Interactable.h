#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "Interactable.generated.h"

class USphereComponent;
class UStaticMeshComponent;
class UWidgetComponent;

DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnInteracted, AActor*, Interactor, AInteractable*, Interactable);

/**
 * Base class for all interactable objects in the Tower.
 *
 * Subclasses: ATowerChest, ATowerShrine, ATowerStairs
 *
 * Features:
 * - Proximity detection (shows "Press E" prompt)
 * - Interaction cooldown
 * - Visual highlight when in range
 * - Blueprint-assignable interaction event
 */
UCLASS(Abstract)
class TOWERGAME_API AInteractable : public AActor
{
    GENERATED_BODY()

public:
    AInteractable();

    virtual void BeginPlay() override;
    virtual void Tick(float DeltaTime) override;

    /** Attempt interaction. Returns true if successful. */
    UFUNCTION(BlueprintCallable, Category = "Interaction")
    bool TryInteract(AActor* Interactor);

    /** Check if a player is in interaction range */
    UFUNCTION(BlueprintPure, Category = "Interaction")
    bool IsPlayerInRange() const { return bPlayerInRange; }

    // ============ Config ============

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Interaction")
    float InteractionRadius = 200.0f;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Interaction")
    float CooldownSeconds = 1.0f;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Interaction")
    FString InteractionPrompt = TEXT("Press E");

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Interaction")
    bool bSingleUse = false;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Interaction")
    bool bRequiresFloorClear = false;

    // ============ Events ============

    UPROPERTY(BlueprintAssignable, Category = "Interaction")
    FOnInteracted OnInteracted;

    // ============ Components ============

    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Interaction")
    UStaticMeshComponent* BaseMesh;

    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Interaction")
    USphereComponent* InteractionZone;

protected:
    /** Override in subclasses for specific behavior */
    virtual void ExecuteInteraction(AActor* Interactor);

    UPROPERTY(BlueprintReadOnly, Category = "Interaction")
    bool bUsed = false;

private:
    bool bPlayerInRange = false;
    float CooldownTimer = 0.0f;

    UFUNCTION()
    void OnOverlapBegin(UPrimitiveComponent* OverlappedComp, AActor* OtherActor,
        UPrimitiveComponent* OtherComp, int32 OtherBodyIndex,
        bool bFromSweep, const FHitResult& SweepResult);

    UFUNCTION()
    void OnOverlapEnd(UPrimitiveComponent* OverlappedComp, AActor* OtherActor,
        UPrimitiveComponent* OtherComp, int32 OtherBodyIndex);
};

// ============ Chest ============

UCLASS()
class TOWERGAME_API ATowerChest : public AInteractable
{
    GENERATED_BODY()

public:
    ATowerChest();

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Chest")
    int32 FloorLevel = 1;

    UPROPERTY(BlueprintReadOnly, Category = "Chest")
    bool bOpened = false;

protected:
    virtual void ExecuteInteraction(AActor* Interactor) override;
};

// ============ Shrine ============

UCLASS()
class TOWERGAME_API ATowerShrine : public AInteractable
{
    GENERATED_BODY()

public:
    ATowerShrine();

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Shrine")
    FString FactionName = TEXT("seekers");

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Shrine")
    int32 StandingReward = 5;

protected:
    virtual void ExecuteInteraction(AActor* Interactor) override;
};

// ============ Stairs ============

UCLASS()
class TOWERGAME_API ATowerStairs : public AInteractable
{
    GENERATED_BODY()

public:
    ATowerStairs();

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Stairs")
    bool bGoingUp = true;

protected:
    virtual void ExecuteInteraction(AActor* Interactor) override;
};
