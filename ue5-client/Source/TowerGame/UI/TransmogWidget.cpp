#include "TransmogWidget.h"
#include "Components/ScrollBox.h"
#include "Components/TextBlock.h"
#include "Components/Button.h"
#include "Components/Image.h"
#include "Components/Border.h"
#include "Components/UniformGridPanel.h"
#include "Dom/JsonObject.h"
#include "Dom/JsonValue.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

void UTransmogWidget::NativeConstruct()
{
    Super::NativeConstruct();

    // Bind button callbacks
    if (ApplyButton)
        ApplyButton->OnClicked.AddDynamic(this, &UTransmogWidget::OnApplyClicked);
    if (RemoveButton)
        RemoveButton->OnClicked.AddDynamic(this, &UTransmogWidget::OnRemoveClicked);
    if (PreviewButton)
        PreviewButton->OnClicked.AddDynamic(this, &UTransmogWidget::OnPreviewClicked);
    if (DyePrimaryButton)
        DyePrimaryButton->OnClicked.AddDynamic(this, &UTransmogWidget::OnDyePrimaryClicked);
    if (DyeSecondaryButton)
        DyeSecondaryButton->OnClicked.AddDynamic(this, &UTransmogWidget::OnDyeSecondaryClicked);
    if (DyeAccentButton)
        DyeAccentButton->OnClicked.AddDynamic(this, &UTransmogWidget::OnDyeAccentClicked);
    if (SavePresetButton)
        SavePresetButton->OnClicked.AddDynamic(this, &UTransmogWidget::OnSavePresetClicked);
    if (LoadPresetButton)
        LoadPresetButton->OnClicked.AddDynamic(this, &UTransmogWidget::OnLoadPresetClicked);
    if (CloseButton)
        CloseButton->OnClicked.AddDynamic(this, &UTransmogWidget::OnCloseClicked);

    // Disable action buttons until a cosmetic is selected
    if (ApplyButton) ApplyButton->SetIsEnabled(false);
    if (RemoveButton) RemoveButton->SetIsEnabled(false);
    if (PreviewButton) PreviewButton->SetIsEnabled(false);

    RebuildSlotGrid();
    RebuildCosmeticList();
    RebuildDyeList();
    RebuildPresetList();
    UpdateTitleDisplay();
}

// ============================================================================
// Data Loading
// ============================================================================

void UTransmogWidget::LoadUnlockedCosmetics(const FString& CosmeticsJson)
{
    AllCosmetics.Empty();

    TSharedPtr<FJsonValue> Parsed;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(CosmeticsJson);
    if (!FJsonSerializer::Deserialize(Reader, Parsed)) return;

    const TArray<TSharedPtr<FJsonValue>>* Items = nullptr;
    // Support both root array and { "cosmetics": [...] } wrapper
    if (Parsed->Type == EJson::Array)
    {
        Items = &Parsed->AsArray();
    }
    else if (Parsed->AsObject() && Parsed->AsObject()->TryGetArrayField(TEXT("cosmetics"), Items))
    {
        // Items already set
    }
    else
    {
        return;
    }

    for (const TSharedPtr<FJsonValue>& Val : *Items)
    {
        const TSharedPtr<FJsonObject>& Obj = Val->AsObject();
        if (!Obj) continue;

        FCosmeticItemDisplay Item;
        Item.Id = Obj->GetStringField(TEXT("id"));
        Item.Name = Obj->GetStringField(TEXT("name"));
        Item.Description = Obj->HasField(TEXT("description")) ?
            Obj->GetStringField(TEXT("description")) : TEXT("");
        Item.Slot = ParseSlot(Obj->GetStringField(TEXT("slot")));
        Item.Rarity = Obj->HasField(TEXT("rarity")) ?
            Obj->GetStringField(TEXT("rarity")) : TEXT("Common");
        Item.AssetRef = Obj->HasField(TEXT("asset_ref")) ?
            Obj->GetStringField(TEXT("asset_ref")) : TEXT("");
        Item.bDyeable = Obj->HasField(TEXT("dyeable")) ?
            Obj->GetBoolField(TEXT("dyeable")) : false;
        Item.bUnlocked = Obj->HasField(TEXT("unlocked")) ?
            Obj->GetBoolField(TEXT("unlocked")) : false;

        // Build source description from source object
        const TSharedPtr<FJsonObject>* SourceObj;
        if (Obj->TryGetObjectField(TEXT("source"), SourceObj))
        {
            Item.SourceDescription = BuildSourceDescription(*SourceObj);
        }
        else if (Obj->HasField(TEXT("source_description")))
        {
            Item.SourceDescription = Obj->GetStringField(TEXT("source_description"));
        }

        AllCosmetics.Add(Item);
    }

    UE_LOG(LogTemp, Log, TEXT("TransmogWidget: Loaded %d cosmetics"), AllCosmetics.Num());
    RebuildCosmeticList();
}

