#include "AbilityBarWidget.h"
#include "Components/HorizontalBox.h"
#include "Components/HorizontalBoxSlot.h"
#include "Components/TextBlock.h"
#include "Components/ProgressBar.h"
#include "Components/Image.h"
#include "Components/VerticalBox.h"
#include "Components/Border.h"
#include "Components/Button.h"
#include "Components/Overlay.h"
#include "Dom/JsonObject.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

// ============================================================
// FAbilityDisplayData helpers
// ============================================================

FString FAbilityDisplayData::GetTargetLabel() const
{
    switch (TargetType)
    {
    case EAbilityTarget::Melee:         return TEXT("Melee");
    case EAbilityTarget::Ranged:        return TEXT("Ranged");
    case EAbilityTarget::SelfAoE:       return TEXT("Self AoE");
    case EAbilityTarget::GroundTarget:  return TEXT("Ground Target");
    case EAbilityTarget::AllyTarget:    return TEXT("Ally Target");
    case EAbilityTarget::PartyAoE:      return TEXT("Party AoE");
    case EAbilityTarget::SelfOnly:      return TEXT("Self Only");
    default:                            return TEXT("Unknown");
    }
}

FString FAbilityDisplayData::GetIconChar() const
{
    if (IconTag.Contains(TEXT("sword")))     return TEXT("X");
    if (IconTag.Contains(TEXT("parry")))     return TEXT("P");
    if (IconTag.Contains(TEXT("staff")))     return TEXT("+");
    if (IconTag.Contains(TEXT("gauntlet")))  return TEXT("G");
    if (IconTag.Contains(TEXT("dodge")))     return TEXT(">");
    if (IconTag.Contains(TEXT("shield")))    return TEXT("O");
    if (IconTag.Contains(TEXT("heal")))      return TEXT("+");
    if (IconTag.Contains(TEXT("buff")))      return TEXT("^");
    if (IconTag.Contains(TEXT("burst")))     return TEXT("*");
    if (IconTag.Contains(TEXT("element")))   return TEXT("~");
    return TEXT("?");
}

// ============================================================
// UAbilityBarWidget
// ============================================================

void UAbilityBarWidget::NativeConstruct()
{
    Super::NativeConstruct();

    // Initialize all slot states
    for (int32 i = 0; i < ABILITY_SLOT_COUNT; i++)
    {
        SlotStates[i] = FAbilitySlotState();
    }

    HideTooltip();
    RebuildDisplay();
}

void UAbilityBarWidget::NativeTick(const FGeometry& MyGeometry, float InDeltaTime)
{
    Super::NativeTick(MyGeometry, InDeltaTime);

    UpdateCooldowns(InDeltaTime);

    // Tick flash timers and update visuals
    for (int32 i = 0; i < ABILITY_SLOT_COUNT; i++)
    {
        if (SlotStates[i].FlashTimer > 0.0f)
        {
            SlotStates[i].FlashTimer -= InDeltaTime;
            if (SlotStates[i].FlashTimer < 0.0f)
            {
                SlotStates[i].FlashTimer = 0.0f;
            }
        }

        UpdateSlotVisuals(i);
    }
}

// ============================================================
// Ability Management
// ============================================================

