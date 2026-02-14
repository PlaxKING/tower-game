#include "WorldEventWidget.h"
#include "Components/TextBlock.h"
#include "Components/ProgressBar.h"
#include "Components/VerticalBox.h"
#include "Dom/JsonObject.h"
#include "Dom/JsonValue.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

void UWorldEventWidget::NativeConstruct()
{
    Super::NativeConstruct();
    SetVisibility(ESlateVisibility::Collapsed);
}

void UWorldEventWidget::NativeTick(const FGeometry& MyGeometry, float InDeltaTime)
{
    Super::NativeTick(MyGeometry, InDeltaTime);

    bool bChanged = false;

    for (FWorldEventDisplay& Event : ActiveEvents)
    {
        Event.RemainingTime -= InDeltaTime;
        Event.FlashTimer -= InDeltaTime;

        if (Event.RemainingTime <= 0.0f)
        {
            bChanged = true;
        }
    }

    // Remove expired
    int32 Before = ActiveEvents.Num();
    ActiveEvents.RemoveAll([](const FWorldEventDisplay& E) {
        return E.RemainingTime <= 0.0f;
    });

    if (ActiveEvents.Num() != Before)
    {
        bChanged = true;
    }

    if (bChanged)
    {
        RebuildDisplay();
    }

    // Update primary event timer bar
    if (ActiveEvents.Num() > 0 && ActiveEventTimer)
    {
        const FWorldEventDisplay& Primary = ActiveEvents[0];
        float Progress = FMath::Clamp(Primary.RemainingTime / Primary.Duration, 0.0f, 1.0f);
        ActiveEventTimer->SetPercent(Progress);
    }

    // Hide when no events
    if (ActiveEvents.Num() == 0)
    {
        SetVisibility(ESlateVisibility::Collapsed);
    }
}

void UWorldEventWidget::ShowEvent(const FString& Name, const FString& Description,
    EEventTriggerType TriggerType, EEventSeverity Severity, float Duration)
{
    // Check if event already active
    for (FWorldEventDisplay& Existing : ActiveEvents)
    {
        if (Existing.Name == Name)
        {
            // Refresh duration
            Existing.RemainingTime = Duration;
            Existing.FlashTimer = IntroFlashDuration;
            RebuildDisplay();
            return;
        }
    }

    FWorldEventDisplay Event;
    Event.Name = Name;
    Event.Description = Description;
    Event.TriggerType = TriggerType;
    Event.Severity = Severity;
    Event.Duration = Duration;
    Event.RemainingTime = Duration;
    Event.FlashTimer = IntroFlashDuration;

    // Insert sorted by severity (Critical first)
    int32 InsertIdx = 0;
    for (int32 i = 0; i < ActiveEvents.Num(); i++)
    {
        if (static_cast<uint8>(ActiveEvents[i].Severity) >= static_cast<uint8>(Severity))
        {
            InsertIdx = i + 1;
        }
        else
        {
            break;
        }
    }
    ActiveEvents.Insert(Event, InsertIdx);

    // Trim to max
    while (ActiveEvents.Num() > MaxDisplayEvents)
    {
        ActiveEvents.RemoveAt(ActiveEvents.Num() - 1);
    }

    SetVisibility(ESlateVisibility::Visible);
    RebuildDisplay();

    UE_LOG(LogTemp, Log, TEXT("World event: %s [%s] (%.0fs)"),
        *Name, *GetSeverityLabel(Severity), Duration);
}

void UWorldEventWidget::ShowEventFromJson(const FString& EventJson)
{
    TSharedPtr<FJsonObject> Json;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(EventJson);
    if (!FJsonSerializer::Deserialize(Reader, Json) || !Json.IsValid()) return;

    FString Name = Json->GetStringField(TEXT("name"));
    FString Description = Json->GetStringField(TEXT("description"));
    FString TriggerStr = Json->GetStringField(TEXT("trigger_type"));
    FString SeverityStr = Json->GetStringField(TEXT("severity"));
    float Duration = Json->GetNumberField(TEXT("duration_secs"));

    EEventTriggerType TriggerType = ParseTriggerType(TriggerStr);
    EEventSeverity Severity = ParseSeverity(SeverityStr);

    ShowEvent(Name, Description, TriggerType, Severity, Duration);
}