void UTransmogWidget::LoadUnlockedDyes(const FString& DyesJson)
{
    AllDyes.Empty();

    TSharedPtr<FJsonValue> Parsed;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(DyesJson);
    if (!FJsonSerializer::Deserialize(Reader, Parsed)) return;

    const TArray<TSharedPtr<FJsonValue>>* Items = nullptr;
    if (Parsed->Type == EJson::Array)
    {
        Items = &Parsed->AsArray();
    }
    else if (Parsed->AsObject() && Parsed->AsObject()->TryGetArrayField(TEXT("dyes"), Items))
    {
        // Items already set
    }
    else
    {
        return;
    }

    for (const TSharedPtr<FJsonValue>& Val : *Items)
    {
        const TSharedPtr<FJsonObject>& Obj = Val->AsObject();
        if (!Obj) continue;

        FDyeDisplay Dye;
        Dye.Id = Obj->GetStringField(TEXT("id"));
        Dye.Name = Obj->GetStringField(TEXT("name"));
        Dye.bUnlocked = Obj->HasField(TEXT("unlocked")) ?
            Obj->GetBoolField(TEXT("unlocked")) : false;

        // Color can be nested object or flat fields
        const TSharedPtr<FJsonObject>* ColorObj;
        if (Obj->TryGetObjectField(TEXT("color"), ColorObj))
        {
            Dye.R = (*ColorObj)->GetNumberField(TEXT("r"));
            Dye.G = (*ColorObj)->GetNumberField(TEXT("g"));
            Dye.B = (*ColorObj)->GetNumberField(TEXT("b"));
            Dye.Metallic = (*ColorObj)->HasField(TEXT("metallic")) ?
                (*ColorObj)->GetNumberField(TEXT("metallic")) : 0.0f;
            Dye.Glossiness = (*ColorObj)->HasField(TEXT("glossiness")) ?
                (*ColorObj)->GetNumberField(TEXT("glossiness")) : 0.5f;
        }
        else
        {
            Dye.R = Obj->HasField(TEXT("r")) ? Obj->GetNumberField(TEXT("r")) : 1.0f;
            Dye.G = Obj->HasField(TEXT("g")) ? Obj->GetNumberField(TEXT("g")) : 1.0f;
            Dye.B = Obj->HasField(TEXT("b")) ? Obj->GetNumberField(TEXT("b")) : 1.0f;
            Dye.Metallic = Obj->HasField(TEXT("metallic")) ?
                Obj->GetNumberField(TEXT("metallic")) : 0.0f;
            Dye.Glossiness = Obj->HasField(TEXT("glossiness")) ?
                Obj->GetNumberField(TEXT("glossiness")) : 0.5f;
        }

        AllDyes.Add(Dye);
    }

    UE_LOG(LogTemp, Log, TEXT("TransmogWidget: Loaded %d dyes"), AllDyes.Num());
    RebuildDyeList();
}

// ============================================================================
// Slot Selection
// ============================================================================

void UTransmogWidget::SelectSlot(ECosmeticSlot SlotType)
{
    CurrentSlot = SlotType;
    SelectedCosmeticIndex = -1;

    if (SelectedSlotNameText)
    {
        SelectedSlotNameText->SetText(FText::FromString(GetSlotDisplayName(SlotType)));
    }

    RebuildCosmeticList();
    UpdateCosmeticDetail();
    UpdateDyeSwatches();
}

// ============================================================================
// Transmog Operations
// ============================================================================

void UTransmogWidget::ApplyTransmog(ECosmeticSlot SlotType, const FString& CosmeticId)
{
    // Verify cosmetic is unlocked
    const FCosmeticItemDisplay* Found = nullptr;
    for (const FCosmeticItemDisplay& Item : AllCosmetics)
    {
        if (Item.Id == CosmeticId)
        {
            Found = &Item;
            break;
        }
    }

    if (!Found || !Found->bUnlocked)
    {
        UE_LOG(LogTemp, Warning, TEXT("TransmogWidget: Cannot apply locked cosmetic '%s'"), *CosmeticId);
        return;
    }

    ActiveTransmogs.Add(SlotType, CosmeticId);
    bIsPreviewing = false;
    PreviewingCosmeticId.Empty();

    OnTransmogApplied.Broadcast(SlotType, CosmeticId);

    if (PreviewStatusText)
    {
        PreviewStatusText->SetText(FText::FromString(
            FString::Printf(TEXT("Applied: %s"), *Found->Name)));
        PreviewStatusText->SetColorAndOpacity(FSlateColor(FLinearColor(0.3f, 1.0f, 0.5f)));
    }

    RebuildSlotGrid();
    UpdateCosmeticDetail();
}