void UAbilityBarWidget::LoadAbilities(const FString& AbilitiesJson)
{
    KnownAbilities.Empty();

    TSharedPtr<FJsonValue> Parsed;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(AbilitiesJson);
    if (!FJsonSerializer::Deserialize(Reader, Parsed))
    {
        UE_LOG(LogTemp, Warning, TEXT("AbilityBar: Failed to parse abilities JSON"));
        return;
    }

    const TSharedPtr<FJsonObject>& Root = Parsed->AsObject();
    if (!Root)
    {
        UE_LOG(LogTemp, Warning, TEXT("AbilityBar: JSON root is not an object"));
        return;
    }

    // Parse known_abilities map
    const TSharedPtr<FJsonObject>* KnownObj = nullptr;
    if (Root->TryGetObjectField(TEXT("known_abilities"), KnownObj))
    {
        for (const auto& Pair : (*KnownObj)->Values)
        {
            const TSharedPtr<FJsonObject>& AbilityObj = Pair.Value->AsObject();
            if (!AbilityObj) continue;

            FAbilityDisplayData Data;
            Data.Id = AbilityObj->GetStringField(TEXT("id"));
            Data.Name = AbilityObj->GetStringField(TEXT("name"));
            Data.Description = AbilityObj->GetStringField(TEXT("description"));
            Data.IconTag = AbilityObj->GetStringField(TEXT("icon_tag"));
            Data.Cooldown = AbilityObj->GetNumberField(TEXT("cooldown"));
            Data.Range = AbilityObj->GetNumberField(TEXT("range"));
            Data.Radius = AbilityObj->GetNumberField(TEXT("radius"));
            Data.CastTime = AbilityObj->GetNumberField(TEXT("cast_time"));
            Data.TargetType = ParseTargetType(AbilityObj->GetStringField(TEXT("target")));

            // Parse cost sub-object
            const TSharedPtr<FJsonObject>* CostObj = nullptr;
            if (AbilityObj->TryGetObjectField(TEXT("cost"), CostObj))
            {
                Data.Cost = ParseCost(*CostObj);
            }

            Data.bIsReady = true;

            KnownAbilities.Add(Data.Id, Data);
        }
    }

    // Parse slot assignments
    const TArray<TSharedPtr<FJsonValue>>* SlotsArray = nullptr;
    if (Root->TryGetArrayField(TEXT("slots"), SlotsArray))
    {
        for (int32 i = 0; i < SlotsArray->Num() && i < ABILITY_SLOT_COUNT; i++)
        {
            const TSharedPtr<FJsonValue>& SlotVal = (*SlotsArray)[i];
            if (SlotVal->IsNull())
            {
                SlotStates[i].AbilityId = TEXT("");
            }
            else
            {
                SlotStates[i].AbilityId = SlotVal->AsString();
            }
            SlotStates[i].CooldownRemaining = 0.0f;
            SlotStates[i].CooldownTotal = 0.0f;
            SlotStates[i].FlashTimer = 0.0f;
        }
    }

    UE_LOG(LogTemp, Log, TEXT("AbilityBar: Loaded %d abilities, %d slots"),
        KnownAbilities.Num(), ABILITY_SLOT_COUNT);

    RebuildDisplay();
}

void UAbilityBarWidget::SetSlot(int32 SlotIndex, const FString& AbilityId)
{
    if (SlotIndex < 0 || SlotIndex >= ABILITY_SLOT_COUNT)
    {
        UE_LOG(LogTemp, Warning, TEXT("AbilityBar: Invalid slot index %d"), SlotIndex);
        return;
    }

    if (!KnownAbilities.Contains(AbilityId))
    {
        UE_LOG(LogTemp, Warning, TEXT("AbilityBar: Unknown ability '%s'"), *AbilityId);
        return;
    }

    // Remove from any existing slot (prevent duplicates)
    for (int32 i = 0; i < ABILITY_SLOT_COUNT; i++)
    {
        if (SlotStates[i].AbilityId == AbilityId)
        {
            SlotStates[i].AbilityId = TEXT("");
            UpdateSlotVisuals(i);
        }
    }

    SlotStates[SlotIndex].AbilityId = AbilityId;
    SlotStates[SlotIndex].CooldownRemaining = 0.0f;
    SlotStates[SlotIndex].CooldownTotal = 0.0f;
    SlotStates[SlotIndex].FlashTimer = 0.0f;

    RebuildDisplay();
}

void UAbilityBarWidget::ClearSlot(int32 SlotIndex)
{
    if (SlotIndex < 0 || SlotIndex >= ABILITY_SLOT_COUNT) return;

    SlotStates[SlotIndex].AbilityId = TEXT("");
    SlotStates[SlotIndex].CooldownRemaining = 0.0f;
    SlotStates[SlotIndex].CooldownTotal = 0.0f;
    SlotStates[SlotIndex].FlashTimer = 0.0f;

    RebuildDisplay();
}

