#include "CraftingWidget.h"
#include "Components/ScrollBox.h"
#include "Components/TextBlock.h"
#include "Components/Button.h"
#include "Components/VerticalBox.h"
#include "Components/HorizontalBox.h"
#include "Components/ProgressBar.h"
#include "Dom/JsonObject.h"
#include "Dom/JsonValue.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

void UCraftingWidget::NativeConstruct()
{
    Super::NativeConstruct();

    MaterialSlots.SetNum(MaxMaterialSlots);

    if (CraftButton)
    {
        CraftButton->OnClicked.AddDynamic(this, &UCraftingWidget::OnCraftClicked);
        CraftButton->SetIsEnabled(false);
    }
    if (CloseButton)
    {
        CloseButton->OnClicked.AddDynamic(this, &UCraftingWidget::OnCloseClicked);
    }

    RebuildRecipeList();
}

void UCraftingWidget::SetRecipes(const TArray<FCraftingRecipeData>& Recipes)
{
    AvailableRecipes = Recipes;
    SelectedRecipeIndex = -1;
    RebuildRecipeList();
}

void UCraftingWidget::AddRecipeFromJson(const FString& RecipeJson)
{
    TSharedPtr<FJsonObject> Json;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(RecipeJson);
    if (!FJsonSerializer::Deserialize(Reader, Json) || !Json.IsValid()) return;

    FCraftingRecipeData Recipe;
    Recipe.Name = Json->GetStringField(TEXT("name"));
    Recipe.ResultCategory = Json->GetStringField(TEXT("result_category"));
    Recipe.MaterialCount = Json->GetIntegerField(TEXT("material_count"));
    Recipe.MinRarity = Json->HasField(TEXT("min_rarity")) ?
        Json->GetStringField(TEXT("min_rarity")) : TEXT("Common");
    Recipe.ShardCost = Json->HasField(TEXT("shard_cost")) ?
        Json->GetIntegerField(TEXT("shard_cost")) : 0;

    const TArray<TSharedPtr<FJsonValue>>* Tags;
    if (Json->TryGetArrayField(TEXT("required_tags"), Tags))
    {
        for (const TSharedPtr<FJsonValue>& TagVal : *Tags)
        {
            const TArray<TSharedPtr<FJsonValue>>& Pair = TagVal->AsArray();
            if (Pair.Num() >= 2)
            {
                Recipe.RequiredTagNames.Add(Pair[0]->AsString());
                Recipe.RequiredTagValues.Add(Pair[1]->AsNumber());
            }
        }
    }

    AvailableRecipes.Add(Recipe);
    RebuildRecipeList();
}

void UCraftingWidget::SelectRecipe(int32 Index)
{
    if (Index >= 0 && Index < AvailableRecipes.Num())
    {
        SelectedRecipeIndex = Index;
        ClearAllSlots();
        UpdateRecipeDetail();
        UpdatePreview();
    }
}

void UCraftingWidget::PlaceMaterial(int32 SlotIndex, const FString& ItemName,
    const FString& Rarity, const TArray<FString>& TagNames, const TArray<float>& TagValues)
{
    if (SlotIndex < 0 || SlotIndex >= MaterialSlots.Num()) return;

    FCraftMaterialSlot& Slot = MaterialSlots[SlotIndex];
    Slot.ItemName = ItemName;
    Slot.Rarity = Rarity;
    Slot.TagNames = TagNames;
    Slot.TagValues = TagValues;
    Slot.bOccupied = true;

    UpdateMaterialSlots();
    UpdatePreview();
}

void UCraftingWidget::ClearSlot(int32 SlotIndex)
{
    if (SlotIndex < 0 || SlotIndex >= MaterialSlots.Num()) return;

    MaterialSlots[SlotIndex] = FCraftMaterialSlot();
    UpdateMaterialSlots();
    UpdatePreview();
}

void UCraftingWidget::ClearAllSlots()
{
    for (FCraftMaterialSlot& Slot : MaterialSlots)
    {
        Slot = FCraftMaterialSlot();
    }
    UpdateMaterialSlots();
    UpdatePreview();
}

void UCraftingWidget::AttemptCraft()
{
    if (SelectedRecipeIndex < 0 || SelectedRecipeIndex >= AvailableRecipes.Num()) return;
    if (!CurrentPreview.bCanCraft) return;

    OnCraftAttempt.Broadcast(AvailableRecipes[SelectedRecipeIndex]);
}

void UCraftingWidget::SetPlayerShards(int64 Shards)
{
    PlayerShards = Shards;
    if (PlayerShardsText)
    {
        PlayerShardsText->SetText(FText::FromString(
            FString::Printf(TEXT("Shards: %lld"), PlayerShards)));
    }
    UpdatePreview();
}

