#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Character.h"
#include "InputActionValue.h"
#include "TowerPlayerCharacter.generated.h"

class UInputMappingContext;
class UInputAction;
class UCameraComponent;
class USpringArmComponent;
class UTowerGameSubsystem;

/**
 * Tower Player Character — third-person action combat character.
 * Communicates with Rust core through UTowerGameSubsystem for
 * damage calculations, semantic interactions, and combat timing.
 */
UCLASS()
class TOWERGAME_API ATowerPlayerCharacter : public ACharacter
{
    GENERATED_BODY()

public:
    ATowerPlayerCharacter();

    virtual void SetupPlayerInputComponent(UInputComponent* PlayerInputComponent) override;
    virtual void BeginPlay() override;
    virtual void Tick(float DeltaTime) override;

    // ============ Components ============

    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Camera")
    USpringArmComponent* CameraBoom;

    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Camera")
    UCameraComponent* FollowCamera;

    // ============ Input ============

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Input")
    UInputMappingContext* DefaultMappingContext;

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Input")
    UInputAction* MoveAction;

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Input")
    UInputAction* LookAction;

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Input")
    UInputAction* JumpAction;

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Input")
    UInputAction* AttackAction;

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Input")
    UInputAction* DodgeAction;

    UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Input")
    UInputAction* InteractAction;

    // ============ Combat State ============

    UPROPERTY(BlueprintReadOnly, Category = "Combat")
    int32 ComboStep = 0;

    UPROPERTY(BlueprintReadOnly, Category = "Combat")
    float ComboTimer = 0.0f;

    UPROPERTY(BlueprintReadOnly, Category = "Combat")
    bool bIsAttacking = false;

    UPROPERTY(BlueprintReadOnly, Category = "Combat")
    bool bIsDodging = false;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Combat")
    float BaseDamage = 30.0f;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Combat")
    float ComboWindow = 0.8f;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Combat")
    int32 MaxCombo = 3;

    // ============ Stats ============

    UPROPERTY(BlueprintReadOnly, Category = "Stats")
    float CurrentHp = 100.0f;

    UPROPERTY(BlueprintReadOnly, Category = "Stats")
    float MaxHp = 100.0f;

    UPROPERTY(BlueprintReadOnly, Category = "Stats")
    float KineticEnergy = 100.0f;

    UPROPERTY(BlueprintReadOnly, Category = "Stats")
    float ThermalEnergy = 100.0f;

    UPROPERTY(BlueprintReadOnly, Category = "Stats")
    float SemanticEnergy = 100.0f;

    // ============ Gameplay ============

    UFUNCTION(BlueprintCallable, Category = "Combat")
    void PerformAttack();

    UFUNCTION(BlueprintCallable, Category = "Combat")
    void PerformDodge();

    UFUNCTION(BlueprintCallable, Category = "Interaction")
    void Interact();

    UFUNCTION(BlueprintCallable, Category = "Stats")
    void TakeCombatDamage(float Amount);

protected:
    void Move(const FInputActionValue& Value);
    void Look(const FInputActionValue& Value);

private:
    UTowerGameSubsystem* GetTowerSubsystem() const;
    void ResetCombo();
    void RegenerateResources(float DeltaTime);
};
