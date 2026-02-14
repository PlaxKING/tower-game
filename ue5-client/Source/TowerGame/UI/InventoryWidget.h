#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "InventoryWidget.generated.h"

class UScrollBox;
class UTextBlock;
class UImage;
class UButton;
class UBorder;
class UGridPanel;
class UUniformGridPanel;

/**
 * Inventory item data — parsed from Rust loot JSON.
 */
USTRUCT(BlueprintType)
struct FInventoryItem
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadOnly, Category = "Inventory")
    FString ItemName;

    UPROPERTY(BlueprintReadOnly, Category = "Inventory")
    FString Category; // CombatResource, Material, Consumable, Equipment, Currency, EchoFragment

    UPROPERTY(BlueprintReadOnly, Category = "Inventory")
    FString Rarity; // Common, Uncommon, Rare, Epic, Legendary, Mythic

    UPROPERTY(BlueprintReadOnly, Category = "Inventory")
    int32 Quantity = 1;

    UPROPERTY(BlueprintReadOnly, Category = "Inventory")
    FString LootJson; // Raw JSON for server sync

    FLinearColor GetRarityColor() const
    {
        if (Rarity == TEXT("Uncommon")) return FLinearColor(0.2f, 0.9f, 0.3f);
        if (Rarity == TEXT("Rare")) return FLinearColor(0.2f, 0.4f, 1.0f);
        if (Rarity == TEXT("Epic")) return FLinearColor(0.7f, 0.2f, 0.9f);
        if (Rarity == TEXT("Legendary")) return FLinearColor(1.0f, 0.65f, 0.0f);
        if (Rarity == TEXT("Mythic")) return FLinearColor(1.0f, 0.15f, 0.15f);
        return FLinearColor(0.8f, 0.8f, 0.8f); // Common
    }
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnItemSelected, const FInventoryItem&, Item);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnItemUsed, const FInventoryItem&, Item);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnItemDropped, const FInventoryItem&, Item);

/**
 * Inventory panel widget.
 *
 * Layout:
 *   [Left Panel]  Grid of item slots (6 columns x N rows)
 *   [Right Panel]  Selected item details + Use/Drop buttons
 *   [Bottom]  Currency display (Tower Shards, Echo Fragments)
 *
 * Items are stored as FInventoryItem structs parsed from Rust loot JSON.
 * Toggle visibility with Tab key (handled by PlayerCharacter input).
 */
UCLASS()
class TOWERGAME_API UInventoryWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;

    // ============ Inventory Management ============

    /** Add item to inventory (called when loot is collected) */
    UFUNCTION(BlueprintCallable, Category = "Inventory")
    void AddItem(const FInventoryItem& Item);

    /** Remove item by index */
    UFUNCTION(BlueprintCallable, Category = "Inventory")
    void RemoveItem(int32 Index);

    /** Add item from raw loot JSON */
    UFUNCTION(BlueprintCallable, Category = "Inventory")
    void AddItemFromJson(const FString& LootJson);

    /** Get all items */
    UFUNCTION(BlueprintPure, Category = "Inventory")
    const TArray<FInventoryItem>& GetItems() const { return Items; }

    /** Get total currency */
    UFUNCTION(BlueprintPure, Category = "Inventory")
    int32 GetTowerShards() const { return TowerShards; }

    UFUNCTION(BlueprintPure, Category = "Inventory")
    int32 GetEchoFragments() const { return EchoFragments; }

    // ============ Config ============

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Inventory")
    int32 MaxSlots = 60;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Inventory")
    int32 GridColumns = 6;

    // ============ Events ============

    UPROPERTY(BlueprintAssignable, Category = "Inventory")
    FOnItemSelected OnItemSelected;

    UPROPERTY(BlueprintAssignable, Category = "Inventory")
    FOnItemUsed OnItemUsed;

    UPROPERTY(BlueprintAssignable, Category = "Inventory")
    FOnItemDropped OnItemDropped;

    // ============ Bound Widgets ============

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Inventory")
    UTextBlock* TitleText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Inventory")
    UScrollBox* ItemScrollBox;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Inventory")
    UTextBlock* SelectedItemName;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Inventory")
    UTextBlock* SelectedItemCategory;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Inventory")
    UTextBlock* SelectedItemRarity;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Inventory")
    UTextBlock* SelectedItemQuantity;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Inventory")
    UButton* UseButton;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Inventory")
    UButton* DropButton;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Inventory")
    UTextBlock* ShardsText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Inventory")
    UTextBlock* FragmentsText;

protected:
    /** Rebuild the visual grid from current Items array */
    void RebuildGrid();

    /** Update detail panel from selected item */
    void UpdateDetailPanel();

    /** Select item at index */
    UFUNCTION()
    void SelectItem(int32 Index);

    UFUNCTION()
    void OnUseClicked();

    UFUNCTION()
    void OnDropClicked();

private:
    UPROPERTY()
    TArray<FInventoryItem> Items;

    int32 SelectedIndex = -1;
    int32 TowerShards = 0;
    int32 EchoFragments = 0;
};
