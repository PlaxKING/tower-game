#include "DamageNumberComponent.h"
#include "Engine/Engine.h"
#include "Kismet/GameplayStatics.h"
#include "GameFramework/PlayerController.h"

UDamageNumberComponent::UDamageNumberComponent()
{
    PrimaryComponentTick.bCanEverTick = true;
}

void UDamageNumberComponent::TickComponent(float DeltaTime, ELevelTick TickType, FActorComponentTickFunction* ThisTickFunction)
{
    Super::TickComponent(DeltaTime, TickType, ThisTickFunction);

    AActor* Owner = GetOwner();
    if (!Owner) return;

    APlayerController* PC = UGameplayStatics::GetPlayerController(this, 0);
    if (!PC) return;

    // Update and render active numbers
    for (int32 i = ActiveNumbers.Num() - 1; i >= 0; --i)
    {
        FFloatingNumber& Num = ActiveNumbers[i];
        Num.TimeRemaining -= DeltaTime;

        if (Num.TimeRemaining <= 0.0f)
        {
            ActiveNumbers.RemoveAt(i);
            continue;
        }

        // Calculate progress (0 = just spawned, 1 = about to disappear)
        float Progress = 1.0f - (Num.TimeRemaining / Num.TotalTime);

        // Rise upward
        Num.WorldOffset.Z = FloatHeight * Progress;

        // Add slight horizontal drift for stacking
        float Drift = FMath::Sin(Progress * PI) * 20.0f * (i % 3 - 1);
        FVector WorldPos = Owner->GetActorLocation() + Num.WorldOffset + FVector(Drift, 0, 0);

        // Project to screen
        FVector2D ScreenPos;
        if (PC->ProjectWorldLocationToScreen(WorldPos, ScreenPos))
        {
            // Fade out in last 30%
            float Alpha = (Progress > 0.7f) ? (1.0f - Progress) / 0.3f : 1.0f;

            // Scale: start big, settle to normal, shrink at end
            float DisplayScale = Num.Scale;
            if (Progress < 0.1f)
            {
                DisplayScale *= 1.0f + (1.0f - Progress / 0.1f) * 0.5f; // Pop in
            }
            else if (Progress > 0.8f)
            {
                DisplayScale *= (1.0f - Progress) / 0.2f; // Shrink out
            }

            // Draw on HUD (uses debug string; in production use UMG widget)
            FColor DrawColor = Num.Color.ToFColor(true);
            DrawColor.A = static_cast<uint8>(Alpha * 255);

            GEngine->AddOnScreenDebugMessage(
                -1, 0.0f, DrawColor,
                Num.Text,
                true,
                FVector2D(DisplayScale, DisplayScale)
            );
        }
    }
}

void UDamageNumberComponent::ShowDamage(float Amount, bool bIsCrit, bool bIsHealing)
{
    FString Text;
    FLinearColor Color;
    float Scale = 1.0f;

    if (bIsHealing)
    {
        Text = FString::Printf(TEXT("+%.0f"), Amount);
        Color = HealColor;
    }
    else
    {
        Text = FString::Printf(TEXT("%.0f"), Amount);
        Color = bIsCrit ? CritColor : DamageColor;
        if (bIsCrit)
        {
            Scale = CritScale;
            Text += TEXT("!");
        }
    }

    SpawnNumber(Text, Color, Scale);
}

void UDamageNumberComponent::ShowStatusText(const FString& Text, FLinearColor Color)
{
    SpawnNumber(Text, Color, 1.2f);
}

void UDamageNumberComponent::SpawnNumber(const FString& Text, FLinearColor Color, float Scale)
{
    // Remove oldest if at max
    if (ActiveNumbers.Num() >= MaxNumbers)
    {
        ActiveNumbers.RemoveAt(0);
    }

    FFloatingNumber Num;
    Num.Text = Text;
    Num.Color = Color;
    Num.Scale = Scale;
    Num.WorldOffset = FVector(FMath::RandRange(-10.0f, 10.0f), 0, 0);
    Num.TimeRemaining = FloatDuration;
    Num.TotalTime = FloatDuration;

    ActiveNumbers.Add(Num);
}
