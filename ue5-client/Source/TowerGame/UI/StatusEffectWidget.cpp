#include "StatusEffectWidget.h"
#include "Components/HorizontalBox.h"
#include "Components/HorizontalBoxSlot.h"
#include "Components/TextBlock.h"
#include "Components/ProgressBar.h"
#include "Components/Image.h"
#include "Components/VerticalBox.h"
#include "Components/Border.h"

FLinearColor FActiveStatusEffect::GetColor() const
{
    switch (Type)
    {
    // DoT - Red/Orange family
    case EStatusType::Burning:      return FLinearColor(1.0f, 0.4f, 0.1f);  // Orange
    case EStatusType::Poisoned:     return FLinearColor(0.4f, 0.8f, 0.1f);  // Yellow-green
    case EStatusType::Bleeding:     return FLinearColor(0.8f, 0.1f, 0.1f);  // Dark red

    // CC - Blue/White family
    case EStatusType::Stunned:      return FLinearColor(1.0f, 1.0f, 0.3f);  // Yellow
    case EStatusType::Frozen:       return FLinearColor(0.5f, 0.8f, 1.0f);  // Ice blue
    case EStatusType::Silenced:     return FLinearColor(0.6f, 0.3f, 0.8f);  // Purple

    // Debuffs - Dark tones
    case EStatusType::Weakened:     return FLinearColor(0.5f, 0.3f, 0.3f);  // Dull red
    case EStatusType::Slowed:       return FLinearColor(0.3f, 0.3f, 0.5f);  // Dull blue
    case EStatusType::Exposed:      return FLinearColor(0.8f, 0.5f, 0.2f);  // Amber
    case EStatusType::Corrupted:    return FLinearColor(0.2f, 0.0f, 0.2f);  // Deep purple

    // Buffs - Bright tones
    case EStatusType::Empowered:    return FLinearColor(1.0f, 0.2f, 0.2f);  // Bright red
    case EStatusType::Hastened:     return FLinearColor(0.2f, 1.0f, 0.4f);  // Bright green
    case EStatusType::Shielded:     return FLinearColor(0.9f, 0.9f, 0.3f);  // Gold
    case EStatusType::Regenerating: return FLinearColor(0.3f, 1.0f, 0.3f);  // Green
    case EStatusType::SemanticFocus:return FLinearColor(0.3f, 0.6f, 1.0f);  // Bright blue

    default: return FLinearColor::White;
    }
}

FString FActiveStatusEffect::GetDisplayName() const
{
    switch (Type)
    {
    case EStatusType::Burning:      return TEXT("BRN");
    case EStatusType::Poisoned:     return TEXT("PSN");
    case EStatusType::Bleeding:     return TEXT("BLD");
    case EStatusType::Stunned:      return TEXT("STN");
    case EStatusType::Frozen:       return TEXT("FRZ");
    case EStatusType::Silenced:     return TEXT("SIL");
    case EStatusType::Weakened:     return TEXT("WKN");
    case EStatusType::Slowed:       return TEXT("SLW");
    case EStatusType::Exposed:      return TEXT("EXP");
    case EStatusType::Corrupted:    return TEXT("CRP");
    case EStatusType::Empowered:    return TEXT("EMP");
    case EStatusType::Hastened:     return TEXT("HST");
    case EStatusType::Shielded:     return TEXT("SHD");
    case EStatusType::Regenerating: return TEXT("RGN");
    case EStatusType::SemanticFocus:return TEXT("SEM");
    default: return TEXT("???");
    }
}

FString FActiveStatusEffect::GetIconChar() const
{
    switch (Type)
    {
    case EStatusType::Burning:      return TEXT("*");
    case EStatusType::Poisoned:     return TEXT("~");
    case EStatusType::Bleeding:     return TEXT("!");
    case EStatusType::Stunned:      return TEXT("#");
    case EStatusType::Frozen:       return TEXT("@");
    case EStatusType::Silenced:     return TEXT("X");
    case EStatusType::Weakened:     return TEXT("-");
    case EStatusType::Slowed:       return TEXT("v");
    case EStatusType::Exposed:      return TEXT("o");
    case EStatusType::Corrupted:    return TEXT("%");
    case EStatusType::Empowered:    return TEXT("+");
    case EStatusType::Hastened:     return TEXT(">");
    case EStatusType::Shielded:     return TEXT("O");
    case EStatusType::Regenerating: return TEXT("+");
    case EStatusType::SemanticFocus:return TEXT("^");
    default: return TEXT("?");
    }
}

