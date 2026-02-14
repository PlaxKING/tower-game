#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "WorldEventWidget.generated.h"

class UTextBlock;
class UProgressBar;
class UVerticalBox;

/// Event severity matching Rust EventSeverity
UENUM(BlueprintType)
enum class EEventSeverity : uint8
{
    Minor,
    Moderate,
    Major,
    Critical,
};

/// Event trigger type matching Rust EventTriggerType
UENUM(BlueprintType)
enum class EEventTriggerType : uint8
{
    BreathShift,
    SemanticResonance,
    EchoConvergence,
    FloorAnomaly,
    FactionClash,
    CorruptionSurge,
    TowerMemory,
};

/// Active world event display data
USTRUCT(BlueprintType)
struct FWorldEventDisplay
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString Name;
    UPROPERTY(BlueprintReadWrite) FString Description;
    UPROPERTY(BlueprintReadWrite) EEventTriggerType TriggerType = EEventTriggerType::BreathShift;
    UPROPERTY(BlueprintReadWrite) EEventSeverity Severity = EEventSeverity::Minor;
    UPROPERTY(BlueprintReadWrite) float Duration = 30.0f;
    UPROPERTY(BlueprintReadWrite) float RemainingTime = 30.0f;
    UPROPERTY(BlueprintReadWrite) float FlashTimer = 0.0f;
};

/**
 * Displays active procedural world events.
 * Shows event name, description, severity indicator, and remaining duration.
 * Matches Rust events module (7 trigger types, 4 severities).
 */
UCLASS()
class TOWERGAME_API UWorldEventWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;
    virtual void NativeTick(const FGeometry& MyGeometry, float InDeltaTime) override;

    // --- API ---
    UFUNCTION(BlueprintCallable) void ShowEvent(const FString& Name, const FString& Description,
        EEventTriggerType TriggerType, EEventSeverity Severity, float Duration);
    UFUNCTION(BlueprintCallable) void ShowEventFromJson(const FString& EventJson);
    UFUNCTION(BlueprintCallable) void RemoveEvent(const FString& Name);
    UFUNCTION(BlueprintCallable) void ClearAll();

protected:
    UPROPERTY(meta = (BindWidgetOptional)) UVerticalBox* EventListBox = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* ActiveEventTitle = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* ActiveEventDesc = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UProgressBar* ActiveEventTimer = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* SeverityText = nullptr;

    UPROPERTY(EditDefaultsOnly) int32 MaxDisplayEvents = 3;
    UPROPERTY(EditDefaultsOnly) float IntroFlashDuration = 2.0f;

    TArray<FWorldEventDisplay> ActiveEvents;

    void RebuildDisplay();
    FLinearColor GetTriggerColor(EEventTriggerType Type) const;
    FLinearColor GetSeverityColor(EEventSeverity Severity) const;
    FString GetTriggerIcon(EEventTriggerType Type) const;
    FString GetSeverityLabel(EEventSeverity Severity) const;

    EEventTriggerType ParseTriggerType(const FString& TypeStr) const;
    EEventSeverity ParseSeverity(const FString& SeverityStr) const;
};
