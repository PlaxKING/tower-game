#pragma once

#include "CoreMinimal.h"
#include "Components/ActorComponent.h"
#include "FloorTransitionComponent.generated.h"

class ATowerGameMode;

/**
 * Floor transition states.
 */
UENUM(BlueprintType)
enum class ETransitionState : uint8
{
    Idle            UMETA(DisplayName = "Idle"),
    FadingOut       UMETA(DisplayName = "Fading Out"),
    Loading         UMETA(DisplayName = "Loading"),
    FadingIn        UMETA(DisplayName = "Fading In"),
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnFloorTransitionStart, int32, NewFloor);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnFloorTransitionEnd, int32, NewFloor);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnFloorLoadProgress, float, Progress);

/**
 * Manages floor-to-floor transitions with visual effects.
 *
 * Sequence:
 * 1. Fade to black (0.5s)
 * 2. Destroy old floor tiles/monsters
 * 3. Generate new floor via Rust core (tower_core.dll)
 * 4. Spawn new tiles and monsters
 * 5. Position player at entrance
 * 6. Fade in from black (0.5s)
 *
 * Attach to GameMode or PlayerController.
 */
UCLASS(ClassGroup = (World), meta = (BlueprintSpawnableComponent))
class TOWERGAME_API UFloorTransitionComponent : public UActorComponent
{
    GENERATED_BODY()

public:
    UFloorTransitionComponent();

    virtual void TickComponent(float DeltaTime, ELevelTick TickType,
        FActorComponentTickFunction* ThisTickFunction) override;

    // ============ API ============

    /** Start transition to a new floor */
    UFUNCTION(BlueprintCallable, Category = "FloorTransition")
    void TransitionToFloor(int32 NewFloor);

    /** Get current transition state */
    UFUNCTION(BlueprintPure, Category = "FloorTransition")
    ETransitionState GetState() const { return State; }

    /** Is transition in progress? */
    UFUNCTION(BlueprintPure, Category = "FloorTransition")
    bool IsTransitioning() const { return State != ETransitionState::Idle; }

    /** Get loading progress (0.0-1.0) */
    UFUNCTION(BlueprintPure, Category = "FloorTransition")
    float GetProgress() const { return LoadProgress; }

    // ============ Config ============

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "FloorTransition")
    float FadeOutDuration = 0.5f;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "FloorTransition")
    float FadeInDuration = 0.5f;

    /** Minimum loading screen time (prevents flash) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "FloorTransition")
    float MinLoadTime = 0.5f;

    // ============ Events ============

    UPROPERTY(BlueprintAssignable, Category = "FloorTransition")
    FOnFloorTransitionStart OnTransitionStart;

    UPROPERTY(BlueprintAssignable, Category = "FloorTransition")
    FOnFloorTransitionEnd OnTransitionEnd;

    UPROPERTY(BlueprintAssignable, Category = "FloorTransition")
    FOnFloorLoadProgress OnLoadProgress;

private:
    ETransitionState State = ETransitionState::Idle;
    float StateTimer = 0.0f;
    float LoadProgress = 0.0f;
    int32 TargetFloor = 0;
    bool bFloorGenerated = false;

    void BeginFadeOut();
    void UpdateFadeOut(float DeltaTime);
    void BeginLoading();
    void UpdateLoading(float DeltaTime);
    void BeginFadeIn();
    void UpdateFadeIn(float DeltaTime);
    void FinishTransition();

    /** Set screen fade via camera manager */
    void SetScreenFade(float Alpha);
};
