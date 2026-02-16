#include "ReplayControlWidget.h"
#include "Components/Button.h"
#include "Components/TextBlock.h"
#include "Components/Slider.h"
#include "Components/CheckBox.h"
#include "Components/ProgressBar.h"
#include "Bridge/ProceduralCoreBridge.h"
#include "Kismet/GameplayStatics.h"
#include "Json.h"
#include "JsonUtilities.h"

void UReplayControlWidget::NativeConstruct()
{
    Super::NativeConstruct();

    // Initialize bridge reference
    GetBridge();

    // Bind play/pause/stop buttons
    if (PlayButton)
    {
        PlayButton->OnClicked.AddDynamic(this, &UReplayControlWidget::OnPlayClicked);
    }
    if (PauseButton)
    {
        PauseButton->OnClicked.AddDynamic(this, &UReplayControlWidget::OnPauseClicked);
    }
    if (StopButton)
    {
        StopButton->OnClicked.AddDynamic(this, &UReplayControlWidget::OnStopClicked);
    }

    // Bind speed slider (0.1x to 10x, displayed as log scale 0-1)
    if (SpeedSlider)
    {
        SpeedSlider->SetMinValue(0.0f);
        SpeedSlider->SetMaxValue(1.0f);
        SpeedSlider->SetValue(0.5f); // Default to 1.0x (midpoint)
        SpeedSlider->OnValueChanged.AddDynamic(this, &UReplayControlWidget::OnSpeedChanged);
    }

    // Bind loop checkbox
    if (LoopCheckBox)
    {
        LoopCheckBox->SetIsChecked(false);
        LoopCheckBox->OnCheckStateChanged.AddDynamic(this, &UReplayControlWidget::OnLoopToggled);
    }

    // Bind timeline scrubber
    if (TimelineScrubber)
    {
        TimelineScrubber->SetMinValue(0.0f);
        TimelineScrubber->SetMaxValue(1.0f);
        TimelineScrubber->SetValue(0.0f);
        TimelineScrubber->OnValueChanged.AddDynamic(this, &UReplayControlWidget::OnTimelineScrubberChanged);
    }

    // Bind progress bar
    if (ProgressBar)
    {
        ProgressBar->SetPercent(0.0f);
    }

    // Update UI with initial state
    UpdateUI();

    UE_LOG(LogTemp, Warning, TEXT("ReplayControlWidget constructed"));
}

void UReplayControlWidget::NativeTick(const FGeometry& MyGeometry, float InDeltaTime)
{
    Super::NativeTick(MyGeometry, InDeltaTime);

    // Only update playback if playing
    if (CurrentState != EReplayState::Playing)
    {
        return;
    }

    // Accumulate delta time and advance frames
    AccumulatedDeltaTime += InDeltaTime * PlaybackSpeed;
    float FrameDuration = 1.0f / FramesPerSecond;

    while (AccumulatedDeltaTime >= FrameDuration)
    {
        AccumulatedDeltaTime -= FrameDuration;
        AdvanceFrame();
    }

    // Update UI display
    UpdateUI();

    // Check if we've finished playback
    if (CurrentFrameIndex >= TotalFrames)
    {
        if (bLoopPlayback)
        {
            SeekToFrame(0);
            Play();
        }
        else
        {
            CurrentState = EReplayState::Finished;
        }
    }
}

bool UReplayControlWidget::LoadReplayFromJson(const FString& RecordingJson)
{
    if (RecordingJson.IsEmpty())
    {
        UE_LOG(LogTemp, Error, TEXT("ReplayControlWidget: Recording JSON is empty"));
        CurrentState = EReplayState::Error;
        return false;
    }

    // Store the raw replay JSON
    CurrentReplayJson = RecordingJson;

    // Try to parse metadata
    if (!ParseReplayMetadata(RecordingJson))
    {
        UE_LOG(LogTemp, Error, TEXT("ReplayControlWidget: Failed to parse replay metadata"));
        CurrentState = EReplayState::Error;
        return false;
    }

    // Call replay_create_playback on Rust side
    if (Bridge)
    {
        // Note: In a real implementation, we'd need to expose replay_create_playback
        // For now, we'll store the JSON and note that this would call the bridge
        PlaybackJson = RecordingJson;
        UE_LOG(LogTemp, Warning, TEXT("ReplayControlWidget: Loaded replay with %d frames"), TotalFrames);
    }

    CurrentState = EReplayState::Idle;
    CurrentFrameIndex = 0;
    AccumulatedDeltaTime = 0.0f;

    UpdateUI();
    return true;
}