void UStatusEffectWidget::NativeConstruct()
{
    Super::NativeConstruct();
    RebuildDisplay();
}

void UStatusEffectWidget::NativeTick(const FGeometry& MyGeometry, float InDeltaTime)
{
    Super::NativeTick(MyGeometry, InDeltaTime);

    // Tick down all effects
    bool bNeedsRebuild = false;
    for (int32 i = ActiveEffects.Num() - 1; i >= 0; i--)
    {
        ActiveEffects[i].RemainingTime -= InDeltaTime;
        if (ActiveEffects[i].RemainingTime <= 0.0f)
        {
            ActiveEffects.RemoveAt(i);
            bNeedsRebuild = true;
        }
    }

    if (bNeedsRebuild)
    {
        RebuildDisplay();
    }
}

void UStatusEffectWidget::AddEffect(EStatusType Type, float Duration, float Strength, int32 Stacks)
{
    // Check if already exists — refresh/stack
    for (FActiveStatusEffect& Existing : ActiveEffects)
    {
        if (Existing.Type == Type)
        {
            Existing.RemainingTime = FMath::Max(Existing.RemainingTime, Duration);
            Existing.TotalDuration = Duration;
            Existing.Strength = FMath::Max(Existing.Strength, Strength);
            Existing.Stacks = FMath::Min(Existing.Stacks + Stacks, 5); // Max 5 stacks
            RebuildDisplay();
            return;
        }
    }

    // New effect
    if (ActiveEffects.Num() >= MaxDisplayedEffects) return;

    FActiveStatusEffect NewEffect;
    NewEffect.Type = Type;
    NewEffect.RemainingTime = Duration;
    NewEffect.TotalDuration = Duration;
    NewEffect.Strength = Strength;
    NewEffect.Stacks = Stacks;

    ActiveEffects.Add(NewEffect);
    RebuildDisplay();
}

void UStatusEffectWidget::RemoveEffect(EStatusType Type)
{
    ActiveEffects.RemoveAll([Type](const FActiveStatusEffect& E) { return E.Type == Type; });
    RebuildDisplay();
}

void UStatusEffectWidget::ClearAllEffects()
{
    ActiveEffects.Empty();
    RebuildDisplay();
}

bool UStatusEffectWidget::HasEffect(EStatusType Type) const
{
    return ActiveEffects.ContainsByPredicate([Type](const FActiveStatusEffect& E) {
        return E.Type == Type;
    });
}

void UStatusEffectWidget::RebuildDisplay()
{
    if (BuffBox) BuffBox->ClearChildren();
    if (DebuffBox) DebuffBox->ClearChildren();

    for (const FActiveStatusEffect& Effect : ActiveEffects)
    {
        UHorizontalBox* TargetBox = Effect.IsBuff() ? BuffBox : DebuffBox;
        if (!TargetBox) continue;

        // Create a vertical box for each effect: icon + timer
        UVerticalBox* EffectBox = NewObject<UVerticalBox>(this);

        // Effect label (abbreviation + stacks)
        UTextBlock* Label = NewObject<UTextBlock>(this);
        FString LabelText = Effect.GetDisplayName();
        if (Effect.Stacks > 1)
        {
            LabelText += FString::Printf(TEXT("x%d"), Effect.Stacks);
        }
        Label->SetText(FText::FromString(LabelText));
        Label->SetColorAndOpacity(FSlateColor(Effect.GetColor()));

        FSlateFontInfo Font = Label->GetFont();
        Font.Size = 10;
        Label->SetFont(Font);

        EffectBox->AddChild(Label);

        // Timer text
        UTextBlock* Timer = NewObject<UTextBlock>(this);
        int32 SecsRemaining = FMath::CeilToInt(Effect.RemainingTime);
        Timer->SetText(FText::FromString(FString::Printf(TEXT("%ds"), SecsRemaining)));

        FSlateFontInfo SmallFont = Timer->GetFont();
        SmallFont.Size = 8;
        Timer->SetFont(SmallFont);
        Timer->SetColorAndOpacity(FSlateColor(FLinearColor(0.7f, 0.7f, 0.7f)));

        EffectBox->AddChild(Timer);

        TargetBox->AddChild(EffectBox);
    }
}
