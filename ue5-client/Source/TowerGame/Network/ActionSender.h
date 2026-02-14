#pragma once

#include "CoreMinimal.h"
#include "Components/ActorComponent.h"
#include "ActionSender.generated.h"

class UTowerGRPCClientManager;

// ============ Action Type Enum ============

UENUM(BlueprintType)
enum class EPlayerActionType : uint8
{
	Move        UMETA(DisplayName = "Move"),
	Attack      UMETA(DisplayName = "Attack"),
	Parry       UMETA(DisplayName = "Parry"),
	Dodge       UMETA(DisplayName = "Dodge"),
	UseAbility  UMETA(DisplayName = "Use Ability"),
	Interact    UMETA(DisplayName = "Interact"),
};

// ============ Action Data Structs ============

USTRUCT(BlueprintType)
struct FMoveActionData
{
	GENERATED_BODY()

	UPROPERTY(EditAnywhere, BlueprintReadWrite)
	FVector Direction = FVector::ZeroVector;

	UPROPERTY(EditAnywhere, BlueprintReadWrite)
	bool bSprinting = false;
};

USTRUCT(BlueprintType)
struct FAttackActionData
{
	GENERATED_BODY()

	UPROPERTY(EditAnywhere, BlueprintReadWrite)
	FString WeaponId;

	UPROPERTY(EditAnywhere, BlueprintReadWrite)
	int32 ComboStep = 0;

	UPROPERTY(EditAnywhere, BlueprintReadWrite)
	FVector Direction = FVector::ZeroVector;
};

USTRUCT(BlueprintType)
struct FParryActionData
{
	GENERATED_BODY()

	UPROPERTY(EditAnywhere, BlueprintReadWrite)
	int64 TimingMs = 0;
};

USTRUCT(BlueprintType)
struct FDodgeActionData
{
	GENERATED_BODY()

	UPROPERTY(EditAnywhere, BlueprintReadWrite)
	FVector Direction = FVector::ZeroVector;
};

USTRUCT(BlueprintType)
struct FAbilityActionData
{
	GENERATED_BODY()

	UPROPERTY(EditAnywhere, BlueprintReadWrite)
	FString AbilityId;

	UPROPERTY(EditAnywhere, BlueprintReadWrite)
	FVector TargetPosition = FVector::ZeroVector;

	UPROPERTY(EditAnywhere, BlueprintReadWrite)
	uint64 TargetEntity = 0;
};

USTRUCT(BlueprintType)
struct FInteractActionData
{
	GENERATED_BODY()

	UPROPERTY(EditAnywhere, BlueprintReadWrite)
	uint64 TargetEntity = 0;

	UPROPERTY(EditAnywhere, BlueprintReadWrite)
	FString InteractionType;
};

USTRUCT(BlueprintType)
struct FPlayerActionPacket
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadOnly)
	uint64 PlayerId = 0;

	UPROPERTY(BlueprintReadOnly)
	EPlayerActionType ActionType = EPlayerActionType::Move;

	/** Action-specific payload serialized as JSON */
	UPROPERTY(BlueprintReadOnly)
	FString ActionDataJson;

	/** Server timestamp at send time (ms since epoch) */
	UPROPERTY(BlueprintReadOnly)
	int64 Timestamp = 0;

	UPROPERTY(BlueprintReadOnly)
	uint64 SequenceNumber = 0;

	/** Local time when this packet was created (for timeout tracking) */
	double LocalSendTime = 0.0;
};

USTRUCT(BlueprintType)
struct FActionResult
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadOnly)
	uint64 SequenceNumber = 0;

	UPROPERTY(BlueprintReadOnly)
	bool bAccepted = false;

	UPROPERTY(BlueprintReadOnly)
	FString RejectionReason;

	/** Server-authoritative state changes as JSON (position corrections, HP changes, etc.) */
	UPROPERTY(BlueprintReadOnly)
	FString StateChangesJson;
};

// ============ Delegates ============

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnActionAccepted, uint64, SequenceNumber);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnActionRejected, uint64, SequenceNumber, const FString&, Reason);

/**
 * Queues and sends player actions to the Rust procedural core via gRPC.
 *
 * Responsibilities:
 * - Validates player input before sending
 * - Assigns monotonic sequence numbers for prediction rollback
 * - Rate-limits each action type to prevent spam
 * - Queues actions and transmits via UTowerGRPCClientManager
 * - Tracks pending (unacknowledged) actions for client prediction
 * - Times out stale actions that never received a server response
 *
 * Attach to the local player character (ATowerPlayerCharacter).
 */
UCLASS(ClassGroup = (Network), meta = (BlueprintSpawnableComponent))
class TOWERGAME_API UTowerActionSender : public UActorComponent
{
	GENERATED_BODY()

public:
	UTowerActionSender();

