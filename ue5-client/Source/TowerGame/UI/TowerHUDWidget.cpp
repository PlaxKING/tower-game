#include "TowerHUDWidget.h"
#include "TowerGame/Player/TowerPlayerCharacter.h"
#include "TowerGame/Core/TowerGameState.h"
#include "Components/ProgressBar.h"
#include "Components/TextBlock.h"
#include "Kismet/GameplayStatics.h"

void UTowerHUDWidget::NativeConstruct()
{
    Super::NativeConstruct();
    RefreshHUD();
}

void UTowerHUDWidget::NativeTick(const FGeometry& MyGeometry, float InDeltaTime)
{
    Super::NativeTick(MyGeometry, InDeltaTime);
    RefreshHUD();
}

void UTowerHUDWidget::RefreshHUD()
{
    ATowerPlayerCharacter* Player = GetPlayerCharacter();

    // ---- Health ----
    if (Player && HealthBar)
    {
        float Pct = Player->MaxHp > 0.0f ? Player->CurrentHp / Player->MaxHp : 0.0f;
        HealthBar->SetPercent(Pct);

        // Color: green > yellow > red
        FLinearColor BarColor;
        if (Pct > 0.5f)
            BarColor = FLinearColor::LerpUsingHSV(FLinearColor::Yellow, FLinearColor::Green, (Pct - 0.5f) * 2.0f);
        else
            BarColor = FLinearColor::LerpUsingHSV(FLinearColor::Red, FLinearColor::Yellow, Pct * 2.0f);
        HealthBar->SetFillColorAndOpacity(BarColor);
    }
    if (Player && HealthText)
    {
        HealthText->SetText(FText::FromString(
            FString::Printf(TEXT("%.0f / %.0f"), Player->CurrentHp, Player->MaxHp)));
    }

    // ---- Resources ----
    if (Player)
    {
        if (KineticBar)
        {
            KineticBar->SetPercent(Player->KineticEnergy / 100.0f);
            KineticBar->SetFillColorAndOpacity(FLinearColor(1.0f, 0.6f, 0.1f)); // Orange
        }
        if (ThermalBar)
        {
            ThermalBar->SetPercent(Player->ThermalEnergy / 100.0f);
            ThermalBar->SetFillColorAndOpacity(FLinearColor(0.2f, 0.6f, 1.0f)); // Blue
        }
        if (SemanticBar)
        {
            SemanticBar->SetPercent(Player->SemanticEnergy / 100.0f);
            SemanticBar->SetFillColorAndOpacity(FLinearColor(0.6f, 0.2f, 0.9f)); // Purple
        }
    }

    // ---- Combo ----
    if (Player && ComboText)
    {
        if (Player->ComboStep > 0)
        {
            ComboText->SetText(FText::FromString(
                FString::Printf(TEXT("COMBO x%d"), Player->ComboStep)));
            ComboText->SetVisibility(ESlateVisibility::Visible);
        }
        else
        {
            ComboText->SetVisibility(ESlateVisibility::Hidden);
        }
    }

    // ---- World State ----
    ATowerGameState* GS = GetGameState();

    if (GS && FloorText)
    {
        FloorText->SetText(FText::FromString(
            FString::Printf(TEXT("Floor %d"), GS->ActiveFloor)));
    }

    if (GS && BreathText)
    {
        FString BreathDisplay = FString::Printf(TEXT("%s (%.0f%%)"),
            *GS->BreathPhase, GS->BreathProgress * 100.0f);
        BreathText->SetText(FText::FromString(BreathDisplay));

        // Color based on phase
        FSlateColor PhaseColor;
        if (GS->BreathPhase == TEXT("Inhale"))
            PhaseColor = FSlateColor(FLinearColor(0.3f, 0.8f, 0.3f)); // Green
        else if (GS->BreathPhase == TEXT("Hold"))
            PhaseColor = FSlateColor(FLinearColor(1.0f, 0.8f, 0.2f)); // Gold
        else if (GS->BreathPhase == TEXT("Exhale"))
            PhaseColor = FSlateColor(FLinearColor(0.8f, 0.3f, 0.3f)); // Red
        else
            PhaseColor = FSlateColor(FLinearColor(0.5f, 0.5f, 0.7f)); // Gray-blue
        BreathText->SetColorAndOpacity(PhaseColor);
    }

    if (GS && MonstersText)
    {
        if (GS->MonstersRemaining > 0)
        {
            MonstersText->SetText(FText::FromString(
                FString::Printf(TEXT("Monsters: %d"), GS->MonstersRemaining)));
        }
        else if (GS->bStairsUnlocked)
        {
            MonstersText->SetText(FText::FromString(TEXT("Stairs Unlocked!")));
        }
        else
        {
            MonstersText->SetText(FText::GetEmpty());
        }
    }
}

ATowerPlayerCharacter* UTowerHUDWidget::GetPlayerCharacter() const
{
    APlayerController* PC = GetOwningPlayer();
    if (!PC) return nullptr;
    return Cast<ATowerPlayerCharacter>(PC->GetPawn());
}

ATowerGameState* UTowerHUDWidget::GetGameState() const
{
    return GetWorld() ? GetWorld()->GetGameState<ATowerGameState>() : nullptr;
}
