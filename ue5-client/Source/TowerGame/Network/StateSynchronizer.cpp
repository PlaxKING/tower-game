#include "StateSynchronizer.h"
#include "MatchConnection.h"
#include "Kismet/GameplayStatics.h"
#include "Dom/JsonObject.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"
#include "Serialization/JsonWriter.h"
#include "Engine/World.h"

DEFINE_LOG_CATEGORY_STATIC(LogStateSync, Log, All);

// ============================================================================
// Construction & Lifecycle
// ============================================================================

UTowerStateSynchronizer::UTowerStateSynchronizer()
{
	PrimaryComponentTick.bCanEverTick = true;
	// Tick every frame for smooth interpolation; polling is rate-limited internally
	PrimaryComponentTick.TickInterval = 0.0f;
}

void UTowerStateSynchronizer::BeginPlay()
{
	Super::BeginPlay();

	UMatchConnection* Match = GetMatchConnection();
	if (Match)
	{
		Match->OnMatchData.AddDynamic(this, &UTowerStateSynchronizer::OnMatchDataReceived);
	}
}

void UTowerStateSynchronizer::EndPlay(const EEndPlayReason::Type EndPlayReason)
{
	StopSync();

	UMatchConnection* Match = GetMatchConnection();
	if (Match)
	{
		Match->OnMatchData.RemoveDynamic(this, &UTowerStateSynchronizer::OnMatchDataReceived);
	}

	Super::EndPlay(EndPlayReason);
}

// ============================================================================
// Tick
// ============================================================================

void UTowerStateSynchronizer::TickComponent(float DeltaTime, ELevelTick TickType,
	FActorComponentTickFunction* ThisTickFunction)
{
	Super::TickComponent(DeltaTime, TickType, ThisTickFunction);

	if (!bSyncing) return;

	// Advance interpolation time
	InterpolationTime += DeltaTime;

	// Rate-limited server polling
	SyncTimer += DeltaTime;
	const float SyncInterval = 1.0f / FMath::Max(SyncRate, 1.0f);

	if (SyncTimer >= SyncInterval)
	{
		SyncTimer -= SyncInterval;
		PollServerState();
	}
}

// ============================================================================
// Controls
// ============================================================================

void UTowerStateSynchronizer::BeginSync()
{
	if (bSyncing) return;

	bSyncing = true;
	SyncTimer = 0.0f;
	NextSequenceNumber = 1;
	LastConfirmedServerTick = 0;
	EstimatedRTT = 0.0f;
	SmoothedRTT = 0.0f;
	InterpolationTime = 0.0;

	SnapshotBuffer.Empty();
	SnapshotBuffer.Reserve(MaxSnapshotBufferSize);
	PendingActions.Empty();
	PreviousEntityStateHashes.Empty();

	UE_LOG(LogStateSync, Log, TEXT("StateSynchronizer: started (rate=%.0fHz, interp=%.0fms, prediction=%s)"),
		SyncRate, InterpolationDelay * 1000.0f, bPredictionEnabled ? TEXT("on") : TEXT("off"));
}

void UTowerStateSynchronizer::StopSync()
{
	if (!bSyncing) return;

	bSyncing = false;
	SnapshotBuffer.Empty();
	PendingActions.Empty();
	PreviousEntityStateHashes.Empty();

	UE_LOG(LogStateSync, Log, TEXT("StateSynchronizer: stopped"));
}

// ============================================================================
// State Access
// ============================================================================

