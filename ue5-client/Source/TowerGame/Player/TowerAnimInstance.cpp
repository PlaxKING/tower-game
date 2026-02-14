#include "TowerAnimInstance.h"
#include "TowerPlayerCharacter.h"
#include "GameFramework/CharacterMovementComponent.h"

void UTowerAnimInstance::NativeInitializeAnimation()
{
    Super::NativeInitializeAnimation();

    OwnerCharacter = Cast<ATowerPlayerCharacter>(TryGetPawnOwner());
}

void UTowerAnimInstance::NativeUpdateAnimation(float DeltaSeconds)
{
    Super::NativeUpdateAnimation(DeltaSeconds);

    if (!OwnerCharacter) return;

    UCharacterMovementComponent* Movement = OwnerCharacter->GetCharacterMovement();
    if (!Movement) return;

    // Movement
    FVector Velocity = OwnerCharacter->GetVelocity();
    Speed = Velocity.Size2D();
    VerticalVelocity = Velocity.Z;
    bIsInAir = Movement->IsFalling();
    bIsFalling = bIsInAir && VerticalVelocity < -100.0f;

    // Direction relative to actor facing (for strafe blending)
    if (Speed > 10.0f)
    {
        FRotator ActorRot = OwnerCharacter->GetActorRotation();
        FRotator VelocityRot = Velocity.Rotation();
        FRotator Delta = (VelocityRot - ActorRot).GetNormalized();
        Direction = Delta.Yaw;
    }
    else
    {
        Direction = 0.0f;
    }

    // Combat state from character
    bIsAttacking = OwnerCharacter->bIsAttacking;
    ComboStep = OwnerCharacter->ComboStep;
    bIsDodging = OwnerCharacter->bIsDodging;

    // Health
    HealthPercent = (OwnerCharacter->MaxHp > 0.0f)
        ? OwnerCharacter->CurrentHp / OwnerCharacter->MaxHp
        : 0.0f;
    bIsDead = OwnerCharacter->CurrentHp <= 0.0f;

    // Hurt flash timer
    if (HurtTimer > 0.0f)
    {
        HurtTimer -= DeltaSeconds;
        bIsHurt = true;
    }
    else
    {
        bIsHurt = false;
    }
}