	virtual void BeginPlay() override;
	virtual void EndPlay(const EEndPlayReason::Type EndPlayReason) override;
	virtual void TickComponent(float DeltaTime, ELevelTick TickType, FActorComponentTickFunction* ThisTickFunction) override;

	// ============ Config ============

	/** Maximum number of unacknowledged actions before blocking new sends */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "ActionSender|Config", meta = (ClampMin = "1", ClampMax = "128"))
	int32 MaxPendingActions = 32;

	/** Minimum interval between actions of the same type (seconds) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "ActionSender|Config", meta = (ClampMin = "0.0"))
	float MinActionInterval = 0.05f;

	/** Enable input validation before sending */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "ActionSender|Config")
	bool bEnableInputValidation = true;

	/** Timeout in seconds for pending actions before they are considered lost */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "ActionSender|Config", meta = (ClampMin = "0.5"))
	float PendingActionTimeout = 5.0f;

	// ============ Send Actions ============

	UFUNCTION(BlueprintCallable, Category = "ActionSender")
	bool SendMoveAction(FVector Direction, bool bSprinting);

	UFUNCTION(BlueprintCallable, Category = "ActionSender")
	bool SendAttackAction(const FString& WeaponId, int32 ComboStep, FVector Direction);

	UFUNCTION(BlueprintCallable, Category = "ActionSender")
	bool SendParryAction(int64 TimingMs);

	UFUNCTION(BlueprintCallable, Category = "ActionSender")
	bool SendDodgeAction(FVector Direction);

	UFUNCTION(BlueprintCallable, Category = "ActionSender")
	bool SendAbilityAction(const FString& AbilityId, FVector TargetPos, uint64 TargetEntity);

	UFUNCTION(BlueprintCallable, Category = "ActionSender")
	bool SendInteractAction(uint64 TargetEntity, const FString& InteractionType);

	// ============ Result Processing ============

	/** Called when the server responds with an action result */
	UFUNCTION(BlueprintCallable, Category = "ActionSender")
	void ProcessActionResult(const FActionResult& Result);

	/** Number of actions awaiting server acknowledgement */
	UFUNCTION(BlueprintPure, Category = "ActionSender")
	int32 GetPendingActionCount() const { return PendingActions.Num(); }

	/** Check if a specific sequence number is still pending */
	UFUNCTION(BlueprintPure, Category = "ActionSender")
	bool IsActionPending(uint64 SequenceNumber) const;

	// ============ Events ============

	UPROPERTY(BlueprintAssignable, Category = "ActionSender|Events")
	FOnActionAccepted OnActionAccepted;

	UPROPERTY(BlueprintAssignable, Category = "ActionSender|Events")
	FOnActionRejected OnActionRejected;

private:
	/** Monotonically increasing sequence counter */
	uint64 SequenceCounter = 0;

	/** Actions sent but not yet acknowledged by the server */
	UPROPERTY()
	TArray<FPlayerActionPacket> PendingActions;

	/** Last send time per action type for rate limiting */
	TMap<EPlayerActionType, double> LastActionTime;

	/** Cached reference to the gRPC client manager */
	UPROPERTY()
	TObjectPtr<UTowerGRPCClientManager> CachedClientManager;

	// ============ Internal Helpers ============

	/** Find the gRPC client manager subsystem */
	UTowerGRPCClientManager* GetClientManager();

	/** Check rate limit for the given action type. Returns true if allowed. */
	bool CheckRateLimit(EPlayerActionType ActionType);

	/** Check if we can accept another pending action */
	bool CanEnqueueAction() const;

	/** Validate a direction vector (non-zero, finite) */
	bool ValidateDirection(const FVector& Direction) const;

	/** Validate a string ID is non-empty */
	bool ValidateStringId(const FString& Id) const;

	/** Create a packet, assign sequence number, set timestamp */
	FPlayerActionPacket CreatePacket(EPlayerActionType ActionType, const FString& ActionDataJson);

	/** Enqueue packet and transmit via gRPC */
	bool EnqueueAndSend(FPlayerActionPacket& Packet);

	/** Serialize action data structs to JSON */
	static FString SerializeMoveData(const FMoveActionData& Data);
	static FString SerializeAttackData(const FAttackActionData& Data);
	static FString SerializeParryData(const FParryActionData& Data);
	static FString SerializeDodgeData(const FDodgeActionData& Data);
	static FString SerializeAbilityData(const FAbilityActionData& Data);
	static FString SerializeInteractData(const FInteractActionData& Data);

	/** Remove timed-out actions from the pending queue */
	void PurgeTimedOutActions();

	/** Get current player ID from the owning player state */
	uint64 GetLocalPlayerId() const;
};
