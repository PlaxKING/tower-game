#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Character.h"
#include "RemotePlayer.generated.h"

class UStaticMeshComponent;

/**
 * Represents another player on the same floor.
 *
 * Receives position/rotation updates from the match handler via WebSocket
 * and smoothly interpolates between them. Shows combat actions (attacks,
 * dodges, deaths) from the remote player.
 *
 * Spawned/despawned by UPlayerSyncComponent when PlayerJoined/PlayerLeft
 * op codes are received.
 */
UCLASS()
class TOWERGAME_API ARemotePlayer : public ACharacter
{
    GENERATED_BODY()

public:
    ARemotePlayer();

    virtual void Tick(float DeltaTime) override;

    // ============ Identity ============

    /** Nakama user ID of this remote player */
    UPROPERTY(BlueprintReadOnly, Category = "RemotePlayer")
    FString UserId;

    /** Display name */
    UPROPERTY(BlueprintReadOnly, Category = "RemotePlayer")
    FString DisplayName;

    // ============ Sync State ============

    /** Apply a position update from the match handler */
    UFUNCTION(BlueprintCallable, Category = "RemotePlayer")
    void ApplyPositionUpdate(FVector NewPosition, FRotator NewRotation);

    /** Apply an attack animation */
    UFUNCTION(BlueprintCallable, Category = "RemotePlayer")
    void PlayAttackAnimation(int32 ComboStep, int32 WeaponType);

    /** Apply dodge animation */
    UFUNCTION(BlueprintCallable, Category = "RemotePlayer")
    void PlayDodgeAnimation();

    /** Show death (ragdoll or collapse) */
    UFUNCTION(BlueprintCallable, Category = "RemotePlayer")
    void ShowDeath();

    /** Reset to alive state */
    UFUNCTION(BlueprintCallable, Category = "RemotePlayer")
    void Respawn(FVector SpawnLocation);

    // ============ Config ============

    /** How fast to interpolate position (units per second) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "RemotePlayer")
    float InterpSpeed = 15.0f;

    /** Max distance before teleporting instead of interpolating */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "RemotePlayer")
    float TeleportThreshold = 500.0f;

    /** Rotation interpolation speed */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "RemotePlayer")
    float RotInterpSpeed = 10.0f;

    // ============ Visual State ============

    UPROPERTY(BlueprintReadOnly, Category = "RemotePlayer")
    bool bIsDead = false;

    UPROPERTY(BlueprintReadOnly, Category = "RemotePlayer")
    bool bIsAttacking = false;

    UPROPERTY(BlueprintReadOnly, Category = "RemotePlayer")
    int32 CurrentComboStep = 0;

    UPROPERTY(BlueprintReadOnly, Category = "RemotePlayer")
    float Speed = 0.0f;

    /** Nameplate mesh above head */
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "RemotePlayer")
    UStaticMeshComponent* NameplateMesh;

private:
    FVector TargetPosition;
    FRotator TargetRotation;
    FVector PreviousPosition;
    float TimeSinceLastUpdate = 0.0f;
    bool bHasReceivedUpdate = false;
};
