#pragma once

#include "CoreMinimal.h"
#include "GameFramework/GameStateBase.h"
#include "TowerGameState.generated.h"

/**
 * Tower Game State — replicated state visible to all players.
 * Tracks Breath of Tower cycle, floor progress, and global events.
 */
UCLASS()
class TOWERGAME_API ATowerGameState : public AGameStateBase
{
    GENERATED_BODY()

public:
    ATowerGameState();

    virtual void Tick(float DeltaSeconds) override;

    // ============ Breath of Tower ============

    /** Current breath phase name (Inhale/Hold/Exhale/Pause) */
    UPROPERTY(BlueprintReadOnly, Replicated, Category = "Tower|World")
    FString BreathPhase = TEXT("Inhale");

    /** Phase progress 0.0 — 1.0 */
    UPROPERTY(BlueprintReadOnly, Replicated, Category = "Tower|World")
    float BreathProgress = 0.0f;

    /** Monster spawn multiplier from breath cycle */
    UPROPERTY(BlueprintReadOnly, Replicated, Category = "Tower|World")
    float MonsterSpawnMultiplier = 1.0f;

    /** Semantic field strength multiplier from breath cycle */
    UPROPERTY(BlueprintReadOnly, Replicated, Category = "Tower|World")
    float SemanticFieldStrength = 1.0f;

    /** Total game time in seconds */
    UPROPERTY(BlueprintReadOnly, Replicated, Category = "Tower|World")
    float TotalGameTime = 0.0f;

    // ============ Floor State ============

    /** Current floor all players are on */
    UPROPERTY(BlueprintReadOnly, Replicated, Category = "Tower|State")
    int32 ActiveFloor = 1;

    /** Number of monsters remaining on current floor */
    UPROPERTY(BlueprintReadOnly, Replicated, Category = "Tower|State")
    int32 MonstersRemaining = 0;

    /** Is the stairs unlocked (all monsters defeated)? */
    UPROPERTY(BlueprintReadOnly, Replicated, Category = "Tower|State")
    bool bStairsUnlocked = false;

    // ============ API ============

    /** Called by GameMode when a monster dies */
    UFUNCTION(BlueprintCallable, Category = "Tower|State")
    void OnMonsterDefeated();

    /** Update breath state from Rust core JSON */
    void UpdateBreathFromJson(const FString& BreathJson);

    virtual void GetLifetimeReplicatedProps(TArray<FLifetimeProperty>& OutLifetimeProps) const override;
};
