#include "GraphicsSettingsWidget.h"
#include "Components/TextBlock.h"
#include "Components/Button.h"
#include "Components/Slider.h"
#include "Components/ComboBoxString.h"
#include "Components/CheckBox.h"
#include "GameFramework/GameUserSettings.h"
#include "Kismet/KismetSystemLibrary.h"

void UGraphicsSettingsWidget::NativeConstruct()
{
    Super::NativeConstruct();

    // Bind buttons
    if (ApplyButton)
        ApplyButton->OnClicked.AddDynamic(this, &UGraphicsSettingsWidget::OnApplyClicked);
    if (RevertButton)
        RevertButton->OnClicked.AddDynamic(this, &UGraphicsSettingsWidget::OnRevertClicked);
    if (DetectButton)
        DetectButton->OnClicked.AddDynamic(this, &UGraphicsSettingsWidget::OnDetectClicked);

    // Bind combo box changes
    if (PresetCombo)
        PresetCombo->OnSelectionChanged.AddDynamic(this, &UGraphicsSettingsWidget::OnPresetChanged);

    // Bind render scale slider
    if (RenderScaleSlider)
        RenderScaleSlider->OnValueChanged.AddDynamic(this, &UGraphicsSettingsWidget::OnRenderScaleChanged);

    PopulateResolutions();
    PopulateQualityOptions();

    // Load current settings from engine
    UGameUserSettings* GameSettings = GEngine ? GEngine->GetGameUserSettings() : nullptr;
    if (GameSettings)
    {
        CurrentSettings.Resolution = GameSettings->GetScreenResolution();
        CurrentSettings.bFullscreen = GameSettings->GetFullscreenMode() == EWindowMode::Fullscreen;
        CurrentSettings.bVSync = GameSettings->IsVSyncEnabled();
        CurrentSettings.FPSLimit = GameSettings->GetFrameRateLimit();
    }

    PendingSettings = CurrentSettings;
    UpdateDisplayFromSettings();
}

void UGraphicsSettingsWidget::ApplySettings()
{
    ReadSettingsFromUI();
    CurrentSettings = PendingSettings;

    UGameUserSettings* GameSettings = GEngine ? GEngine->GetGameUserSettings() : nullptr;
    if (GameSettings)
    {
        GameSettings->SetScreenResolution(CurrentSettings.Resolution);
        GameSettings->SetFullscreenMode(CurrentSettings.bFullscreen ? EWindowMode::Fullscreen : EWindowMode::Windowed);
        GameSettings->SetVSyncEnabled(CurrentSettings.bVSync);
        GameSettings->SetFrameRateLimit(CurrentSettings.FPSLimit);

        // Quality scalability
        GameSettings->SetShadowQuality(CurrentSettings.ShadowQuality);
        GameSettings->SetTextureQuality(CurrentSettings.TextureQuality);
        GameSettings->SetVisualEffectQuality(CurrentSettings.EffectsQuality);
        GameSettings->SetPostProcessingQuality(CurrentSettings.PostProcessQuality);
        GameSettings->SetAntiAliasingQuality(CurrentSettings.AntiAliasingQuality);
        GameSettings->SetViewDistanceQuality(CurrentSettings.ViewDistance);
        GameSettings->SetFoliageQuality(CurrentSettings.FoliageQuality);

        GameSettings->ApplySettings(true);
        GameSettings->SaveSettings();
    }

    OnApplied.Broadcast(CurrentSettings);
    UE_LOG(LogTemp, Log, TEXT("Graphics settings applied: %dx%d, FPS=%d, VSync=%d"),
        CurrentSettings.Resolution.X, CurrentSettings.Resolution.Y,
        CurrentSettings.FPSLimit, CurrentSettings.bVSync ? 1 : 0);
}

void UGraphicsSettingsWidget::RevertSettings()
{
    PendingSettings = CurrentSettings;
    UpdateDisplayFromSettings();
}