void UAbilityBarWidget::UseAbility(int32 SlotIndex)
{
    if (SlotIndex < 0 || SlotIndex >= ABILITY_SLOT_COUNT)
    {
        OnAbilityFailed.Broadcast(SlotIndex, TEXT("Invalid slot"));
        return;
    }

    const FAbilitySlotState& State = SlotStates[SlotIndex];

    if (!State.IsOccupied())
    {
        OnAbilityFailed.Broadcast(SlotIndex, TEXT("Empty slot"));
        return;
    }

    if (State.IsOnCooldown())
    {
        OnAbilityFailed.Broadcast(SlotIndex, FString::Printf(
            TEXT("On cooldown (%.1fs)"), State.CooldownRemaining));
        return;
    }

    const FAbilityDisplayData* Data = KnownAbilities.Find(State.AbilityId);
    if (!Data)
    {
        OnAbilityFailed.Broadcast(SlotIndex, TEXT("Ability data not found"));
        return;
    }

    if (!Data->bIsReady)
    {
        OnAbilityFailed.Broadcast(SlotIndex, TEXT("Ability not ready"));
        return;
    }

    // Start cooldown
    float EffectiveCooldown = Data->Cooldown * (1.0f - CooldownReductionPercent);
    SlotStates[SlotIndex].CooldownTotal = EffectiveCooldown;
    SlotStates[SlotIndex].CooldownRemaining = EffectiveCooldown;

    // Trigger flash animation
    SlotStates[SlotIndex].FlashTimer = FlashDuration;

    // Broadcast
    OnAbilityUsed.Broadcast(SlotIndex, State.AbilityId);

    UE_LOG(LogTemp, Log, TEXT("AbilityBar: Used [%d] %s (CD: %.1fs)"),
        SlotIndex, *Data->Name, EffectiveCooldown);

    UpdateSlotVisuals(SlotIndex);
}

void UAbilityBarWidget::UpdateCooldowns(float DeltaTime)
{
    for (int32 i = 0; i < ABILITY_SLOT_COUNT; i++)
    {
        if (SlotStates[i].CooldownRemaining > 0.0f)
        {
            SlotStates[i].CooldownRemaining -= DeltaTime;
            if (SlotStates[i].CooldownRemaining < 0.0f)
            {
                SlotStates[i].CooldownRemaining = 0.0f;
            }
        }
    }
}

void UAbilityBarWidget::SetCooldownReduction(float Percent)
{
    CooldownReductionPercent = FMath::Clamp(Percent, 0.0f, 1.0f);
}

// ============================================================
// Queries
// ============================================================

bool UAbilityBarWidget::GetAbilityData(const FString& AbilityId, FAbilityDisplayData& OutData) const
{
    const FAbilityDisplayData* Found = KnownAbilities.Find(AbilityId);
    if (Found)
    {
        OutData = *Found;
        return true;
    }
    return false;
}

bool UAbilityBarWidget::GetSlotAbility(int32 SlotIndex, FAbilityDisplayData& OutData) const
{
    if (SlotIndex < 0 || SlotIndex >= ABILITY_SLOT_COUNT) return false;
    if (!SlotStates[SlotIndex].IsOccupied()) return false;

    return GetAbilityData(SlotStates[SlotIndex].AbilityId, OutData);
}

bool UAbilityBarWidget::IsSlotOnCooldown(int32 SlotIndex) const
{
    if (SlotIndex < 0 || SlotIndex >= ABILITY_SLOT_COUNT) return false;
    return SlotStates[SlotIndex].IsOnCooldown();
}

float UAbilityBarWidget::GetSlotCooldownRemaining(int32 SlotIndex) const
{
    if (SlotIndex < 0 || SlotIndex >= ABILITY_SLOT_COUNT) return 0.0f;
    return SlotStates[SlotIndex].CooldownRemaining;
}

// ============================================================
// Visual Updates
// ============================================================

