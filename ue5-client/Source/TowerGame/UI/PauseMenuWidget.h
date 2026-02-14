#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "PauseMenuWidget.generated.h"

class UButton;
class UTextBlock;
class USlider;
class UCheckBox;

DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnResumeGame);
DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnQuitToTitle);
DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnQuitGame);

/**
 * Pause menu widget with settings.
 *
 * Layout:
 *   PAUSED (title)
 *   [Resume]
 *   [Settings]
 *     - Master Volume slider
 *     - SFX Volume slider
 *     - Music Volume slider
 *     - Mouse Sensitivity slider
 *     - Invert Y checkbox
 *     - Show Damage Numbers checkbox
 *     - Minimap Rotation checkbox
 *   [Quit to Title]
 *   [Quit Game]
 *
 * Toggle with Escape key. Pauses game and shows mouse cursor.
 */
UCLASS()
class TOWERGAME_API UPauseMenuWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;

    /** Show the pause menu and pause the game */
    UFUNCTION(BlueprintCallable, Category = "PauseMenu")
    void ShowPauseMenu();

    /** Hide the pause menu and resume */
    UFUNCTION(BlueprintCallable, Category = "PauseMenu")
    void HidePauseMenu();

    /** Toggle pause state */
    UFUNCTION(BlueprintCallable, Category = "PauseMenu")
    void TogglePause();

    UFUNCTION(BlueprintPure, Category = "PauseMenu")
    bool IsPaused() const { return bIsPaused; }

    // ============ Events ============

    UPROPERTY(BlueprintAssignable, Category = "PauseMenu")
    FOnResumeGame OnResumeGame;

    UPROPERTY(BlueprintAssignable, Category = "PauseMenu")
    FOnQuitToTitle OnQuitToTitle;

    UPROPERTY(BlueprintAssignable, Category = "PauseMenu")
    FOnQuitGame OnQuitGame;

    // ============ Bound Widgets ============

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "PauseMenu")
    UTextBlock* TitleText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "PauseMenu")
    UButton* ResumeButton;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "PauseMenu")
    UButton* SettingsButton;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "PauseMenu")
    UButton* QuitToTitleButton;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "PauseMenu")
    UButton* QuitGameButton;

    // ============ Settings Widgets ============

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "PauseMenu|Settings")
    USlider* MasterVolumeSlider;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "PauseMenu|Settings")
    USlider* SFXVolumeSlider;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "PauseMenu|Settings")
    USlider* MusicVolumeSlider;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "PauseMenu|Settings")
    USlider* MouseSensitivitySlider;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "PauseMenu|Settings")
    UCheckBox* InvertYCheckBox;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "PauseMenu|Settings")
    UCheckBox* ShowDamageNumbersCheckBox;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "PauseMenu|Settings")
    UCheckBox* MinimapRotationCheckBox;

    // ============ Settings Values ============

    UPROPERTY(BlueprintReadWrite, Category = "PauseMenu|Settings")
    float MasterVolume = 1.0f;

    UPROPERTY(BlueprintReadWrite, Category = "PauseMenu|Settings")
    float SFXVolume = 0.8f;

    UPROPERTY(BlueprintReadWrite, Category = "PauseMenu|Settings")
    float MusicVolume = 0.6f;

    UPROPERTY(BlueprintReadWrite, Category = "PauseMenu|Settings")
    float MouseSensitivity = 1.0f;

    UPROPERTY(BlueprintReadWrite, Category = "PauseMenu|Settings")
    bool bInvertY = false;

    UPROPERTY(BlueprintReadWrite, Category = "PauseMenu|Settings")
    bool bShowDamageNumbers = true;

    UPROPERTY(BlueprintReadWrite, Category = "PauseMenu|Settings")
    bool bMinimapRotation = false;

protected:
    UFUNCTION()
    void OnResumeClicked();

    UFUNCTION()
    void OnSettingsClicked();

    UFUNCTION()
    void OnQuitToTitleClicked();

    UFUNCTION()
    void OnQuitGameClicked();

    UFUNCTION()
    void OnMasterVolumeChanged(float Value);

    UFUNCTION()
    void OnSFXVolumeChanged(float Value);

    UFUNCTION()
    void OnMusicVolumeChanged(float Value);

    UFUNCTION()
    void OnMouseSensitivityChanged(float Value);

    UFUNCTION()
    void OnInvertYChanged(bool bIsChecked);

    UFUNCTION()
    void OnShowDamageNumbersChanged(bool bIsChecked);

    UFUNCTION()
    void OnMinimapRotationChanged(bool bIsChecked);

    /** Save settings to config file */
    void SaveSettings();

    /** Load settings from config file */
    void LoadSettings();

    /** Apply audio settings */
    void ApplyAudioSettings();

private:
    bool bIsPaused = false;
    bool bSettingsVisible = false;
};