void UGraphicsSettingsWidget::ApplyPreset(EGraphicsPreset Preset)
{
    PendingSettings.Preset = Preset;

    int32 Quality = 0;
    switch (Preset)
    {
    case EGraphicsPreset::Low:    Quality = 0; break;
    case EGraphicsPreset::Medium: Quality = 1; break;
    case EGraphicsPreset::High:   Quality = 2; break;
    case EGraphicsPreset::Ultra:  Quality = 3; break;
    default: return; // Custom: don't change
    }

    PendingSettings.ShadowQuality = Quality;
    PendingSettings.TextureQuality = Quality;
    PendingSettings.EffectsQuality = Quality;
    PendingSettings.PostProcessQuality = Quality;
    PendingSettings.AntiAliasingQuality = Quality;
    PendingSettings.ViewDistance = Quality;
    PendingSettings.FoliageQuality = Quality;
    PendingSettings.RenderScale = (Quality == 0) ? 75.0f : 100.0f;
    PendingSettings.bAnimeBloom = (Quality >= 1);

    UpdateDisplayFromSettings();
}

void UGraphicsSettingsWidget::DetectOptimalSettings()
{
    UGameUserSettings* GameSettings = GEngine ? GEngine->GetGameUserSettings() : nullptr;
    if (GameSettings)
    {
        GameSettings->RunHardwareBenchmark();
        int32 Overall = GameSettings->GetOverallScalabilityLevel();

        if (Overall <= 0) ApplyPreset(EGraphicsPreset::Low);
        else if (Overall == 1) ApplyPreset(EGraphicsPreset::Medium);
        else if (Overall == 2) ApplyPreset(EGraphicsPreset::High);
        else ApplyPreset(EGraphicsPreset::Ultra);
    }
}

void UGraphicsSettingsWidget::PopulateResolutions()
{
    AvailableResolutions.Empty();
    AvailableResolutions.Add(FIntPoint(1280, 720));
    AvailableResolutions.Add(FIntPoint(1366, 768));
    AvailableResolutions.Add(FIntPoint(1600, 900));
    AvailableResolutions.Add(FIntPoint(1920, 1080));
    AvailableResolutions.Add(FIntPoint(2560, 1440));
    AvailableResolutions.Add(FIntPoint(3840, 2160));

    if (ResolutionCombo)
    {
        ResolutionCombo->ClearOptions();
        for (const auto& Res : AvailableResolutions)
        {
            ResolutionCombo->AddOption(FString::Printf(TEXT("%dx%d"), Res.X, Res.Y));
        }
    }

    // Window mode
    if (WindowModeCombo)
    {
        WindowModeCombo->ClearOptions();
        WindowModeCombo->AddOption(TEXT("Fullscreen"));
        WindowModeCombo->AddOption(TEXT("Windowed"));
        WindowModeCombo->AddOption(TEXT("Borderless"));
    }

    // FPS limit
    if (FPSLimitCombo)
    {
        FPSLimitCombo->ClearOptions();
        FPSLimitCombo->AddOption(TEXT("30"));
        FPSLimitCombo->AddOption(TEXT("60"));
        FPSLimitCombo->AddOption(TEXT("90"));
        FPSLimitCombo->AddOption(TEXT("120"));
        FPSLimitCombo->AddOption(TEXT("144"));
        FPSLimitCombo->AddOption(TEXT("Unlimited"));
    }
}

void UGraphicsSettingsWidget::PopulateQualityOptions()
{
    TArray<UComboBoxString*> QualityCombos = {
        ShadowCombo, TextureCombo, EffectsCombo,
        PostProcessCombo, AACombo, ViewDistCombo
    };

    for (auto* Combo : QualityCombos)
    {
        if (!Combo) continue;
        Combo->ClearOptions();
        Combo->AddOption(TEXT("Low"));
        Combo->AddOption(TEXT("Medium"));
        Combo->AddOption(TEXT("High"));
        Combo->AddOption(TEXT("Ultra"));
    }

    // Preset combo
    if (PresetCombo)
    {
        PresetCombo->ClearOptions();
        PresetCombo->AddOption(TEXT("Low"));
        PresetCombo->AddOption(TEXT("Medium"));
        PresetCombo->AddOption(TEXT("High"));
        PresetCombo->AddOption(TEXT("Ultra"));
        PresetCombo->AddOption(TEXT("Custom"));
    }
}

