#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "Components/ProgressBar.h"
#include "Components/Slider.h"
#include "Components/CheckBox.h"
#include "Components/TextBlock.h"
#include "Components/Button.h"
#include "ReplayControlWidget.generated.h"

class FProceduralCoreBridge;

/**
 * Replay playback status enumeration.
 */
UENUM(BlueprintType)
enum class EReplayState : uint8
{
    Idle = 0,          // No replay loaded
    Playing = 1,       // Playback in progress
    Paused = 2,        // Playback paused
    Finished = 3,      // Playback completed
    Error = 4          // Error state
};

/**
 * Replay control widget for playback management.
 *
 * Features:
 *   - Play/Pause/Stop/Seek controls
 *   - Speed slider (0.1x to 10x)
 *   - Loop toggle
 *   - Progress bar (current_frame_idx / total_frames)
 *   - Timeline scrubber
 *   - State display (Idle/Playing/Paused/Finished/Error)
 *   - LoadReplayFromJson() to load recording
 *   - NativeTick() to update playback
 *
 * Layout:
 *   [Replay Title]
 *   [Play] [Pause] [Stop] | Speed: [====0====] 1.0x
 *   [Loop] | State: Playing
 *   [Progress: ================>        ] 125/500 frames
 *   [Timeline Scrubber: -----+--------- ] Tick: 2500
 *
 * All FFI calls to Rust procedural core are made via ProceduralCoreBridge.
 */
UCLASS()
class TOWERGAME_API UReplayControlWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;
    virtual void NativeTick(const FGeometry& MyGeometry, float InDeltaTime) override;

    // ============ Replay Loading ============

    /**
     * Load a replay from JSON recording data.
     * Calls replay_create_playback(recording_json) on Rust side.
     *
     * @param RecordingJson  Raw recording JSON from Rust
     * @return               true if successfully loaded, false if error
     */
    UFUNCTION(BlueprintCallable, Category = "Replay")
    bool LoadReplayFromJson(const FString& RecordingJson);

    // ============ Playback Controls ============

    /** Start playback */
    UFUNCTION(BlueprintCallable, Category = "Replay")
    void Play();

    /** Pause playback */
    UFUNCTION(BlueprintCallable, Category = "Replay")
    void Pause();

    /** Stop playback and reset to frame 0 */
    UFUNCTION(BlueprintCallable, Category = "Replay")
    void Stop();

    /** Seek to specific frame index */
    UFUNCTION(BlueprintCallable, Category = "Replay")
    void SeekToFrame(int32 FrameIndex);

    /** Seek to specific tick value */
    UFUNCTION(BlueprintCallable, Category = "Replay")
    void SeekToTick(int64 Tick);

    // ============ State Queries ============

    UFUNCTION(BlueprintPure, Category = "Replay")
    EReplayState GetReplayState() const { return CurrentState; }

    UFUNCTION(BlueprintPure, Category = "Replay")
    int32 GetCurrentFrameIndex() const { return CurrentFrameIndex; }

    UFUNCTION(BlueprintPure, Category = "Replay")
    int32 GetTotalFrames() const { return TotalFrames; }

    UFUNCTION(BlueprintPure, Category = "Replay")
    float GetPlaybackSpeed() const { return PlaybackSpeed; }

    UFUNCTION(BlueprintPure, Category = "Replay")
    bool IsLooping() const { return bLoopPlayback; }

    UFUNCTION(BlueprintPure, Category = "Replay")
    FString GetStateDisplayText() const;

    // ============ Config Properties ============

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Replay")
    float MinPlaybackSpeed = 0.1f;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Replay")
    float MaxPlaybackSpeed = 10.0f;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Replay")
    float FramesPerSecond = 30.0f;

    // ============ Bound Widgets ============

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Replay")
    UTextBlock* TitleText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Replay")
    UButton* PlayButton;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Replay")
    UButton* PauseButton;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Replay")
    UButton* StopButton;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Replay")
    USlider* SpeedSlider;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Replay")
    UTextBlock* SpeedValueText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Replay")
    UCheckBox* LoopCheckBox;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Replay")
    UProgressBar* ProgressBar;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Replay")
    UTextBlock* ProgressText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Replay")
    USlider* TimelineScrubber;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Replay")
    UTextBlock* TimelineTickText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Replay")
    UTextBlock* StateText;

protected:
    // Button callbacks
    UFUNCTION()
    void OnPlayClicked();

    UFUNCTION()
    void OnPauseClicked();

    UFUNCTION()
    void OnStopClicked();

    UFUNCTION()
    void OnSpeedChanged(float Value);

    UFUNCTION()
    void OnLoopToggled(bool bIsChecked);

    UFUNCTION()
    void OnTimelineScrubberChanged(float Value);

    /** Update all UI elements from current state */
    void UpdateUI();

    /** Parse replay JSON and extract metadata */
    bool ParseReplayMetadata(const FString& ReplayJson);

    /** Get current playback status from Rust via FFI */
    bool UpdatePlaybackStatus();

    /** Format frame/tick information for display */
    FString FormatProgressText() const;
    FString FormatTickText() const;

private:
    // Playback state
    EReplayState CurrentState = EReplayState::Idle;
    FString CurrentReplayJson;
    FString PlaybackJson;  // JSON returned from replay_create_playback

    // Playback parameters
    int32 CurrentFrameIndex = 0;
    int32 TotalFrames = 0;
    int64 CurrentTick = 0;
    int64 TotalTicks = 0;
    float PlaybackSpeed = 1.0f;
    bool bLoopPlayback = false;
    bool bScrubbing = false;

    // Accumulated frame time for frame-based playback
    float AccumulatedDeltaTime = 0.0f;

    // Pointer to ProceduralCoreBridge (obtained from Game State or Singleton)
    FProceduralCoreBridge* Bridge = nullptr;

    /** Get or initialize ProceduralCoreBridge */
    FProceduralCoreBridge* GetBridge();

    /** Advance playback by one frame */
    void AdvanceFrame();
};
