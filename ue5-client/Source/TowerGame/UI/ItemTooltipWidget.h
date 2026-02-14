#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "UI/InventoryWidget.h"
#include "ItemTooltipWidget.generated.h"

class UTextBlock;
class UVerticalBox;
class UBorder;

/**
 * Item tooltip widget — shows detailed item info on hover.
 *
 * Layout:
 *   [Item Name] (rarity colored)
 *   [Category]
 *   [Rarity]
 *   -----
 *   Quantity: N
 *   -----
 *   Semantic Tags:
 *     fire: 0.45
 *     corruption: 0.30
 *   -----
 *   (flavor text based on tags)
 *
 * Appears near cursor when hovering over inventory items.
 * Border color matches rarity.
 */
UCLASS()
class TOWERGAME_API UItemTooltipWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;

    // ============ API ============

    /** Show tooltip for an item */
    UFUNCTION(BlueprintCallable, Category = "Tooltip")
    void ShowForItem(const FInventoryItem& Item);

    /** Show tooltip from JSON */
    UFUNCTION(BlueprintCallable, Category = "Tooltip")
    void ShowFromJson(const FString& ItemJson);

    /** Hide tooltip */
    UFUNCTION(BlueprintCallable, Category = "Tooltip")
    void HideTooltip();

    /** Move tooltip to screen position */
    UFUNCTION(BlueprintCallable, Category = "Tooltip")
    void SetScreenPosition(FVector2D Position);

    /** Is tooltip visible? */
    UFUNCTION(BlueprintPure, Category = "Tooltip")
    bool IsTooltipVisible() const { return GetVisibility() == ESlateVisibility::Visible; }

    // ============ Bound Widgets ============

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Tooltip")
    UTextBlock* ItemNameText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Tooltip")
    UTextBlock* CategoryText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Tooltip")
    UTextBlock* RarityText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Tooltip")
    UTextBlock* QuantityText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Tooltip")
    UVerticalBox* TagsBox;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Tooltip")
    UTextBlock* FlavorText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Tooltip")
    UBorder* TooltipBorder;

protected:
    /** Generate flavor text from item semantic tags */
    FString GenerateFlavorText(const FInventoryItem& Item) const;

    /** Parse semantic tags from loot JSON */
    TArray<TPair<FString, float>> ParseSemanticTags(const FString& LootJson) const;
};
