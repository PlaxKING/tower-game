#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "GraphicsSettingsWidget.generated.h"

class UTextBlock;
class UButton;
class USlider;
class UComboBoxString;
class UCheckBox;

/// Graphics quality preset
UENUM(BlueprintType)
enum class ETowerGraphicsPreset : uint8
{
    Low,
    Medium,
    High,
    Ultra,
    Custom,
};

/// Full graphics settings struct
USTRUCT(BlueprintType)
struct FGraphicsSettings
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) ETowerGraphicsPreset Preset = ETowerGraphicsPreset::High;
    UPROPERTY(BlueprintReadWrite) FIntPoint Resolution = FIntPoint(1920, 1080);
    UPROPERTY(BlueprintReadWrite) bool bFullscreen = true;
    UPROPERTY(BlueprintReadWrite) bool bBorderlessWindowed = false;
    UPROPERTY(BlueprintReadWrite) int32 FPSLimit = 60;
    UPROPERTY(BlueprintReadWrite) bool bVSync = true;

    // Quality settings (0-3: Low/Med/High/Ultra)
    UPROPERTY(BlueprintReadWrite) int32 ShadowQuality = 2;
    UPROPERTY(BlueprintReadWrite) int32 TextureQuality = 2;
    UPROPERTY(BlueprintReadWrite) int32 EffectsQuality = 2;
    UPROPERTY(BlueprintReadWrite) int32 PostProcessQuality = 2;
    UPROPERTY(BlueprintReadWrite) int32 AntiAliasingQuality = 2;
    UPROPERTY(BlueprintReadWrite) int32 ViewDistance = 2;
    UPROPERTY(BlueprintReadWrite) int32 FoliageQuality = 2;

    // Anime-specific
    UPROPERTY(BlueprintReadWrite) bool bCelShading = true;
    UPROPERTY(BlueprintReadWrite) bool bAnimeBloom = true;
    UPROPERTY(BlueprintReadWrite) float RenderScale = 100.0f;  // 50-200%
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnGraphicsApplied, const FGraphicsSettings&, Settings);

/**
 * Graphics settings panel for desktop.
 * Resolution, FPS, quality presets, anime-specific rendering options.
 * From dopopensource.txt Categories 18, 21.
 */
UCLASS()
class TOWERGAME_API UGraphicsSettingsWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;

    UFUNCTION(BlueprintCallable) void ApplySettings();
    UFUNCTION(BlueprintCallable) void RevertSettings();
    UFUNCTION(BlueprintCallable) void ApplyPreset(ETowerGraphicsPreset Preset);
    UFUNCTION(BlueprintCallable) void DetectOptimalSettings();

    UFUNCTION(BlueprintPure) FGraphicsSettings GetCurrentSettings() const { return CurrentSettings; }
    UFUNCTION(BlueprintPure) FGraphicsSettings GetPendingSettings() const { return PendingSettings; }

    UPROPERTY(BlueprintAssignable) FOnGraphicsApplied OnApplied;

protected:
    // Resolution & Display
    UPROPERTY(meta = (BindWidgetOptional)) UComboBoxString* ResolutionCombo = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UComboBoxString* WindowModeCombo = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UComboBoxString* FPSLimitCombo = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UCheckBox* VSyncCheck = nullptr;

    // Quality
    UPROPERTY(meta = (BindWidgetOptional)) UComboBoxString* PresetCombo = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UComboBoxString* ShadowCombo = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UComboBoxString* TextureCombo = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UComboBoxString* EffectsCombo = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UComboBoxString* PostProcessCombo = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UComboBoxString* AACombo = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UComboBoxString* ViewDistCombo = nullptr;

    // Anime settings
    UPROPERTY(meta = (BindWidgetOptional)) UCheckBox* CelShadingCheck = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UCheckBox* AnimeBloomCheck = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) USlider* RenderScaleSlider = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* RenderScaleText = nullptr;

    // Buttons
    UPROPERTY(meta = (BindWidgetOptional)) UButton* ApplyButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* RevertButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* DetectButton = nullptr;

    // FPS counter
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* FPSCounterText = nullptr;

    FGraphicsSettings CurrentSettings;
    FGraphicsSettings PendingSettings;

    TArray<FIntPoint> AvailableResolutions;

    void PopulateResolutions();
    void PopulateQualityOptions();
    void UpdateDisplayFromSettings();
    void ReadSettingsFromUI();
    FString QualityLevelName(int32 Level) const;

    UFUNCTION() void OnApplyClicked();
    UFUNCTION() void OnRevertClicked();
    UFUNCTION() void OnDetectClicked();
    UFUNCTION() void OnPresetChanged(FString SelectedItem, ESelectInfo::Type SelectionType);
    UFUNCTION() void OnRenderScaleChanged(float Value);
};