void UAbilityBarWidget::RebuildDisplay()
{
    for (int32 i = 0; i < ABILITY_SLOT_COUNT; i++)
    {
        UTextBlock* KeyLabel = nullptr;
        UTextBlock* IconText = nullptr;
        UTextBlock* CooldownText = nullptr;
        UProgressBar* CooldownBar = nullptr;
        UBorder* Border = nullptr;

        GetSlotWidgets(i, KeyLabel, IconText, CooldownText, CooldownBar, Border);

        // Set keybind label (1-6)
        if (KeyLabel)
        {
            KeyLabel->SetText(FText::FromString(FString::Printf(TEXT("%d"), i + 1)));

            FSlateFontInfo Font = KeyLabel->GetFont();
            Font.Size = 9;
            KeyLabel->SetFont(Font);
            KeyLabel->SetColorAndOpacity(FSlateColor(FLinearColor(0.6f, 0.6f, 0.6f)));
        }

        // Set icon / ability name
        if (IconText)
        {
            if (SlotStates[i].IsOccupied())
            {
                const FAbilityDisplayData* Data = KnownAbilities.Find(SlotStates[i].AbilityId);
                if (Data)
                {
                    IconText->SetText(FText::FromString(Data->GetIconChar()));
                    FSlateFontInfo Font = IconText->GetFont();
                    Font.Size = 18;
                    IconText->SetFont(Font);
                }
            }
            else
            {
                IconText->SetText(FText::FromString(TEXT("-")));
                FSlateFontInfo Font = IconText->GetFont();
                Font.Size = 18;
                IconText->SetFont(Font);
                IconText->SetColorAndOpacity(FSlateColor(FLinearColor(0.4f, 0.4f, 0.4f)));
            }
        }

        // Initialize cooldown display
        if (CooldownText)
        {
            CooldownText->SetText(FText::GetEmpty());
            CooldownText->SetVisibility(ESlateVisibility::Hidden);
        }

        if (CooldownBar)
        {
            CooldownBar->SetPercent(0.0f);
        }

        UpdateSlotVisuals(i);
    }
}

void UAbilityBarWidget::UpdateSlotVisuals(int32 SlotIndex)
{
    if (SlotIndex < 0 || SlotIndex >= ABILITY_SLOT_COUNT) return;

    UTextBlock* KeyLabel = nullptr;
    UTextBlock* IconText = nullptr;
    UTextBlock* CooldownText = nullptr;
    UProgressBar* CooldownBar = nullptr;
    UBorder* Border = nullptr;

    GetSlotWidgets(SlotIndex, KeyLabel, IconText, CooldownText, CooldownBar, Border);

    const FAbilitySlotState& State = SlotStates[SlotIndex];

    // --- Cooldown overlay ---
    if (CooldownText)
    {
        if (State.IsOnCooldown())
        {
            int32 SecsRemaining = FMath::CeilToInt(State.CooldownRemaining);
            CooldownText->SetText(FText::FromString(FString::Printf(TEXT("%ds"), SecsRemaining)));
            CooldownText->SetVisibility(ESlateVisibility::HitTestInvisible);
            CooldownText->SetColorAndOpacity(FSlateColor(FLinearColor(1.0f, 0.8f, 0.2f)));
        }
        else
        {
            CooldownText->SetText(FText::GetEmpty());
            CooldownText->SetVisibility(ESlateVisibility::Hidden);
        }
    }

    // --- Cooldown progress bar (sweeps from 1.0 to 0.0 as cooldown expires) ---
    if (CooldownBar)
    {
        CooldownBar->SetPercent(State.GetCooldownProgress());

        if (State.IsOnCooldown())
        {
            CooldownBar->SetVisibility(ESlateVisibility::HitTestInvisible);
            CooldownBar->SetFillColorAndOpacity(FLinearColor(0.2f, 0.2f, 0.2f, 0.6f));
        }
        else
        {
            CooldownBar->SetVisibility(ESlateVisibility::Hidden);
        }
    }

    // --- Tint (gray on cooldown, flash on use, normal otherwise) ---
    if (IconText)
    {
        if (State.IsFlashing())
        {
            float Alpha = State.FlashTimer / FlashDuration;
            FLinearColor Lerped = FMath::Lerp(ReadyTint, FlashColor, Alpha);
            IconText->SetColorAndOpacity(FSlateColor(Lerped));
        }
        else if (State.IsOnCooldown())
        {
            IconText->SetColorAndOpacity(FSlateColor(CooldownTint));
        }
        else if (State.IsOccupied())
        {
            IconText->SetColorAndOpacity(FSlateColor(ReadyTint));
        }
    }

    // --- Border tint ---
    if (Border)
    {
        if (State.IsFlashing())
        {
            float Alpha = State.FlashTimer / FlashDuration;
            FLinearColor BorderFlash = FMath::Lerp(
                FLinearColor(0.15f, 0.15f, 0.15f, 1.0f),
                FLinearColor(1.0f, 1.0f, 1.0f, 1.0f),
                Alpha);
            Border->SetBrushColor(BorderFlash);
        }
        else if (State.IsOnCooldown())
        {
            Border->SetBrushColor(FLinearColor(0.1f, 0.1f, 0.1f, 1.0f));
        }
        else if (State.IsOccupied())
        {
            Border->SetBrushColor(FLinearColor(0.15f, 0.15f, 0.15f, 1.0f));
        }
        else
        {
            Border->SetBrushColor(FLinearColor(0.08f, 0.08f, 0.08f, 0.5f));
        }
    }
}