void UTransmogWidget::RemoveTransmog(ECosmeticSlot SlotType)
{
    if (ActiveTransmogs.Remove(SlotType) > 0)
    {
        if (PreviewStatusText)
        {
            PreviewStatusText->SetText(FText::FromString(
                FString::Printf(TEXT("Cleared %s override"), *GetSlotDisplayName(SlotType))));
            PreviewStatusText->SetColorAndOpacity(FSlateColor(FLinearColor(0.8f, 0.8f, 0.3f)));
        }

        RebuildSlotGrid();
        UpdateCosmeticDetail();
    }
}

void UTransmogWidget::PreviewCosmetic(const FString& CosmeticId)
{
    PreviewingCosmeticId = CosmeticId;
    bIsPreviewing = true;

    // Find cosmetic name for display
    for (const FCosmeticItemDisplay& Item : AllCosmetics)
    {
        if (Item.Id == CosmeticId)
        {
            if (PreviewStatusText)
            {
                PreviewStatusText->SetText(FText::FromString(
                    FString::Printf(TEXT("Previewing: %s"), *Item.Name)));
                PreviewStatusText->SetColorAndOpacity(FSlateColor(FLinearColor(0.5f, 0.8f, 1.0f)));
            }
            break;
        }
    }
}

void UTransmogWidget::CancelPreview()
{
    if (bIsPreviewing)
    {
        bIsPreviewing = false;
        PreviewingCosmeticId.Empty();

        if (PreviewStatusText)
        {
            PreviewStatusText->SetText(FText::FromString(TEXT("Preview cancelled")));
            PreviewStatusText->SetColorAndOpacity(FSlateColor(FLinearColor(0.6f, 0.6f, 0.6f)));
        }
    }
}

// ============================================================================
// Dye Operations
// ============================================================================

void UTransmogWidget::ApplyDye(ECosmeticSlot SlotType, EDyeChannel Channel, const FString& DyeId)
{
    // Verify dye is unlocked
    bool bDyeUnlocked = false;
    for (const FDyeDisplay& Dye : AllDyes)
    {
        if (Dye.Id == DyeId && Dye.bUnlocked)
        {
            bDyeUnlocked = true;
            break;
        }
    }

    if (!bDyeUnlocked)
    {
        UE_LOG(LogTemp, Warning, TEXT("TransmogWidget: Cannot apply locked dye '%s'"), *DyeId);
        return;
    }

    // Verify slot has a transmog and it is dyeable
    FString* ActiveId = ActiveTransmogs.Find(SlotType);
    if (!ActiveId)
    {
        UE_LOG(LogTemp, Warning, TEXT("TransmogWidget: No transmog on slot to dye"));
        return;
    }

    OnDyeApplied.Broadcast(SlotType, Channel, DyeId);
    UpdateDyeSwatches();
}

void UTransmogWidget::SelectDyeChannel(EDyeChannel Channel)
{
    CurrentDyeChannel = Channel;

    if (DyeChannelNameText)
    {
        DyeChannelNameText->SetText(FText::FromString(GetDyeChannelName(Channel)));
    }

    RebuildDyeList();
}

// ============================================================================
// Preset Management
// ============================================================================

void UTransmogWidget::SavePreset(const FString& PresetName)
{
    if (PresetName.IsEmpty()) return;

    FOutfitPreset Preset;
    Preset.Name = PresetName;

    for (const auto& Pair : ActiveTransmogs)
    {
        FTransmogOverrideEntry Entry;
        Entry.Slot = Pair.Key;
        Entry.CosmeticId = Pair.Value;
        Preset.Overrides.Add(Entry);
    }

    SavedPresets.Add(PresetName, Preset);
    OnPresetSaved.Broadcast(PresetName);

    UE_LOG(LogTemp, Log, TEXT("TransmogWidget: Saved preset '%s' with %d overrides"),
        *PresetName, Preset.Overrides.Num());

    RebuildPresetList();
}

void UTransmogWidget::LoadPreset(const FString& PresetName)
{
    FOutfitPreset* Preset = SavedPresets.Find(PresetName);
    if (!Preset)
    {
        UE_LOG(LogTemp, Warning, TEXT("TransmogWidget: Preset '%s' not found"), *PresetName);
        return;
    }

    ActiveTransmogs.Empty();

    for (const FTransmogOverrideEntry& Entry : Preset->Overrides)
    {
        ActiveTransmogs.Add(Entry.Slot, Entry.CosmeticId);
    }

    UE_LOG(LogTemp, Log, TEXT("TransmogWidget: Loaded preset '%s' with %d overrides"),
        *PresetName, Preset->Overrides.Num());

    if (PreviewStatusText)
    {
        PreviewStatusText->SetText(FText::FromString(
            FString::Printf(TEXT("Loaded preset: %s"), *PresetName)));
        PreviewStatusText->SetColorAndOpacity(FSlateColor(FLinearColor(0.3f, 1.0f, 0.8f)));
    }

    RebuildSlotGrid();
    RebuildCosmeticList();
    UpdateCosmeticDetail();
}

