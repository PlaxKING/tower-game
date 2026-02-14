#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "CraftingWidget.generated.h"

class UScrollBox;
class UTextBlock;
class UButton;
class UVerticalBox;
class UHorizontalBox;
class UProgressBar;

/// Crafting recipe data from Rust core
USTRUCT(BlueprintType)
struct FCraftingRecipeData
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString Name;
    UPROPERTY(BlueprintReadWrite) FString ResultCategory; // Weapon, Armor, Accessory, Consumable, Enhancement
    UPROPERTY(BlueprintReadWrite) int32 MaterialCount = 0;
    UPROPERTY(BlueprintReadWrite) FString MinRarity;
    UPROPERTY(BlueprintReadWrite) int64 ShardCost = 0;
    UPROPERTY(BlueprintReadWrite) TArray<FString> RequiredTagNames;
    UPROPERTY(BlueprintReadWrite) TArray<float> RequiredTagValues;
};

/// Material slot in the crafting grid
USTRUCT(BlueprintType)
struct FCraftMaterialSlot
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString ItemName;
    UPROPERTY(BlueprintReadWrite) FString Rarity;
    UPROPERTY(BlueprintReadWrite) TArray<FString> TagNames;
    UPROPERTY(BlueprintReadWrite) TArray<float> TagValues;
    UPROPERTY(BlueprintReadWrite) bool bOccupied = false;
};

/// Result preview
USTRUCT(BlueprintType)
struct FCraftResultPreview
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString Name;
    UPROPERTY(BlueprintReadWrite) FString Category;
    UPROPERTY(BlueprintReadWrite) FString Rarity;
    UPROPERTY(BlueprintReadWrite) float Quality = 0.0f;
    UPROPERTY(BlueprintReadWrite) float Similarity = 0.0f;
    UPROPERTY(BlueprintReadWrite) bool bCanCraft = false;
    UPROPERTY(BlueprintReadWrite) FString FailReason;
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnCraftAttempt, const FCraftingRecipeData&, Recipe);
DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnCraftingClosed);

/**
 * Crafting UI: select recipe, place materials, preview result, craft.
 * Matches Rust economy::crafting system (semantic tag matching, quality from similarity).
 */
UCLASS()
class TOWERGAME_API UCraftingWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;

    // --- Data ---
    UFUNCTION(BlueprintCallable) void SetRecipes(const TArray<FCraftingRecipeData>& Recipes);
    UFUNCTION(BlueprintCallable) void AddRecipeFromJson(const FString& RecipeJson);
    UFUNCTION(BlueprintCallable) void SelectRecipe(int32 Index);
    UFUNCTION(BlueprintCallable) void PlaceMaterial(int32 SlotIndex, const FString& ItemName,
        const FString& Rarity, const TArray<FString>& TagNames, const TArray<float>& TagValues);
    UFUNCTION(BlueprintCallable) void ClearSlot(int32 SlotIndex);
    UFUNCTION(BlueprintCallable) void ClearAllSlots();
    UFUNCTION(BlueprintCallable) void AttemptCraft();
    UFUNCTION(BlueprintCallable) void SetPlayerShards(int64 Shards);
    UFUNCTION(BlueprintCallable) void ShowCraftResult(const FString& ResultJson);
    UFUNCTION(BlueprintCallable) void Close();

    // --- Events ---
    UPROPERTY(BlueprintAssignable) FOnCraftAttempt OnCraftAttempt;
    UPROPERTY(BlueprintAssignable) FOnCraftingClosed OnCraftingClosed;

protected:
    // --- Bound Widgets ---
    UPROPERTY(meta = (BindWidgetOptional)) UScrollBox* RecipeListBox = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* RecipeNameText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* RecipeCategoryText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* MaterialRequirementText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* ShardCostText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* PlayerShardsText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UHorizontalBox* MaterialSlotsBox = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* PreviewNameText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* PreviewRarityText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UProgressBar* QualityBar = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* QualityText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* SimilarityText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* CraftButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* CloseButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* ResultMessageText = nullptr;

    // --- State ---
    TArray<FCraftingRecipeData> AvailableRecipes;
    int32 SelectedRecipeIndex = -1;
    TArray<FCraftMaterialSlot> MaterialSlots;
    FCraftResultPreview CurrentPreview;
    int64 PlayerShards = 0;

    UPROPERTY(EditDefaultsOnly) int32 MaxMaterialSlots = 6;

    // --- Internal ---
    void RebuildRecipeList();
    void UpdateRecipeDetail();
    void UpdateMaterialSlots();
    void UpdatePreview();
    float CalculateSimilarity() const;
    FLinearColor GetCategoryColor(const FString& Category) const;
    FLinearColor GetRarityColor(const FString& Rarity) const;

    UFUNCTION() void OnCraftClicked();
    UFUNCTION() void OnCloseClicked();
};