void UAbilityBarWidget::ShowTooltip(int32 SlotIndex)
{
    if (SlotIndex < 0 || SlotIndex >= ABILITY_SLOT_COUNT) return;

    FAbilityDisplayData Data;
    if (!GetSlotAbility(SlotIndex, Data))
    {
        HideTooltip();
        return;
    }

    HoveredSlot = SlotIndex;

    if (TooltipBox)
    {
        TooltipBox->SetVisibility(ESlateVisibility::HitTestInvisible);
    }

    if (TooltipNameText)
    {
        TooltipNameText->SetText(FText::FromString(Data.Name));
        TooltipNameText->SetColorAndOpacity(FSlateColor(FLinearColor(1.0f, 0.85f, 0.3f)));
    }

    if (TooltipDescText)
    {
        TooltipDescText->SetText(FText::FromString(Data.Description));
    }

    if (TooltipCostText)
    {
        FString CostStr;
        if (Data.Cost.Kinetic > 0.0f)
            CostStr += FString::Printf(TEXT("KIN: %.0f  "), Data.Cost.Kinetic);
        if (Data.Cost.Thermal > 0.0f)
            CostStr += FString::Printf(TEXT("THR: %.0f  "), Data.Cost.Thermal);
        if (Data.Cost.Semantic > 0.0f)
            CostStr += FString::Printf(TEXT("SEM: %.0f  "), Data.Cost.Semantic);
        if (CostStr.IsEmpty())
            CostStr = TEXT("Free");

        TooltipCostText->SetText(FText::FromString(CostStr));
    }

    if (TooltipTargetText)
    {
        TooltipTargetText->SetText(FText::FromString(
            FString::Printf(TEXT("Target: %s"), *Data.GetTargetLabel())));
    }

    if (TooltipRangeText)
    {
        FString RangeStr;
        if (Data.Range > 0.0f)
            RangeStr += FString::Printf(TEXT("Range: %.0f  "), Data.Range);
        if (Data.Radius > 0.0f)
            RangeStr += FString::Printf(TEXT("Radius: %.0f"), Data.Radius);
        if (RangeStr.IsEmpty())
            RangeStr = TEXT("Melee range");

        TooltipRangeText->SetText(FText::FromString(RangeStr));
    }

    if (TooltipCooldownText)
    {
        TooltipCooldownText->SetText(FText::FromString(
            FString::Printf(TEXT("Cooldown: %.1fs"), Data.Cooldown)));
    }

    if (TooltipCastTimeText)
    {
        FString CastStr = Data.CastTime > 0.0f
            ? FString::Printf(TEXT("Cast: %.1fs"), Data.CastTime)
            : TEXT("Instant");
        TooltipCastTimeText->SetText(FText::FromString(CastStr));
    }
}