void UCraftingWidget::ShowCraftResult(const FString& ResultJson)
{
    TSharedPtr<FJsonObject> Json;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(ResultJson);
    if (!FJsonSerializer::Deserialize(Reader, Json) || !Json.IsValid()) return;

    FString Name = Json->GetStringField(TEXT("name"));
    FString Category = Json->GetStringField(TEXT("category"));
    FString Rarity = Json->GetStringField(TEXT("rarity"));
    float Quality = Json->GetNumberField(TEXT("quality"));

    if (ResultMessageText)
    {
        FString Msg = FString::Printf(TEXT("Crafted: %s (%s %s) Quality: %.0f%%"),
            *Name, *Rarity, *Category, Quality * 100.0f);
        ResultMessageText->SetText(FText::FromString(Msg));
        ResultMessageText->SetColorAndOpacity(FSlateColor(GetRarityColor(Rarity)));
    }

    ClearAllSlots();
}

void UCraftingWidget::Close()
{
    SetVisibility(ESlateVisibility::Collapsed);

    APlayerController* PC = GetOwningPlayer();
    if (PC)
    {
        PC->SetShowMouseCursor(false);
        PC->SetInputMode(FInputModeGameOnly());
    }

    OnCraftingClosed.Broadcast();
}

void UCraftingWidget::RebuildRecipeList()
{
    if (!RecipeListBox) return;
    RecipeListBox->ClearChildren();

    for (int32 i = 0; i < AvailableRecipes.Num(); i++)
    {
        const FCraftingRecipeData& Recipe = AvailableRecipes[i];

        UTextBlock* EntryText = NewObject<UTextBlock>(this);
        FString Display = FString::Printf(TEXT("[%s] %s (%d mats, %lld shards)"),
            *Recipe.ResultCategory, *Recipe.Name, Recipe.MaterialCount, Recipe.ShardCost);
        EntryText->SetText(FText::FromString(Display));

        FLinearColor Color = GetCategoryColor(Recipe.ResultCategory);
        if (i == SelectedRecipeIndex)
        {
            Color = FLinearColor(1.0f, 1.0f, 0.3f); // Highlighted yellow
        }
        EntryText->SetColorAndOpacity(FSlateColor(Color));

        FSlateFontInfo Font = EntryText->GetFont();
        Font.Size = 12;
        EntryText->SetFont(Font);

        RecipeListBox->AddChild(EntryText);
    }
}

void UCraftingWidget::UpdateRecipeDetail()
{
    if (SelectedRecipeIndex < 0 || SelectedRecipeIndex >= AvailableRecipes.Num()) return;
    const FCraftingRecipeData& Recipe = AvailableRecipes[SelectedRecipeIndex];

    if (RecipeNameText)
    {
        RecipeNameText->SetText(FText::FromString(Recipe.Name));
        RecipeNameText->SetColorAndOpacity(FSlateColor(GetCategoryColor(Recipe.ResultCategory)));
    }

    if (RecipeCategoryText)
    {
        RecipeCategoryText->SetText(FText::FromString(
            FString::Printf(TEXT("Category: %s"), *Recipe.ResultCategory)));
    }

    if (MaterialRequirementText)
    {
        FString TagStr;
        for (int32 i = 0; i < Recipe.RequiredTagNames.Num(); i++)
        {
            if (i > 0) TagStr += TEXT(", ");
            TagStr += FString::Printf(TEXT("%s(%.1f)"),
                *Recipe.RequiredTagNames[i], Recipe.RequiredTagValues[i]);
        }

        MaterialRequirementText->SetText(FText::FromString(
            FString::Printf(TEXT("Needs %d materials with: %s\nMin rarity: %s"),
                Recipe.MaterialCount, *TagStr, *Recipe.MinRarity)));
    }

    if (ShardCostText)
    {
        bool bCanAfford = PlayerShards >= Recipe.ShardCost;
        ShardCostText->SetText(FText::FromString(
            FString::Printf(TEXT("Cost: %lld shards"), Recipe.ShardCost)));
        ShardCostText->SetColorAndOpacity(FSlateColor(
            bCanAfford ? FLinearColor(0.3f, 1.0f, 0.3f) : FLinearColor(1.0f, 0.3f, 0.3f)));
    }

    RebuildRecipeList(); // Update highlight
}

