#include "TowerGameState.h"
#include "TowerGameSubsystem.h"
#include "Kismet/GameplayStatics.h"
#include "Net/UnrealNetwork.h"
#include "Dom/JsonObject.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

ATowerGameState::ATowerGameState()
{
    PrimaryActorTick.bCanEverTick = true;
    PrimaryActorTick.TickInterval = 1.0f; // Update breath once per second
}

void ATowerGameState::Tick(float DeltaSeconds)
{
    Super::Tick(DeltaSeconds);

    TotalGameTime += DeltaSeconds;

    // Update breath state from Rust
    if (HasAuthority())
    {
        UGameInstance* GI = UGameplayStatics::GetGameInstance(this);
        if (GI)
        {
            UTowerGameSubsystem* Sub = GI->GetSubsystem<UTowerGameSubsystem>();
            if (Sub && Sub->IsRustCoreReady())
            {
                Sub->GameElapsedTime = TotalGameTime;
                FString BreathJson = Sub->GetBreathState(TotalGameTime);
                UpdateBreathFromJson(BreathJson);
            }
        }
    }
}

void ATowerGameState::UpdateBreathFromJson(const FString& BreathJson)
{
    if (BreathJson.IsEmpty()) return;

    TSharedPtr<FJsonObject> JsonObj;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(BreathJson);
    if (!FJsonSerializer::Deserialize(Reader, JsonObj) || !JsonObj.IsValid()) return;

    // Field names match Rust BreathState struct
    BreathPhase = JsonObj->GetStringField(TEXT("phase"));
    BreathProgress = JsonObj->GetNumberField(TEXT("phase_progress"));
    MonsterSpawnMultiplier = JsonObj->GetNumberField(TEXT("monster_spawn_mult"));
    SemanticFieldStrength = JsonObj->GetNumberField(TEXT("semantic_intensity"));
}

void ATowerGameState::OnMonsterDefeated()
{
    MonstersRemaining = FMath::Max(0, MonstersRemaining - 1);

    if (MonstersRemaining <= 0)
    {
        bStairsUnlocked = true;
        UE_LOG(LogTemp, Log, TEXT("All monsters defeated! Stairs unlocked on floor %d"), ActiveFloor);
    }
}

void ATowerGameState::GetLifetimeReplicatedProps(TArray<FLifetimeProperty>& OutLifetimeProps) const
{
    Super::GetLifetimeReplicatedProps(OutLifetimeProps);

    DOREPLIFETIME(ATowerGameState, BreathPhase);
    DOREPLIFETIME(ATowerGameState, BreathProgress);
    DOREPLIFETIME(ATowerGameState, MonsterSpawnMultiplier);
    DOREPLIFETIME(ATowerGameState, SemanticFieldStrength);
    DOREPLIFETIME(ATowerGameState, TotalGameTime);
    DOREPLIFETIME(ATowerGameState, ActiveFloor);
    DOREPLIFETIME(ATowerGameState, MonstersRemaining);
    DOREPLIFETIME(ATowerGameState, bStairsUnlocked);
}
