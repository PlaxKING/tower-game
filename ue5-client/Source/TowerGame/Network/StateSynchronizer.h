#pragma once

#include "CoreMinimal.h"
#include "Components/ActorComponent.h"
#include "StateSynchronizer.generated.h"

class UMatchConnection;

// ============================================================================
// Enums
// ============================================================================

/** World cycle phase — mirrors Rust BreathPhase */
UENUM(BlueprintType)
enum class EWorldCyclePhase : uint8
{
	Inhale   UMETA(DisplayName = "Inhale"),
	Hold     UMETA(DisplayName = "Hold"),
	Exhale   UMETA(DisplayName = "Exhale"),
	Pause    UMETA(DisplayName = "Pause"),
};

/** Action types that can be predicted client-side */
UENUM(BlueprintType)
enum class EPredictedActionType : uint8
{
	None,
	Move,
	Attack,
	Dash,
	Jump,
	UseAbility,
	Interact,
	Dodge,
};

/** Status effects for monsters — mirrors Rust StatusType */
UENUM(BlueprintType)
enum class EMonsterStatusEffect : uint8
{
	None,
	Burning,
	Poisoned,
	Bleeding,
	Stunned,
	Frozen,
	Silenced,
	Weakened,
	Slowed,
	Exposed,
	Corrupted,
	Empowered,
	Hastened,
	Shielded,
	Regenerating,
	SemanticFocus,
};

/** Monster combat phase — mirrors Rust AttackPhase */
UENUM(BlueprintType)
enum class EMonsterCombatPhase : uint8
{
	Idle,
	Windup,
	Active,
	Recovery,
};

// ============================================================================
// Snapshot Structs
// ============================================================================

/** Snapshot of a single player's state at a point in time */
USTRUCT(BlueprintType)
struct FPlayerStateSnapshot
{
	GENERATED_BODY()

	/** Unique entity identifier from the Rust core */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	int64 EntityId = 0;

	/** World position */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	FVector Position = FVector::ZeroVector;

	/** Rotation */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	FRotator Rotation = FRotator::ZeroRotator;

	/** Current health */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	float Health = 100.0f;

	/** Current resources (kinetic, thermal, semantic, rage packed as XYZW) */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	FVector4 Resources = FVector4(0.0, 0.0, 0.0, 0.0);

	/** Server timestamp when this snapshot was taken (seconds) */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	double Timestamp = 0.0;
};

/** Snapshot of a single monster's state at a point in time */
USTRUCT(BlueprintType)
struct FMonsterStateSnapshot
{
	GENERATED_BODY()

	/** Unique entity identifier (entity_hash from Rust replication) */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	int64 EntityId = 0;

	/** World position */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	FVector Position = FVector::ZeroVector;

	/** Current health */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	float Health = 0.0f;

	/** Current combat phase */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	EMonsterCombatPhase CombatPhase = EMonsterCombatPhase::Idle;

	/** Active status effects on this monster */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	TArray<EMonsterStatusEffect> StatusEffects;
};

/** Full world state buffer containing all entity snapshots for one tick */
USTRUCT(BlueprintType)
struct FWorldStateBuffer
{
	GENERATED_BODY()

	/** All player snapshots in this frame */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	TArray<FPlayerStateSnapshot> PlayerSnapshots;

	/** All monster snapshots in this frame */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	TArray<FMonsterStateSnapshot> MonsterSnapshots;

	/** Current world cycle phase (Breath of Tower) */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	EWorldCyclePhase WorldCyclePhase = EWorldCyclePhase::Inhale;

	/** Authoritative server tick number */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	int64 ServerTick = 0;

	/** Server timestamp when this buffer was created */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	double ServerTimestamp = 0.0;

	bool IsValid() const { return ServerTick > 0; }
};

/** A pending client-side predicted action awaiting server confirmation */
USTRUCT(BlueprintType)
struct FPendingAction
{
	GENERATED_BODY()

	/** Monotonically increasing sequence number */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	int64 SequenceNumber = 0;

	/** Type of action being predicted */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	EPredictedActionType ActionType = EPredictedActionType::None;

	/** Client timestamp when action was issued */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	double Timestamp = 0.0;

	/** Client-predicted position after this action */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	FVector PredictedPosition = FVector::ZeroVector;

	/** Client-predicted rotation after this action */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	FRotator PredictedRotation = FRotator::ZeroRotator;

	/** Client-predicted health after this action */
	UPROPERTY(BlueprintReadOnly, Category = "Sync")
	float PredictedHealth = 0.0f;
};

// ============================================================================
// Delegates
// ============================================================================

/** Broadcast when a new authoritative world state arrives from the server */
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnStateUpdated, const FWorldStateBuffer&, NewState);