void UCraftingWidget::UpdateMaterialSlots()
{
    if (!MaterialSlotsBox) return;
    MaterialSlotsBox->ClearChildren();

    int32 RequiredCount = 0;
    if (SelectedRecipeIndex >= 0 && SelectedRecipeIndex < AvailableRecipes.Num())
    {
        RequiredCount = AvailableRecipes[SelectedRecipeIndex].MaterialCount;
    }

    for (int32 i = 0; i < FMath::Max(RequiredCount, MaxMaterialSlots); i++)
    {
        UTextBlock* SlotText = NewObject<UTextBlock>(this);

        if (i < MaterialSlots.Num() && MaterialSlots[i].bOccupied)
        {
            SlotText->SetText(FText::FromString(
                FString::Printf(TEXT("[%d] %s (%s)"),
                    i + 1, *MaterialSlots[i].ItemName, *MaterialSlots[i].Rarity)));
            SlotText->SetColorAndOpacity(FSlateColor(GetRarityColor(MaterialSlots[i].Rarity)));
        }
        else
        {
            bool bRequired = i < RequiredCount;
            SlotText->SetText(FText::FromString(
                FString::Printf(TEXT("[%d] %s"), i + 1,
                    bRequired ? TEXT("< Empty (required) >") : TEXT("< Empty >"))));
            SlotText->SetColorAndOpacity(FSlateColor(
                bRequired ? FLinearColor(0.6f, 0.6f, 0.3f) : FLinearColor(0.4f, 0.4f, 0.4f)));
        }

        FSlateFontInfo Font = SlotText->GetFont();
        Font.Size = 11;
        SlotText->SetFont(Font);

        MaterialSlotsBox->AddChild(SlotText);
    }
}

void UCraftingWidget::UpdatePreview()
{
    if (SelectedRecipeIndex < 0 || SelectedRecipeIndex >= AvailableRecipes.Num())
    {
        CurrentPreview = FCraftResultPreview();
        if (CraftButton) CraftButton->SetIsEnabled(false);
        return;
    }

    const FCraftingRecipeData& Recipe = AvailableRecipes[SelectedRecipeIndex];

    // Count placed materials
    int32 PlacedCount = 0;
    for (const FCraftMaterialSlot& Slot : MaterialSlots)
    {
        if (Slot.bOccupied) PlacedCount++;
    }

    // Check requirements
    CurrentPreview.bCanCraft = true;
    CurrentPreview.FailReason.Empty();

    if (PlacedCount < Recipe.MaterialCount)
    {
        CurrentPreview.bCanCraft = false;
        CurrentPreview.FailReason = FString::Printf(TEXT("Need %d materials (%d placed)"),
            Recipe.MaterialCount, PlacedCount);
    }

    if (PlayerShards < Recipe.ShardCost)
    {
        CurrentPreview.bCanCraft = false;
        CurrentPreview.FailReason = FString::Printf(TEXT("Need %lld shards (have %lld)"),
            Recipe.ShardCost, PlayerShards);
    }

    // Calculate similarity
    CurrentPreview.Similarity = CalculateSimilarity();

    if (CurrentPreview.Similarity < 0.4f && PlacedCount >= Recipe.MaterialCount)
    {
        CurrentPreview.bCanCraft = false;
        CurrentPreview.FailReason = TEXT("Tag mismatch - materials don't match recipe");
    }

    // Quality from similarity
    CurrentPreview.Quality = FMath::Clamp((CurrentPreview.Similarity - 0.4f) / 0.6f, 0.0f, 1.0f);
    CurrentPreview.Category = Recipe.ResultCategory;

    // Predict rarity
    if (CurrentPreview.Quality > 0.8f)
    {
        CurrentPreview.Rarity = TEXT("Upgraded!");
    }
    else
    {
        CurrentPreview.Rarity = TEXT("Average material rarity");
    }

    // Update UI
    if (PreviewNameText)
    {
        if (CurrentPreview.bCanCraft)
        {
            PreviewNameText->SetText(FText::FromString(
                FString::Printf(TEXT("Preview: %s"), *Recipe.Name)));
            PreviewNameText->SetColorAndOpacity(FSlateColor(FLinearColor(0.3f, 1.0f, 0.5f)));
        }
        else
        {
            PreviewNameText->SetText(FText::FromString(CurrentPreview.FailReason));
            PreviewNameText->SetColorAndOpacity(FSlateColor(FLinearColor(1.0f, 0.4f, 0.3f)));
        }
    }

    if (PreviewRarityText)
    {
        PreviewRarityText->SetText(FText::FromString(CurrentPreview.Rarity));
    }

    if (QualityBar)
    {
        QualityBar->SetPercent(CurrentPreview.Quality);

        FLinearColor BarColor;
        if (CurrentPreview.Quality > 0.8f)
            BarColor = FLinearColor(1.0f, 0.84f, 0.0f); // Gold
        else if (CurrentPreview.Quality > 0.5f)
            BarColor = FLinearColor(0.3f, 1.0f, 0.3f); // Green
        else if (CurrentPreview.Quality > 0.2f)
            BarColor = FLinearColor(1.0f, 0.6f, 0.1f); // Orange
        else
            BarColor = FLinearColor(0.5f, 0.5f, 0.5f); // Gray
        QualityBar->SetFillColorAndOpacity(BarColor);
    }

    if (QualityText)
    {
        QualityText->SetText(FText::FromString(
            FString::Printf(TEXT("Quality: %.0f%%"), CurrentPreview.Quality * 100.0f)));
    }

    if (SimilarityText)
    {
        SimilarityText->SetText(FText::FromString(
            FString::Printf(TEXT("Tag Match: %.0f%%"), CurrentPreview.Similarity * 100.0f)));
    }

    if (CraftButton)
    {
        CraftButton->SetIsEnabled(CurrentPreview.bCanCraft);
    }
}

