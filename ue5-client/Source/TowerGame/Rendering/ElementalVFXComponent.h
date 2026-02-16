#pragma once

#include "CoreMinimal.h"
#include "Components/ActorComponent.h"
#include "NiagaraComponent.h"
#include "ElementalVFXComponent.generated.h"

class UNiagaraSystem;
class UNiagaraComponent;

/**
 * Element types matching Rust semantic tags.
 * Each element has distinct particle behavior and color palette.
 */
UENUM(BlueprintType)
enum class EElementalType : uint8
{
    None        UMETA(DisplayName = "None"),
    Fire        UMETA(DisplayName = "Fire"),
    Water       UMETA(DisplayName = "Water"),
    Earth       UMETA(DisplayName = "Earth"),
    Wind        UMETA(DisplayName = "Wind"),
    Void        UMETA(DisplayName = "Void"),
    Corruption  UMETA(DisplayName = "Corruption"),
};

/**
 * VFX trigger types — when to spawn particles.
 */
UENUM(BlueprintType)
enum class EVFXTrigger : uint8
{
    OnHit           UMETA(DisplayName = "On Hit"),
    OnDeath         UMETA(DisplayName = "On Death"),
    Ambient         UMETA(DisplayName = "Ambient Aura"),
    OnDodge         UMETA(DisplayName = "On Dodge"),
    OnComboFinish   UMETA(DisplayName = "On Combo Finish"),
    OnBreathShift   UMETA(DisplayName = "On Breath Shift"),
};

/**
 * Niagara-based elemental VFX manager.
 *
 * Attach to any actor (player, monster, weapon) to add elemental
 * particle effects. Dynamically sets colors and parameters based
 * on the element type.
 *
 * Supports:
 * - Ambient aura (looping particles around actor)
 * - Hit impact burst (one-shot on damage dealt)
 * - Death explosion (one-shot on monster defeat)
 * - Dodge trail (brief trail on evade)
 * - Combo finisher burst (scaled by combo step)
 * - Breath shift pulse (when Tower breath phase changes)
 *
 * Uses Niagara User Parameters for runtime customization:
 *   - "ElementColor" (FLinearColor)
 *   - "Intensity" (float)
 *   - "ParticleScale" (float)
 *   - "SpawnRate" (float)
 */
UCLASS(ClassGroup = (VFX), meta = (BlueprintSpawnableComponent))
class TOWERGAME_API UElementalVFXComponent : public UActorComponent
{
    GENERATED_BODY()

public:
    UElementalVFXComponent();

    virtual void BeginPlay() override;
    virtual void EndPlay(const EEndPlayReason::Type EndPlayReason) override;

    // ============ Element Config ============

    /** Primary element for this actor */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VFX|Element")
    EElementalType Element = EElementalType::None;

    /** Secondary element (for hybrid monsters) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VFX|Element")
    EElementalType SecondaryElement = EElementalType::None;

    /** Overall VFX intensity multiplier */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VFX|Element", meta = (ClampMin = "0.0", ClampMax = "5.0"))
    float Intensity = 1.0f;

    /** Scale multiplier for all particle sizes */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VFX|Element", meta = (ClampMin = "0.1", ClampMax = "3.0"))
    float ParticleScale = 1.0f;

    // ============ Niagara Systems ============

    /** Ambient aura system (looping) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VFX|Systems")
    UNiagaraSystem* AuraSystem;

    /** Hit impact system (burst) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VFX|Systems")
    UNiagaraSystem* HitSystem;

    /** Death explosion system (burst) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VFX|Systems")
    UNiagaraSystem* DeathSystem;

    /** Dodge trail system (brief) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VFX|Systems")
    UNiagaraSystem* DodgeSystem;

    /** Combo finisher system (burst, scaled) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VFX|Systems")
    UNiagaraSystem* ComboFinishSystem;

    // ============ Controls ============

    /** Set element at runtime (updates all active VFX) */
    UFUNCTION(BlueprintCallable, Category = "VFX|Element")
    void SetElement(EElementalType NewElement);

    /** Trigger a one-shot VFX */
    UFUNCTION(BlueprintCallable, Category = "VFX|Element")
    void TriggerVFX(EVFXTrigger Trigger, FVector Location, float Scale = 1.0f);

    /** Start ambient aura (looping) */
    UFUNCTION(BlueprintCallable, Category = "VFX|Element")
    void StartAura();

    /** Stop ambient aura */
    UFUNCTION(BlueprintCallable, Category = "VFX|Element")
    void StopAura();

    /** Check if aura is active */
    UFUNCTION(BlueprintPure, Category = "VFX|Element")
    bool IsAuraActive() const { return AuraComponent != nullptr && AuraComponent->IsActive(); }

    /** Get color for an element type */
    UFUNCTION(BlueprintPure, Category = "VFX|Element")
    static FLinearColor GetElementColor(EElementalType InElement);

    /** Get secondary color for element (used for gradient/accents) */
    UFUNCTION(BlueprintPure, Category = "VFX|Element")
    static FLinearColor GetElementSecondaryColor(EElementalType InElement);

private:
    UPROPERTY()
    UNiagaraComponent* AuraComponent;

    /** Spawn a Niagara system with element parameters applied */
    UNiagaraComponent* SpawnElementalVFX(UNiagaraSystem* System, FVector Location, float Scale, bool bAutoDestroy);

    /** Apply element color and parameters to a Niagara component */
    void ApplyElementParameters(UNiagaraComponent* NiagaraComp, float Scale);
};