void UGraphicsSettingsWidget::UpdateDisplayFromSettings()
{
    // Resolution
    if (ResolutionCombo)
    {
        FString ResStr = FString::Printf(TEXT("%dx%d"),
            PendingSettings.Resolution.X, PendingSettings.Resolution.Y);
        ResolutionCombo->SetSelectedOption(ResStr);
    }

    // VSync
    if (VSyncCheck)
        VSyncCheck->SetIsChecked(PendingSettings.bVSync);

    // Quality combos
    auto SetQuality = [](UComboBoxString* Combo, int32 Level) {
        if (!Combo) return;
        TArray<FString> Options = { TEXT("Low"), TEXT("Medium"), TEXT("High"), TEXT("Ultra") };
        if (Options.IsValidIndex(Level))
            Combo->SetSelectedOption(Options[Level]);
    };

    SetQuality(ShadowCombo, PendingSettings.ShadowQuality);
    SetQuality(TextureCombo, PendingSettings.TextureQuality);
    SetQuality(EffectsCombo, PendingSettings.EffectsQuality);
    SetQuality(PostProcessCombo, PendingSettings.PostProcessQuality);
    SetQuality(AACombo, PendingSettings.AntiAliasingQuality);
    SetQuality(ViewDistCombo, PendingSettings.ViewDistance);

    // Anime settings
    if (CelShadingCheck)
        CelShadingCheck->SetIsChecked(PendingSettings.bCelShading);
    if (AnimeBloomCheck)
        AnimeBloomCheck->SetIsChecked(PendingSettings.bAnimeBloom);
    if (RenderScaleSlider)
        RenderScaleSlider->SetValue(PendingSettings.RenderScale / 200.0f); // 0-200 → 0-1
    if (RenderScaleText)
        RenderScaleText->SetText(FText::FromString(
            FString::Printf(TEXT("%.0f%%"), PendingSettings.RenderScale)));
}

void UGraphicsSettingsWidget::ReadSettingsFromUI()
{
    // Read from UI into PendingSettings
    if (VSyncCheck)
        PendingSettings.bVSync = VSyncCheck->IsChecked();
    if (CelShadingCheck)
        PendingSettings.bCelShading = CelShadingCheck->IsChecked();
    if (AnimeBloomCheck)
        PendingSettings.bAnimeBloom = AnimeBloomCheck->IsChecked();

    auto ReadQuality = [](UComboBoxString* Combo) -> int32 {
        if (!Combo) return 2;
        FString Selected = Combo->GetSelectedOption();
        if (Selected == TEXT("Low")) return 0;
        if (Selected == TEXT("Medium")) return 1;
        if (Selected == TEXT("High")) return 2;
        if (Selected == TEXT("Ultra")) return 3;
        return 2;
    };

    PendingSettings.ShadowQuality = ReadQuality(ShadowCombo);
    PendingSettings.TextureQuality = ReadQuality(TextureCombo);
    PendingSettings.EffectsQuality = ReadQuality(EffectsCombo);
    PendingSettings.PostProcessQuality = ReadQuality(PostProcessCombo);
    PendingSettings.AntiAliasingQuality = ReadQuality(AACombo);
    PendingSettings.ViewDistance = ReadQuality(ViewDistCombo);
}

FString UGraphicsSettingsWidget::QualityLevelName(int32 Level) const
{
    switch (Level)
    {
    case 0: return TEXT("Low");
    case 1: return TEXT("Medium");
    case 2: return TEXT("High");
    case 3: return TEXT("Ultra");
    default: return TEXT("Custom");
    }
}

void UGraphicsSettingsWidget::OnApplyClicked()  { ApplySettings(); }
void UGraphicsSettingsWidget::OnRevertClicked() { RevertSettings(); }
void UGraphicsSettingsWidget::OnDetectClicked() { DetectOptimalSettings(); }

void UGraphicsSettingsWidget::OnPresetChanged(FString SelectedItem, ESelectInfo::Type SelectionType)
{
    if (SelectedItem == TEXT("Low"))    ApplyPreset(EGraphicsPreset::Low);
    else if (SelectedItem == TEXT("Medium")) ApplyPreset(EGraphicsPreset::Medium);
    else if (SelectedItem == TEXT("High"))   ApplyPreset(EGraphicsPreset::High);
    else if (SelectedItem == TEXT("Ultra"))  ApplyPreset(EGraphicsPreset::Ultra);
}

void UGraphicsSettingsWidget::OnRenderScaleChanged(float Value)
{
    PendingSettings.RenderScale = FMath::Clamp(Value * 200.0f, 50.0f, 200.0f);
    if (RenderScaleText)
        RenderScaleText->SetText(FText::FromString(
            FString::Printf(TEXT("%.0f%%"), PendingSettings.RenderScale)));
}
