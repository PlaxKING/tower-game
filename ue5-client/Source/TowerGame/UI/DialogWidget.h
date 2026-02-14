#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "DialogWidget.generated.h"

class UTextBlock;
class UVerticalBox;
class UButton;
class UImage;

/**
 * Dialog choice for the player.
 */
USTRUCT(BlueprintType)
struct FDialogChoiceData
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadOnly, Category = "Dialog")
    FString Text;

    UPROPERTY(BlueprintReadOnly, Category = "Dialog")
    int32 NextNodeId = 0;

    UPROPERTY(BlueprintReadOnly, Category = "Dialog")
    bool bAvailable = true;

    UPROPERTY(BlueprintReadOnly, Category = "Dialog")
    FString RequirementHint; // "Requires: Seekers Respected" etc.
};

/**
 * Dialog node data from the Rust quest system.
 */
USTRUCT(BlueprintType)
struct FDialogNodeData
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadOnly, Category = "Dialog")
    int32 NodeId = 0;

    UPROPERTY(BlueprintReadOnly, Category = "Dialog")
    FString Speaker;

    UPROPERTY(BlueprintReadOnly, Category = "Dialog")
    FString Text;

    UPROPERTY(BlueprintReadOnly, Category = "Dialog")
    TArray<FDialogChoiceData> Choices;

    UPROPERTY(BlueprintReadOnly, Category = "Dialog")
    FString Faction;
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnDialogChoice, int32, NodeId, int32, ChoiceIndex);
DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnDialogClosed);

/**
 * NPC dialog widget — visual novel style conversation UI.
 *
 * Layout:
 *   [Speaker portrait area]
 *   [Speaker Name] (faction colored)
 *   [Dialog text - typewriter effect]
 *   -----
 *   [Choice 1]
 *   [Choice 2]
 *   [Choice 3]
 *   -----
 *   (press Escape to close)
 *
 * Features:
 * - Typewriter text reveal (configurable speed)
 * - Click to skip typewriter / advance
 * - Faction-colored speaker name
 * - Grayed out choices that don't meet requirements
 * - Requirement hints on unavailable choices
 */
UCLASS()
class TOWERGAME_API UDialogWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;
    virtual void NativeTick(const FGeometry& MyGeometry, float InDeltaTime) override;

    // ============ API ============

    /** Show a dialog node */
    UFUNCTION(BlueprintCallable, Category = "Dialog")
    void ShowNode(const FDialogNodeData& Node);

    /** Show dialog from JSON */
    UFUNCTION(BlueprintCallable, Category = "Dialog")
    void ShowNodeFromJson(const FString& NodeJson);

    /** Close dialog */
    UFUNCTION(BlueprintCallable, Category = "Dialog")
    void CloseDialog();

    /** Skip typewriter effect */
    UFUNCTION(BlueprintCallable, Category = "Dialog")
    void SkipTypewriter();

    /** Is dialog currently showing? */
    UFUNCTION(BlueprintPure, Category = "Dialog")
    bool IsShowing() const { return bShowing; }

    /** Is typewriter still revealing text? */
    UFUNCTION(BlueprintPure, Category = "Dialog")
    bool IsTypewriting() const { return bTypewriting; }

    // ============ Config ============

    /** Characters per second for typewriter effect */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Dialog")
    float TypewriterSpeed = 40.0f;

    // ============ Events ============

    UPROPERTY(BlueprintAssignable, Category = "Dialog")
    FOnDialogChoice OnChoiceMade;

    UPROPERTY(BlueprintAssignable, Category = "Dialog")
    FOnDialogClosed OnClosed;

    // ============ Bound Widgets ============

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Dialog")
    UTextBlock* SpeakerNameText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Dialog")
    UTextBlock* DialogText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Dialog")
    UVerticalBox* ChoicesBox;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Dialog")
    UTextBlock* ContinueHintText;

protected:
    void BuildChoices(const TArray<FDialogChoiceData>& Choices);

    /** Get faction color for speaker styling */
    FLinearColor GetFactionColor(const FString& Faction) const;

private:
    FDialogNodeData CurrentNode;
    FString FullText;
    int32 RevealedChars = 0;
    float TypewriterTimer = 0.0f;
    bool bShowing = false;
    bool bTypewriting = false;
};