FWorldStateBuffer UTowerStateSynchronizer::GetInterpolatedState() const
{
	if (SnapshotBuffer.Num() == 0)
	{
		return FWorldStateBuffer();
	}

	if (SnapshotBuffer.Num() == 1)
	{
		return SnapshotBuffer[0];
	}

	// Render time is behind real-time by InterpolationDelay
	const double RenderTime = InterpolationTime - static_cast<double>(InterpolationDelay);

	// Find the two snapshots that bracket RenderTime
	int32 FromIndex = -1;
	int32 ToIndex = -1;

	for (int32 i = SnapshotBuffer.Num() - 1; i >= 1; --i)
	{
		if (SnapshotBuffer[i - 1].ServerTimestamp <= RenderTime &&
			SnapshotBuffer[i].ServerTimestamp >= RenderTime)
		{
			FromIndex = i - 1;
			ToIndex = i;
			break;
		}
	}

	// If RenderTime is before all snapshots, return the oldest
	if (FromIndex < 0)
	{
		// If RenderTime is beyond all snapshots, return latest
		if (RenderTime >= SnapshotBuffer.Last().ServerTimestamp)
		{
			return SnapshotBuffer.Last();
		}
		return SnapshotBuffer[0];
	}

	const FWorldStateBuffer& From = SnapshotBuffer[FromIndex];
	const FWorldStateBuffer& To = SnapshotBuffer[ToIndex];

	const double TimeDelta = To.ServerTimestamp - From.ServerTimestamp;
	if (TimeDelta <= 0.0)
	{
		return To;
	}

	const float Alpha = static_cast<float>(
		FMath::Clamp((RenderTime - From.ServerTimestamp) / TimeDelta, 0.0, 1.0));

	return LerpWorldState(From, To, Alpha);
}

FWorldStateBuffer UTowerStateSynchronizer::GetLatestServerState() const
{
	if (SnapshotBuffer.Num() == 0)
	{
		return FWorldStateBuffer();
	}
	return SnapshotBuffer.Last();
}

int64 UTowerStateSynchronizer::GetLastServerTick() const
{
	if (SnapshotBuffer.Num() == 0)
	{
		return 0;
	}
	return SnapshotBuffer.Last().ServerTick;
}

// ============================================================================
// Prediction
// ============================================================================

int64 UTowerStateSynchronizer::PredictAction(EPredictedActionType ActionType,
	FVector PredictedPosition, FRotator PredictedRotation, float PredictedHealth)
{
	if (!bPredictionEnabled)
	{
		return -1;
	}

	if (PendingActions.Num() >= MaxPendingActions)
	{
		UE_LOG(LogStateSync, Warning,
			TEXT("StateSynchronizer: pending action queue full (%d/%d), dropping prediction"),
			PendingActions.Num(), MaxPendingActions);
		return -1;
	}

	FPendingAction Action;
	Action.SequenceNumber = NextSequenceNumber++;
	Action.ActionType = ActionType;
	Action.Timestamp = FPlatformTime::Seconds();
	Action.PredictedPosition = PredictedPosition;
	Action.PredictedRotation = PredictedRotation;
	Action.PredictedHealth = PredictedHealth;

	PendingActions.Add(Action);

	if (bDebugLogging)
	{
		UE_LOG(LogStateSync, Verbose, TEXT("StateSynchronizer: predicted action seq=%lld type=%d pos=%s"),
			Action.SequenceNumber, static_cast<int32>(ActionType), *PredictedPosition.ToString());
	}

	return Action.SequenceNumber;
}

// ============================================================================
// Reconciliation
// ============================================================================

void UTowerStateSynchronizer::ReconcileState(const FWorldStateBuffer& AuthoritativeState)
{
	if (!bPredictionEnabled || PendingActions.Num() == 0)
	{
		return;
	}

	// Find the local player's authoritative snapshot
	// Convention: first player snapshot is the local player
	if (AuthoritativeState.PlayerSnapshots.Num() == 0)
	{
		return;
	}

	const FPlayerStateSnapshot& ServerPlayerState = AuthoritativeState.PlayerSnapshots[0];

	// Determine which pending actions the server has acknowledged.
	// The server state encompasses everything up to its tick, so we remove
	// all pending actions whose timestamp is <= the server timestamp.
	int64 AckedUpTo = 0;
	for (const FPendingAction& Action : PendingActions)
	{
		if (Action.Timestamp <= AuthoritativeState.ServerTimestamp)
		{
			AckedUpTo = Action.SequenceNumber;
		}
		else
		{
			break;
		}
	}

	if (AckedUpTo > 0)
	{
		AcknowledgeActionsUpTo(AckedUpTo);
	}

	// Replay remaining un-acked actions on top of server state
	const FVector ReplayedPosition = ReplayPendingActions(ServerPlayerState.Position);

	// Get the latest predicted position (from the most recent pending action)
	FVector CurrentPredicted = ServerPlayerState.Position;
	if (PendingActions.Num() > 0)
	{
		CurrentPredicted = PendingActions.Last().PredictedPosition;
	}

	// Check for desync
	const float DesyncDistance = FVector::Dist(ReplayedPosition, CurrentPredicted);

	if (DesyncDistance > DesyncThreshold)
	{
		// Significant desync — broadcast and snap to server state
		OnDesyncDetected.Broadcast(DesyncDistance, AuthoritativeState.ServerTick);

		UE_LOG(LogStateSync, Warning,
			TEXT("StateSynchronizer: desync detected (%.1f units) at server tick %lld"),
			DesyncDistance, AuthoritativeState.ServerTick);
	}
	else if (DesyncDistance > KINDA_SMALL_NUMBER && PendingActions.Num() > 0)
	{
		// Minor correction — notify and adjust pending predictions
		OnPredictionCorrected.Broadcast(
			PendingActions[0].SequenceNumber,
			ReplayedPosition,
			CurrentPredicted
		);

		if (bDebugLogging)
		{
			UE_LOG(LogStateSync, Verbose,
				TEXT("StateSynchronizer: prediction corrected by %.1f units"),
				DesyncDistance);
		}
	}
}