void UReplayControlWidget::Play()
{
    if (CurrentReplayJson.IsEmpty())
    {
        UE_LOG(LogTemp, Warning, TEXT("ReplayControlWidget: Cannot play - no replay loaded"));
        return;
    }

    if (CurrentState == EReplayState::Finished && !bLoopPlayback)
    {
        SeekToFrame(0);
    }

    CurrentState = EReplayState::Playing;
    AccumulatedDeltaTime = 0.0f;
    UE_LOG(LogTemp, Warning, TEXT("ReplayControlWidget: Play started"));
    UpdateUI();
}

void UReplayControlWidget::Pause()
{
    if (CurrentState == EReplayState::Playing)
    {
        CurrentState = EReplayState::Paused;
        UE_LOG(LogTemp, Warning, TEXT("ReplayControlWidget: Paused at frame %d"), CurrentFrameIndex);
        UpdateUI();
    }
}

void UReplayControlWidget::Stop()
{
    CurrentState = EReplayState::Idle;
    CurrentFrameIndex = 0;
    CurrentTick = 0;
    AccumulatedDeltaTime = 0.0f;
    UE_LOG(LogTemp, Warning, TEXT("ReplayControlWidget: Stopped"));
    UpdateUI();
}

void UReplayControlWidget::SeekToFrame(int32 FrameIndex)
{
    if (FrameIndex >= TotalFrames)
    {
        CurrentFrameIndex = TotalFrames - 1;
    }
    else
    {
        CurrentFrameIndex = FrameIndex;
    }

    // Update tick based on frame index
    if (TotalFrames > 0 && TotalTicks > 0)
    {
        CurrentTick = (uint64)((float)CurrentFrameIndex / (float)TotalFrames * (float)TotalTicks);
    }

    AccumulatedDeltaTime = 0.0f;
    UE_LOG(LogTemp, Warning, TEXT("ReplayControlWidget: Seeked to frame %d (tick %llu)"),
           CurrentFrameIndex, CurrentTick);
    UpdateUI();
}

void UReplayControlWidget::SeekToTick(int64 Tick)
{
    if (TotalTicks > 0)
    {
        uint32 NewFrame = (uint32)((float)Tick / (float)TotalTicks * (float)TotalFrames);
        SeekToFrame(NewFrame);
    }
}

FString UReplayControlWidget::GetStateDisplayText() const
{
    switch (CurrentState)
    {
        case EReplayState::Idle:
            return TEXT("Idle");
        case EReplayState::Playing:
            return TEXT("Playing");
        case EReplayState::Paused:
            return TEXT("Paused");
        case EReplayState::Finished:
            return TEXT("Finished");
        case EReplayState::Error:
            return TEXT("Error");
        default:
            return TEXT("Unknown");
    }
}

void UReplayControlWidget::OnPlayClicked()
{
    Play();
}

void UReplayControlWidget::OnPauseClicked()
{
    Pause();
}

void UReplayControlWidget::OnStopClicked()
{
    Stop();
}

void UReplayControlWidget::OnSpeedChanged(float Value)
{
    // Convert slider value (0-1) to playback speed (0.1x - 10x) using exponential scale
    // log scale: 0 = 0.1x, 0.5 = 1.0x, 1.0 = 10x
    float LogValue = Value * 2.0f - 1.0f; // Convert 0-1 to -1 to 1
    PlaybackSpeed = FMath::Pow(10.0f, LogValue); // 10^-1 = 0.1x, 10^0 = 1x, 10^1 = 10x

    // Clamp to valid range
    PlaybackSpeed = FMath::Clamp(PlaybackSpeed, MinPlaybackSpeed, MaxPlaybackSpeed);

    if (SpeedValueText)
    {
        SpeedValueText->SetText(FText::FromString(FString::Printf(TEXT("%.1fx"), PlaybackSpeed)));
    }

    UE_LOG(LogTemp, Warning, TEXT("ReplayControlWidget: Speed changed to %.2fx"), PlaybackSpeed);
}

void UReplayControlWidget::OnLoopToggled(bool bIsChecked)
{
    bLoopPlayback = bIsChecked;
    UE_LOG(LogTemp, Warning, TEXT("ReplayControlWidget: Loop %s"), bLoopPlayback ? TEXT("enabled") : TEXT("disabled"));
}

void UReplayControlWidget::OnTimelineScrubberChanged(float Value)
{
    if (!bScrubbing)
    {
        bScrubbing = true;
    }

    // Convert normalized scrubber position (0-1) to frame index
    uint32 NewFrame = (uint32)(Value * TotalFrames);
    SeekToFrame(NewFrame);

    bScrubbing = false;
}

