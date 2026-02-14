#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "AbilityBarWidget.generated.h"

class UHorizontalBox;
class UTextBlock;
class UImage;
class UProgressBar;
class UButton;
class UVerticalBox;
class UBorder;
class UOverlay;

/**
 * Targeting type for abilities — mirrors Rust AbilityTarget enum.
 */
UENUM(BlueprintType)
enum class EAbilityTarget : uint8
{
    Melee           UMETA(DisplayName = "Melee"),
    Ranged          UMETA(DisplayName = "Ranged"),
    SelfAoE         UMETA(DisplayName = "Self AoE"),
    GroundTarget    UMETA(DisplayName = "Ground Target"),
    AllyTarget      UMETA(DisplayName = "Ally Target"),
    PartyAoE        UMETA(DisplayName = "Party AoE"),
    SelfOnly        UMETA(DisplayName = "Self Only"),
};

/**
 * Resource cost for an ability — mirrors Rust AbilityCost.
 */
USTRUCT(BlueprintType)
struct FAbilityCost
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite, Category = "Ability")
    float Kinetic = 0.0f;

    UPROPERTY(BlueprintReadWrite, Category = "Ability")
    float Thermal = 0.0f;

    UPROPERTY(BlueprintReadWrite, Category = "Ability")
    float Semantic = 0.0f;

    float TotalCost() const { return Kinetic + Thermal + Semantic; }

    /** Get the primary cost type name for display */
    FString GetPrimaryCostLabel() const
    {
        if (Kinetic >= Thermal && Kinetic >= Semantic && Kinetic > 0.0f)
            return FString::Printf(TEXT("%.0f KIN"), Kinetic);
        if (Thermal >= Kinetic && Thermal >= Semantic && Thermal > 0.0f)
            return FString::Printf(TEXT("%.0f THR"), Thermal);
        if (Semantic > 0.0f)
            return FString::Printf(TEXT("%.0f SEM"), Semantic);
        return TEXT("Free");
    }
};

/**
 * Display data for an ability in the hotbar — mirrors Rust Ability struct.
 */
USTRUCT(BlueprintType)
struct FAbilityDisplayData
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite, Category = "Ability")
    FString Id;

    UPROPERTY(BlueprintReadWrite, Category = "Ability")
    FString Name;

    UPROPERTY(BlueprintReadWrite, Category = "Ability")
    FString Description;

    UPROPERTY(BlueprintReadWrite, Category = "Ability")
    FString IconTag;

    UPROPERTY(BlueprintReadWrite, Category = "Ability")
    float Cooldown = 0.0f;

    UPROPERTY(BlueprintReadWrite, Category = "Ability")
    FAbilityCost Cost;

    UPROPERTY(BlueprintReadWrite, Category = "Ability")
    EAbilityTarget TargetType = EAbilityTarget::Melee;

    UPROPERTY(BlueprintReadWrite, Category = "Ability")
    float Range = 0.0f;

    UPROPERTY(BlueprintReadWrite, Category = "Ability")
    float Radius = 0.0f;

    UPROPERTY(BlueprintReadWrite, Category = "Ability")
    float CastTime = 0.0f;

    UPROPERTY(BlueprintReadWrite, Category = "Ability")
    bool bIsReady = true;

    /** Get target type as display string */
    FString GetTargetLabel() const;

    /** Get icon placeholder character for the ability */
    FString GetIconChar() const;
};

/**
 * Per-slot cooldown tracking state.
 */
USTRUCT()
struct FAbilitySlotState
{
    GENERATED_BODY()

    /** Ability ID assigned to this slot (empty = unoccupied) */
    FString AbilityId;

    /** Total cooldown duration for the current cycle */
    float CooldownTotal = 0.0f;

    /** Remaining cooldown in seconds */
    float CooldownRemaining = 0.0f;

    /** Flash timer for use-animation feedback */
    float FlashTimer = 0.0f;

    bool IsOnCooldown() const { return CooldownRemaining > 0.0f; }
    bool IsFlashing() const { return FlashTimer > 0.0f; }
    bool IsOccupied() const { return !AbilityId.IsEmpty(); }
    float GetCooldownProgress() const
    {
        return CooldownTotal > 0.0f ? CooldownRemaining / CooldownTotal : 0.0f;
    }
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnAbilityUsed, int32, SlotIndex, const FString&, AbilityId);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnAbilityFailed, int32, SlotIndex, const FString&, Reason);

/** Number of ability hotbar slots */
static constexpr int32 ABILITY_SLOT_COUNT = 6;

/**
 * Ability hotbar widget.
 *
 * Layout:
 *   [HorizontalBox] 6 ability slots, each containing:
 *     - Keybind label (1-6)
 *     - Ability icon
 *     - Cooldown overlay text (remaining seconds)
 *     - Cooldown sweep progress bar
 *
 * Slots are grayed out when on cooldown, flash white on use.
 * Hover tooltip shows ability name, description, cost, targeting, range/radius.
 *
 * Data loaded from Rust JSON via LoadAbilities().
 * Mirrors Rust AbilityLoadout (6 slots max) and AbilityCooldownTracker.
 */