TArray<FString> UTransmogWidget::GetPresetNames() const
{
    TArray<FString> Names;
    SavedPresets.GetKeys(Names);
    return Names;
}

// ============================================================================
// Title & Aura
// ============================================================================

void UTransmogWidget::SetTitle(const FString& TitleId)
{
    // Verify title is unlocked
    for (const FCosmeticItemDisplay& Item : AllCosmetics)
    {
        if (Item.Id == TitleId && Item.Slot == ECosmeticSlot::Title && Item.bUnlocked)
        {
            ActiveTitleId = TitleId;
            UpdateTitleDisplay();
            OnTransmogApplied.Broadcast(ECosmeticSlot::Title, TitleId);
            return;
        }
    }

    UE_LOG(LogTemp, Warning, TEXT("TransmogWidget: Cannot set locked or invalid title '%s'"), *TitleId);
}

void UTransmogWidget::SetAura(const FString& AuraId)
{
    // Verify aura is unlocked
    for (const FCosmeticItemDisplay& Item : AllCosmetics)
    {
        if (Item.Id == AuraId && Item.Slot == ECosmeticSlot::Aura && Item.bUnlocked)
        {
            ActiveAuraId = AuraId;
            OnTransmogApplied.Broadcast(ECosmeticSlot::Aura, AuraId);
            return;
        }
    }

    UE_LOG(LogTemp, Warning, TEXT("TransmogWidget: Cannot set locked or invalid aura '%s'"), *AuraId);
}

// ============================================================================
// Queries
// ============================================================================

TArray<FCosmeticItemDisplay> UTransmogWidget::GetCosmeticsForSlot(ECosmeticSlot SlotType) const
{
    TArray<FCosmeticItemDisplay> Filtered;
    for (const FCosmeticItemDisplay& Item : AllCosmetics)
    {
        if (Item.Slot == SlotType)
        {
            Filtered.Add(Item);
        }
    }
    return Filtered;
}

TArray<FDyeDisplay> UTransmogWidget::GetUnlockedDyes() const
{
    TArray<FDyeDisplay> Unlocked;
    for (const FDyeDisplay& Dye : AllDyes)
    {
        if (Dye.bUnlocked)
        {
            Unlocked.Add(Dye);
        }
    }
    return Unlocked;
}

FString UTransmogWidget::GetActiveTransmog(ECosmeticSlot SlotType) const
{
    const FString* Found = ActiveTransmogs.Find(SlotType);
    return Found ? *Found : FString();
}

// ============================================================================
// UI Rebuild
// ============================================================================

void UTransmogWidget::RebuildSlotGrid()
{
    if (!SlotButtonGrid) return;
    SlotButtonGrid->ClearChildren();

    // All equipment slots to display in the grid
    const ECosmeticSlot Slots[] = {
        ECosmeticSlot::HeadOverride,
        ECosmeticSlot::ChestOverride,
        ECosmeticSlot::LegsOverride,
        ECosmeticSlot::BootsOverride,
        ECosmeticSlot::GlovesOverride,
        ECosmeticSlot::WeaponSkin,
        ECosmeticSlot::BackAccessory,
        ECosmeticSlot::Aura,
        ECosmeticSlot::Emote,
        ECosmeticSlot::Title,
        ECosmeticSlot::ProfileBorder,
        ECosmeticSlot::NameplateStyle,
    };

    for (int32 i = 0; i < UE_ARRAY_COUNT(Slots); i++)
    {
        ECosmeticSlot SlotType = Slots[i];

        UTextBlock* SlotText = NewObject<UTextBlock>(this);
        FString DisplayName = GetSlotDisplayName(SlotType);

        // Show active override if present
        const FString* ActiveId = ActiveTransmogs.Find(SlotType);
        if (ActiveId && !ActiveId->IsEmpty())
        {
            // Find cosmetic name for display
            for (const FCosmeticItemDisplay& Item : AllCosmetics)
            {
                if (Item.Id == *ActiveId)
                {
                    DisplayName += FString::Printf(TEXT("\n  > %s"), *Item.Name);
                    break;
                }
            }
        }

        SlotText->SetText(FText::FromString(DisplayName));

        // Highlight selected slot
        FLinearColor Color = (SlotType == CurrentSlot)
            ? FLinearColor(1.0f, 1.0f, 0.3f) // Selected = yellow
            : (ActiveId ? FLinearColor(0.3f, 1.0f, 0.5f) : FLinearColor(0.6f, 0.6f, 0.6f));
        SlotText->SetColorAndOpacity(FSlateColor(Color));

        FSlateFontInfo Font = SlotText->GetFont();
        Font.Size = (SlotType == CurrentSlot) ? 13 : 11;
        SlotText->SetFont(Font);

        SlotButtonGrid->AddChildToUniformGrid(SlotText, i / 4, i % 4);
    }
}