/** Broadcast when the client prediction was wrong and a correction was applied */
DECLARE_DYNAMIC_MULTICAST_DELEGATE_ThreeParams(
	FOnPredictionCorrected,
	int64, SequenceNumber,
	FVector, CorrectedPosition,
	FVector, PredictedPosition
);

/** Broadcast when significant desync is detected between client and server */
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(
	FOnDesyncDetected,
	float, DesyncDistance,
	int64, ServerTick
);

// ============================================================================
// UTowerStateSynchronizer
// ============================================================================

/**
 * Handles real-time state synchronization between the UE5 client and
 * the Rust procedural core server.
 *
 * Responsibilities:
 * - Periodic state polling from the Rust server at a configurable tick rate
 * - Client-side prediction for player movement and actions
 * - Server reconciliation when authoritative state arrives
 * - State interpolation between snapshots for smooth visual updates
 * - Delta compression: only sync changed state to reduce bandwidth
 *
 * Attach to the player controller or game mode. Uses UMatchConnection
 * for communication with the server.
 *
 * Prediction model:
 * 1. Client issues an action (move, attack, etc.)
 * 2. Action is applied locally immediately for responsiveness
 * 3. Action is queued as FPendingAction with a sequence number
 * 4. When server confirms, the pending action is removed
 * 5. If server state diverges, reconciliation replays un-acked actions
 *
 * Interpolation model:
 * - Rendering runs at InterpolationDelay behind the latest server state
 * - Smooth lerp between the two most recent snapshots
 * - Teleport if gap exceeds TeleportThreshold
 */
UCLASS(ClassGroup = (Network), meta = (BlueprintSpawnableComponent))
class TOWERGAME_API UTowerStateSynchronizer : public UActorComponent
{
	GENERATED_BODY()

public:
	UTowerStateSynchronizer();

	virtual void BeginPlay() override;
	virtual void EndPlay(const EEndPlayReason::Type EndPlayReason) override;
	virtual void TickComponent(float DeltaTime, ELevelTick TickType,
		FActorComponentTickFunction* ThisTickFunction) override;

	// ============ Configuration ============

	/** How many state polls per second to request from the server (Hz) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Sync|Config", meta = (ClampMin = "1", ClampMax = "60"))
	float SyncRate = 20.0f;

	/** Delay in seconds for interpolation buffer (renders behind real-time) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Sync|Config", meta = (ClampMin = "0.0", ClampMax = "0.5"))
	float InterpolationDelay = 0.1f;

	/** Enable client-side prediction for local player actions */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Sync|Config")
	bool bPredictionEnabled = true;

	/** Maximum number of unconfirmed predicted actions before stalling input */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Sync|Config", meta = (ClampMin = "4", ClampMax = "128"))
	int32 MaxPendingActions = 32;

	/** Distance threshold to trigger a teleport instead of interpolation */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Sync|Config")
	float TeleportThreshold = 500.0f;

	/** Distance threshold to consider a prediction a desync */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Sync|Config")
	float DesyncThreshold = 50.0f;

	/** Maximum number of snapshots retained in the circular buffer */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Sync|Config", meta = (ClampMin = "8", ClampMax = "256"))
	int32 MaxSnapshotBufferSize = 64;

	// ============ Controls ============

	/** Begin synchronization with the server. Call after match connection is established. */
	UFUNCTION(BlueprintCallable, Category = "Sync")
	void BeginSync();

	/** Stop synchronization and clear all buffers. */
	UFUNCTION(BlueprintCallable, Category = "Sync")
	void StopSync();

	/** Is the synchronizer currently active? */
	UFUNCTION(BlueprintPure, Category = "Sync")
	bool IsSyncing() const { return bSyncing; }

	// ============ State Access ============

	/**
	 * Get the interpolated world state for the current render frame.
	 * This blends between the two snapshots surrounding the interpolation time.
	 */
	UFUNCTION(BlueprintPure, Category = "Sync")
	FWorldStateBuffer GetInterpolatedState() const;

	/** Get the latest raw (non-interpolated) server state */
	UFUNCTION(BlueprintPure, Category = "Sync")
	FWorldStateBuffer GetLatestServerState() const;

	/** Get the current number of buffered snapshots */
	UFUNCTION(BlueprintPure, Category = "Sync")
	int32 GetBufferedSnapshotCount() const { return SnapshotBuffer.Num(); }

	/** Get the number of pending (unconfirmed) actions */
	UFUNCTION(BlueprintPure, Category = "Sync")
	int32 GetPendingActionCount() const { return PendingActions.Num(); }

	/** Get current estimated round-trip time in seconds */
	UFUNCTION(BlueprintPure, Category = "Sync")
	float GetEstimatedRTT() const { return EstimatedRTT; }

	/** Get the last known server tick */
	UFUNCTION(BlueprintPure, Category = "Sync")
	int64 GetLastServerTick() const;

