#include "TowerNPC.h"
#include "Components/SphereComponent.h"
#include "Components/WidgetComponent.h"
#include "GameFramework/CharacterMovementComponent.h"
#include "Kismet/KismetMathLibrary.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"
#include "GameFramework/PlayerController.h"

ATowerNPC::ATowerNPC()
{
    PrimaryActorTick.bCanEverTick = true;

    // Interaction sphere
    InteractionZone = CreateDefaultSubobject<USphereComponent>(TEXT("InteractionZone"));
    InteractionZone->SetupAttachment(RootComponent);
    InteractionZone->SetSphereRadius(InteractionRadius);
    InteractionZone->SetCollisionProfileName(TEXT("OverlapAllDynamic"));
    InteractionZone->SetGenerateOverlapEvents(true);

    // Nameplate widget above head
    NameplateWidget = CreateDefaultSubobject<UWidgetComponent>(TEXT("Nameplate"));
    NameplateWidget->SetupAttachment(RootComponent);
    NameplateWidget->SetRelativeLocation(FVector(0.0f, 0.0f, 120.0f));
    NameplateWidget->SetWidgetSpace(EWidgetSpace::Screen);
    NameplateWidget->SetDrawSize(FVector2D(200.0f, 50.0f));

    // NPCs don't move by default
    if (GetCharacterMovement())
    {
        GetCharacterMovement()->MaxWalkSpeed = 0.0f;
    }
}

void ATowerNPC::BeginPlay()
{
    Super::BeginPlay();

    // Bind overlaps
    InteractionZone->OnComponentBeginOverlap.AddDynamic(this, &ATowerNPC::OnOverlapBegin);
    InteractionZone->OnComponentEndOverlap.AddDynamic(this, &ATowerNPC::OnOverlapEnd);

    // Update interaction zone radius
    InteractionZone->SetSphereRadius(InteractionRadius);

    UE_LOG(LogTemp, Log, TEXT("NPC '%s' (%s) spawned"),
        *NPCName, *GetFactionDisplayName());
}

void ATowerNPC::Tick(float DeltaTime)
{
    Super::Tick(DeltaTime);

    // Slowly look at player when in conversation
    if (bPlayerInRange && IsInConversation())
    {
        LookAtPlayer(DeltaTime);
    }

    // Idle look around behavior
    if (!IsInConversation())
    {
        IdleLookTimer += DeltaTime;
        if (IdleLookTimer > 5.0f)
        {
            IdleLookTimer = 0.0f;
            // Slight head turn effect — Blueprints handle actual animation
        }
    }
}

bool ATowerNPC::TryInteract(AActor* Interactor)
{
    if (!Interactor || !bPlayerInRange) return false;
    if (IsInConversation()) return false;

    DialogState = ENPCDialogState::Greeting;
    CurrentDialogNodeId = 0;

    OnInteracted.Broadcast(this, Interactor);

    UE_LOG(LogTemp, Log, TEXT("Player interacted with NPC '%s'"), *NPCName);
    return true;
}

void ATowerNPC::EndInteraction()
{
    DialogState = ENPCDialogState::Idle;
    CurrentDialogNodeId = 0;

    UE_LOG(LogTemp, Log, TEXT("Ended interaction with NPC '%s'"), *NPCName);
}

