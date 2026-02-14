#include "DeathScreenWidget.h"
#include "Components/TextBlock.h"
#include "Components/Button.h"
#include "Components/ProgressBar.h"

void UDeathScreenWidget::NativeConstruct()
{
    Super::NativeConstruct();

    if (RespawnButton)
    {
        RespawnButton->OnClicked.AddDynamic(this, &UDeathScreenWidget::OnRespawnClicked);
        RespawnButton->SetIsEnabled(false);
    }
    if (LobbyButton)
    {
        LobbyButton->OnClicked.AddDynamic(this, &UDeathScreenWidget::OnLobbyClicked);
    }

    SetVisibility(ESlateVisibility::Collapsed);
}

void UDeathScreenWidget::NativeTick(const FGeometry& MyGeometry, float InDeltaTime)
{
    Super::NativeTick(MyGeometry, InDeltaTime);

    if (!bShowing) return;

    // Fade in effect
    if (FadeInTimer < 1.0f)
    {
        FadeInTimer += InDeltaTime * 0.5f; // 2 second fade
        SetRenderOpacity(FMath::Clamp(FadeInTimer, 0.0f, 1.0f));
    }

    // Cooldown timer
    if (CooldownTimer > 0.0f)
    {
        CooldownTimer -= InDeltaTime;

        if (RespawnCooldownBar)
        {
            float Progress = 1.0f - (CooldownTimer / RespawnCooldown);
            RespawnCooldownBar->SetPercent(FMath::Clamp(Progress, 0.0f, 1.0f));
        }

        if (CooldownTimer <= 0.0f)
        {
            CooldownTimer = 0.0f;
            if (RespawnButton)
            {
                RespawnButton->SetIsEnabled(true);
            }
        }
    }
}

void UDeathScreenWidget::ShowDeathScreen(int32 FloorReached, int32 MonstersSlain,
    float TimeSurvived, const FString& EchoType)
{
    bShowing = true;
    CooldownTimer = RespawnCooldown;
    FadeInTimer = 0.0f;

    SetVisibility(ESlateVisibility::Visible);
    SetRenderOpacity(0.0f); // Start transparent

    if (RespawnButton)
    {
        RespawnButton->SetIsEnabled(false);
    }

    // Title
    if (TitleText)
    {
        TitleText->SetText(FText::FromString(TEXT("YOU DIED")));
        TitleText->SetColorAndOpacity(FSlateColor(FLinearColor(0.8f, 0.1f, 0.1f)));
    }

    // Flavor text
    if (FlavorText)
    {
        FString Flavor = FString::Printf(
            TEXT("Your echo lingers on floor %d..."), FloorReached);
        FlavorText->SetText(FText::FromString(Flavor));
    }

    // Echo type
    if (EchoTypeText)
    {
        FString EchoDisplay;
        FLinearColor EchoColor;

        if (EchoType == TEXT("lingering"))
        {
            EchoDisplay = TEXT("Echo Type: Lingering (Blue)");
            EchoColor = FLinearColor(0.3f, 0.5f, 1.0f);
        }
        else if (EchoType == TEXT("aggressive"))
        {
            EchoDisplay = TEXT("Echo Type: Aggressive (Red)");
            EchoColor = FLinearColor(1.0f, 0.2f, 0.2f);
        }
        else if (EchoType == TEXT("helpful"))
        {
            EchoDisplay = TEXT("Echo Type: Helpful (Green)");
            EchoColor = FLinearColor(0.2f, 1.0f, 0.3f);
        }
        else
        {
            EchoDisplay = TEXT("Echo Type: Warning (Orange)");
            EchoColor = FLinearColor(1.0f, 0.6f, 0.1f);
        }

        EchoTypeText->SetText(FText::FromString(EchoDisplay));
        EchoTypeText->SetColorAndOpacity(FSlateColor(EchoColor));
    }

    // Stats
    if (FloorText)
    {
        FloorText->SetText(FText::FromString(
            FString::Printf(TEXT("Floor reached: %d"), FloorReached)));
    }
    if (MonstersText)
    {
        MonstersText->SetText(FText::FromString(
            FString::Printf(TEXT("Monsters slain: %d"), MonstersSlain)));
    }
    if (TimeText)
    {
        int32 Mins = FMath::FloorToInt(TimeSurvived / 60.0f);
        int32 Secs = FMath::FloorToInt(FMath::Fmod(TimeSurvived, 60.0f));
        TimeText->SetText(FText::FromString(
            FString::Printf(TEXT("Time survived: %02d:%02d"), Mins, Secs)));
    }

    // Show mouse cursor
    APlayerController* PC = GetOwningPlayer();
    if (PC)
    {
        PC->SetShowMouseCursor(true);
        PC->SetInputMode(FInputModeUIOnly());
    }
}

void UDeathScreenWidget::HideDeathScreen()
{
    bShowing = false;
    SetVisibility(ESlateVisibility::Collapsed);

    APlayerController* PC = GetOwningPlayer();
    if (PC)
    {
        PC->SetShowMouseCursor(false);
        PC->SetInputMode(FInputModeGameOnly());
    }
}

void UDeathScreenWidget::OnRespawnClicked()
{
    HideDeathScreen();
    OnRespawnRequested.Broadcast();
}

void UDeathScreenWidget::OnLobbyClicked()
{
    HideDeathScreen();
    OnReturnToLobby.Broadcast();
}
