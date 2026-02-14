#pragma once

#include "CoreMinimal.h"
#include "Animation/AnimInstance.h"
#include "TowerAnimInstance.generated.h"

/**
 * Animation instance for Tower Player Character.
 *
 * Drives the Animation Blueprint state machine with gameplay values.
 * States: Idle, Walk, Run, Jump, Fall, Attack (1-5), Dodge, Block, Parry, Death.
 *
 * Connects to ATowerPlayerCharacter for combat state and resources.
 */
UCLASS()
class TOWERGAME_API UTowerAnimInstance : public UAnimInstance
{
    GENERATED_BODY()

public:
    virtual void NativeInitializeAnimation() override;
    virtual void NativeUpdateAnimation(float DeltaSeconds) override;

    // ============ Movement ============

    UPROPERTY(BlueprintReadOnly, Category = "Animation|Movement")
    float Speed = 0.0f;

    UPROPERTY(BlueprintReadOnly, Category = "Animation|Movement")
    float Direction = 0.0f;

    UPROPERTY(BlueprintReadOnly, Category = "Animation|Movement")
    bool bIsInAir = false;

    UPROPERTY(BlueprintReadOnly, Category = "Animation|Movement")
    bool bIsFalling = false;

    UPROPERTY(BlueprintReadOnly, Category = "Animation|Movement")
    float VerticalVelocity = 0.0f;

    // ============ Combat ============

    UPROPERTY(BlueprintReadOnly, Category = "Animation|Combat")
    bool bIsAttacking = false;

    UPROPERTY(BlueprintReadOnly, Category = "Animation|Combat")
    int32 ComboStep = 0;

    UPROPERTY(BlueprintReadOnly, Category = "Animation|Combat")
    bool bIsDodging = false;

    UPROPERTY(BlueprintReadOnly, Category = "Animation|Combat")
    bool bIsBlocking = false;

    UPROPERTY(BlueprintReadOnly, Category = "Animation|Combat")
    bool bIsParrying = false;

    // ============ State ============

    UPROPERTY(BlueprintReadOnly, Category = "Animation|State")
    bool bIsDead = false;

    UPROPERTY(BlueprintReadOnly, Category = "Animation|State")
    float HealthPercent = 1.0f;

    UPROPERTY(BlueprintReadOnly, Category = "Animation|State")
    bool bIsHurt = false;

    UPROPERTY(BlueprintReadOnly, Category = "Animation|State")
    float HurtTimer = 0.0f;

    // ============ Weapon ============

    /** Current weapon type index for anim set selection */
    UPROPERTY(BlueprintReadOnly, Category = "Animation|Weapon")
    int32 WeaponType = 0;

    /** Attack speed multiplier (for playrate) */
    UPROPERTY(BlueprintReadOnly, Category = "Animation|Weapon")
    float AttackSpeedMultiplier = 1.0f;

private:
    UPROPERTY()
    class ATowerPlayerCharacter* OwnerCharacter;
};
