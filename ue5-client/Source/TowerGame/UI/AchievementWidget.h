#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "AchievementWidget.generated.h"

class UTextBlock;
class UVerticalBox;
class UHorizontalBox;
class UProgressBar;
class UImage;
class UScrollBox;

/// Achievement category — mirrors Rust AchievementCategory
UENUM(BlueprintType)
enum class EAchievementCategory : uint8
{
    Combat,
    Exploration,
    Semantic,
    Social,
    Crafting,
    Survival,
    Mastery,
    Tower,
};

/// Achievement tier — mirrors Rust AchievementTier
UENUM(BlueprintType)
enum class EAchievementTier : uint8
{
    Bronze,
    Silver,
    Gold,
    Platinum,
    Mythic,
};

/// Achievement display data
USTRUCT(BlueprintType)
struct FAchievementDisplayData
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString Id;
    UPROPERTY(BlueprintReadWrite) FString Name;
    UPROPERTY(BlueprintReadWrite) FString Description;
    UPROPERTY(BlueprintReadWrite) EAchievementCategory Category = EAchievementCategory::Combat;
    UPROPERTY(BlueprintReadWrite) EAchievementTier Tier = EAchievementTier::Bronze;
    UPROPERTY(BlueprintReadWrite) float Progress = 0.0f;  // 0.0 - 1.0
    UPROPERTY(BlueprintReadWrite) bool bUnlocked = false;
    UPROPERTY(BlueprintReadWrite) bool bHidden = false;
    UPROPERTY(BlueprintReadWrite) int32 ShardReward = 0;
    UPROPERTY(BlueprintReadWrite) FString UnlockedDate;
};

/// Achievement category tab data
USTRUCT(BlueprintType)
struct FAchievementCategoryTab
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) EAchievementCategory Category = EAchievementCategory::Combat;
    UPROPERTY(BlueprintReadWrite) int32 Total = 0;
    UPROPERTY(BlueprintReadWrite) int32 Unlocked = 0;
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnAchievementSelected, const FAchievementDisplayData&, Achievement);

/**
 * Achievement display panel.
 * Shows achievements organized by category, with progress tracking.
 * Mirrors Rust achievements module (8 categories, 5 tiers, 6 condition types).
 */
UCLASS()
class TOWERGAME_API UAchievementWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;

    // --- Data Loading ---

    UFUNCTION(BlueprintCallable, Category = "Achievements")
    void LoadFromJson(const FString& AchievementsJson);

    UFUNCTION(BlueprintCallable, Category = "Achievements")
    void AddAchievement(const FAchievementDisplayData& Data);

    UFUNCTION(BlueprintCallable, Category = "Achievements")
    void UpdateProgress(const FString& AchievementId, float NewProgress);

    UFUNCTION(BlueprintCallable, Category = "Achievements")
    void MarkUnlocked(const FString& AchievementId);

    // --- Filtering ---

    UFUNCTION(BlueprintCallable, Category = "Achievements")
    void FilterByCategory(EAchievementCategory Category);

    UFUNCTION(BlueprintCallable, Category = "Achievements")
    void ShowAll();

    UFUNCTION(BlueprintCallable, Category = "Achievements")
    void ToggleHidden(bool bShow);

    // --- Queries ---

    UFUNCTION(BlueprintPure, Category = "Achievements")
    int32 GetTotalCount() const { return AllAchievements.Num(); }

    UFUNCTION(BlueprintPure, Category = "Achievements")
    int32 GetUnlockedCount() const;

    UFUNCTION(BlueprintPure, Category = "Achievements")
    float GetOverallProgress() const;

    UFUNCTION(BlueprintPure, Category = "Achievements")
    TArray<FAchievementCategoryTab> GetCategoryTabs() const;

    // --- Toast notification for newly unlocked ---

    UFUNCTION(BlueprintCallable, Category = "Achievements")
    void ShowUnlockToast(const FString& AchievementId);

    // --- Events ---

    UPROPERTY(BlueprintAssignable, Category = "Achievements")
    FOnAchievementSelected OnAchievementSelected;

protected:
    UPROPERTY(meta = (BindWidgetOptional)) UScrollBox* AchievementList = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UHorizontalBox* CategoryTabsBox = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* TotalProgressText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UProgressBar* OverallProgressBar = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* CategoryFilterText = nullptr;

    // Detail panel for selected achievement
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* DetailName = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* DetailDesc = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* DetailTier = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UProgressBar* DetailProgress = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* DetailReward = nullptr;

    // Toast (slides in from top-right when achievement unlocked)
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* ToastTitle = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* ToastDesc = nullptr;

    UPROPERTY(EditDefaultsOnly, Category = "Achievements")
    float ToastDuration = 5.0f;

    UPROPERTY(EditDefaultsOnly, Category = "Achievements")
    bool bShowHidden = false;

    TArray<FAchievementDisplayData> AllAchievements;
    EAchievementCategory CurrentFilter = EAchievementCategory::Combat;
    bool bFilterActive = false;

    void RebuildList();
    FLinearColor GetCategoryColor(EAchievementCategory Category) const;
    FLinearColor GetTierColor(EAchievementTier Tier) const;
    FString GetCategoryName(EAchievementCategory Category) const;
    FString GetTierName(EAchievementTier Tier) const;
    FString GetCategoryIcon(EAchievementCategory Category) const;

    EAchievementCategory ParseCategory(const FString& Str) const;
    EAchievementTier ParseTier(const FString& Str) const;
};
