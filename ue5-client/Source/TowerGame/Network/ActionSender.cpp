#include "ActionSender.h"
#include "Dom/JsonObject.h"
#include "Serialization/JsonWriter.h"
#include "Serialization/JsonSerializer.h"
#include "Engine/World.h"
#include "GameFramework/PlayerState.h"
#include "GameFramework/Pawn.h"

// Forward-declared; include when the gRPC module is integrated
// #include "GRPCClient/TowerGRPCClientManager.h"

DEFINE_LOG_CATEGORY_STATIC(LogActionSender, Log, All);

// ============================================================================
// Lifecycle
// ============================================================================

UTowerActionSender::UTowerActionSender()
{
	PrimaryComponentTick.bCanEverTick = true;
	PrimaryComponentTick.TickInterval = 0.1f; // 10 Hz for timeout checks
}

void UTowerActionSender::BeginPlay()
{
	Super::BeginPlay();

	SequenceCounter = 0;
	PendingActions.Empty();
	LastActionTime.Empty();
	CachedClientManager = nullptr;

	UE_LOG(LogActionSender, Log, TEXT("ActionSender initialized on %s"), *GetOwner()->GetName());
}

void UTowerActionSender::EndPlay(const EEndPlayReason::Type EndPlayReason)
{
	if (PendingActions.Num() > 0)
	{
		UE_LOG(LogActionSender, Warning,
			TEXT("ActionSender shutting down with %d pending actions"), PendingActions.Num());
	}

	PendingActions.Empty();
	LastActionTime.Empty();
	CachedClientManager = nullptr;

	Super::EndPlay(EndPlayReason);
}

void UTowerActionSender::TickComponent(float DeltaTime, ELevelTick TickType,
	FActorComponentTickFunction* ThisTickFunction)
{
	Super::TickComponent(DeltaTime, TickType, ThisTickFunction);

	PurgeTimedOutActions();
}

// ============================================================================
// Send Actions
// ============================================================================

bool UTowerActionSender::SendMoveAction(FVector Direction, bool bSprinting)
{
	if (bEnableInputValidation && !ValidateDirection(Direction))
	{
		UE_LOG(LogActionSender, Warning, TEXT("SendMoveAction: invalid direction"));
		return false;
	}

	if (!CheckRateLimit(EPlayerActionType::Move)) return false;
	if (!CanEnqueueAction()) return false;

	FMoveActionData Data;
	Data.Direction = Direction.GetSafeNormal();
	Data.bSprinting = bSprinting;

	FPlayerActionPacket Packet = CreatePacket(EPlayerActionType::Move, SerializeMoveData(Data));
	return EnqueueAndSend(Packet);
}

bool UTowerActionSender::SendAttackAction(const FString& WeaponId, int32 ComboStep, FVector Direction)
{
	if (bEnableInputValidation)
	{
		if (!ValidateStringId(WeaponId))
		{
			UE_LOG(LogActionSender, Warning, TEXT("SendAttackAction: empty WeaponId"));
			return false;
		}
		if (ComboStep < 0)
		{
			UE_LOG(LogActionSender, Warning, TEXT("SendAttackAction: negative ComboStep %d"), ComboStep);
			return false;
		}
		if (!ValidateDirection(Direction))
		{
			UE_LOG(LogActionSender, Warning, TEXT("SendAttackAction: invalid direction"));
			return false;
		}
	}

	if (!CheckRateLimit(EPlayerActionType::Attack)) return false;
	if (!CanEnqueueAction()) return false;

	FAttackActionData Data;
	Data.WeaponId = WeaponId;
	Data.ComboStep = ComboStep;
	Data.Direction = Direction.GetSafeNormal();

	FPlayerActionPacket Packet = CreatePacket(EPlayerActionType::Attack, SerializeAttackData(Data));
	return EnqueueAndSend(Packet);
}

bool UTowerActionSender::SendParryAction(int64 TimingMs)
{
	if (bEnableInputValidation && TimingMs < 0)
	{
		UE_LOG(LogActionSender, Warning, TEXT("SendParryAction: negative TimingMs %lld"), TimingMs);
		return false;
	}

	if (!CheckRateLimit(EPlayerActionType::Parry)) return false;
	if (!CanEnqueueAction()) return false;

	FParryActionData Data;
	Data.TimingMs = TimingMs;

	FPlayerActionPacket Packet = CreatePacket(EPlayerActionType::Parry, SerializeParryData(Data));
	return EnqueueAndSend(Packet);
}

