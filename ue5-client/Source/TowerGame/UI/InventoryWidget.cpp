#include "InventoryWidget.h"
#include "Components/ScrollBox.h"
#include "Components/TextBlock.h"
#include "Components/Button.h"
#include "Components/Image.h"
#include "Components/Border.h"
#include "Dom/JsonObject.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

void UInventoryWidget::NativeConstruct()
{
    Super::NativeConstruct();

    if (UseButton)
    {
        UseButton->OnClicked.AddDynamic(this, &UInventoryWidget::OnUseClicked);
    }
    if (DropButton)
    {
        DropButton->OnClicked.AddDynamic(this, &UInventoryWidget::OnDropClicked);
    }

    RebuildGrid();
    UpdateDetailPanel();
}

void UInventoryWidget::AddItem(const FInventoryItem& Item)
{
    // Currency items go directly to counters
    if (Item.Category == TEXT("Currency"))
    {
        if (Item.ItemName.Contains(TEXT("Shard")))
        {
            TowerShards += Item.Quantity;
        }
        else if (Item.ItemName.Contains(TEXT("Echo")) || Item.ItemName.Contains(TEXT("Fragment")))
        {
            EchoFragments += Item.Quantity;
        }

        // Update currency display
        if (ShardsText)
        {
            ShardsText->SetText(FText::AsNumber(TowerShards));
        }
        if (FragmentsText)
        {
            FragmentsText->SetText(FText::AsNumber(EchoFragments));
        }
        return;
    }

    // Stackable check — same name+rarity
    for (FInventoryItem& Existing : Items)
    {
        if (Existing.ItemName == Item.ItemName && Existing.Rarity == Item.Rarity)
        {
            Existing.Quantity += Item.Quantity;
            RebuildGrid();
            return;
        }
    }

    // New item slot
    if (Items.Num() < MaxSlots)
    {
        Items.Add(Item);
        RebuildGrid();
    }
    else
    {
        UE_LOG(LogTemp, Warning, TEXT("Inventory full! Cannot add %s"), *Item.ItemName);
    }
}

void UInventoryWidget::RemoveItem(int32 Index)
{
    if (Items.IsValidIndex(Index))
    {
        Items.RemoveAt(Index);

        if (SelectedIndex == Index)
        {
            SelectedIndex = -1;
        }
        else if (SelectedIndex > Index)
        {
            SelectedIndex--;
        }

        RebuildGrid();
        UpdateDetailPanel();
    }
}

void UInventoryWidget::AddItemFromJson(const FString& LootJson)
{
    TSharedPtr<FJsonObject> Json;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(LootJson);

    if (!FJsonSerializer::Deserialize(Reader, Json) || !Json.IsValid())
    {
        UE_LOG(LogTemp, Warning, TEXT("Failed to parse loot JSON"));
        return;
    }

    FInventoryItem Item;
    Item.ItemName = Json->GetStringField(TEXT("name"));
    Item.Category = Json->GetStringField(TEXT("category"));
    Item.Rarity = Json->GetStringField(TEXT("rarity"));
    Item.Quantity = Json->GetIntegerField(TEXT("quantity"));
    Item.LootJson = LootJson;

    AddItem(Item);
}

void UInventoryWidget::RebuildGrid()
{
    if (!ItemScrollBox) return;

    ItemScrollBox->ClearChildren();

    for (int32 i = 0; i < Items.Num(); i++)
    {
        const FInventoryItem& Item = Items[i];

        // Create a text button for each item slot
        UTextBlock* SlotText = NewObject<UTextBlock>(this);

        FString DisplayText = FString::Printf(TEXT("[%s] %s x%d"),
            *Item.Rarity.Left(1), *Item.ItemName, Item.Quantity);

        SlotText->SetText(FText::FromString(DisplayText));

        // Color by rarity
        FSlateColor RarityColor(Item.GetRarityColor());
        SlotText->SetColorAndOpacity(RarityColor);

        // Highlight selected
        if (i == SelectedIndex)
        {
            FSlateFontInfo Font = SlotText->GetFont();
            Font.Size = 14;
            SlotText->SetFont(Font);
        }

        ItemScrollBox->AddChild(SlotText);
    }

    // Update currency display
    if (ShardsText)
    {
        ShardsText->SetText(FText::AsNumber(TowerShards));
    }
    if (FragmentsText)
    {
        FragmentsText->SetText(FText::AsNumber(EchoFragments));
    }
}

void UInventoryWidget::UpdateDetailPanel()
{
    if (!Items.IsValidIndex(SelectedIndex))
    {
        if (SelectedItemName) SelectedItemName->SetText(FText::FromString(TEXT("No item selected")));
        if (SelectedItemCategory) SelectedItemCategory->SetText(FText::GetEmpty());
        if (SelectedItemRarity) SelectedItemRarity->SetText(FText::GetEmpty());
        if (SelectedItemQuantity) SelectedItemQuantity->SetText(FText::GetEmpty());
        return;
    }

    const FInventoryItem& Item = Items[SelectedIndex];

    if (SelectedItemName)
    {
        SelectedItemName->SetText(FText::FromString(Item.ItemName));
        SelectedItemName->SetColorAndOpacity(FSlateColor(Item.GetRarityColor()));
    }
    if (SelectedItemCategory)
    {
        SelectedItemCategory->SetText(FText::FromString(Item.Category));
    }
    if (SelectedItemRarity)
    {
        SelectedItemRarity->SetText(FText::FromString(Item.Rarity));
        SelectedItemRarity->SetColorAndOpacity(FSlateColor(Item.GetRarityColor()));
    }
    if (SelectedItemQuantity)
    {
        SelectedItemQuantity->SetText(FText::AsNumber(Item.Quantity));
    }
}

void UInventoryWidget::SelectItem(int32 Index)
{
    if (Items.IsValidIndex(Index))
    {
        SelectedIndex = Index;
        OnItemSelected.Broadcast(Items[Index]);
        UpdateDetailPanel();
        RebuildGrid(); // Refresh highlight
    }
}

void UInventoryWidget::OnUseClicked()
{
    if (Items.IsValidIndex(SelectedIndex))
    {
        FInventoryItem Item = Items[SelectedIndex];
        OnItemUsed.Broadcast(Item);

        // Consumables are used up
        if (Item.Category == TEXT("Consumable") || Item.Category == TEXT("CombatResource"))
        {
            Items[SelectedIndex].Quantity--;
            if (Items[SelectedIndex].Quantity <= 0)
            {
                RemoveItem(SelectedIndex);
            }
            else
            {
                RebuildGrid();
                UpdateDetailPanel();
            }
        }
    }
}

void UInventoryWidget::OnDropClicked()
{
    if (Items.IsValidIndex(SelectedIndex))
    {
        FInventoryItem Item = Items[SelectedIndex];
        OnItemDropped.Broadcast(Item);
        RemoveItem(SelectedIndex);
    }
}
