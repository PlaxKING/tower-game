#include "ItemTooltipWidget.h"
#include "Components/TextBlock.h"
#include "Components/VerticalBox.h"
#include "Components/Border.h"
#include "Components/CanvasPanelSlot.h"
#include "Blueprint/WidgetLayoutLibrary.h"
#include "Dom/JsonObject.h"
#include "Dom/JsonValue.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

void UItemTooltipWidget::NativeConstruct()
{
    Super::NativeConstruct();
    SetVisibility(ESlateVisibility::Collapsed);
}

void UItemTooltipWidget::ShowForItem(const FInventoryItem& Item)
{
    SetVisibility(ESlateVisibility::HitTestInvisible);

    FLinearColor RarityColor = Item.GetRarityColor();

    // Name
    if (ItemNameText)
    {
        ItemNameText->SetText(FText::FromString(Item.ItemName));
        ItemNameText->SetColorAndOpacity(FSlateColor(RarityColor));
    }

    // Category
    if (CategoryText)
    {
        CategoryText->SetText(FText::FromString(Item.Category));
    }

    // Rarity
    if (RarityText)
    {
        RarityText->SetText(FText::FromString(Item.Rarity));
        RarityText->SetColorAndOpacity(FSlateColor(RarityColor));
    }

    // Quantity
    if (QuantityText)
    {
        QuantityText->SetText(FText::FromString(
            FString::Printf(TEXT("Quantity: %d"), Item.Quantity)));
    }

    // Border color
    if (TooltipBorder)
    {
        TooltipBorder->SetBrushColor(RarityColor.ToFColor(true));
    }

    // Semantic tags
    if (TagsBox)
    {
        TagsBox->ClearChildren();

        TArray<TPair<FString, float>> Tags = ParseSemanticTags(Item.LootJson);
        if (Tags.Num() > 0)
        {
            UTextBlock* TagHeader = NewObject<UTextBlock>(this);
            TagHeader->SetText(FText::FromString(TEXT("Semantic Tags:")));
            TagHeader->SetColorAndOpacity(FSlateColor(FLinearColor(0.6f, 0.6f, 0.6f)));
            FSlateFontInfo HeaderFont = TagHeader->GetFont();
            HeaderFont.Size = 9;
            TagHeader->SetFont(HeaderFont);
            TagsBox->AddChild(TagHeader);

            for (const auto& Tag : Tags)
            {
                UTextBlock* TagText = NewObject<UTextBlock>(this);
                TagText->SetText(FText::FromString(
                    FString::Printf(TEXT("  %s: %.2f"), *Tag.Key, Tag.Value)));

                // Color tags by element
                FLinearColor TagColor(0.7f, 0.7f, 0.7f);
                if (Tag.Key == TEXT("fire")) TagColor = FLinearColor(1.0f, 0.4f, 0.1f);
                else if (Tag.Key == TEXT("water")) TagColor = FLinearColor(0.2f, 0.6f, 1.0f);
                else if (Tag.Key == TEXT("earth")) TagColor = FLinearColor(0.6f, 0.4f, 0.2f);
                else if (Tag.Key == TEXT("wind")) TagColor = FLinearColor(0.6f, 1.0f, 0.7f);
                else if (Tag.Key == TEXT("void")) TagColor = FLinearColor(0.5f, 0.2f, 0.8f);
                else if (Tag.Key == TEXT("corruption")) TagColor = FLinearColor(0.3f, 0.0f, 0.2f);

                TagText->SetColorAndOpacity(FSlateColor(TagColor));
                FSlateFontInfo TagFont = TagText->GetFont();
                TagFont.Size = 9;
                TagText->SetFont(TagFont);
                TagsBox->AddChild(TagText);
            }
        }
    }

    // Flavor text
    if (FlavorText)
    {
        FString Flavor = GenerateFlavorText(Item);
        if (!Flavor.IsEmpty())
        {
            FlavorText->SetText(FText::FromString(Flavor));
            FlavorText->SetColorAndOpacity(FSlateColor(FLinearColor(0.5f, 0.5f, 0.4f)));
            FlavorText->SetVisibility(ESlateVisibility::Visible);
        }
        else
        {
            FlavorText->SetVisibility(ESlateVisibility::Collapsed);
        }
    }
}

void UItemTooltipWidget::ShowFromJson(const FString& ItemJson)
{
    TSharedPtr<FJsonObject> Json;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(ItemJson);
    if (!FJsonSerializer::Deserialize(Reader, Json) || !Json.IsValid()) return;

    FInventoryItem Item;
    Item.ItemName = Json->GetStringField(TEXT("name"));
    Item.Category = Json->HasField(TEXT("category")) ? Json->GetStringField(TEXT("category")) : TEXT("Unknown");
    Item.Rarity = Json->HasField(TEXT("rarity")) ? Json->GetStringField(TEXT("rarity")) : TEXT("Common");
    Item.Quantity = Json->HasField(TEXT("quantity")) ? Json->GetIntegerField(TEXT("quantity")) : 1;
    Item.LootJson = ItemJson;

    ShowForItem(Item);
}

void UItemTooltipWidget::HideTooltip()
{
    SetVisibility(ESlateVisibility::Collapsed);
}

void UItemTooltipWidget::SetScreenPosition(FVector2D Position)
{
    SetPositionInViewport(Position);
}

FString UItemTooltipWidget::GenerateFlavorText(const FInventoryItem& Item) const
{
    if (Item.Category == TEXT("Currency"))
    {
        return TEXT("The universal currency of the Tower. Sought by all who climb.");
    }
    if (Item.Category == TEXT("EchoFragment"))
    {
        return TEXT("A crystallized memory from a fallen climber. Hums with fading intent.");
    }
    if (Item.Category == TEXT("CombatResource"))
    {
        if (Item.ItemName.Contains(TEXT("Thermal")))
            return TEXT("Concentrated heat energy. Burns to the touch.");
        if (Item.ItemName.Contains(TEXT("Ember")))
            return TEXT("A spark of elemental fire. Warm even through gloves.");
        return TEXT("Raw combat energy, waiting to be unleashed.");
    }
    if (Item.Category == TEXT("Material"))
    {
        return TEXT("A crafting material imbued with semantic resonance.");
    }
    if (Item.Category == TEXT("Consumable"))
    {
        return TEXT("A restorative draught. Drink wisely — supplies are scarce above floor 20.");
    }

    return TEXT("");
}

TArray<TPair<FString, float>> UItemTooltipWidget::ParseSemanticTags(const FString& LootJson) const
{
    TArray<TPair<FString, float>> Result;

    if (LootJson.IsEmpty()) return Result;

    TSharedPtr<FJsonObject> Json;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(LootJson);
    if (!FJsonSerializer::Deserialize(Reader, Json) || !Json.IsValid()) return Result;

    const TArray<TSharedPtr<FJsonValue>>* Tags;
    if (Json->TryGetArrayField(TEXT("semantic_tags"), Tags))
    {
        for (const TSharedPtr<FJsonValue>& TagVal : *Tags)
        {
            const TArray<TSharedPtr<FJsonValue>>& TagPair = TagVal->AsArray();
            if (TagPair.Num() >= 2)
            {
                FString TagName = TagPair[0]->AsString();
                float TagValue = TagPair[1]->AsNumber();
                Result.Add(TPair<FString, float>(TagName, TagValue));
            }
        }
    }

    return Result;
}
