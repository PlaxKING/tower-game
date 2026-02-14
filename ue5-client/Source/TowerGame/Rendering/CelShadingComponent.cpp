#include "CelShadingComponent.h"
#include "Components/PostProcessComponent.h"
#include "Engine/Engine.h"

UCelShadingComponent::UCelShadingComponent()
{
    PrimaryComponentTick.bCanEverTick = false;
}

void UCelShadingComponent::BeginPlay()
{
    Super::BeginPlay();
    SetupPostProcess();
    ApplySettings();
}

void UCelShadingComponent::SetupPostProcess()
{
    AActor* Owner = GetOwner();
    if (!Owner) return;

    PostProcessComp = NewObject<UPostProcessComponent>(Owner, TEXT("CelShadingPP"));
    PostProcessComp->RegisterComponent();
    PostProcessComp->bUnbound = true; // Affects entire scene
    PostProcessComp->Priority = 10.0f; // High priority to override defaults

    Owner->AddInstanceComponent(PostProcessComp);

    UE_LOG(LogTemp, Log, TEXT("CelShading post-process component created"));
}

void UCelShadingComponent::ApplySettings()
{
    if (!PostProcessComp) return;

    FPostProcessSettings& PP = PostProcessComp->Settings;

    // ===== Bloom (anime glow) =====
    PP.bOverride_BloomIntensity = true;
    PP.BloomIntensity = BloomIntensity;

    PP.bOverride_BloomThreshold = true;
    PP.BloomThreshold = 0.8f;

    // ===== Color grading for anime look =====
    PP.bOverride_ColorSaturation = true;
    PP.ColorSaturation = FVector4(SaturationBoost, SaturationBoost, SaturationBoost, 1.0f);

    PP.bOverride_ColorContrast = true;
    PP.ColorContrast = FVector4(1.15f, 1.15f, 1.15f, 1.0f); // Slightly higher contrast

    PP.bOverride_ColorGamma = true;
    PP.ColorGamma = FVector4(0.95f, 0.95f, 0.95f, 1.0f); // Slightly brighter midtones

    // ===== Shadow color tint =====
    PP.bOverride_ColorGainShadows = true;
    PP.ColorGainShadows = FVector4(
        ShadowTint.R * 2.0f,
        ShadowTint.G * 2.0f,
        ShadowTint.B * 2.0f,
        1.0f
    );

    // ===== Ambient occlusion (softer for anime) =====
    PP.bOverride_AmbientOcclusionIntensity = true;
    PP.AmbientOcclusionIntensity = 0.3f;

    PP.bOverride_AmbientOcclusionRadius = true;
    PP.AmbientOcclusionRadius = 100.0f;

    // ===== Tone curve for flat shading look =====
    PP.bOverride_AutoExposureBias = true;
    PP.AutoExposureBias = 0.5f;

    PP.bOverride_AutoExposureMinBrightness = true;
    PP.AutoExposureMinBrightness = 0.5f;

    PP.bOverride_AutoExposureMaxBrightness = true;
    PP.AutoExposureMaxBrightness = 2.0f;

    UE_LOG(LogTemp, Log, TEXT("CelShading applied: %d steps, outline %.1fpx, bloom %.1f, saturation %.1f"),
        LightSteps, OutlineThickness, BloomIntensity, SaturationBoost);
}

void UCelShadingComponent::SetBreathPhaseTint(const FString& Phase)
{
    if (!bApplyBreathTint || !PostProcessComp) return;

    FPostProcessSettings& PP = PostProcessComp->Settings;
    PP.bOverride_SceneColorTint = true;

    if (Phase == TEXT("Inhale"))
    {
        // Warm golden tint — monsters spawning
        PP.SceneColorTint = FLinearColor(1.05f, 1.0f, 0.9f, 1.0f);
    }
    else if (Phase == TEXT("Hold"))
    {
        // Red alert — peak danger
        PP.SceneColorTint = FLinearColor(1.1f, 0.95f, 0.9f, 1.0f);
    }
    else if (Phase == TEXT("Exhale"))
    {
        // Cool blue — loot rain
        PP.SceneColorTint = FLinearColor(0.9f, 0.95f, 1.1f, 1.0f);
    }
    else // Pause
    {
        // Neutral — rest phase
        PP.SceneColorTint = FLinearColor(1.0f, 1.0f, 1.0f, 1.0f);
    }
}