void UTransmogWidget::RebuildCosmeticList()
{
    if (!CosmeticListBox) return;
    CosmeticListBox->ClearChildren();

    TArray<FCosmeticItemDisplay> SlotCosmetics = GetCosmeticsForSlot(CurrentSlot);

    // Sort: unlocked first, then by rarity
    SlotCosmetics.Sort([](const FCosmeticItemDisplay& A, const FCosmeticItemDisplay& B)
    {
        if (A.bUnlocked != B.bUnlocked) return A.bUnlocked;
        // Rarity ordering: Mythic > Legendary > Epic > Rare > Uncommon > Common
        auto RarityRank = [](const FString& R) -> int32
        {
            if (R == TEXT("Mythic")) return 5;
            if (R == TEXT("Legendary")) return 4;
            if (R == TEXT("Epic")) return 3;
            if (R == TEXT("Rare")) return 2;
            if (R == TEXT("Uncommon")) return 1;
            return 0;
        };
        return RarityRank(A.Rarity) > RarityRank(B.Rarity);
    });

    for (int32 i = 0; i < SlotCosmetics.Num(); i++)
    {
        const FCosmeticItemDisplay& Item = SlotCosmetics[i];

        UTextBlock* EntryText = NewObject<UTextBlock>(this);

        FString LockIcon = Item.bUnlocked ? TEXT("") : TEXT("[Locked] ");
        FString ActiveMark = TEXT("");
        const FString* ActiveId = ActiveTransmogs.Find(CurrentSlot);
        if (ActiveId && *ActiveId == Item.Id)
        {
            ActiveMark = TEXT(" [Active]");
        }

        FString Display = FString::Printf(TEXT("%s%s (%s)%s"),
            *LockIcon, *Item.Name, *Item.Rarity, *ActiveMark);
        EntryText->SetText(FText::FromString(Display));

        // Unlocked items get rarity color, locked items are grayed out
        FLinearColor Color;
        if (Item.bUnlocked)
        {
            Color = GetRarityColor(Item.Rarity);
        }
        else
        {
            Color = FLinearColor(0.35f, 0.35f, 0.35f); // Grayed out
        }

        // Highlight selected
        if (i == SelectedCosmeticIndex)
        {
            Color = FLinearColor(1.0f, 1.0f, 0.3f);
        }

        EntryText->SetColorAndOpacity(FSlateColor(Color));

        FSlateFontInfo Font = EntryText->GetFont();
        Font.Size = 12;
        EntryText->SetFont(Font);

        CosmeticListBox->AddChild(EntryText);
    }

    // Update count display
    int32 UnlockedCount = 0;
    for (const FCosmeticItemDisplay& Item : SlotCosmetics)
    {
        if (Item.bUnlocked) UnlockedCount++;
    }

    if (CosmeticCountText)
    {
        CosmeticCountText->SetText(FText::FromString(
            FString::Printf(TEXT("%d / %d unlocked"), UnlockedCount, SlotCosmetics.Num())));
    }
}