void UWorldEventWidget::RemoveEvent(const FString& Name)
{
    ActiveEvents.RemoveAll([&Name](const FWorldEventDisplay& E) {
        return E.Name == Name;
    });
    RebuildDisplay();
}

void UWorldEventWidget::ClearAll()
{
    ActiveEvents.Empty();
    SetVisibility(ESlateVisibility::Collapsed);
}

void UWorldEventWidget::RebuildDisplay()
{
    // Primary event (biggest/most severe)
    if (ActiveEvents.Num() > 0)
    {
        const FWorldEventDisplay& Primary = ActiveEvents[0];

        if (ActiveEventTitle)
        {
            FString Icon = GetTriggerIcon(Primary.TriggerType);
            FString Label = GetSeverityLabel(Primary.Severity);
            ActiveEventTitle->SetText(FText::FromString(
                FString::Printf(TEXT("%s %s [%s]"), *Icon, *Primary.Name, *Label)));

            // Flash effect during intro
            FLinearColor TitleColor = GetTriggerColor(Primary.TriggerType);
            if (Primary.FlashTimer > 0.0f)
            {
                float Flash = FMath::Sin(Primary.FlashTimer * 8.0f) * 0.5f + 0.5f;
                TitleColor = FMath::Lerp(TitleColor, FLinearColor::White, Flash * 0.5f);
            }
            ActiveEventTitle->SetColorAndOpacity(FSlateColor(TitleColor));
        }

        if (ActiveEventDesc)
        {
            ActiveEventDesc->SetText(FText::FromString(Primary.Description));
            ActiveEventDesc->SetColorAndOpacity(FSlateColor(FLinearColor(0.8f, 0.8f, 0.8f)));
        }

        if (ActiveEventTimer)
        {
            float Progress = FMath::Clamp(Primary.RemainingTime / Primary.Duration, 0.0f, 1.0f);
            ActiveEventTimer->SetPercent(Progress);
            ActiveEventTimer->SetFillColorAndOpacity(GetSeverityColor(Primary.Severity));
        }

        if (SeverityText)
        {
            int32 SecsRemaining = FMath::CeilToInt(Primary.RemainingTime);
            int32 Mins = SecsRemaining / 60;
            int32 Secs = SecsRemaining % 60;
            SeverityText->SetText(FText::FromString(
                FString::Printf(TEXT("%02d:%02d"), Mins, Secs)));
            SeverityText->SetColorAndOpacity(FSlateColor(GetSeverityColor(Primary.Severity)));
        }
    }

    // Secondary events list
    if (!EventListBox) return;
    EventListBox->ClearChildren();

    for (int32 i = 1; i < ActiveEvents.Num(); i++)
    {
        const FWorldEventDisplay& Event = ActiveEvents[i];

        UTextBlock* EntryText = NewObject<UTextBlock>(this);

        FString Icon = GetTriggerIcon(Event.TriggerType);
        int32 SecsRemain = FMath::CeilToInt(Event.RemainingTime);
        FString Display = FString::Printf(TEXT("%s %s (%ds)"),
            *Icon, *Event.Name, SecsRemain);
        EntryText->SetText(FText::FromString(Display));
        EntryText->SetColorAndOpacity(FSlateColor(GetTriggerColor(Event.TriggerType)));

        FSlateFontInfo Font = EntryText->GetFont();
        Font.Size = 10;
        EntryText->SetFont(Font);

        EventListBox->AddChild(EntryText);
    }
}

