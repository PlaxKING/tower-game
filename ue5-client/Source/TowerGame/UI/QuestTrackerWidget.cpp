#include "QuestTrackerWidget.h"
#include "Components/VerticalBox.h"
#include "Components/TextBlock.h"
#include "Dom/JsonObject.h"
#include "Dom/JsonValue.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

void UQuestTrackerWidget::NativeConstruct()
{
    Super::NativeConstruct();
    RebuildDisplay();
}

void UQuestTrackerWidget::NativeTick(const FGeometry& MyGeometry, float InDeltaTime)
{
    Super::NativeTick(MyGeometry, InDeltaTime);

    // Tick flash timers
    TArray<int32> ExpiredKeys;
    for (auto& Pair : ObjectiveFlashTimers)
    {
        Pair.Value -= InDeltaTime;
        if (Pair.Value <= 0.0f)
        {
            ExpiredKeys.Add(Pair.Key);
        }
    }
    for (int32 Key : ExpiredKeys)
    {
        ObjectiveFlashTimers.Remove(Key);
    }
}

void UQuestTrackerWidget::TrackQuest(const FTrackedQuest& Quest)
{
    // Check limit
    if (TrackedQuests.Num() >= MaxTrackedQuests)
    {
        UE_LOG(LogTemp, Warning, TEXT("Max tracked quests reached (%d)"), MaxTrackedQuests);
        return;
    }

    // Don't duplicate
    for (const FTrackedQuest& Existing : TrackedQuests)
    {
        if (Existing.QuestId == Quest.QuestId) return;
    }

    TrackedQuests.Add(Quest);
    RebuildDisplay();
}

void UQuestTrackerWidget::UntrackQuest(int32 QuestId)
{
    TrackedQuests.RemoveAll([QuestId](const FTrackedQuest& Q) { return Q.QuestId == QuestId; });
    RebuildDisplay();
}

void UQuestTrackerWidget::UpdateObjective(int32 QuestId, int32 ObjectiveIndex, int32 NewCurrent, bool bIsComplete)
{
    for (FTrackedQuest& Quest : TrackedQuests)
    {
        if (Quest.QuestId != QuestId) continue;

        if (Quest.Objectives.IsValidIndex(ObjectiveIndex))
        {
            Quest.Objectives[ObjectiveIndex].Current = NewCurrent;
            Quest.Objectives[ObjectiveIndex].bComplete = bIsComplete;

            // Flash indicator
            int32 FlashKey = QuestId * 100 + ObjectiveIndex;
            ObjectiveFlashTimers.Add(FlashKey, 2.0f);

            OnObjectiveUpdated.Broadcast(QuestId, ObjectiveIndex);

            // Check if all objectives done
            bool bAllDone = true;
            for (const FQuestObjectiveData& Obj : Quest.Objectives)
            {
                if (!Obj.bComplete) { bAllDone = false; break; }
            }
            if (bAllDone)
            {
                CompleteQuest(QuestId);
            }
        }
        break;
    }

    RebuildDisplay();
}

void UQuestTrackerWidget::CompleteQuest(int32 QuestId)
{
    for (FTrackedQuest& Quest : TrackedQuests)
    {
        if (Quest.QuestId == QuestId)
        {
            Quest.bComplete = true;
            break;
        }
    }

    OnQuestCompleted.Broadcast(QuestId);
    RebuildDisplay();

    UE_LOG(LogTemp, Log, TEXT("Quest %d completed!"), QuestId);
}