void UTransmogWidget::UpdateCosmeticDetail()
{
    TArray<FCosmeticItemDisplay> SlotCosmetics = GetCosmeticsForSlot(CurrentSlot);

    if (!SlotCosmetics.IsValidIndex(SelectedCosmeticIndex))
    {
        if (CosmeticNameText) CosmeticNameText->SetText(FText::FromString(TEXT("Select a cosmetic")));
        if (CosmeticDescText) CosmeticDescText->SetText(FText::GetEmpty());
        if (CosmeticRarityText) CosmeticRarityText->SetText(FText::GetEmpty());
        if (CosmeticSourceText) CosmeticSourceText->SetText(FText::GetEmpty());
        if (ApplyButton) ApplyButton->SetIsEnabled(false);
        if (RemoveButton) RemoveButton->SetIsEnabled(ActiveTransmogs.Contains(CurrentSlot));
        if (PreviewButton) PreviewButton->SetIsEnabled(false);
        return;
    }

    const FCosmeticItemDisplay& Item = SlotCosmetics[SelectedCosmeticIndex];

    if (CosmeticNameText)
    {
        CosmeticNameText->SetText(FText::FromString(Item.Name));
        CosmeticNameText->SetColorAndOpacity(FSlateColor(
            Item.bUnlocked ? GetRarityColor(Item.Rarity) : FLinearColor(0.4f, 0.4f, 0.4f)));
    }

    if (CosmeticDescText)
    {
        CosmeticDescText->SetText(FText::FromString(Item.Description));
    }

    if (CosmeticRarityText)
    {
        CosmeticRarityText->SetText(FText::FromString(Item.Rarity));
        CosmeticRarityText->SetColorAndOpacity(FSlateColor(GetRarityColor(Item.Rarity)));
    }

    if (CosmeticSourceText)
    {
        FString SourceText = Item.bUnlocked
            ? TEXT("Unlocked")
            : FString::Printf(TEXT("Unlock: %s"), *Item.SourceDescription);
        CosmeticSourceText->SetText(FText::FromString(SourceText));
        CosmeticSourceText->SetColorAndOpacity(FSlateColor(
            Item.bUnlocked ? FLinearColor(0.3f, 1.0f, 0.5f) : FLinearColor(0.8f, 0.5f, 0.2f)));
    }

    // Enable Apply only for unlocked cosmetics
    if (ApplyButton)
        ApplyButton->SetIsEnabled(Item.bUnlocked);

    // Enable Remove if this slot has an active transmog
    if (RemoveButton)
        RemoveButton->SetIsEnabled(ActiveTransmogs.Contains(CurrentSlot));

    // Enable Preview for unlocked cosmetics (shows before applying)
    if (PreviewButton)
        PreviewButton->SetIsEnabled(Item.bUnlocked);
}

void UTransmogWidget::RebuildDyeList()
{
    if (!DyeListBox) return;
    DyeListBox->ClearChildren();

    for (int32 i = 0; i < AllDyes.Num(); i++)
    {
        const FDyeDisplay& Dye = AllDyes[i];

        UTextBlock* DyeText = NewObject<UTextBlock>(this);

        FString LockStr = Dye.bUnlocked ? TEXT("") : TEXT("[Locked] ");
        FString MetalStr = Dye.Metallic > 0.5f ? TEXT(" [Metallic]") : TEXT("");
        FString GlossStr = Dye.Glossiness > 0.7f ? TEXT(" [Glossy]") : TEXT("");

        DyeText->SetText(FText::FromString(
            FString::Printf(TEXT("%s%s%s%s"), *LockStr, *Dye.Name, *MetalStr, *GlossStr)));

        FLinearColor DisplayColor = Dye.bUnlocked
            ? Dye.ToLinearColor()
            : FLinearColor(0.35f, 0.35f, 0.35f);
        DyeText->SetColorAndOpacity(FSlateColor(DisplayColor));

        FSlateFontInfo Font = DyeText->GetFont();
        Font.Size = 11;
        DyeText->SetFont(Font);

        DyeListBox->AddChild(DyeText);
    }

    if (DyeChannelNameText)
    {
        DyeChannelNameText->SetText(FText::FromString(GetDyeChannelName(CurrentDyeChannel)));
    }
}

void UTransmogWidget::UpdateDyeSwatches()
{
    // Update swatch colors based on current dye channel selections
    // Swatches display a solid color of the currently applied dye for each channel
    // (Full implementation depends on tracking per-slot per-channel dye assignments)

    auto SetSwatchColor = [](UImage* Swatch, const FLinearColor& Color)
    {
        if (Swatch)
        {
            Swatch->SetColorAndOpacity(Color);
        }
    };

    // Default neutral swatches
    SetSwatchColor(DyePrimarySwatch, FLinearColor(0.5f, 0.5f, 0.5f));
    SetSwatchColor(DyeSecondarySwatch, FLinearColor(0.5f, 0.5f, 0.5f));
    SetSwatchColor(DyeAccentSwatch, FLinearColor(0.5f, 0.5f, 0.5f));
}

void UTransmogWidget::RebuildPresetList()
{
    if (!PresetListBox) return;
    PresetListBox->ClearChildren();

    for (const auto& Pair : SavedPresets)
    {
        const FOutfitPreset& Preset = Pair.Value;

        UTextBlock* PresetText = NewObject<UTextBlock>(this);
        PresetText->SetText(FText::FromString(
            FString::Printf(TEXT("%s (%d items)"), *Preset.Name, Preset.Overrides.Num())));
        PresetText->SetColorAndOpacity(FSlateColor(FLinearColor(0.7f, 0.85f, 1.0f)));

        FSlateFontInfo Font = PresetText->GetFont();
        Font.Size = 12;
        PresetText->SetFont(Font);

        PresetListBox->AddChild(PresetText);
    }
}