void UAbilityBarWidget::HideTooltip()
{
    HoveredSlot = -1;
    if (TooltipBox)
    {
        TooltipBox->SetVisibility(ESlateVisibility::Collapsed);
    }
}

// ============================================================
// Widget Lookup
// ============================================================

void UAbilityBarWidget::GetSlotWidgets(int32 SlotIndex, UTextBlock*& OutKeyLabel,
    UTextBlock*& OutIconText, UTextBlock*& OutCooldownText,
    UProgressBar*& OutCooldownBar, UBorder*& OutBorder) const
{
    OutKeyLabel = nullptr;
    OutIconText = nullptr;
    OutCooldownText = nullptr;
    OutCooldownBar = nullptr;
    OutBorder = nullptr;

    switch (SlotIndex)
    {
    case 0:
        OutKeyLabel = Slot0_KeyLabel;
        OutIconText = Slot0_IconText;
        OutCooldownText = Slot0_CooldownText;
        OutCooldownBar = Slot0_CooldownBar;
        OutBorder = Slot0_Border;
        break;
    case 1:
        OutKeyLabel = Slot1_KeyLabel;
        OutIconText = Slot1_IconText;
        OutCooldownText = Slot1_CooldownText;
        OutCooldownBar = Slot1_CooldownBar;
        OutBorder = Slot1_Border;
        break;
    case 2:
        OutKeyLabel = Slot2_KeyLabel;
        OutIconText = Slot2_IconText;
        OutCooldownText = Slot2_CooldownText;
        OutCooldownBar = Slot2_CooldownBar;
        OutBorder = Slot2_Border;
        break;
    case 3:
        OutKeyLabel = Slot3_KeyLabel;
        OutIconText = Slot3_IconText;
        OutCooldownText = Slot3_CooldownText;
        OutCooldownBar = Slot3_CooldownBar;
        OutBorder = Slot3_Border;
        break;
    case 4:
        OutKeyLabel = Slot4_KeyLabel;
        OutIconText = Slot4_IconText;
        OutCooldownText = Slot4_CooldownText;
        OutCooldownBar = Slot4_CooldownBar;
        OutBorder = Slot4_Border;
        break;
    case 5:
        OutKeyLabel = Slot5_KeyLabel;
        OutIconText = Slot5_IconText;
        OutCooldownText = Slot5_CooldownText;
        OutCooldownBar = Slot5_CooldownBar;
        OutBorder = Slot5_Border;
        break;
    default:
        break;
    }
}

// ============================================================
// JSON Parsing Helpers
// ============================================================

EAbilityTarget UAbilityBarWidget::ParseTargetType(const FString& Str) const
{
    if (Str == TEXT("Melee"))           return EAbilityTarget::Melee;
    if (Str == TEXT("Ranged"))          return EAbilityTarget::Ranged;
    if (Str == TEXT("SelfAoE"))         return EAbilityTarget::SelfAoE;
    if (Str == TEXT("GroundTarget"))    return EAbilityTarget::GroundTarget;
    if (Str == TEXT("AllyTarget"))      return EAbilityTarget::AllyTarget;
    if (Str == TEXT("PartyAoE"))        return EAbilityTarget::PartyAoE;
    if (Str == TEXT("SelfOnly"))        return EAbilityTarget::SelfOnly;
    return EAbilityTarget::Melee;
}

FAbilityCost UAbilityBarWidget::ParseCost(const TSharedPtr<FJsonObject>& CostObj) const
{
    FAbilityCost Cost;
    if (!CostObj) return Cost;

    Cost.Kinetic = CostObj->GetNumberField(TEXT("kinetic"));
    Cost.Thermal = CostObj->GetNumberField(TEXT("thermal"));
    Cost.Semantic = CostObj->GetNumberField(TEXT("semantic"));

    return Cost;
}
