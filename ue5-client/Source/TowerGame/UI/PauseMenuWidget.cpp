#include "PauseMenuWidget.h"
#include "Components/Button.h"
#include "Components/TextBlock.h"
#include "Components/Slider.h"
#include "Components/CheckBox.h"
#include "Kismet/GameplayStatics.h"
#include "GameFramework/GameUserSettings.h"
#include "GameFramework/PlayerController.h"

void UPauseMenuWidget::NativeConstruct()
{
    Super::NativeConstruct();

    // Load saved settings
    LoadSettings();

    // Bind buttons
    if (ResumeButton)
    {
        ResumeButton->OnClicked.AddDynamic(this, &UPauseMenuWidget::OnResumeClicked);
    }
    if (SettingsButton)
    {
        SettingsButton->OnClicked.AddDynamic(this, &UPauseMenuWidget::OnSettingsClicked);
    }
    if (QuitToTitleButton)
    {
        QuitToTitleButton->OnClicked.AddDynamic(this, &UPauseMenuWidget::OnQuitToTitleClicked);
    }
    if (QuitGameButton)
    {
        QuitGameButton->OnClicked.AddDynamic(this, &UPauseMenuWidget::OnQuitGameClicked);
    }

    // Bind sliders
    if (MasterVolumeSlider)
    {
        MasterVolumeSlider->SetValue(MasterVolume);
        MasterVolumeSlider->OnValueChanged.AddDynamic(this, &UPauseMenuWidget::OnMasterVolumeChanged);
    }
    if (SFXVolumeSlider)
    {
        SFXVolumeSlider->SetValue(SFXVolume);
        SFXVolumeSlider->OnValueChanged.AddDynamic(this, &UPauseMenuWidget::OnSFXVolumeChanged);
    }
    if (MusicVolumeSlider)
    {
        MusicVolumeSlider->SetValue(MusicVolume);
        MusicVolumeSlider->OnValueChanged.AddDynamic(this, &UPauseMenuWidget::OnMusicVolumeChanged);
    }
    if (MouseSensitivitySlider)
    {
        MouseSensitivitySlider->SetValue(MouseSensitivity);
        MouseSensitivitySlider->OnValueChanged.AddDynamic(this, &UPauseMenuWidget::OnMouseSensitivityChanged);
    }

    // Bind checkboxes
    if (InvertYCheckBox)
    {
        InvertYCheckBox->SetIsChecked(bInvertY);
        InvertYCheckBox->OnCheckStateChanged.AddDynamic(this, &UPauseMenuWidget::OnInvertYChanged);
    }
    if (ShowDamageNumbersCheckBox)
    {
        ShowDamageNumbersCheckBox->SetIsChecked(bShowDamageNumbers);
        ShowDamageNumbersCheckBox->OnCheckStateChanged.AddDynamic(this, &UPauseMenuWidget::OnShowDamageNumbersChanged);
    }
    if (MinimapRotationCheckBox)
    {
        MinimapRotationCheckBox->SetIsChecked(bMinimapRotation);
        MinimapRotationCheckBox->OnCheckStateChanged.AddDynamic(this, &UPauseMenuWidget::OnMinimapRotationChanged);
    }

    // Start hidden
    SetVisibility(ESlateVisibility::Collapsed);
}

void UPauseMenuWidget::ShowPauseMenu()
{
    bIsPaused = true;
    SetVisibility(ESlateVisibility::Visible);

    APlayerController* PC = GetOwningPlayer();
    if (PC)
    {
        PC->SetPause(true);
        PC->SetShowMouseCursor(true);
        PC->SetInputMode(FInputModeUIOnly());
    }
}

void UPauseMenuWidget::HidePauseMenu()
{
    bIsPaused = false;
    SetVisibility(ESlateVisibility::Collapsed);

    APlayerController* PC = GetOwningPlayer();
    if (PC)
    {
        PC->SetPause(false);
        PC->SetShowMouseCursor(false);
        PC->SetInputMode(FInputModeGameOnly());
    }

    OnResumeGame.Broadcast();
}

void UPauseMenuWidget::TogglePause()
{
    if (bIsPaused)
    {
        HidePauseMenu();
    }
    else
    {
        ShowPauseMenu();
    }
}

void UPauseMenuWidget::OnResumeClicked()
{
    HidePauseMenu();
}