void UTowerStateSynchronizer::AcknowledgeActionsUpTo(int64 SequenceNumber)
{
	PendingActions.RemoveAll([SequenceNumber](const FPendingAction& Action)
	{
		return Action.SequenceNumber <= SequenceNumber;
	});
}

FVector UTowerStateSynchronizer::ReplayPendingActions(FVector ServerPosition) const
{
	FVector Position = ServerPosition;

	for (const FPendingAction& Action : PendingActions)
	{
		// Re-apply the delta that each pending action represents.
		// The predicted position includes cumulative movement, so we compute
		// the incremental offset from the previous prediction.
		// For simplicity, we apply the full predicted position of the last action.
		// A more sophisticated approach would store per-action deltas.
		Position = Action.PredictedPosition;
	}

	return Position;
}

// ============================================================================
// Interpolation
// ============================================================================

FWorldStateBuffer UTowerStateSynchronizer::LerpWorldState(
	const FWorldStateBuffer& A, const FWorldStateBuffer& B, float Alpha) const
{
	FWorldStateBuffer Result;
	Result.ServerTick = B.ServerTick;
	Result.ServerTimestamp = FMath::Lerp(A.ServerTimestamp, B.ServerTimestamp, static_cast<double>(Alpha));
	Result.WorldCyclePhase = (Alpha < 0.5f) ? A.WorldCyclePhase : B.WorldCyclePhase;

	// Interpolate player snapshots — match by EntityId
	for (const FPlayerStateSnapshot& SnapB : B.PlayerSnapshots)
	{
		const FPlayerStateSnapshot* MatchA = nullptr;
		for (const FPlayerStateSnapshot& SnapA : A.PlayerSnapshots)
		{
			if (SnapA.EntityId == SnapB.EntityId)
			{
				MatchA = &SnapA;
				break;
			}
		}

		if (MatchA)
		{
			Result.PlayerSnapshots.Add(LerpPlayerSnapshot(*MatchA, SnapB, Alpha));
		}
		else
		{
			// New entity not present in A — use B directly
			Result.PlayerSnapshots.Add(SnapB);
		}
	}

	// Interpolate monster snapshots — match by EntityId
	for (const FMonsterStateSnapshot& SnapB : B.MonsterSnapshots)
	{
		const FMonsterStateSnapshot* MatchA = nullptr;
		for (const FMonsterStateSnapshot& SnapA : A.MonsterSnapshots)
		{
			if (SnapA.EntityId == SnapB.EntityId)
			{
				MatchA = &SnapA;
				break;
			}
		}

		if (MatchA)
		{
			Result.MonsterSnapshots.Add(LerpMonsterSnapshot(*MatchA, SnapB, Alpha));
		}
		else
		{
			Result.MonsterSnapshots.Add(SnapB);
		}
	}

	return Result;
}