FLinearColor UWorldEventWidget::GetTriggerColor(EEventTriggerType Type) const
{
    switch (Type)
    {
    case EEventTriggerType::BreathShift:       return FLinearColor(0.4f, 0.8f, 1.0f);  // Sky blue
    case EEventTriggerType::SemanticResonance:  return FLinearColor(0.9f, 0.7f, 0.2f);  // Golden
    case EEventTriggerType::EchoConvergence:    return FLinearColor(0.6f, 0.3f, 1.0f);  // Purple
    case EEventTriggerType::FloorAnomaly:       return FLinearColor(0.2f, 1.0f, 0.8f);  // Cyan
    case EEventTriggerType::FactionClash:       return FLinearColor(1.0f, 0.5f, 0.2f);  // Orange
    case EEventTriggerType::CorruptionSurge:    return FLinearColor(1.0f, 0.1f, 0.3f);  // Red
    case EEventTriggerType::TowerMemory:        return FLinearColor(0.8f, 0.8f, 1.0f);  // Pale blue
    default: return FLinearColor(0.7f, 0.7f, 0.7f);
    }
}

FLinearColor UWorldEventWidget::GetSeverityColor(EEventSeverity Severity) const
{
    switch (Severity)
    {
    case EEventSeverity::Minor:    return FLinearColor(0.5f, 0.7f, 0.5f);
    case EEventSeverity::Moderate: return FLinearColor(0.9f, 0.7f, 0.2f);
    case EEventSeverity::Major:    return FLinearColor(1.0f, 0.4f, 0.1f);
    case EEventSeverity::Critical: return FLinearColor(1.0f, 0.1f, 0.1f);
    default: return FLinearColor(0.5f, 0.5f, 0.5f);
    }
}

FString UWorldEventWidget::GetTriggerIcon(EEventTriggerType Type) const
{
    switch (Type)
    {
    case EEventTriggerType::BreathShift:       return TEXT("{B}");
    case EEventTriggerType::SemanticResonance:  return TEXT("{S}");
    case EEventTriggerType::EchoConvergence:    return TEXT("{E}");
    case EEventTriggerType::FloorAnomaly:       return TEXT("{A}");
    case EEventTriggerType::FactionClash:       return TEXT("{F}");
    case EEventTriggerType::CorruptionSurge:    return TEXT("{C}");
    case EEventTriggerType::TowerMemory:        return TEXT("{M}");
    default: return TEXT("{?}");
    }
}

FString UWorldEventWidget::GetSeverityLabel(EEventSeverity Severity) const
{
    switch (Severity)
    {
    case EEventSeverity::Minor:    return TEXT("Minor");
    case EEventSeverity::Moderate: return TEXT("Moderate");
    case EEventSeverity::Major:    return TEXT("MAJOR");
    case EEventSeverity::Critical: return TEXT("CRITICAL");
    default: return TEXT("Unknown");
    }
}

EEventTriggerType UWorldEventWidget::ParseTriggerType(const FString& TypeStr) const
{
    if (TypeStr == TEXT("BreathShift"))       return EEventTriggerType::BreathShift;
    if (TypeStr == TEXT("SemanticResonance")) return EEventTriggerType::SemanticResonance;
    if (TypeStr == TEXT("EchoConvergence"))   return EEventTriggerType::EchoConvergence;
    if (TypeStr == TEXT("FloorAnomaly"))      return EEventTriggerType::FloorAnomaly;
    if (TypeStr == TEXT("FactionClash"))      return EEventTriggerType::FactionClash;
    if (TypeStr == TEXT("CorruptionSurge"))   return EEventTriggerType::CorruptionSurge;
    if (TypeStr == TEXT("TowerMemory"))       return EEventTriggerType::TowerMemory;
    return EEventTriggerType::BreathShift;
}

EEventSeverity UWorldEventWidget::ParseSeverity(const FString& SeverityStr) const
{
    if (SeverityStr == TEXT("Minor"))    return EEventSeverity::Minor;
    if (SeverityStr == TEXT("Moderate")) return EEventSeverity::Moderate;
    if (SeverityStr == TEXT("Major"))    return EEventSeverity::Major;
    if (SeverityStr == TEXT("Critical")) return EEventSeverity::Critical;
    return EEventSeverity::Minor;
}