void UPauseMenuWidget::OnSettingsClicked()
{
    bSettingsVisible = !bSettingsVisible;

    // Toggle visibility of settings widgets
    ESlateVisibility SettingsVis = bSettingsVisible ?
        ESlateVisibility::Visible : ESlateVisibility::Collapsed;

    if (MasterVolumeSlider) MasterVolumeSlider->SetVisibility(SettingsVis);
    if (SFXVolumeSlider) SFXVolumeSlider->SetVisibility(SettingsVis);
    if (MusicVolumeSlider) MusicVolumeSlider->SetVisibility(SettingsVis);
    if (MouseSensitivitySlider) MouseSensitivitySlider->SetVisibility(SettingsVis);
    if (InvertYCheckBox) InvertYCheckBox->SetVisibility(SettingsVis);
    if (ShowDamageNumbersCheckBox) ShowDamageNumbersCheckBox->SetVisibility(SettingsVis);
    if (MinimapRotationCheckBox) MinimapRotationCheckBox->SetVisibility(SettingsVis);
}

void UPauseMenuWidget::OnQuitToTitleClicked()
{
    HidePauseMenu();
    OnQuitToTitle.Broadcast();
}

void UPauseMenuWidget::OnQuitGameClicked()
{
    SaveSettings();
    OnQuitGame.Broadcast();

    APlayerController* PC = GetOwningPlayer();
    if (PC)
    {
        UKismetSystemLibrary::QuitGame(GetWorld(), PC, EQuitPreference::Quit, false);
    }
}

void UPauseMenuWidget::OnMasterVolumeChanged(float Value)
{
    MasterVolume = Value;
    ApplyAudioSettings();
    SaveSettings();
}

void UPauseMenuWidget::OnSFXVolumeChanged(float Value)
{
    SFXVolume = Value;
    ApplyAudioSettings();
    SaveSettings();
}

void UPauseMenuWidget::OnMusicVolumeChanged(float Value)
{
    MusicVolume = Value;
    ApplyAudioSettings();
    SaveSettings();
}

void UPauseMenuWidget::OnMouseSensitivityChanged(float Value)
{
    MouseSensitivity = FMath::Clamp(Value, 0.1f, 3.0f);
    SaveSettings();
}

void UPauseMenuWidget::OnInvertYChanged(bool bIsChecked)
{
    bInvertY = bIsChecked;
    SaveSettings();
}

void UPauseMenuWidget::OnShowDamageNumbersChanged(bool bIsChecked)
{
    bShowDamageNumbers = bIsChecked;
    SaveSettings();
}

void UPauseMenuWidget::OnMinimapRotationChanged(bool bIsChecked)
{
    bMinimapRotation = bIsChecked;
    SaveSettings();
}

void UPauseMenuWidget::SaveSettings()
{
    GConfig->SetFloat(TEXT("TowerGame.Settings"), TEXT("MasterVolume"), MasterVolume, GGameIni);
    GConfig->SetFloat(TEXT("TowerGame.Settings"), TEXT("SFXVolume"), SFXVolume, GGameIni);
    GConfig->SetFloat(TEXT("TowerGame.Settings"), TEXT("MusicVolume"), MusicVolume, GGameIni);
    GConfig->SetFloat(TEXT("TowerGame.Settings"), TEXT("MouseSensitivity"), MouseSensitivity, GGameIni);
    GConfig->SetBool(TEXT("TowerGame.Settings"), TEXT("InvertY"), bInvertY, GGameIni);
    GConfig->SetBool(TEXT("TowerGame.Settings"), TEXT("ShowDamageNumbers"), bShowDamageNumbers, GGameIni);
    GConfig->SetBool(TEXT("TowerGame.Settings"), TEXT("MinimapRotation"), bMinimapRotation, GGameIni);
    GConfig->Flush(false, GGameIni);
}

void UPauseMenuWidget::LoadSettings()
{
    GConfig->GetFloat(TEXT("TowerGame.Settings"), TEXT("MasterVolume"), MasterVolume, GGameIni);
    GConfig->GetFloat(TEXT("TowerGame.Settings"), TEXT("SFXVolume"), SFXVolume, GGameIni);
    GConfig->GetFloat(TEXT("TowerGame.Settings"), TEXT("MusicVolume"), MusicVolume, GGameIni);
    GConfig->GetFloat(TEXT("TowerGame.Settings"), TEXT("MouseSensitivity"), MouseSensitivity, GGameIni);
    GConfig->GetBool(TEXT("TowerGame.Settings"), TEXT("InvertY"), bInvertY, GGameIni);
    GConfig->GetBool(TEXT("TowerGame.Settings"), TEXT("ShowDamageNumbers"), bShowDamageNumbers, GGameIni);
    GConfig->GetBool(TEXT("TowerGame.Settings"), TEXT("MinimapRotation"), bMinimapRotation, GGameIni);

    ApplyAudioSettings();
}

void UPauseMenuWidget::ApplyAudioSettings()
{
    // Audio volumes are applied through Sound Classes/Mix in UE5
    // The actual implementation depends on the project's sound setup
    UE_LOG(LogTemp, Log, TEXT("Audio settings applied: Master=%.2f, SFX=%.2f, Music=%.2f"),
        MasterVolume, SFXVolume, MusicVolume);
}