	// ============ Prediction ============

	/**
	 * Register a predicted action. The action is applied locally immediately
	 * and queued for server reconciliation.
	 * @return The sequence number assigned to this prediction, or -1 if the
	 *         pending queue is full.
	 */
	UFUNCTION(BlueprintCallable, Category = "Sync")
	int64 PredictAction(EPredictedActionType ActionType, FVector PredictedPosition,
		FRotator PredictedRotation, float PredictedHealth);

	/**
	 * Called internally when server state arrives. Compares server authority
	 * against pending predictions and corrects if needed.
	 */
	void ReconcileState(const FWorldStateBuffer& AuthoritativeState);

	// ============ Events ============

	/** Broadcast when a new interpolated state is available */
	UPROPERTY(BlueprintAssignable, Category = "Sync|Events")
	FOnStateUpdated OnStateUpdated;

	/** Broadcast when a prediction was incorrect and had to be corrected */
	UPROPERTY(BlueprintAssignable, Category = "Sync|Events")
	FOnPredictionCorrected OnPredictionCorrected;

	/** Broadcast when a significant desync is detected */
	UPROPERTY(BlueprintAssignable, Category = "Sync|Events")
	FOnDesyncDetected OnDesyncDetected;

	// ============ Debug ============

	/** Enable verbose logging for sync diagnostics */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Sync|Debug")
	bool bDebugLogging = false;

private:
	// ============ Internal State ============

	/** Whether the synchronizer is currently active */
	bool bSyncing = false;

	/** Timer accumulator for sync polling interval */
	float SyncTimer = 0.0f;

	/** Current render interpolation time (behind real-time by InterpolationDelay) */
	double InterpolationTime = 0.0;

	/** Next sequence number for predicted actions */
	int64 NextSequenceNumber = 1;

	/** Last confirmed server tick for delta detection */
	int64 LastConfirmedServerTick = 0;

	/** Estimated round-trip time in seconds */
	float EstimatedRTT = 0.0f;

	/** Smoothed RTT for jitter compensation */
	float SmoothedRTT = 0.0f;

	/** Timestamp of the last poll request sent */
	double LastPollSentTime = 0.0;

	// ============ Buffers ============

	/** Circular buffer of received world state snapshots for interpolation */
	TArray<FWorldStateBuffer> SnapshotBuffer;

	/** Queue of actions predicted locally but not yet confirmed by server */
	TArray<FPendingAction> PendingActions;

	/**
	 * Previous snapshot data for delta detection.
	 * Key: EntityId, Value: last synced position hash.
	 * Used to skip re-processing entities whose state has not changed.
	 */
	TMap<int64, int32> PreviousEntityStateHashes;

	// ============ Internal Methods ============

	/** Get the match connection subsystem */
	UMatchConnection* GetMatchConnection() const;

	/** Poll the server for the latest world state */
	void PollServerState();

	/** Push a new snapshot into the circular buffer, evicting the oldest if full */
	void BufferSnapshot(const FWorldStateBuffer& Snapshot);

	/** Interpolate between two world state buffers at the given alpha */
	FWorldStateBuffer LerpWorldState(const FWorldStateBuffer& A, const FWorldStateBuffer& B, float Alpha) const;

	/** Interpolate a single player snapshot */
	FPlayerStateSnapshot LerpPlayerSnapshot(const FPlayerStateSnapshot& A, const FPlayerStateSnapshot& B, float Alpha) const;

	/** Interpolate a single monster snapshot */
	FMonsterStateSnapshot LerpMonsterSnapshot(const FMonsterStateSnapshot& A, const FMonsterStateSnapshot& B, float Alpha) const;

	/** Remove all pending actions with sequence number <= the given value */
	void AcknowledgeActionsUpTo(int64 SequenceNumber);

	/** Re-apply un-acknowledged pending actions on top of server state */
	FVector ReplayPendingActions(FVector ServerPosition) const;

	/** Compute a simple hash of an entity's mutable state for delta detection */
	int32 ComputeEntityStateHash(const FPlayerStateSnapshot& Snapshot) const;
	int32 ComputeEntityStateHash(const FMonsterStateSnapshot& Snapshot) const;

	/** Check if an entity's state has changed since the last sync */
	bool HasEntityStateChanged(int64 EntityId, int32 NewHash) const;

	/** Parse incoming JSON state data from the match connection */
	bool ParseWorldStateFromJson(const FString& JsonString, FWorldStateBuffer& OutState) const;

	/** Handle incoming match data that contains world state */
	UFUNCTION()
	void OnMatchDataReceived(EMatchOpCode OpCode, const FString& DataJson);

	/** Update RTT estimate based on poll round-trip */
	void UpdateRTTEstimate(double SendTime, double ReceiveTime);
};