FPlayerStateSnapshot UTowerStateSynchronizer::LerpPlayerSnapshot(
	const FPlayerStateSnapshot& A, const FPlayerStateSnapshot& B, float Alpha) const
{
	FPlayerStateSnapshot Result;
	Result.EntityId = B.EntityId;
	Result.Timestamp = FMath::Lerp(A.Timestamp, B.Timestamp, static_cast<double>(Alpha));

	// Position: lerp or teleport
	const float Distance = FVector::Dist(A.Position, B.Position);
	if (Distance > TeleportThreshold)
	{
		Result.Position = B.Position;
	}
	else
	{
		Result.Position = FMath::Lerp(A.Position, B.Position, Alpha);
	}

	// Rotation: shortest path slerp
	Result.Rotation = FMath::Lerp(A.Rotation, B.Rotation, Alpha);

	// Health: lerp for smooth bar transitions
	Result.Health = FMath::Lerp(A.Health, B.Health, Alpha);

	// Resources: lerp
	Result.Resources = FVector4(
		FMath::Lerp(A.Resources.X, B.Resources.X, static_cast<double>(Alpha)),
		FMath::Lerp(A.Resources.Y, B.Resources.Y, static_cast<double>(Alpha)),
		FMath::Lerp(A.Resources.Z, B.Resources.Z, static_cast<double>(Alpha)),
		FMath::Lerp(A.Resources.W, B.Resources.W, static_cast<double>(Alpha))
	);

	return Result;
}

FMonsterStateSnapshot UTowerStateSynchronizer::LerpMonsterSnapshot(
	const FMonsterStateSnapshot& A, const FMonsterStateSnapshot& B, float Alpha) const
{
	FMonsterStateSnapshot Result;
	Result.EntityId = B.EntityId;

	// Position: lerp or teleport
	const float Distance = FVector::Dist(A.Position, B.Position);
	if (Distance > TeleportThreshold)
	{
		Result.Position = B.Position;
	}
	else
	{
		Result.Position = FMath::Lerp(A.Position, B.Position, Alpha);
	}

	// Health: lerp for smooth bar
	Result.Health = FMath::Lerp(A.Health, B.Health, Alpha);

	// Combat phase and status effects: use the latest (no interpolation for discrete states)
	Result.CombatPhase = (Alpha < 0.5f) ? A.CombatPhase : B.CombatPhase;
	Result.StatusEffects = B.StatusEffects;

	return Result;
}

// ============================================================================
// Server Communication
// ============================================================================

UMatchConnection* UTowerStateSynchronizer::GetMatchConnection() const
{
	UGameInstance* GI = GetOwner() ? GetOwner()->GetGameInstance() : nullptr;
	return GI ? GI->GetSubsystem<UMatchConnection>() : nullptr;
}

void UTowerStateSynchronizer::PollServerState()
{
	UMatchConnection* Match = GetMatchConnection();
	if (!Match || !Match->IsConnected()) return;

	// Record send time for RTT estimation
	LastPollSentTime = FPlatformTime::Seconds();

	// Build request with last confirmed tick for delta sync
	TSharedRef<FJsonObject> Request = MakeShared<FJsonObject>();
	Request->SetNumberField(TEXT("last_tick"), static_cast<double>(LastConfirmedServerTick));
	Request->SetNumberField(TEXT("client_time"), LastPollSentTime);

	// Include the last acknowledged sequence number so the server knows
	// which predicted actions have been processed
	if (PendingActions.Num() > 0)
	{
		Request->SetNumberField(TEXT("last_acked_seq"),
			static_cast<double>(PendingActions[0].SequenceNumber));
	}

	FString RequestJson;
	TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&RequestJson);
	FJsonSerializer::Serialize(Request, Writer);

	Match->SendMatchData(EMatchOpCode::BreathSync, RequestJson);

	if (bDebugLogging)
	{
		UE_LOG(LogStateSync, Verbose, TEXT("StateSynchronizer: poll sent (last_tick=%lld, pending=%d)"),
			LastConfirmedServerTick, PendingActions.Num());
	}
}

