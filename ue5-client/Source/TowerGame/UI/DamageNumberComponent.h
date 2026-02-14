#pragma once

#include "CoreMinimal.h"
#include "Components/ActorComponent.h"
#include "DamageNumberComponent.generated.h"

class UWidgetComponent;

/**
 * Spawns floating damage numbers above actors.
 * Supports different colors for damage types, crits, healing, etc.
 *
 * Usage: Attach to any actor that can take damage.
 * Call ShowDamage() to spawn a floating number that rises and fades.
 */
UCLASS(ClassGroup = (UI), meta = (BlueprintSpawnableComponent))
class TOWERGAME_API UDamageNumberComponent : public UActorComponent
{
    GENERATED_BODY()

public:
    UDamageNumberComponent();

    virtual void TickComponent(float DeltaTime, ELevelTick TickType, FActorComponentTickFunction* ThisTickFunction) override;

    /** Show a damage number above the owner */
    UFUNCTION(BlueprintCallable, Category = "DamageNumbers")
    void ShowDamage(float Amount, bool bIsCrit = false, bool bIsHealing = false);

    /** Show a status text (e.g. "PARRY!", "DODGE!", "IMMUNE") */
    UFUNCTION(BlueprintCallable, Category = "DamageNumbers")
    void ShowStatusText(const FString& Text, FLinearColor Color);

    // ============ Config ============

    /** How long numbers float before disappearing */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "DamageNumbers")
    float FloatDuration = 1.2f;

    /** How high numbers float (units) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "DamageNumbers")
    float FloatHeight = 120.0f;

    /** Normal damage color */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "DamageNumbers")
    FLinearColor DamageColor = FLinearColor(1.0f, 1.0f, 1.0f, 1.0f);

    /** Critical damage color */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "DamageNumbers")
    FLinearColor CritColor = FLinearColor(1.0f, 0.8f, 0.0f, 1.0f);

    /** Healing color */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "DamageNumbers")
    FLinearColor HealColor = FLinearColor(0.2f, 1.0f, 0.3f, 1.0f);

    /** Crit damage scale multiplier */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "DamageNumbers")
    float CritScale = 1.5f;

    /** Max simultaneous numbers */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "DamageNumbers")
    int32 MaxNumbers = 8;

private:
    struct FFloatingNumber
    {
        FString Text;
        FLinearColor Color;
        float Scale;
        FVector WorldOffset;
        float TimeRemaining;
        float TotalTime;
    };

    TArray<FFloatingNumber> ActiveNumbers;

    void SpawnNumber(const FString& Text, FLinearColor Color, float Scale);
};
