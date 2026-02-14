#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "QuestTrackerWidget.generated.h"

class UVerticalBox;
class UTextBlock;

/**
 * Quest objective data for UI display.
 */
USTRUCT(BlueprintType)
struct FQuestObjectiveData
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadOnly, Category = "Quest")
    FString Description;

    UPROPERTY(BlueprintReadOnly, Category = "Quest")
    int32 Current = 0;

    UPROPERTY(BlueprintReadOnly, Category = "Quest")
    int32 Required = 0;

    UPROPERTY(BlueprintReadOnly, Category = "Quest")
    bool bComplete = false;

    FString GetProgressText() const
    {
        if (Required > 0)
        {
            return FString::Printf(TEXT("%s (%d/%d)"), *Description, Current, Required);
        }
        return Description;
    }
};

/**
 * Tracked quest data for UI display.
 */
USTRUCT(BlueprintType)
struct FTrackedQuest
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadOnly, Category = "Quest")
    int32 QuestId = 0;

    UPROPERTY(BlueprintReadOnly, Category = "Quest")
    FString QuestName;

    UPROPERTY(BlueprintReadOnly, Category = "Quest")
    FString GiverFaction;

    UPROPERTY(BlueprintReadOnly, Category = "Quest")
    TArray<FQuestObjectiveData> Objectives;

    UPROPERTY(BlueprintReadOnly, Category = "Quest")
    bool bComplete = false;

    /** Get overall completion percent */
    float GetCompletionPercent() const
    {
        if (Objectives.Num() == 0) return 0.0f;
        int32 Done = 0;
        for (const auto& Obj : Objectives)
        {
            if (Obj.bComplete) Done++;
        }
        return (float)Done / (float)Objectives.Num();
    }
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnQuestCompleted, int32, QuestId);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnObjectiveUpdated, int32, QuestId, int32, ObjectiveIndex);

/**
 * Quest tracker widget — shows active quests on the right side of the HUD.
 *
 * Layout:
 *   Active Quests (title)
 *   [Quest Name 1] (faction color)
 *     - Objective 1 (3/5)
 *     - Objective 2 (done)
 *   [Quest Name 2]
 *     - Objective 1 (0/1)
 *
 * Max tracked quests: 3 (rest visible in quest log via inventory).
 * Objectives flash when updated, quests glow on completion.
 */
UCLASS()
class TOWERGAME_API UQuestTrackerWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;
    virtual void NativeTick(const FGeometry& MyGeometry, float InDeltaTime) override;

    // ============ Quest Management ============

    /** Add a quest to track */
    UFUNCTION(BlueprintCallable, Category = "Quest")
    void TrackQuest(const FTrackedQuest& Quest);

    /** Remove a quest from tracking */
    UFUNCTION(BlueprintCallable, Category = "Quest")
    void UntrackQuest(int32 QuestId);

    /** Update objective progress */
    UFUNCTION(BlueprintCallable, Category = "Quest")
    void UpdateObjective(int32 QuestId, int32 ObjectiveIndex, int32 NewCurrent, bool bIsComplete);

    /** Mark quest as complete */
    UFUNCTION(BlueprintCallable, Category = "Quest")
    void CompleteQuest(int32 QuestId);

    /** Add quest from JSON (Rust format) */
    UFUNCTION(BlueprintCallable, Category = "Quest")
    void AddQuestFromJson(const FString& QuestJson);

    /** Get tracked quests */
    UFUNCTION(BlueprintPure, Category = "Quest")
    const TArray<FTrackedQuest>& GetTrackedQuests() const { return TrackedQuests; }

    // ============ Config ============

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Quest")
    int32 MaxTrackedQuests = 3;

    // ============ Events ============

    UPROPERTY(BlueprintAssignable, Category = "Quest")
    FOnQuestCompleted OnQuestCompleted;

    UPROPERTY(BlueprintAssignable, Category = "Quest")
    FOnObjectiveUpdated OnObjectiveUpdated;

    // ============ Bound Widgets ============

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Quest")
    UTextBlock* TitleText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Quest")
    UVerticalBox* QuestListBox;

protected:
    void RebuildDisplay();

    /** Get faction color for quest styling */
    FLinearColor GetFactionColor(const FString& Faction) const;

private:
    UPROPERTY()
    TArray<FTrackedQuest> TrackedQuests;

    // Flash animation state
    TMap<int32, float> ObjectiveFlashTimers; // QuestId*100+ObjIndex -> timer
};
