#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Character.h"
#include "TowerNPC.generated.h"

class USphereComponent;
class UWidgetComponent;

/// NPC faction — matches Rust Faction enum
UENUM(BlueprintType)
enum class ENPCFaction : uint8
{
    AscendingOrder,
    DeepDwellers,
    EchoKeepers,
    FreeClimbers,
};

/// Dialog state — matches Rust DialogState
UENUM(BlueprintType)
enum class ENPCDialogState : uint8
{
    Idle,
    Greeting,
    QuestOffer,
    Trading,
    Farewell,
};

/// Dialog choice presented to the player
USTRUCT(BlueprintType)
struct FNPCDialogChoice
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString Text;
    UPROPERTY(BlueprintReadWrite) int32 NextNodeId = 0;
    UPROPERTY(BlueprintReadWrite) FString EffectJson;
};

/// Dialog node — one step in a conversation
USTRUCT(BlueprintType)
struct FNPCDialogNode
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) int32 NodeId = 0;
    UPROPERTY(BlueprintReadWrite) FString Speaker;
    UPROPERTY(BlueprintReadWrite) FString Text;
    UPROPERTY(BlueprintReadWrite) TArray<FNPCDialogChoice> Choices;
};

/// Quest objective display
USTRUCT(BlueprintType)
struct FNPCQuestObjective
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString Description;
    UPROPERTY(BlueprintReadWrite) int32 Current = 0;
    UPROPERTY(BlueprintReadWrite) int32 Target = 0;
    UPROPERTY(BlueprintReadWrite) bool bComplete = false;
};

/// Quest offered by NPC
USTRUCT(BlueprintType)
struct FNPCQuest
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) int32 QuestId = 0;
    UPROPERTY(BlueprintReadWrite) FString Name;
    UPROPERTY(BlueprintReadWrite) FString Description;
    UPROPERTY(BlueprintReadWrite) ENPCFaction GiverFaction = ENPCFaction::AscendingOrder;
    UPROPERTY(BlueprintReadWrite) TArray<FNPCQuestObjective> Objectives;
    UPROPERTY(BlueprintReadWrite) int32 ShardReward = 0;
    UPROPERTY(BlueprintReadWrite) int32 XpReward = 0;
    UPROPERTY(BlueprintReadWrite) float ReputationReward = 0.0f;
    UPROPERTY(BlueprintReadWrite) int32 RequiredFloor = 0;
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnNPCInteracted, ATowerNPC*, NPC, AActor*, Interactor);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnDialogChoiceMade, int32, ChoiceIndex, int32, NextNodeId);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnQuestAccepted, const FNPCQuest&, Quest);

/**
 * Tower NPC actor — faction-aligned characters that offer dialog and quests.
 * Matches Rust faction::npcs module (Npc, DialogNode, Quest structs).
 *
 * Features:
 * - Faction affiliation with visual tint
 * - Dialog tree navigation via JSON from Rust
 * - Quest offering and tracking
 * - Proximity-based interaction prompt
 * - Idle animation state
 * - Semantic tags for procedural personality
 */
UCLASS()
class TOWERGAME_API ATowerNPC : public ACharacter
{
    GENERATED_BODY()

public:
    ATowerNPC();

    virtual void BeginPlay() override;
    virtual void Tick(float DeltaTime) override;

    // --- Interaction ---

    UFUNCTION(BlueprintCallable, Category = "NPC")
    bool TryInteract(AActor* Interactor);

    UFUNCTION(BlueprintCallable, Category = "NPC")
    void EndInteraction();

    UFUNCTION(BlueprintPure, Category = "NPC")
    bool IsPlayerInRange() const { return bPlayerInRange; }

    UFUNCTION(BlueprintPure, Category = "NPC")
    bool IsInConversation() const { return DialogState != ENPCDialogState::Idle; }

    // --- Dialog ---

    UFUNCTION(BlueprintCallable, Category = "NPC|Dialog")
    void LoadDialogFromJson(const FString& DialogJson);

    UFUNCTION(BlueprintCallable, Category = "NPC|Dialog")
    void SelectChoice(int32 ChoiceIndex);

    UFUNCTION(BlueprintPure, Category = "NPC|Dialog")
    FNPCDialogNode GetCurrentDialogNode() const;

    // --- Quests ---

    UFUNCTION(BlueprintCallable, Category = "NPC|Quest")
    void LoadQuestsFromJson(const FString& QuestsJson);

    UFUNCTION(BlueprintCallable, Category = "NPC|Quest")
    bool OfferQuest(int32 QuestIndex);

    UFUNCTION(BlueprintPure, Category = "NPC|Quest")
    TArray<FNPCQuest> GetAvailableQuests() const { return AvailableQuests; }

    // --- Configuration ---

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "NPC")
    FString NPCName = TEXT("Tower Denizen");

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "NPC")
    ENPCFaction Faction = ENPCFaction::AscendingOrder;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "NPC")
    float InteractionRadius = 300.0f;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "NPC")
    TMap<FString, float> SemanticTags;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "NPC")
    bool bIsQuestGiver = true;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "NPC")
    bool bIsTrader = false;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "NPC")
    FString IdleAnimationName = TEXT("Idle");

    // --- Events ---

    UPROPERTY(BlueprintAssignable, Category = "NPC")
    FOnNPCInteracted OnInteracted;

    UPROPERTY(BlueprintAssignable, Category = "NPC")
    FOnDialogChoiceMade OnDialogChoice;

    UPROPERTY(BlueprintAssignable, Category = "NPC")
    FOnQuestAccepted OnQuestAccepted;

    // --- Components ---

    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "NPC")
    USphereComponent* InteractionZone;

    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "NPC")
    UWidgetComponent* NameplateWidget;

protected:
    UPROPERTY(BlueprintReadOnly, Category = "NPC")
    ENPCDialogState DialogState = ENPCDialogState::Idle;

    TArray<FNPCDialogNode> DialogTree;
    int32 CurrentDialogNodeId = 0;

    TArray<FNPCQuest> AvailableQuests;

    bool bPlayerInRange = false;
    float IdleLookTimer = 0.0f;

    FLinearColor GetFactionColor() const;
    FString GetFactionDisplayName() const;

    void LookAtPlayer(float DeltaTime);

    UFUNCTION()
    void OnOverlapBegin(UPrimitiveComponent* OverlappedComp, AActor* OtherActor,
        UPrimitiveComponent* OtherComp, int32 OtherBodyIndex,
        bool bFromSweep, const FHitResult& SweepResult);

    UFUNCTION()
    void OnOverlapEnd(UPrimitiveComponent* OverlappedComp, AActor* OtherActor,
        UPrimitiveComponent* OtherComp, int32 OtherBodyIndex);
};