void ATowerNPC::LoadDialogFromJson(const FString& DialogJson)
{
    DialogTree.Empty();

    TSharedPtr<FJsonValue> Parsed;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(DialogJson);
    if (!FJsonSerializer::Deserialize(Reader, Parsed)) return;

    const TArray<TSharedPtr<FJsonValue>>* Nodes;
    if (!Parsed->TryGetArray(Nodes)) return;

    for (const auto& NodeVal : *Nodes)
    {
        const TSharedPtr<FJsonObject>& NodeObj = NodeVal->AsObject();
        if (!NodeObj) continue;

        FNPCDialogNode Node;
        Node.NodeId = NodeObj->GetIntegerField(TEXT("id"));
        Node.Speaker = NodeObj->GetStringField(TEXT("speaker"));
        Node.Text = NodeObj->GetStringField(TEXT("text"));

        const TArray<TSharedPtr<FJsonValue>>* Choices;
        if (NodeObj->TryGetArrayField(TEXT("choices"), Choices))
        {
            for (const auto& ChoiceVal : *Choices)
            {
                const TSharedPtr<FJsonObject>& ChoiceObj = ChoiceVal->AsObject();
                if (!ChoiceObj) continue;

                FNPCDialogChoice Choice;
                Choice.Text = ChoiceObj->GetStringField(TEXT("text"));
                Choice.NextNodeId = ChoiceObj->GetIntegerField(TEXT("next_node"));
                Node.Choices.Add(Choice);
            }
        }

        DialogTree.Add(Node);
    }

    UE_LOG(LogTemp, Log, TEXT("Loaded %d dialog nodes for NPC '%s'"),
        DialogTree.Num(), *NPCName);
}

void ATowerNPC::SelectChoice(int32 ChoiceIndex)
{
    FNPCDialogNode Current = GetCurrentDialogNode();
    if (!Current.Choices.IsValidIndex(ChoiceIndex)) return;

    const FNPCDialogChoice& Choice = Current.Choices[ChoiceIndex];
    int32 NextId = Choice.NextNodeId;

    OnDialogChoice.Broadcast(ChoiceIndex, NextId);

    // Check if next node exists
    bool bFound = false;
    for (const auto& Node : DialogTree)
    {
        if (Node.NodeId == NextId)
        {
            CurrentDialogNodeId = NextId;
            bFound = true;
            break;
        }
    }

    if (!bFound)
    {
        // End of dialog tree
        DialogState = ENPCDialogState::Farewell;
    }
}

FNPCDialogNode ATowerNPC::GetCurrentDialogNode() const
{
    for (const auto& Node : DialogTree)
    {
        if (Node.NodeId == CurrentDialogNodeId)
        {
            return Node;
        }
    }

    // Return empty node
    FNPCDialogNode Empty;
    Empty.Speaker = NPCName;
    Empty.Text = TEXT("...");
    return Empty;
}

void ATowerNPC::LoadQuestsFromJson(const FString& QuestsJson)
{
    AvailableQuests.Empty();

    TSharedPtr<FJsonValue> Parsed;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(QuestsJson);
    if (!FJsonSerializer::Deserialize(Reader, Parsed)) return;

    const TArray<TSharedPtr<FJsonValue>>* Quests;
    if (!Parsed->TryGetArray(Quests)) return;

    for (const auto& QVal : *Quests)
    {
        const TSharedPtr<FJsonObject>& QObj = QVal->AsObject();
        if (!QObj) continue;

        FNPCQuest Quest;
        Quest.QuestId = QObj->GetIntegerField(TEXT("id"));
        Quest.Name = QObj->GetStringField(TEXT("name"));
        Quest.Description = QObj->GetStringField(TEXT("description"));
        Quest.RequiredFloor = QObj->GetIntegerField(TEXT("required_floor"));

        // Parse rewards
        const TArray<TSharedPtr<FJsonValue>>* Rewards;
        if (QObj->TryGetArrayField(TEXT("rewards"), Rewards))
        {
            for (const auto& RVal : *Rewards)
            {
                const TSharedPtr<FJsonObject>& RObj = RVal->AsObject();
                if (!RObj) continue;

                FString Type = RObj->GetStringField(TEXT("type"));
                if (Type == TEXT("Shards"))
                    Quest.ShardReward = RObj->GetIntegerField(TEXT("amount"));
                else if (Type == TEXT("Xp"))
                    Quest.XpReward = RObj->GetIntegerField(TEXT("amount"));
                else if (Type == TEXT("Reputation"))
                    Quest.ReputationReward = RObj->GetNumberField(TEXT("amount"));
            }
        }

        // Parse objectives
        const TArray<TSharedPtr<FJsonValue>>* Objectives;
        if (QObj->TryGetArrayField(TEXT("objectives"), Objectives))
        {
            for (const auto& OVal : *Objectives)
            {
                const TSharedPtr<FJsonObject>& OObj = OVal->AsObject();
                if (!OObj) continue;

                FNPCQuestObjective Obj;
                Obj.Description = OObj->GetStringField(TEXT("description"));
                Obj.Current = OObj->GetIntegerField(TEXT("current"));
                Obj.Target = OObj->GetIntegerField(TEXT("target"));
                Obj.bComplete = Obj.Current >= Obj.Target;
                Quest.Objectives.Add(Obj);
            }
        }

        AvailableQuests.Add(Quest);
    }

    UE_LOG(LogTemp, Log, TEXT("Loaded %d quests for NPC '%s'"),
        AvailableQuests.Num(), *NPCName);
}