float UCraftingWidget::CalculateSimilarity() const
{
    if (SelectedRecipeIndex < 0 || SelectedRecipeIndex >= AvailableRecipes.Num()) return 0.0f;
    const FCraftingRecipeData& Recipe = AvailableRecipes[SelectedRecipeIndex];

    // Combine material tags (average)
    TMap<FString, float> CombinedTags;
    int32 OccupiedCount = 0;

    for (const FCraftMaterialSlot& Slot : MaterialSlots)
    {
        if (!Slot.bOccupied) continue;
        OccupiedCount++;

        for (int32 i = 0; i < Slot.TagNames.Num() && i < Slot.TagValues.Num(); i++)
        {
            float* Existing = CombinedTags.Find(Slot.TagNames[i]);
            if (Existing)
            {
                *Existing += Slot.TagValues[i];
            }
            else
            {
                CombinedTags.Add(Slot.TagNames[i], Slot.TagValues[i]);
            }
        }
    }

    if (OccupiedCount == 0) return 0.0f;

    // Average
    for (auto& Pair : CombinedTags)
    {
        Pair.Value /= OccupiedCount;
    }

    // Cosine similarity with recipe tags
    float DotProduct = 0.0f;
    float MagA = 0.0f;
    float MagB = 0.0f;

    for (int32 i = 0; i < Recipe.RequiredTagNames.Num() && i < Recipe.RequiredTagValues.Num(); i++)
    {
        float RecipeVal = Recipe.RequiredTagValues[i];
        float* MaterialVal = CombinedTags.Find(Recipe.RequiredTagNames[i]);
        float MatVal = MaterialVal ? *MaterialVal : 0.0f;

        DotProduct += RecipeVal * MatVal;
        MagB += RecipeVal * RecipeVal;
    }

    for (const auto& Pair : CombinedTags)
    {
        MagA += Pair.Value * Pair.Value;
    }

    MagA = FMath::Sqrt(MagA);
    MagB = FMath::Sqrt(MagB);

    if (MagA < KINDA_SMALL_NUMBER || MagB < KINDA_SMALL_NUMBER) return 0.0f;

    return FMath::Clamp(DotProduct / (MagA * MagB), 0.0f, 1.0f);
}

FLinearColor UCraftingWidget::GetCategoryColor(const FString& Category) const
{
    if (Category == TEXT("Weapon"))      return FLinearColor(1.0f, 0.4f, 0.3f);
    if (Category == TEXT("Armor"))       return FLinearColor(0.3f, 0.6f, 1.0f);
    if (Category == TEXT("Accessory"))   return FLinearColor(0.7f, 0.3f, 1.0f);
    if (Category == TEXT("Consumable"))  return FLinearColor(0.3f, 1.0f, 0.5f);
    if (Category == TEXT("Enhancement")) return FLinearColor(1.0f, 0.84f, 0.0f);
    return FLinearColor(0.7f, 0.7f, 0.7f);
}

FLinearColor UCraftingWidget::GetRarityColor(const FString& Rarity) const
{
    if (Rarity == TEXT("Common"))    return FLinearColor(0.7f, 0.7f, 0.7f);
    if (Rarity == TEXT("Uncommon"))  return FLinearColor(0.3f, 0.9f, 0.3f);
    if (Rarity == TEXT("Rare"))      return FLinearColor(0.3f, 0.5f, 1.0f);
    if (Rarity == TEXT("Epic"))      return FLinearColor(0.7f, 0.3f, 1.0f);
    if (Rarity == TEXT("Legendary")) return FLinearColor(1.0f, 0.6f, 0.1f);
    if (Rarity == TEXT("Mythic"))    return FLinearColor(1.0f, 0.2f, 0.2f);
    return FLinearColor(0.5f, 0.5f, 0.5f);
}

void UCraftingWidget::OnCraftClicked()
{
    AttemptCraft();
}

void UCraftingWidget::OnCloseClicked()
{
    Close();
}