void UReplayControlWidget::UpdateUI()
{
    // Update progress bar
    if (ProgressBar && TotalFrames > 0)
    {
        float Progress = (float)CurrentFrameIndex / (float)TotalFrames;
        ProgressBar->SetPercent(FMath::Clamp(Progress, 0.0f, 1.0f));
    }

    // Update progress text (frame count)
    if (ProgressText)
    {
        ProgressText->SetText(FText::FromString(FormatProgressText()));
    }

    // Update timeline scrubber
    if (TimelineScrubber && TotalFrames > 0)
    {
        float ScrubberPosition = (float)CurrentFrameIndex / (float)TotalFrames;
        TimelineScrubber->SetValue(FMath::Clamp(ScrubberPosition, 0.0f, 1.0f));
    }

    // Update timeline tick text
    if (TimelineTickText)
    {
        TimelineTickText->SetText(FText::FromString(FormatTickText()));
    }

    // Update state text
    if (StateText)
    {
        StateText->SetText(FText::FromString(FString::Printf(TEXT("State: %s"), *GetStateDisplayText())));
    }

    // Update button states
    if (PlayButton)
    {
        PlayButton->SetIsEnabled(CurrentState != EReplayState::Playing && !CurrentReplayJson.IsEmpty());
    }
    if (PauseButton)
    {
        PauseButton->SetIsEnabled(CurrentState == EReplayState::Playing);
    }
    if (StopButton)
    {
        StopButton->SetIsEnabled(CurrentState == EReplayState::Playing || CurrentState == EReplayState::Paused);
    }
}

bool UReplayControlWidget::ParseReplayMetadata(const FString& ReplayJson)
{
    // Parse JSON to extract replay metadata
    TSharedPtr<FJsonObject> JsonObject;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(ReplayJson);

    if (!FJsonSerializer::Deserialize(Reader, JsonObject) || !JsonObject.IsValid())
    {
        UE_LOG(LogTemp, Error, TEXT("ReplayControlWidget: Failed to parse replay JSON"));
        return false;
    }

    // Extract frame count
    if (JsonObject->HasField(TEXT("total_frames")))
    {
        TotalFrames = JsonObject->GetIntegerField(TEXT("total_frames"));
    }
    else if (JsonObject->HasField(TEXT("frames")))
    {
        // Try to get length of frames array as fallback
        TArray<TSharedPtr<FJsonValue>> Frames = JsonObject->GetArrayField(TEXT("frames"));
        TotalFrames = Frames.Num();
    }
    else
    {
        UE_LOG(LogTemp, Warning, TEXT("ReplayControlWidget: Could not determine total frame count"));
        TotalFrames = 100; // Default fallback
    }

    // Extract tick count
    if (JsonObject->HasField(TEXT("total_ticks")))
    {
        TotalTicks = JsonObject->GetIntegerField(TEXT("total_ticks"));
    }
    else
    {
        TotalTicks = TotalFrames * 100; // Estimate based on frames
    }

    UE_LOG(LogTemp, Warning, TEXT("ReplayControlWidget: Parsed metadata - %d frames, %llu ticks"),
           TotalFrames, TotalTicks);

    return true;
}

bool UReplayControlWidget::UpdatePlaybackStatus()
{
    if (!Bridge || PlaybackJson.IsEmpty())
    {
        return false;
    }

    // In a real implementation, would call:
    // FString StatusJson = Bridge->ReplayGetSnapshot();
    // And parse current_frame_idx, current_tick, playback_state from response

    // For now, we manage state locally
    return true;
}

FString UReplayControlWidget::FormatProgressText() const
{
    return FString::Printf(TEXT("%d / %d frames"), CurrentFrameIndex, TotalFrames);
}

FString UReplayControlWidget::FormatTickText() const
{
    return FString::Printf(TEXT("Tick: %llu / %llu"), CurrentTick, TotalTicks);
}

void UReplayControlWidget::AdvanceFrame()
{
    if (CurrentFrameIndex < TotalFrames)
    {
        CurrentFrameIndex++;

        // Update tick based on frame progression
        if (TotalFrames > 0 && TotalTicks > 0)
        {
            CurrentTick = (uint64)((float)CurrentFrameIndex / (float)TotalFrames * (float)TotalTicks);
        }

        // In a real implementation, would call:
        // FString InputEvent = Bridge->ReplayGetInput(CurrentFrameIndex);
        // And apply the input event to the game state
    }
}

FProceduralCoreBridge* UReplayControlWidget::GetBridge()
{
    if (Bridge != nullptr)
    {
        return Bridge;
    }

    // Try to get bridge from game state or singleton
    // For now, we'll attempt to get it through the world's game state
    UWorld* World = GetWorld();
    if (World)
    {
        // In a production implementation, would fetch from GameState or a Singleton subsystem
        // For this prototype, Bridge is obtained when needed
        UE_LOG(LogTemp, Warning, TEXT("ReplayControlWidget: Bridge access deferred to runtime"));
    }

    return Bridge;
}