bool UTowerActionSender::SendDodgeAction(FVector Direction)
{
	if (bEnableInputValidation && !ValidateDirection(Direction))
	{
		UE_LOG(LogActionSender, Warning, TEXT("SendDodgeAction: invalid direction"));
		return false;
	}

	if (!CheckRateLimit(EPlayerActionType::Dodge)) return false;
	if (!CanEnqueueAction()) return false;

	FDodgeActionData Data;
	Data.Direction = Direction.GetSafeNormal();

	FPlayerActionPacket Packet = CreatePacket(EPlayerActionType::Dodge, SerializeDodgeData(Data));
	return EnqueueAndSend(Packet);
}

bool UTowerActionSender::SendAbilityAction(const FString& AbilityId, FVector TargetPos, uint64 TargetEntity)
{
	if (bEnableInputValidation && !ValidateStringId(AbilityId))
	{
		UE_LOG(LogActionSender, Warning, TEXT("SendAbilityAction: empty AbilityId"));
		return false;
	}

	if (!CheckRateLimit(EPlayerActionType::UseAbility)) return false;
	if (!CanEnqueueAction()) return false;

	FAbilityActionData Data;
	Data.AbilityId = AbilityId;
	Data.TargetPosition = TargetPos;
	Data.TargetEntity = TargetEntity;

	FPlayerActionPacket Packet = CreatePacket(EPlayerActionType::UseAbility, SerializeAbilityData(Data));
	return EnqueueAndSend(Packet);
}

bool UTowerActionSender::SendInteractAction(uint64 TargetEntity, const FString& InteractionType)
{
	if (bEnableInputValidation)
	{
		if (TargetEntity == 0)
		{
			UE_LOG(LogActionSender, Warning, TEXT("SendInteractAction: zero TargetEntity"));
			return false;
		}
		if (!ValidateStringId(InteractionType))
		{
			UE_LOG(LogActionSender, Warning, TEXT("SendInteractAction: empty InteractionType"));
			return false;
		}
	}

	if (!CheckRateLimit(EPlayerActionType::Interact)) return false;
	if (!CanEnqueueAction()) return false;

	FInteractActionData Data;
	Data.TargetEntity = TargetEntity;
	Data.InteractionType = InteractionType;

	FPlayerActionPacket Packet = CreatePacket(EPlayerActionType::Interact, SerializeInteractData(Data));
	return EnqueueAndSend(Packet);
}

// ============================================================================
// Result Processing
// ============================================================================

void UTowerActionSender::ProcessActionResult(const FActionResult& Result)
{
	// Remove from pending queue
	int32 RemovedIndex = PendingActions.IndexOfByPredicate(
		[&Result](const FPlayerActionPacket& Packet)
		{
			return Packet.SequenceNumber == Result.SequenceNumber;
		});

	if (RemovedIndex == INDEX_NONE)
	{
		UE_LOG(LogActionSender, Warning,
			TEXT("ProcessActionResult: seq %llu not found in pending (already timed out or duplicate)"),
			Result.SequenceNumber);
		return;
	}

	const FPlayerActionPacket& AckedPacket = PendingActions[RemovedIndex];
	const double RoundTripMs = (FPlatformTime::Seconds() - AckedPacket.LocalSendTime) * 1000.0;

	if (Result.bAccepted)
	{
		UE_LOG(LogActionSender, Verbose,
			TEXT("Action accepted: seq=%llu type=%d rtt=%.1fms"),
			Result.SequenceNumber, static_cast<int32>(AckedPacket.ActionType), RoundTripMs);

		OnActionAccepted.Broadcast(Result.SequenceNumber);
	}
	else
	{
		UE_LOG(LogActionSender, Warning,
			TEXT("Action rejected: seq=%llu reason=\"%s\" rtt=%.1fms"),
			Result.SequenceNumber, *Result.RejectionReason, RoundTripMs);

		OnActionRejected.Broadcast(Result.SequenceNumber, Result.RejectionReason);
	}

	PendingActions.RemoveAt(RemovedIndex);
}

bool UTowerActionSender::IsActionPending(uint64 SequenceNumber) const
{
	return PendingActions.ContainsByPredicate(
		[SequenceNumber](const FPlayerActionPacket& Packet)
		{
			return Packet.SequenceNumber == SequenceNumber;
		});
}

// ============================================================================
// Rate Limiting
// ============================================================================

