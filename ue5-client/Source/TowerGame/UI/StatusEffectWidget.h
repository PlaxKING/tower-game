#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "StatusEffectWidget.generated.h"

class UHorizontalBox;
class UImage;
class UTextBlock;
class UProgressBar;

/**
 * Status effect types matching Rust combat/status.rs
 */
UENUM(BlueprintType)
enum class EStatusType : uint8
{
    // DoT
    Burning         UMETA(DisplayName = "Burning"),
    Poisoned        UMETA(DisplayName = "Poisoned"),
    Bleeding        UMETA(DisplayName = "Bleeding"),

    // CC
    Stunned         UMETA(DisplayName = "Stunned"),
    Frozen          UMETA(DisplayName = "Frozen"),
    Silenced        UMETA(DisplayName = "Silenced"),

    // Debuffs
    Weakened        UMETA(DisplayName = "Weakened"),
    Slowed          UMETA(DisplayName = "Slowed"),
    Exposed         UMETA(DisplayName = "Exposed"),
    Corrupted       UMETA(DisplayName = "Corrupted"),

    // Buffs
    Empowered       UMETA(DisplayName = "Empowered"),
    Hastened        UMETA(DisplayName = "Hastened"),
    Shielded        UMETA(DisplayName = "Shielded"),
    Regenerating    UMETA(DisplayName = "Regenerating"),
    SemanticFocus   UMETA(DisplayName = "Semantic Focus"),
};

/**
 * Active status effect data for UI display.
 */
USTRUCT(BlueprintType)
struct FActiveStatusEffect
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadOnly, Category = "StatusEffect")
    EStatusType Type = EStatusType::Burning;

    UPROPERTY(BlueprintReadOnly, Category = "StatusEffect")
    float RemainingTime = 0.0f;

    UPROPERTY(BlueprintReadOnly, Category = "StatusEffect")
    float TotalDuration = 0.0f;

    UPROPERTY(BlueprintReadOnly, Category = "StatusEffect")
    int32 Stacks = 1;

    UPROPERTY(BlueprintReadOnly, Category = "StatusEffect")
    float Strength = 1.0f;

    /** Get time progress (0.0 = just applied, 1.0 = expired) */
    float GetProgress() const
    {
        return TotalDuration > 0.0f ? 1.0f - (RemainingTime / TotalDuration) : 1.0f;
    }

    /** Is this a buff (positive effect)? */
    bool IsBuff() const
    {
        return Type == EStatusType::Empowered ||
               Type == EStatusType::Hastened ||
               Type == EStatusType::Shielded ||
               Type == EStatusType::Regenerating ||
               Type == EStatusType::SemanticFocus;
    }

    /** Is this a DoT? */
    bool IsDoT() const
    {
        return Type == EStatusType::Burning ||
               Type == EStatusType::Poisoned ||
               Type == EStatusType::Bleeding;
    }

    /** Is this CC? */
    bool IsCC() const
    {
        return Type == EStatusType::Stunned ||
               Type == EStatusType::Frozen ||
               Type == EStatusType::Silenced;
    }

    /** Get display color */
    FLinearColor GetColor() const;

    /** Get short display name */
    FString GetDisplayName() const;

    /** Get icon character (placeholder for actual icons) */
    FString GetIconChar() const;
};

/**
 * Status effect bar — shows active buffs and debuffs.
 *
 * Layout: Horizontal row of status icons with duration timers.
 * Buffs on left (green border), debuffs on right (red border).
 * Each icon shows: type symbol, stack count, remaining time bar.
 *
 * Position: Below HP bar (configurable via Blueprint).
 * Max display: 10 effects (5 buffs + 5 debuffs).
 */
UCLASS()
class TOWERGAME_API UStatusEffectWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;
    virtual void NativeTick(const FGeometry& MyGeometry, float InDeltaTime) override;

    // ============ API ============

    /** Add or refresh a status effect */
    UFUNCTION(BlueprintCallable, Category = "StatusEffect")
    void AddEffect(EStatusType Type, float Duration, float Strength, int32 Stacks = 1);

    /** Remove a status effect */
    UFUNCTION(BlueprintCallable, Category = "StatusEffect")
    void RemoveEffect(EStatusType Type);

    /** Remove all effects */
    UFUNCTION(BlueprintCallable, Category = "StatusEffect")
    void ClearAllEffects();

    /** Get active effects */
    UFUNCTION(BlueprintPure, Category = "StatusEffect")
    const TArray<FActiveStatusEffect>& GetActiveEffects() const { return ActiveEffects; }

    /** Has a specific status? */
    UFUNCTION(BlueprintPure, Category = "StatusEffect")
    bool HasEffect(EStatusType Type) const;

    // ============ Config ============

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "StatusEffect")
    int32 MaxDisplayedEffects = 10;

    // ============ Bound Widgets ============

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "StatusEffect")
    UHorizontalBox* BuffBox;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "StatusEffect")
    UHorizontalBox* DebuffBox;

protected:
    void RebuildDisplay();

private:
    UPROPERTY()
    TArray<FActiveStatusEffect> ActiveEffects;
};
