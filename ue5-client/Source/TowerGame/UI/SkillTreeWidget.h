#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "SkillTreeWidget.generated.h"

class UTextBlock;
class UButton;
class UProgressBar;
class UScrollBox;
class UVerticalBox;
class UHorizontalBox;

/// Mastery domain — mirrors Rust MasteryDomain
UENUM(BlueprintType)
enum class EMasteryDomain : uint8
{
    SwordMastery,
    GreatswordMastery,
    DaggerMastery,
    SpearMastery,
    GauntletMastery,
    StaffMastery,
    ParryMastery,
    DodgeMastery,
    BlockMastery,
    AerialMastery,
    Blacksmithing,
    Alchemy,
    Enchanting,
    Tailoring,
    Cooking,
    Mining,
    Herbalism,
    Salvaging,
    Trading,
    Exploration,
    SemanticAttunement,
};

/// Mastery tier — mirrors Rust MasteryTier
UENUM(BlueprintType)
enum class EMasteryTier : uint8
{
    Novice,
    Apprentice,
    Journeyman,
    Expert,
    Master,
    Grandmaster,
};

/// Mastery category for tab grouping
UENUM(BlueprintType)
enum class EMasteryCategory : uint8
{
    Weapon,
    CombatTechnique,
    Crafting,
    Gathering,
    Other,
};

/// Skill tree node display data
USTRUCT(BlueprintType)
struct FSkillNodeDisplay
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString NodeId;
    UPROPERTY(BlueprintReadWrite) FString Name;
    UPROPERTY(BlueprintReadWrite) FString Description;
    UPROPERTY(BlueprintReadWrite) EMasteryTier RequiredTier = EMasteryTier::Novice;
    UPROPERTY(BlueprintReadWrite) TArray<FString> Prerequisites;
    UPROPERTY(BlueprintReadWrite) bool bUnlocked = false;
    UPROPERTY(BlueprintReadWrite) bool bCanUnlock = false;
    UPROPERTY(BlueprintReadWrite) FString EffectDescription;
};

/// Mastery progress for a single domain
USTRUCT(BlueprintType)
struct FMasteryProgressDisplay
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) EMasteryDomain Domain = EMasteryDomain::SwordMastery;
    UPROPERTY(BlueprintReadWrite) FString DomainName;
    UPROPERTY(BlueprintReadWrite) int64 XP = 0;
    UPROPERTY(BlueprintReadWrite) EMasteryTier Tier = EMasteryTier::Novice;
    UPROPERTY(BlueprintReadWrite) float TierProgress = 0.0f;
    UPROPERTY(BlueprintReadWrite) TArray<FSkillNodeDisplay> Nodes;
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnSkillNodeSelected, EMasteryDomain, Domain, const FString&, NodeId);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnSkillNodeUnlocked, EMasteryDomain, Domain, const FString&, NodeId);

/**
 * Skill Mastery Tree widget.
 * Displays mastery progression per domain with unlockable skill tree nodes.
 * Mirrors Rust mastery module (21 domains, 6 tiers, skill tree with effects).
 */
UCLASS()
class TOWERGAME_API USkillTreeWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;

    // --- Data Loading ---
    UFUNCTION(BlueprintCallable) void LoadFromJson(const FString& MasteryJson);
    UFUNCTION(BlueprintCallable) void SelectDomain(EMasteryDomain Domain);
    UFUNCTION(BlueprintCallable) void SelectCategory(EMasteryCategory Category);
    UFUNCTION(BlueprintCallable) void SelectNode(const FString& NodeId);
    UFUNCTION(BlueprintCallable) void UnlockSelectedNode();

    // --- Queries ---
    UFUNCTION(BlueprintPure) FMasteryProgressDisplay GetCurrentDomain() const;
    UFUNCTION(BlueprintPure) TArray<FMasteryProgressDisplay> GetDomainsInCategory(EMasteryCategory Category) const;

    // --- Events ---
    UPROPERTY(BlueprintAssignable) FOnSkillNodeSelected OnNodeSelected;
    UPROPERTY(BlueprintAssignable) FOnSkillNodeUnlocked OnNodeUnlocked;

protected:
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* DomainNameText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* TierText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UProgressBar* TierProgressBar = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* XPText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UHorizontalBox* CategoryTabsBox = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UScrollBox* DomainListBox = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UVerticalBox* NodeTreeBox = nullptr;

    // Selected node detail
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* NodeNameText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* NodeDescText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* NodeRequirementText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* UnlockButton = nullptr;

    TMap<EMasteryDomain, FMasteryProgressDisplay> AllMasteries;
    EMasteryDomain CurrentDomain = EMasteryDomain::SwordMastery;
    FString SelectedNodeId;

    void RebuildDisplay();
    FString GetTierName(EMasteryTier Tier) const;
    FLinearColor GetTierColor(EMasteryTier Tier) const;
    FString GetCategoryName(EMasteryCategory Category) const;
    EMasteryCategory GetDomainCategory(EMasteryDomain Domain) const;
    EMasteryDomain ParseDomain(const FString& Str) const;
    EMasteryTier ParseTier(const FString& Str) const;

    UFUNCTION() void OnUnlockClicked();
};