void UTransmogWidget::UpdateTitleDisplay()
{
    if (!ActiveTitleText) return;

    if (ActiveTitleId.IsEmpty())
    {
        ActiveTitleText->SetText(FText::FromString(TEXT("No title selected")));
        ActiveTitleText->SetColorAndOpacity(FSlateColor(FLinearColor(0.5f, 0.5f, 0.5f)));
        return;
    }

    for (const FCosmeticItemDisplay& Item : AllCosmetics)
    {
        if (Item.Id == ActiveTitleId)
        {
            ActiveTitleText->SetText(FText::FromString(Item.Name));
            ActiveTitleText->SetColorAndOpacity(FSlateColor(GetRarityColor(Item.Rarity)));
            return;
        }
    }

    ActiveTitleText->SetText(FText::FromString(ActiveTitleId));
}

void UTransmogWidget::SelectCosmeticAtIndex(int32 Index)
{
    TArray<FCosmeticItemDisplay> SlotCosmetics = GetCosmeticsForSlot(CurrentSlot);

    if (Index >= 0 && Index < SlotCosmetics.Num())
    {
        SelectedCosmeticIndex = Index;
        UpdateCosmeticDetail();
        RebuildCosmeticList(); // Refresh highlight
    }
}

// ============================================================================
// Static Helpers
// ============================================================================

FLinearColor UTransmogWidget::GetRarityColor(const FString& Rarity)
{
    if (Rarity == TEXT("Common"))    return FLinearColor(0.8f, 0.8f, 0.8f);   // White
    if (Rarity == TEXT("Uncommon"))  return FLinearColor(0.2f, 0.9f, 0.3f);   // Green
    if (Rarity == TEXT("Rare"))      return FLinearColor(0.2f, 0.4f, 1.0f);   // Blue
    if (Rarity == TEXT("Epic"))      return FLinearColor(0.7f, 0.2f, 0.9f);   // Purple
    if (Rarity == TEXT("Legendary")) return FLinearColor(1.0f, 0.65f, 0.0f);  // Gold
    if (Rarity == TEXT("Mythic"))    return FLinearColor(1.0f, 0.15f, 0.15f); // Red
    return FLinearColor(0.6f, 0.6f, 0.6f);
}

FString UTransmogWidget::GetSlotDisplayName(ECosmeticSlot SlotType)
{
    switch (SlotType)
    {
    case ECosmeticSlot::HeadOverride:   return TEXT("Head Appearance");
    case ECosmeticSlot::ChestOverride:  return TEXT("Chest Appearance");
    case ECosmeticSlot::LegsOverride:   return TEXT("Legs Appearance");
    case ECosmeticSlot::BootsOverride:  return TEXT("Boots Appearance");
    case ECosmeticSlot::GlovesOverride: return TEXT("Gloves Appearance");
    case ECosmeticSlot::WeaponSkin:     return TEXT("Weapon Skin");
    case ECosmeticSlot::BackAccessory:  return TEXT("Back Accessory");
    case ECosmeticSlot::Aura:           return TEXT("Aura Effect");
    case ECosmeticSlot::Emote:          return TEXT("Emote");
    case ECosmeticSlot::Title:          return TEXT("Title");
    case ECosmeticSlot::ProfileBorder:  return TEXT("Profile Border");
    case ECosmeticSlot::NameplateStyle: return TEXT("Nameplate");
    default:                            return TEXT("Unknown");
    }
}

FString UTransmogWidget::GetDyeChannelName(EDyeChannel Channel)
{
    switch (Channel)
    {
    case EDyeChannel::Primary:   return TEXT("Primary");
    case EDyeChannel::Secondary: return TEXT("Secondary");
    case EDyeChannel::Accent:    return TEXT("Accent");
    default:                     return TEXT("Unknown");
    }
}

ECosmeticSlot UTransmogWidget::ParseSlot(const FString& Str)
{
    if (Str == TEXT("HeadOverride"))   return ECosmeticSlot::HeadOverride;
    if (Str == TEXT("ChestOverride"))  return ECosmeticSlot::ChestOverride;
    if (Str == TEXT("LegsOverride"))   return ECosmeticSlot::LegsOverride;
    if (Str == TEXT("BootsOverride"))  return ECosmeticSlot::BootsOverride;
    if (Str == TEXT("GlovesOverride")) return ECosmeticSlot::GlovesOverride;
    if (Str == TEXT("WeaponSkin"))     return ECosmeticSlot::WeaponSkin;
    if (Str == TEXT("BackAccessory"))  return ECosmeticSlot::BackAccessory;
    if (Str == TEXT("Aura"))           return ECosmeticSlot::Aura;
    if (Str == TEXT("Emote"))          return ECosmeticSlot::Emote;
    if (Str == TEXT("Title"))          return ECosmeticSlot::Title;
    if (Str == TEXT("ProfileBorder"))  return ECosmeticSlot::ProfileBorder;
    if (Str == TEXT("NameplateStyle")) return ECosmeticSlot::NameplateStyle;
    return ECosmeticSlot::HeadOverride;
}