bool UTowerActionSender::CheckRateLimit(EPlayerActionType ActionType)
{
	const double Now = FPlatformTime::Seconds();
	const double* LastTime = LastActionTime.Find(ActionType);

	if (LastTime && (Now - *LastTime) < static_cast<double>(MinActionInterval))
	{
		UE_LOG(LogActionSender, Verbose,
			TEXT("Rate limited: action type %d, %.0fms since last"),
			static_cast<int32>(ActionType), (Now - *LastTime) * 1000.0);
		return false;
	}

	LastActionTime.Add(ActionType, Now);
	return true;
}

bool UTowerActionSender::CanEnqueueAction() const
{
	if (PendingActions.Num() >= MaxPendingActions)
	{
		UE_LOG(LogActionSender, Warning,
			TEXT("Pending action queue full (%d/%d). Dropping action."),
			PendingActions.Num(), MaxPendingActions);
		return false;
	}
	return true;
}

// ============================================================================
// Validation
// ============================================================================

bool UTowerActionSender::ValidateDirection(const FVector& Direction) const
{
	if (Direction.IsNearlyZero(KINDA_SMALL_NUMBER))
	{
		return false;
	}
	if (!FMath::IsFinite(Direction.X) || !FMath::IsFinite(Direction.Y) || !FMath::IsFinite(Direction.Z))
	{
		return false;
	}
	return true;
}

bool UTowerActionSender::ValidateStringId(const FString& Id) const
{
	return !Id.IsEmpty() && Id.Len() <= 256;
}

// ============================================================================
// Packet Creation & Sending
// ============================================================================

FPlayerActionPacket UTowerActionSender::CreatePacket(EPlayerActionType ActionType, const FString& ActionDataJson)
{
	FPlayerActionPacket Packet;
	Packet.PlayerId = GetLocalPlayerId();
	Packet.ActionType = ActionType;
	Packet.ActionDataJson = ActionDataJson;
	Packet.Timestamp = FDateTime::UtcNow().ToUnixTimestamp() * 1000
		+ FDateTime::UtcNow().GetMillisecond();
	Packet.SequenceNumber = ++SequenceCounter;
	Packet.LocalSendTime = FPlatformTime::Seconds();
	return Packet;
}

bool UTowerActionSender::EnqueueAndSend(FPlayerActionPacket& Packet)
{
	PendingActions.Add(Packet);

	UTowerGRPCClientManager* Manager = GetClientManager();
	if (!Manager)
	{
		UE_LOG(LogActionSender, Warning,
			TEXT("No gRPC client manager available. Action seq=%llu queued locally."),
			Packet.SequenceNumber);
		return true; // Queued, will send when connection is available
	}

	// Build the wire-format JSON envelope for gRPC transmission
	TSharedRef<FJsonObject> Envelope = MakeShared<FJsonObject>();
	Envelope->SetNumberField(TEXT("player_id"), static_cast<double>(Packet.PlayerId));
	Envelope->SetStringField(TEXT("action_type"), UEnum::GetValueAsString(Packet.ActionType));
	Envelope->SetStringField(TEXT("action_data"), Packet.ActionDataJson);
	Envelope->SetNumberField(TEXT("timestamp"), static_cast<double>(Packet.Timestamp));
	Envelope->SetNumberField(TEXT("sequence_number"), static_cast<double>(Packet.SequenceNumber));

	FString EnvelopeJson;
	TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&EnvelopeJson);
	FJsonSerializer::Serialize(Envelope, Writer);

	// TODO: Replace with actual gRPC call when UTowerGRPCClientManager is implemented
	// Manager->SendPlayerAction(EnvelopeJson);

	UE_LOG(LogActionSender, Verbose,
		TEXT("Sent action: seq=%llu type=%s"),
		Packet.SequenceNumber, *UEnum::GetValueAsString(Packet.ActionType));

	return true;
}

// ============================================================================
// JSON Serialization
// ============================================================================

FString UTowerActionSender::SerializeMoveData(const FMoveActionData& Data)
{
	TSharedRef<FJsonObject> Json = MakeShared<FJsonObject>();
	Json->SetNumberField(TEXT("dir_x"), Data.Direction.X);
	Json->SetNumberField(TEXT("dir_y"), Data.Direction.Y);
	Json->SetNumberField(TEXT("dir_z"), Data.Direction.Z);
	Json->SetBoolField(TEXT("sprinting"), Data.bSprinting);

	FString Output;
	TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&Output);
	FJsonSerializer::Serialize(Json, Writer);
	return Output;
}