bool ATowerNPC::OfferQuest(int32 QuestIndex)
{
    if (!AvailableQuests.IsValidIndex(QuestIndex)) return false;

    const FNPCQuest& Quest = AvailableQuests[QuestIndex];
    DialogState = ENPCDialogState::QuestOffer;

    OnQuestAccepted.Broadcast(Quest);

    UE_LOG(LogTemp, Log, TEXT("NPC '%s' offered quest: %s"), *NPCName, *Quest.Name);
    return true;
}

FLinearColor ATowerNPC::GetFactionColor() const
{
    switch (Faction)
    {
    case ENPCFaction::AscendingOrder: return FLinearColor(0.2f, 0.6f, 1.0f);  // blue
    case ENPCFaction::DeepDwellers:   return FLinearColor(0.6f, 0.3f, 0.8f);  // purple
    case ENPCFaction::EchoKeepers:    return FLinearColor(0.0f, 0.8f, 0.6f);  // teal
    case ENPCFaction::FreeClimbers:   return FLinearColor(1.0f, 0.7f, 0.2f);  // gold
    default:                          return FLinearColor::White;
    }
}

FString ATowerNPC::GetFactionDisplayName() const
{
    switch (Faction)
    {
    case ENPCFaction::AscendingOrder: return TEXT("Ascending Order");
    case ENPCFaction::DeepDwellers:   return TEXT("Deep Dwellers");
    case ENPCFaction::EchoKeepers:    return TEXT("Echo Keepers");
    case ENPCFaction::FreeClimbers:   return TEXT("Free Climbers");
    default:                          return TEXT("Unknown");
    }
}

void ATowerNPC::LookAtPlayer(float DeltaTime)
{
    APlayerController* PC = GetWorld()->GetFirstPlayerController();
    if (!PC || !PC->GetPawn()) return;

    FVector PlayerLoc = PC->GetPawn()->GetActorLocation();
    FVector NPCLoc = GetActorLocation();

    FRotator LookAt = UKismetMathLibrary::FindLookAtRotation(NPCLoc, PlayerLoc);
    FRotator Current = GetActorRotation();

    // Only rotate yaw, keep pitch/roll stable
    FRotator Target = FRotator(Current.Pitch, LookAt.Yaw, Current.Roll);
    FRotator Smoothed = FMath::RInterpTo(Current, Target, DeltaTime, 3.0f);
    SetActorRotation(Smoothed);
}

void ATowerNPC::OnOverlapBegin(UPrimitiveComponent* OverlappedComp, AActor* OtherActor,
    UPrimitiveComponent* OtherComp, int32 OtherBodyIndex,
    bool bFromSweep, const FHitResult& SweepResult)
{
    APawn* PlayerPawn = Cast<APawn>(OtherActor);
    if (PlayerPawn && PlayerPawn->IsPlayerControlled())
    {
        bPlayerInRange = true;
    }
}

void ATowerNPC::OnOverlapEnd(UPrimitiveComponent* OverlappedComp, AActor* OtherActor,
    UPrimitiveComponent* OtherComp, int32 OtherBodyIndex)
{
    APawn* PlayerPawn = Cast<APawn>(OtherActor);
    if (PlayerPawn && PlayerPawn->IsPlayerControlled())
    {
        bPlayerInRange = false;

        // Auto-end conversation if player walks away
        if (IsInConversation())
        {
            EndInteraction();
        }
    }
}