UCLASS()
class TOWERGAME_API UAbilityBarWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;
    virtual void NativeTick(const FGeometry& MyGeometry, float InDeltaTime) override;

    // ============ Ability Management ============

    /** Load all known abilities from JSON (Rust AbilityLoadout::to_json()) */
    UFUNCTION(BlueprintCallable, Category = "Ability")
    void LoadAbilities(const FString& AbilitiesJson);

    /** Assign an ability to a hotbar slot (0-5) */
    UFUNCTION(BlueprintCallable, Category = "Ability")
    void SetSlot(int32 SlotIndex, const FString& AbilityId);

    /** Clear a hotbar slot */
    UFUNCTION(BlueprintCallable, Category = "Ability")
    void ClearSlot(int32 SlotIndex);

    /** Activate the ability in a slot (triggers cooldown + delegates) */
    UFUNCTION(BlueprintCallable, Category = "Ability")
    void UseAbility(int32 SlotIndex);

    /** Tick cooldowns by delta time (called automatically in NativeTick) */
    UFUNCTION(BlueprintCallable, Category = "Ability")
    void UpdateCooldowns(float DeltaTime);

    /** Apply global cooldown reduction percentage (0.0 - 1.0) */
    UFUNCTION(BlueprintCallable, Category = "Ability")
    void SetCooldownReduction(float Percent);

    // ============ Queries ============

    /** Get ability data by ID */
    UFUNCTION(BlueprintPure, Category = "Ability")
    bool GetAbilityData(const FString& AbilityId, FAbilityDisplayData& OutData) const;

    /** Get ability in a specific slot */
    UFUNCTION(BlueprintPure, Category = "Ability")
    bool GetSlotAbility(int32 SlotIndex, FAbilityDisplayData& OutData) const;

    /** Is a slot on cooldown? */
    UFUNCTION(BlueprintPure, Category = "Ability")
    bool IsSlotOnCooldown(int32 SlotIndex) const;

    /** Get remaining cooldown for a slot */
    UFUNCTION(BlueprintPure, Category = "Ability")
    float GetSlotCooldownRemaining(int32 SlotIndex) const;

    // ============ Config ============

    /** Duration of the flash effect when an ability is used */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Ability")
    float FlashDuration = 0.2f;

    /** Color for the flash effect on ability use */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Ability")
    FLinearColor FlashColor = FLinearColor(1.0f, 1.0f, 1.0f, 0.8f);

    /** Color for grayed-out cooldown state */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Ability")
    FLinearColor CooldownTint = FLinearColor(0.3f, 0.3f, 0.3f, 1.0f);

    /** Normal slot tint */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Ability")
    FLinearColor ReadyTint = FLinearColor(1.0f, 1.0f, 1.0f, 1.0f);

    // ============ Events ============

    UPROPERTY(BlueprintAssignable, Category = "Ability")
    FOnAbilityUsed OnAbilityUsed;

    UPROPERTY(BlueprintAssignable, Category = "Ability")
    FOnAbilityFailed OnAbilityFailed;

    // ============ Bound Widgets ============

    /** Container for all 6 ability slot panels */
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UHorizontalBox* AbilitySlotsBox = nullptr;

    // --- Per-slot widgets (bound from Blueprint) ---

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* Slot0_KeyLabel = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* Slot0_IconText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* Slot0_CooldownText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UProgressBar* Slot0_CooldownBar = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UBorder* Slot0_Border = nullptr;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* Slot1_KeyLabel = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* Slot1_IconText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* Slot1_CooldownText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UProgressBar* Slot1_CooldownBar = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UBorder* Slot1_Border = nullptr;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* Slot2_KeyLabel = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* Slot2_IconText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* Slot2_CooldownText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UProgressBar* Slot2_CooldownBar = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UBorder* Slot2_Border = nullptr;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* Slot3_KeyLabel = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* Slot3_IconText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* Slot3_CooldownText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UProgressBar* Slot3_CooldownBar = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UBorder* Slot3_Border = nullptr;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* Slot4_KeyLabel = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* Slot4_IconText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* Slot4_CooldownText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UProgressBar* Slot4_CooldownBar = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UBorder* Slot4_Border = nullptr;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* Slot5_KeyLabel = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* Slot5_IconText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* Slot5_CooldownText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UProgressBar* Slot5_CooldownBar = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UBorder* Slot5_Border = nullptr;

    /** Tooltip panel (shown on hover) */
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UVerticalBox* TooltipBox = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* TooltipNameText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* TooltipDescText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* TooltipCostText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* TooltipTargetText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* TooltipRangeText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* TooltipCooldownText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Ability")
    UTextBlock* TooltipCastTimeText = nullptr;

protected:
    /** Rebuild the entire visual display from current state */
    void RebuildDisplay();

    /** Update a single slot's visuals (cooldown, tint, flash) */
    void UpdateSlotVisuals(int32 SlotIndex);

    /** Show tooltip for a specific slot */
    void ShowTooltip(int32 SlotIndex);

    /** Hide the tooltip */
    void HideTooltip();

    /** Get the bound widget pointers for a given slot index */
    void GetSlotWidgets(int32 SlotIndex, UTextBlock*& OutKeyLabel, UTextBlock*& OutIconText,
        UTextBlock*& OutCooldownText, UProgressBar*& OutCooldownBar, UBorder*& OutBorder) const;

    /** Parse AbilityTarget enum from string */
    EAbilityTarget ParseTargetType(const FString& Str) const;

    /** Build cost struct from JSON object */
    FAbilityCost ParseCost(const TSharedPtr<class FJsonObject>& CostObj) const;

private:
    /** All known abilities keyed by ID */
    TMap<FString, FAbilityDisplayData> KnownAbilities;

    /** Per-slot state (cooldowns, flash, assigned ability) */
    FAbilitySlotState SlotStates[ABILITY_SLOT_COUNT];

    /** Global cooldown reduction multiplier (0.0 = none, 1.0 = instant) */
    float CooldownReductionPercent = 0.0f;

    /** Currently hovered slot for tooltip (-1 = none) */
    int32 HoveredSlot = -1;
};