FString UTowerActionSender::SerializeAttackData(const FAttackActionData& Data)
{
	TSharedRef<FJsonObject> Json = MakeShared<FJsonObject>();
	Json->SetStringField(TEXT("weapon_id"), Data.WeaponId);
	Json->SetNumberField(TEXT("combo_step"), Data.ComboStep);
	Json->SetNumberField(TEXT("dir_x"), Data.Direction.X);
	Json->SetNumberField(TEXT("dir_y"), Data.Direction.Y);
	Json->SetNumberField(TEXT("dir_z"), Data.Direction.Z);

	FString Output;
	TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&Output);
	FJsonSerializer::Serialize(Json, Writer);
	return Output;
}

FString UTowerActionSender::SerializeParryData(const FParryActionData& Data)
{
	TSharedRef<FJsonObject> Json = MakeShared<FJsonObject>();
	Json->SetNumberField(TEXT("timing_ms"), static_cast<double>(Data.TimingMs));

	FString Output;
	TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&Output);
	FJsonSerializer::Serialize(Json, Writer);
	return Output;
}

FString UTowerActionSender::SerializeDodgeData(const FDodgeActionData& Data)
{
	TSharedRef<FJsonObject> Json = MakeShared<FJsonObject>();
	Json->SetNumberField(TEXT("dir_x"), Data.Direction.X);
	Json->SetNumberField(TEXT("dir_y"), Data.Direction.Y);
	Json->SetNumberField(TEXT("dir_z"), Data.Direction.Z);

	FString Output;
	TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&Output);
	FJsonSerializer::Serialize(Json, Writer);
	return Output;
}

FString UTowerActionSender::SerializeAbilityData(const FAbilityActionData& Data)
{
	TSharedRef<FJsonObject> Json = MakeShared<FJsonObject>();
	Json->SetStringField(TEXT("ability_id"), Data.AbilityId);
	Json->SetNumberField(TEXT("target_x"), Data.TargetPosition.X);
	Json->SetNumberField(TEXT("target_y"), Data.TargetPosition.Y);
	Json->SetNumberField(TEXT("target_z"), Data.TargetPosition.Z);
	Json->SetNumberField(TEXT("target_entity"), static_cast<double>(Data.TargetEntity));

	FString Output;
	TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&Output);
	FJsonSerializer::Serialize(Json, Writer);
	return Output;
}

FString UTowerActionSender::SerializeInteractData(const FInteractActionData& Data)
{
	TSharedRef<FJsonObject> Json = MakeShared<FJsonObject>();
	Json->SetNumberField(TEXT("target_entity"), static_cast<double>(Data.TargetEntity));
	Json->SetStringField(TEXT("interaction_type"), Data.InteractionType);

	FString Output;
	TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&Output);
	FJsonSerializer::Serialize(Json, Writer);
	return Output;
}

// ============================================================================
// Timeout Management
// ============================================================================

void UTowerActionSender::PurgeTimedOutActions()
{
	if (PendingActions.Num() == 0) return;

	const double Now = FPlatformTime::Seconds();
	const double TimeoutThreshold = static_cast<double>(PendingActionTimeout);

	for (int32 i = PendingActions.Num() - 1; i >= 0; --i)
	{
		const FPlayerActionPacket& Packet = PendingActions[i];
		const double Age = Now - Packet.LocalSendTime;

		if (Age >= TimeoutThreshold)
		{
			UE_LOG(LogActionSender, Warning,
				TEXT("Action timed out: seq=%llu type=%s age=%.1fs"),
				Packet.SequenceNumber,
				*UEnum::GetValueAsString(Packet.ActionType),
				Age);

			OnActionRejected.Broadcast(Packet.SequenceNumber, TEXT("Timeout"));
			PendingActions.RemoveAt(i);
		}
	}
}

// ============================================================================
// Utilities
// ============================================================================

UTowerGRPCClientManager* UTowerActionSender::GetClientManager()
{
	if (CachedClientManager)
	{
		return CachedClientManager;
	}

	// UTowerGRPCClientManager is expected to be a GameInstanceSubsystem
	UGameInstance* GI = GetOwner() ? GetOwner()->GetGameInstance() : nullptr;
	if (GI)
	{
		// TODO: Uncomment when UTowerGRPCClientManager is implemented
		// CachedClientManager = GI->GetSubsystem<UTowerGRPCClientManager>();
	}

	return CachedClientManager;
}

uint64 UTowerActionSender::GetLocalPlayerId() const
{
	APawn* OwnerPawn = Cast<APawn>(GetOwner());
	if (!OwnerPawn) return 0;

	APlayerState* PS = OwnerPawn->GetPlayerState();
	if (!PS) return 0;

	// Use the engine's unique player ID; the server maps this to its own ID space
	return static_cast<uint64>(PS->GetPlayerId());
}