EDyeChannel UTransmogWidget::ParseDyeChannel(const FString& Str)
{
    if (Str == TEXT("Primary"))   return EDyeChannel::Primary;
    if (Str == TEXT("Secondary")) return EDyeChannel::Secondary;
    if (Str == TEXT("Accent"))    return EDyeChannel::Accent;
    return EDyeChannel::Primary;
}

FString UTransmogWidget::BuildSourceDescription(const TSharedPtr<FJsonObject>& SourceObj)
{
    if (!SourceObj) return TEXT("Unknown");

    if (SourceObj->HasField(TEXT("Achievement")))
        return FString::Printf(TEXT("Achievement: %s"), *SourceObj->GetStringField(TEXT("Achievement")));

    if (SourceObj->HasField(TEXT("season_id")))
        return FString::Printf(TEXT("Season Pass (S%s Lv.%d)"),
            *SourceObj->GetStringField(TEXT("season_id")),
            SourceObj->GetIntegerField(TEXT("level")));

    if (SourceObj->HasField(TEXT("price_shards")))
        return FString::Printf(TEXT("Shop: %lld Shards"),
            (int64)SourceObj->GetNumberField(TEXT("price_shards")));

    if (SourceObj->HasField(TEXT("domain")))
        return FString::Printf(TEXT("Mastery: %s (%s)"),
            *SourceObj->GetStringField(TEXT("domain")),
            *SourceObj->GetStringField(TEXT("tier")));

    if (SourceObj->HasField(TEXT("EventReward")))
        return FString::Printf(TEXT("Event: %s"), *SourceObj->GetStringField(TEXT("EventReward")));

    if (SourceObj->HasField(TEXT("QuestReward")))
        return FString::Printf(TEXT("Quest: %s"), *SourceObj->GetStringField(TEXT("QuestReward")));

    if (SourceObj->HasField(TEXT("floor_range")))
        return FString::Printf(TEXT("Drop: Floors %s (%s)"),
            *SourceObj->GetStringField(TEXT("floor_range")),
            *SourceObj->GetStringField(TEXT("rarity")));

    // Generic fallback: try "type" field
    if (SourceObj->HasField(TEXT("type")))
        return SourceObj->GetStringField(TEXT("type"));

    return TEXT("Unknown source");
}

// ============================================================================
// Button Callbacks
// ============================================================================

void UTransmogWidget::OnApplyClicked()
{
    TArray<FCosmeticItemDisplay> SlotCosmetics = GetCosmeticsForSlot(CurrentSlot);
    if (SlotCosmetics.IsValidIndex(SelectedCosmeticIndex))
    {
        const FCosmeticItemDisplay& Item = SlotCosmetics[SelectedCosmeticIndex];
        if (Item.bUnlocked)
        {
            ApplyTransmog(CurrentSlot, Item.Id);
        }
    }
}

void UTransmogWidget::OnRemoveClicked()
{
    RemoveTransmog(CurrentSlot);
}

void UTransmogWidget::OnPreviewClicked()
{
    TArray<FCosmeticItemDisplay> SlotCosmetics = GetCosmeticsForSlot(CurrentSlot);
    if (SlotCosmetics.IsValidIndex(SelectedCosmeticIndex))
    {
        const FCosmeticItemDisplay& Item = SlotCosmetics[SelectedCosmeticIndex];
        if (bIsPreviewing)
        {
            CancelPreview();
        }
        else
        {
            PreviewCosmetic(Item.Id);
        }
    }
}

void UTransmogWidget::OnDyePrimaryClicked()
{
    SelectDyeChannel(EDyeChannel::Primary);
}

void UTransmogWidget::OnDyeSecondaryClicked()
{
    SelectDyeChannel(EDyeChannel::Secondary);
}

void UTransmogWidget::OnDyeAccentClicked()
{
    SelectDyeChannel(EDyeChannel::Accent);
}

void UTransmogWidget::OnSavePresetClicked()
{
    if (PresetNameInputText)
    {
        FString Name = PresetNameInputText->GetText().ToString();
        if (!Name.IsEmpty())
        {
            SavePreset(Name);
        }
    }
}

void UTransmogWidget::OnLoadPresetClicked()
{
    if (PresetNameInputText)
    {
        FString Name = PresetNameInputText->GetText().ToString();
        if (!Name.IsEmpty())
        {
            LoadPreset(Name);
        }
    }
}

void UTransmogWidget::OnCloseClicked()
{
    CancelPreview();
    SetVisibility(ESlateVisibility::Collapsed);

    APlayerController* PC = GetOwningPlayer();
    if (PC)
    {
        PC->SetShowMouseCursor(false);
        PC->SetInputMode(FInputModeGameOnly());
    }
}