void UTowerStateSynchronizer::OnMatchDataReceived(EMatchOpCode OpCode, const FString& DataJson)
{
	// We listen for BreathSync responses which carry the full world state
	if (OpCode != EMatchOpCode::BreathSync) return;

	const double ReceiveTime = FPlatformTime::Seconds();

	FWorldStateBuffer NewState;
	if (!ParseWorldStateFromJson(DataJson, NewState)) return;

	// Update RTT estimate
	UpdateRTTEstimate(LastPollSentTime, ReceiveTime);

	// Delta compression: check which entities actually changed
	bool bAnyChanged = false;
	for (const FPlayerStateSnapshot& Snap : NewState.PlayerSnapshots)
	{
		const uint32 Hash = ComputeEntityStateHash(Snap);
		if (HasEntityStateChanged(Snap.EntityId, Hash))
		{
			PreviousEntityStateHashes.Add(Snap.EntityId, Hash);
			bAnyChanged = true;
		}
	}
	for (const FMonsterStateSnapshot& Snap : NewState.MonsterSnapshots)
	{
		const uint32 Hash = ComputeEntityStateHash(Snap);
		if (HasEntityStateChanged(Snap.EntityId, Hash))
		{
			PreviousEntityStateHashes.Add(Snap.EntityId, Hash);
			bAnyChanged = true;
		}
	}

	// Always buffer the snapshot (for interpolation continuity) but only
	// fire reconciliation and events when state actually changed
	BufferSnapshot(NewState);
	LastConfirmedServerTick = NewState.ServerTick;

	// Reconcile predictions
	ReconcileState(NewState);

	if (bAnyChanged)
	{
		// Broadcast the interpolated state to listeners
		FWorldStateBuffer Interpolated = GetInterpolatedState();
		OnStateUpdated.Broadcast(Interpolated);
	}

	if (bDebugLogging)
	{
		UE_LOG(LogStateSync, Verbose,
			TEXT("StateSynchronizer: received tick=%lld players=%d monsters=%d rtt=%.0fms changed=%s"),
			NewState.ServerTick, NewState.PlayerSnapshots.Num(), NewState.MonsterSnapshots.Num(),
			EstimatedRTT * 1000.0f, bAnyChanged ? TEXT("yes") : TEXT("no"));
	}
}

// ============================================================================
// Buffer Management
// ============================================================================

void UTowerStateSynchronizer::BufferSnapshot(const FWorldStateBuffer& Snapshot)
{
	if (SnapshotBuffer.Num() >= MaxSnapshotBufferSize)
	{
		// Circular buffer: remove the oldest snapshot
		SnapshotBuffer.RemoveAt(0, EAllowShrinking::No);
	}

	SnapshotBuffer.Add(Snapshot);
}

// ============================================================================
// Delta Compression
// ============================================================================

uint32 UTowerStateSynchronizer::ComputeEntityStateHash(const FPlayerStateSnapshot& Snapshot) const
{
	uint32 Hash = GetTypeHash(Snapshot.EntityId);
	Hash = HashCombine(Hash, GetTypeHash(FMath::RoundToInt(Snapshot.Position.X)));
	Hash = HashCombine(Hash, GetTypeHash(FMath::RoundToInt(Snapshot.Position.Y)));
	Hash = HashCombine(Hash, GetTypeHash(FMath::RoundToInt(Snapshot.Position.Z)));
	Hash = HashCombine(Hash, GetTypeHash(FMath::RoundToInt(Snapshot.Health * 10.0f)));
	Hash = HashCombine(Hash, GetTypeHash(FMath::RoundToInt(Snapshot.Rotation.Yaw)));
	return Hash;
}

uint32 UTowerStateSynchronizer::ComputeEntityStateHash(const FMonsterStateSnapshot& Snapshot) const
{
	uint32 Hash = GetTypeHash(Snapshot.EntityId);
	Hash = HashCombine(Hash, GetTypeHash(FMath::RoundToInt(Snapshot.Position.X)));
	Hash = HashCombine(Hash, GetTypeHash(FMath::RoundToInt(Snapshot.Position.Y)));
	Hash = HashCombine(Hash, GetTypeHash(FMath::RoundToInt(Snapshot.Position.Z)));
	Hash = HashCombine(Hash, GetTypeHash(FMath::RoundToInt(Snapshot.Health * 10.0f)));
	Hash = HashCombine(Hash, GetTypeHash(static_cast<uint8>(Snapshot.CombatPhase)));
	Hash = HashCombine(Hash, GetTypeHash(Snapshot.StatusEffects.Num()));
	return Hash;
}