void UQuestTrackerWidget::AddQuestFromJson(const FString& QuestJson)
{
    TSharedPtr<FJsonObject> Json;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(QuestJson);
    if (!FJsonSerializer::Deserialize(Reader, Json) || !Json.IsValid()) return;

    FTrackedQuest Quest;
    Quest.QuestId = Json->GetIntegerField(TEXT("id"));
    Quest.QuestName = Json->GetStringField(TEXT("name"));
    Quest.GiverFaction = Json->HasField(TEXT("faction")) ? Json->GetStringField(TEXT("faction")) : TEXT("neutral");

    const TArray<TSharedPtr<FJsonValue>>* Objectives;
    if (Json->TryGetArrayField(TEXT("objectives"), Objectives))
    {
        for (const TSharedPtr<FJsonValue>& ObjVal : *Objectives)
        {
            const TSharedPtr<FJsonObject>& ObjJson = ObjVal->AsObject();
            if (!ObjJson.IsValid()) continue;

            FQuestObjectiveData Obj;
            Obj.Description = ObjJson->GetStringField(TEXT("description"));
            Obj.Current = ObjJson->HasField(TEXT("current")) ? ObjJson->GetIntegerField(TEXT("current")) : 0;
            Obj.Required = ObjJson->HasField(TEXT("required")) ? ObjJson->GetIntegerField(TEXT("required")) : 1;
            Obj.bComplete = ObjJson->HasField(TEXT("complete")) ? ObjJson->GetBoolField(TEXT("complete")) : false;

            Quest.Objectives.Add(Obj);
        }
    }

    TrackQuest(Quest);
}

FLinearColor UQuestTrackerWidget::GetFactionColor(const FString& Faction) const
{
    if (Faction == TEXT("seekers"))   return FLinearColor(0.2f, 0.6f, 1.0f);  // Blue
    if (Faction == TEXT("wardens"))   return FLinearColor(0.2f, 0.8f, 0.3f);  // Green
    if (Faction == TEXT("breakers"))  return FLinearColor(1.0f, 0.3f, 0.2f);  // Red
    if (Faction == TEXT("weavers"))   return FLinearColor(0.7f, 0.3f, 1.0f);  // Purple
    return FLinearColor(0.8f, 0.8f, 0.8f);  // Neutral gray
}

void UQuestTrackerWidget::RebuildDisplay()
{
    if (!QuestListBox) return;

    QuestListBox->ClearChildren();

    for (const FTrackedQuest& Quest : TrackedQuests)
    {
        // Quest name header
        UTextBlock* NameText = NewObject<UTextBlock>(this);
        FString NameDisplay = Quest.bComplete ? FString::Printf(TEXT("[Done] %s"), *Quest.QuestName) : Quest.QuestName;
        NameText->SetText(FText::FromString(NameDisplay));

        FLinearColor QuestColor = Quest.bComplete ?
            FLinearColor(0.5f, 1.0f, 0.5f) : GetFactionColor(Quest.GiverFaction);
        NameText->SetColorAndOpacity(FSlateColor(QuestColor));

        FSlateFontInfo NameFont = NameText->GetFont();
        NameFont.Size = 12;
        NameText->SetFont(NameFont);

        QuestListBox->AddChild(NameText);

        // Objectives
        for (int32 i = 0; i < Quest.Objectives.Num(); i++)
        {
            const FQuestObjectiveData& Obj = Quest.Objectives[i];

            UTextBlock* ObjText = NewObject<UTextBlock>(this);
            FString ObjDisplay = FString::Printf(TEXT("  %s %s"),
                Obj.bComplete ? TEXT("[x]") : TEXT("[ ]"),
                *Obj.GetProgressText());
            ObjText->SetText(FText::FromString(ObjDisplay));

            // Flash effect for recently updated objectives
            int32 FlashKey = Quest.QuestId * 100 + i;
            float* FlashTimer = ObjectiveFlashTimers.Find(FlashKey);
            if (FlashTimer && *FlashTimer > 0.0f)
            {
                float Flash = FMath::Sin(*FlashTimer * 5.0f) * 0.5f + 0.5f;
                ObjText->SetColorAndOpacity(FSlateColor(
                    FLinearColor::LerpUsingHSV(FLinearColor::White, FLinearColor(1.0f, 1.0f, 0.0f), Flash)));
            }
            else
            {
                FLinearColor ObjColor = Obj.bComplete ?
                    FLinearColor(0.4f, 0.7f, 0.4f) : FLinearColor(0.8f, 0.8f, 0.8f);
                ObjText->SetColorAndOpacity(FSlateColor(ObjColor));
            }

            FSlateFontInfo ObjFont = ObjText->GetFont();
            ObjFont.Size = 10;
            ObjText->SetFont(ObjFont);

            QuestListBox->AddChild(ObjText);
        }
    }
}
