#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "EchoGhost.generated.h"

class UStaticMeshComponent;
class UNiagaraComponent;

/**
 * Echo Ghost — visual representation of another player's death.
 *
 * Echo types (from Rust death system):
 * - Lingering: stands still, faint glow, basic threat
 * - Aggressive: patrols area, attacks nearby players
 * - Helpful: provides buff zone, heals passersby
 * - Warning: marks dangerous area, flashes red
 *
 * Echoes appear translucent with an ethereal glow.
 * They fade out after their server-side TTL expires (24h default).
 */
UENUM(BlueprintType)
enum class EEchoType : uint8
{
    Lingering    UMETA(DisplayName = "Lingering"),
    Aggressive   UMETA(DisplayName = "Aggressive"),
    Helpful      UMETA(DisplayName = "Helpful"),
    Warning      UMETA(DisplayName = "Warning"),
};

UCLASS()
class TOWERGAME_API AEchoGhost : public AActor
{
    GENERATED_BODY()

public:
    AEchoGhost();

    virtual void BeginPlay() override;
    virtual void Tick(float DeltaTime) override;

    /** Initialize echo from server data */
    UFUNCTION(BlueprintCallable, Category = "Echo")
    void InitFromData(const FString& PlayerName, EEchoType Type, FVector SpawnPosition);

    // ============ Echo Data ============

    UPROPERTY(BlueprintReadOnly, Category = "Echo")
    FString OriginalPlayerName;

    UPROPERTY(BlueprintReadOnly, Category = "Echo")
    EEchoType EchoType = EEchoType::Lingering;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Echo")
    float LifetimeSeconds = 300.0f; // 5 minutes client-side display

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Echo")
    float PulseSpeed = 2.0f;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Echo")
    float BobHeight = 20.0f;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Echo")
    float BobSpeed = 1.5f;

    // ============ Echo Interaction ============

    /** Radius for helpful/warning effects */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Echo")
    float EffectRadius = 300.0f;

    /** Damage for aggressive echoes */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Echo")
    float AggressiveDamage = 15.0f;

    /** Heal amount for helpful echoes */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Echo")
    float HelpfulHealPerSecond = 5.0f;

    // ============ Components ============

    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Echo")
    UStaticMeshComponent* GhostMesh;

private:
    float TimeAlive = 0.0f;
    FVector OriginalPosition;

    FLinearColor GetEchoColor() const;
    void UpdateGhostMaterial();
    void ApplyEchoEffect(float DeltaTime);
};