bool UTowerStateSynchronizer::HasEntityStateChanged(int64 EntityId, uint32 NewHash) const
{
	const uint32* OldHash = PreviousEntityStateHashes.Find(EntityId);
	if (!OldHash)
	{
		return true; // New entity, always considered changed
	}
	return *OldHash != NewHash;
}

// ============================================================================
// RTT Estimation
// ============================================================================

void UTowerStateSynchronizer::UpdateRTTEstimate(double SendTime, double ReceiveTime)
{
	if (SendTime <= 0.0) return;

	const float SampleRTT = static_cast<float>(ReceiveTime - SendTime);
	if (SampleRTT < 0.0f || SampleRTT > 5.0f) return; // Discard nonsensical values

	// Exponential moving average (similar to TCP RTT estimation)
	constexpr float SmoothingFactor = 0.125f;
	if (SmoothedRTT <= 0.0f)
	{
		SmoothedRTT = SampleRTT;
	}
	else
	{
		SmoothedRTT = (1.0f - SmoothingFactor) * SmoothedRTT + SmoothingFactor * SampleRTT;
	}
	EstimatedRTT = SmoothedRTT;
}

// ============================================================================
// JSON Parsing
// ============================================================================

bool UTowerStateSynchronizer::ParseWorldStateFromJson(const FString& JsonString, FWorldStateBuffer& OutState) const
{
	TSharedPtr<FJsonObject> Root;
	TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(JsonString);

	if (!FJsonSerializer::Deserialize(Reader, Root) || !Root.IsValid())
	{
		UE_LOG(LogStateSync, Warning, TEXT("StateSynchronizer: failed to parse world state JSON"));
		return false;
	}

	// Server tick
	OutState.ServerTick = static_cast<int64>(Root->GetNumberField(TEXT("server_tick")));
	OutState.ServerTimestamp = Root->GetNumberField(TEXT("server_time"));

	// World cycle phase
	FString PhaseStr = Root->GetStringField(TEXT("world_phase"));
	if (PhaseStr == TEXT("Inhale"))       OutState.WorldCyclePhase = EWorldCyclePhase::Inhale;
	else if (PhaseStr == TEXT("Hold"))    OutState.WorldCyclePhase = EWorldCyclePhase::Hold;
	else if (PhaseStr == TEXT("Exhale"))  OutState.WorldCyclePhase = EWorldCyclePhase::Exhale;
	else if (PhaseStr == TEXT("Pause"))   OutState.WorldCyclePhase = EWorldCyclePhase::Pause;

	// Player snapshots
	const TArray<TSharedPtr<FJsonValue>>* PlayersArray = nullptr;
	if (Root->TryGetArrayField(TEXT("players"), PlayersArray))
	{
		for (const TSharedPtr<FJsonValue>& PlayerVal : *PlayersArray)
		{
			const TSharedPtr<FJsonObject>& PlayerObj = PlayerVal->AsObject();
			if (!PlayerObj.IsValid()) continue;

			FPlayerStateSnapshot Snap;
			Snap.EntityId = static_cast<int64>(PlayerObj->GetNumberField(TEXT("entity_id")));
			Snap.Position.X = PlayerObj->GetNumberField(TEXT("x"));
			Snap.Position.Y = PlayerObj->GetNumberField(TEXT("y"));
			Snap.Position.Z = PlayerObj->GetNumberField(TEXT("z"));
			Snap.Rotation.Yaw = PlayerObj->GetNumberField(TEXT("yaw"));
			Snap.Rotation.Pitch = PlayerObj->GetNumberField(TEXT("pitch"));
			Snap.Health = static_cast<float>(PlayerObj->GetNumberField(TEXT("health")));
			Snap.Timestamp = PlayerObj->GetNumberField(TEXT("timestamp"));

			// Resources (optional)
			if (PlayerObj->HasField(TEXT("kinetic")))
			{
				Snap.Resources.X = PlayerObj->GetNumberField(TEXT("kinetic"));
				Snap.Resources.Y = PlayerObj->GetNumberField(TEXT("thermal"));
				Snap.Resources.Z = PlayerObj->GetNumberField(TEXT("semantic"));
				Snap.Resources.W = PlayerObj->GetNumberField(TEXT("rage"));
			}

			OutState.PlayerSnapshots.Add(Snap);
		}
	}

	// Monster snapshots
	const TArray<TSharedPtr<FJsonValue>>* MonstersArray = nullptr;
	if (Root->TryGetArrayField(TEXT("monsters"), MonstersArray))
	{
		for (const TSharedPtr<FJsonValue>& MonsterVal : *MonstersArray)
		{
			const TSharedPtr<FJsonObject>& MonsterObj = MonsterVal->AsObject();
			if (!MonsterObj.IsValid()) continue;

			FMonsterStateSnapshot Snap;
			Snap.EntityId = static_cast<int64>(MonsterObj->GetNumberField(TEXT("entity_id")));
			Snap.Position.X = MonsterObj->GetNumberField(TEXT("x"));
			Snap.Position.Y = MonsterObj->GetNumberField(TEXT("y"));
			Snap.Position.Z = MonsterObj->GetNumberField(TEXT("z"));
			Snap.Health = static_cast<float>(MonsterObj->GetNumberField(TEXT("health")));

			// Combat phase
			FString CombatPhaseStr = MonsterObj->GetStringField(TEXT("combat_phase"));
			if (CombatPhaseStr == TEXT("Idle"))           Snap.CombatPhase = EMonsterCombatPhase::Idle;
			else if (CombatPhaseStr == TEXT("Windup"))    Snap.CombatPhase = EMonsterCombatPhase::Windup;
			else if (CombatPhaseStr == TEXT("Active"))    Snap.CombatPhase = EMonsterCombatPhase::Active;
			else if (CombatPhaseStr == TEXT("Recovery"))  Snap.CombatPhase = EMonsterCombatPhase::Recovery;

			// Status effects
			const TArray<TSharedPtr<FJsonValue>>* EffectsArray = nullptr;
			if (MonsterObj->TryGetArrayField(TEXT("status_effects"), EffectsArray))
			{
				for (const TSharedPtr<FJsonValue>& EffectVal : *EffectsArray)
				{
					FString EffectStr = EffectVal->AsString();
					EMonsterStatusEffect Effect = EMonsterStatusEffect::None;

					if (EffectStr == TEXT("Burning"))            Effect = EMonsterStatusEffect::Burning;
					else if (EffectStr == TEXT("Poisoned"))      Effect = EMonsterStatusEffect::Poisoned;
					else if (EffectStr == TEXT("Bleeding"))      Effect = EMonsterStatusEffect::Bleeding;
					else if (EffectStr == TEXT("Stunned"))       Effect = EMonsterStatusEffect::Stunned;
					else if (EffectStr == TEXT("Frozen"))        Effect = EMonsterStatusEffect::Frozen;
					else if (EffectStr == TEXT("Silenced"))      Effect = EMonsterStatusEffect::Silenced;
					else if (EffectStr == TEXT("Weakened"))      Effect = EMonsterStatusEffect::Weakened;
					else if (EffectStr == TEXT("Slowed"))        Effect = EMonsterStatusEffect::Slowed;
					else if (EffectStr == TEXT("Exposed"))       Effect = EMonsterStatusEffect::Exposed;
					else if (EffectStr == TEXT("Corrupted"))     Effect = EMonsterStatusEffect::Corrupted;
					else if (EffectStr == TEXT("Empowered"))     Effect = EMonsterStatusEffect::Empowered;
					else if (EffectStr == TEXT("Hastened"))      Effect = EMonsterStatusEffect::Hastened;
					else if (EffectStr == TEXT("Shielded"))      Effect = EMonsterStatusEffect::Shielded;
					else if (EffectStr == TEXT("Regenerating"))  Effect = EMonsterStatusEffect::Regenerating;
					else if (EffectStr == TEXT("SemanticFocus")) Effect = EMonsterStatusEffect::SemanticFocus;

					if (Effect != EMonsterStatusEffect::None)
					{
						Snap.StatusEffects.Add(Effect);
					}
				}
			}

			OutState.MonsterSnapshots.Add(Snap);
		}
	}

	return true;
}
