#pragma once

#include "CoreMinimal.h"
#include "Subsystems/GameInstanceSubsystem.h"
#include "Interfaces/IHttpRequest.h"
#include "Interfaces/IHttpResponse.h"
#include "GRPCClientManager.generated.h"

// ============================================================
// Enums
// ============================================================

/** Connection state for the Rust procedural core bridge */
UENUM(BlueprintType)
enum class EGRPCConnectionState : uint8
{
	Disconnected    UMETA(DisplayName = "Disconnected"),
	Connecting      UMETA(DisplayName = "Connecting"),
	Connected       UMETA(DisplayName = "Connected"),
	Reconnecting    UMETA(DisplayName = "Reconnecting"),
	Error           UMETA(DisplayName = "Error")
};

/** Transport mode used to communicate with the Rust backend */
UENUM(BlueprintType)
enum class ETransportMode : uint8
{
	/** JSON over HTTP — default, mirrors proto service definitions */
	GRPC    UMETA(DisplayName = "gRPC (JSON-HTTP)"),
	/** Plain JSON over HTTP without gRPC framing */
	JSON    UMETA(DisplayName = "JSON"),
	/** Foreign Function Interface via DLL bridge (tower_core.dll) */
	FFI     UMETA(DisplayName = "FFI/DLL")
};

// ============================================================
// Config struct
// ============================================================

/** Configuration for connecting to the Rust procedural core */
USTRUCT(BlueprintType)
struct FGRPCConfig
{
	GENERATED_BODY()

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "gRPC|Config")
	FString Host = TEXT("127.0.0.1");

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "gRPC|Config")
	int32 Port = 50051;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "gRPC|Config")
	float TimeoutSeconds = 10.0f;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "gRPC|Config")
	int32 MaxRetries = 3;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "gRPC|Config")
	ETransportMode TransportMode = ETransportMode::GRPC;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "gRPC|Config")
	float HealthCheckIntervalSeconds = 5.0f;

	/** Path to the Rust DLL for FFI fallback (relative to Binaries/) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "gRPC|Config")
	FString FFIDllPath = TEXT("tower_core.dll");
};

// ============================================================
// Response structs (mirroring proto messages for Blueprint use)
// ============================================================

USTRUCT(BlueprintType)
struct FTowerVec3
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Types")
	float X = 0.0f;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Types")
	float Y = 0.0f;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Types")
	float Z = 0.0f;

	FVector ToFVector() const { return FVector(X, Y, Z); }

	static FTowerVec3 FromFVector(const FVector& V)
	{
		FTowerVec3 Result;
		Result.X = static_cast<float>(V.X);
		Result.Y = static_cast<float>(V.Y);
		Result.Z = static_cast<float>(V.Z);
		return Result;
	}
};

USTRUCT(BlueprintType)
struct FSemanticTag
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Types")
	FString Name;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Types")
	float Value = 0.0f;
};

USTRUCT(BlueprintType)
struct FDamageModifierResult
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Combat")
	FString Source;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Combat")
	float Multiplier = 1.0f;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Combat")
	FString Description;
};

USTRUCT(BlueprintType)
struct FDamageCalcResult
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Combat")
	float BaseDamage = 0.0f;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Combat")
	float ModifiedDamage = 0.0f;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Combat")
	float CritChance = 0.0f;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Combat")
	float CritDamage = 0.0f;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Combat")
	TArray<FDamageModifierResult> Modifiers;
};

USTRUCT(BlueprintType)
struct FMasteryProgressResult
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Mastery")
	FString Domain;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Mastery")
	int32 NewTier = 0;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Mastery")
	float NewXP = 0.0f;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Mastery")
	float XPToNext = 0.0f;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Mastery")
	bool bTierUp = false;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Mastery")
	TArray<FString> NewlyUnlocked;
};

USTRUCT(BlueprintType)
struct FWalletResult
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Economy")
	int64 Gold = 0;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Economy")
	int64 PremiumCurrency = 0;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Economy")
	int64 SeasonalCurrency = 0;
};

USTRUCT(BlueprintType)
struct FLootItemResult
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Economy")
	FString ItemName;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Economy")
	int32 Rarity = 0;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Economy")
	TArray<FSemanticTag> Tags;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Economy")
	int32 SocketCount = 0;
};

// ============================================================
// Delegates
// ============================================================

/** Fired whenever the connection state changes */
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(
	FOnConnectionStateChanged,
	EGRPCConnectionState, NewState,
	EGRPCConnectionState, OldState
);

/** Fired when any service request completes successfully. RequestId can be correlated by the caller. */
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(
	FOnGRPCRequestCompleted,
	int64, RequestId,
	const FString&, ResponseJson
);

/** Fired when a service request fails */
DECLARE_DYNAMIC_MULTICAST_DELEGATE_ThreeParams(
	FOnGRPCRequestFailed,
	int64, RequestId,
	int32, ErrorCode,
	const FString&, ErrorMessage
);

/** Floor generation response */
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(
	FOnFloorGeneratedResponse,
	int64, RequestId,
	const FString&, FloorJson
);

/** Damage calculation response */
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(
	FOnDamageCalculated,
	int64, RequestId,
	FDamageCalcResult, Result
);

/** Mastery progress response */
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(
	FOnMasteryProgressReceived,
	int64, RequestId,
	FMasteryProgressResult, Result
);

/** Wallet response */
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(
	FOnWalletReceived,
	int64, RequestId,
	FWalletResult, Result
);

/** Loot generation response */
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(
	FOnLootGenerated,
	int64, RequestId,
	const TArray<FLootItemResult>&, Items
);

// ============================================================
// UTowerGRPCClientManager
// ============================================================

/**
 * Manages the connection between UE5 and the Rust procedural core over
 * a JSON-over-HTTP transport that mirrors the proto service definitions.
 *
 * Service endpoints (27 POST + 3 GET = 30 total):
 *   GenerationService:  GenerateFloor, GenerateLoot, SpawnMonsters, GenerateMonsters, GenerateDestructibles, QuerySemanticTags
 *   GameStateService:   GetState, GetWorldCycle, GetPlayerProfile, GetLiveStatus, GetLivePlayer
 *   CombatService:      CalculateDamage, GetCombatState, ProcessAction
 *   MasteryService:     TrackProgress, GetMasteryProfile, ChooseSpecialization, UpdateAbilityLoadout
 *   EconomyService:     GetWallet, Craft, ListAuction, BuyAuction, Trade
 *   DestructionService: ApplyDamage, GetFloorState, Rebuild, GetTemplates
 *   System:             GET /health, GET /metrics, GET /metrics/json
 *
 * Falls back to FFI/DLL bridge (tower_core.dll) when the gRPC server
 * is unreachable and the TransportMode allows it.
 */
UCLASS()
class TOWERGAME_API UTowerGRPCClientManager : public UGameInstanceSubsystem
{
	GENERATED_BODY()

public:
	// ============ Subsystem lifecycle ============

	virtual void Initialize(FSubsystemCollectionBase& Collection) override;
	virtual void Deinitialize() override;
	virtual bool ShouldCreateSubsystem(UObject* Outer) const override { return true; }

	// ============ Configuration ============

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "gRPC|Config")
	FGRPCConfig Config;

	// ============ Connection management ============

	/** Establish connection to the Rust procedural core */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Connection")
	void Connect();

	/** Disconnect from the server and cancel pending requests */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Connection")
	void Disconnect();

	/** Drop current connection and reconnect with exponential backoff */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Connection")
	void Reconnect();

	/** Current connection state */
	UFUNCTION(BlueprintPure, Category = "gRPC|Connection")
	bool IsConnected() const { return ConnectionState == EGRPCConnectionState::Connected; }

	UFUNCTION(BlueprintPure, Category = "gRPC|Connection")
	EGRPCConnectionState GetConnectionState() const { return ConnectionState; }

	UFUNCTION(BlueprintPure, Category = "gRPC|Connection")
	ETransportMode GetActiveTransportMode() const { return ActiveTransport; }

	// ============ GameStateService ============

	/**
	 * Request a procedurally generated floor layout.
	 * Maps to: tower.GenerationService/GenerateFloor
	 * @param TowerSeed  Shared tower seed
	 * @param FloorId    Floor index to generate
	 * @return RequestId for correlating the async response
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Generation")
	int64 RequestFloor(int64 TowerSeed, int32 FloorId);

	// ============ CombatService ============

	/**
	 * Request a damage calculation preview for UI.
	 * Maps to: tower.CombatService/CalculateDamage
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Combat")
	int64 RequestCombatCalc(int64 AttackerId, int64 DefenderId, const FString& WeaponId, const FString& AbilityId);

	// ============ MasteryService ============

	/**
	 * Report mastery XP gain and receive progression result.
	 * Maps to: tower.MasteryService/TrackProgress
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Mastery")
	int64 RequestMasteryProgress(int64 PlayerId, const FString& Domain, const FString& ActionType, float XPAmount);

	// ============ EconomyService ============

	/**
	 * Get player wallet balances.
	 * Maps to: tower.EconomyService/GetWallet
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Economy")
	int64 RequestWallet(int64 PlayerId);

	/**
	 * Generate loot from a defeated entity.
	 * Maps to: tower.GenerationService/GenerateLoot
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Economy")
	int64 RequestLoot(int64 SourceEntityId, int64 PlayerId, float LuckModifier);

	// ============ GameStateService ============

	/**
	 * Get the current Breath of the Tower world cycle.
	 * Maps to: tower.GameStateService/GetWorldCycle
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|GameState")
	int64 RequestWorldCycle(int64 TowerSeed);

	/**
	 * Get live server status from ECS snapshot.
	 * Maps to: tower.GameStateService/GetLiveStatus
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|GameState")
	int64 RequestLiveStatus();

	/**
	 * Get combined player + world state.
	 * Maps to: tower.GameStateService/GetState
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|GameState")
	int64 RequestGameState(int64 PlayerId, int32 FloorId);

	/**
	 * Get a player's full profile with base stats.
	 * Maps to: tower.GameStateService/GetPlayerProfile
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|GameState")
	int64 RequestPlayerProfile(int64 PlayerId);

	// ============ Additional Generation ============

	/**
	 * Spawn monsters in a room.
	 * Maps to: tower.GenerationService/SpawnMonsters
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Generation")
	int64 RequestSpawnMonsters(int64 TowerSeed, int32 FloorId, int32 RoomId);

	/**
	 * Generate destructible objects for a floor.
	 * Maps to: tower.GenerationService/GenerateDestructibles
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Generation")
	int64 RequestGenerateDestructibles(int64 TowerSeed, int32 FloorId);

	/**
	 * Generate grammar-based monsters with full blueprints.
	 * Maps to: tower.GenerationService/GenerateMonsters
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Generation")
	int64 RequestGenerateMonsters(int64 TowerSeed, int32 FloorId, int32 RoomId, int32 Count);

	// ============ Additional Combat ============

	/**
	 * Get entity combat state from ECS.
	 * Maps to: tower.CombatService/GetCombatState
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Combat")
	int64 RequestCombatState(int64 EntityId);

	/**
	 * Process a combat action through ECS bridge.
	 * Maps to: tower.CombatService/ProcessAction
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Combat")
	int64 RequestProcessAction(int64 PlayerId, const FString& ActionType, int64 TargetId, const FString& AbilityId, FVector Position, float Facing);

	// ============ Additional Mastery ============

	/**
	 * Get all mastery domains for a player.
	 * Maps to: tower.MasteryService/GetMasteryProfile
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Mastery")
	int64 RequestMasteryProfile(int64 PlayerId);

	/**
	 * Choose specialization branch for a domain.
	 * Maps to: tower.MasteryService/ChooseSpecialization
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Mastery")
	int64 RequestChooseSpecialization(int64 PlayerId, const FString& Domain, const FString& BranchId);

	// ============ Additional Economy ============

	/**
	 * Craft an item from a recipe.
	 * Maps to: tower.EconomyService/Craft
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Economy")
	int64 RequestCraft(int64 PlayerId, const FString& RecipeId, const TArray<FString>& MaterialItemIds);

	/**
	 * List active auction house entries.
	 * Maps to: tower.EconomyService/ListAuction
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Economy")
	int64 RequestListAuctions(const FString& Category, int32 Page, int32 PerPage);

	/**
	 * Buy an auction listing.
	 * Maps to: tower.EconomyService/BuyAuction
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Economy")
	int64 RequestBuyAuction(int64 PlayerId, int64 AuctionId);

	// ============ DestructionService ============

	/**
	 * Apply destruction damage to an environmental object.
	 * Maps to: tower.DestructionService/ApplyDamage
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Destruction")
	int64 RequestApplyDestructionDamage(int64 PlayerId, int64 TargetEntityId, int32 FloorId,
		FVector ImpactPoint, FVector EntityPosition, float Damage, float Radius,
		const FString& DamageType, const FString& WeaponId, const FString& AbilityId);

	/**
	 * Get destruction state for a floor.
	 * Maps to: tower.DestructionService/GetFloorState
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Destruction")
	int64 RequestFloorDestructionState(int32 FloorId);

	/**
	 * Rebuild a destroyed object.
	 * Maps to: tower.DestructionService/Rebuild
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Destruction")
	int64 RequestRebuild(int64 PlayerId, int64 TargetEntityId, int32 FloorId, const TArray<FString>& MaterialItems);

	/**
	 * Get all destructible templates.
	 * Maps to: tower.DestructionService/GetTemplates
	 */
	UFUNCTION(BlueprintCallable, Category = "gRPC|Destruction")
	int64 RequestDestructionTemplates();

	// ============ Delegates ============

	UPROPERTY(BlueprintAssignable, Category = "gRPC|Events")
	FOnConnectionStateChanged OnConnectionStateChanged;

	UPROPERTY(BlueprintAssignable, Category = "gRPC|Events")
	FOnGRPCRequestCompleted OnRequestCompleted;

	UPROPERTY(BlueprintAssignable, Category = "gRPC|Events")
	FOnGRPCRequestFailed OnRequestFailed;

	UPROPERTY(BlueprintAssignable, Category = "gRPC|Events")
	FOnFloorGeneratedResponse OnFloorGenerated;

	UPROPERTY(BlueprintAssignable, Category = "gRPC|Events")
	FOnDamageCalculated OnDamageCalculated;

	UPROPERTY(BlueprintAssignable, Category = "gRPC|Events")
	FOnMasteryProgressReceived OnMasteryProgressReceived;

	UPROPERTY(BlueprintAssignable, Category = "gRPC|Events")
	FOnWalletReceived OnWalletReceived;

	UPROPERTY(BlueprintAssignable, Category = "gRPC|Events")
	FOnLootGenerated OnLootGenerated;

	// ============ Stats (read-only) ============

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Stats")
	int32 TotalRequestsSent = 0;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Stats")
	int32 TotalRequestsFailed = 0;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Stats")
	int32 ConsecutiveFailures = 0;

	UPROPERTY(BlueprintReadOnly, Category = "gRPC|Stats")
	float AverageLatencyMs = 0.0f;

private:
	// ============ Internal state ============

	EGRPCConnectionState ConnectionState = EGRPCConnectionState::Disconnected;
	ETransportMode ActiveTransport = ETransportMode::GRPC;

	/** Monotonically increasing ID for correlating requests */
	int64 NextRequestId = 1;

	/** Number of reconnect attempts since last successful connection */
	int32 ReconnectAttempts = 0;

	/** Timer handle for periodic health checks */
	FTimerHandle HealthCheckTimerHandle;

	/** Timer handle for reconnect backoff */
	FTimerHandle ReconnectTimerHandle;

	/** Tracks in-flight requests: RequestId -> send timestamp (for latency) */
	TMap<int64, double> InFlightRequests;

	/** Handle to the loaded FFI DLL */
	void* FFIDllHandle = nullptr;

	// FFI function pointer types
	typedef int32 (*FFIHealthCheckFn)();
	typedef const char* (*FFIGenerateFloorFn)(int64, int32);
	typedef const char* (*FFICalculateDamageFn)(int64, int64, const char*, const char*);
	typedef void (*FFIFreeStringFn)(const char*);

	FFIHealthCheckFn FFIHealthCheck = nullptr;
	FFIGenerateFloorFn FFIGenerateFloor = nullptr;
	FFICalculateDamageFn FFICalculateDamage = nullptr;
	FFIFreeStringFn FFIFreeString = nullptr;

	// ============ Helpers ============

	/** Build the base URL for the Rust gRPC-JSON gateway */
	FString GetBaseUrl() const;

	/** Generate next request ID (thread-safe increment) */
	int64 AllocateRequestId();

	/** Set connection state and broadcast delegate */
	void SetConnectionState(EGRPCConnectionState NewState);

	/**
	 * Send an HTTP POST to the Rust server mimicking a gRPC service call.
	 * @param ServicePath  e.g. "/tower.GenerationService/GenerateFloor"
	 * @param PayloadJson  JSON body matching the proto request message
	 * @param RequestId    Caller-assigned ID for correlation
	 * @param OnResponse   Callback with (bSuccess, ResponseBody)
	 */
	void SendRequest(
		const FString& ServicePath,
		const FString& PayloadJson,
		int64 RequestId,
		TFunction<void(bool bSuccess, const FString& ResponseBody)> OnResponse
	);

	/** Process a raw JSON response into the typed delegate for floors */
	void ProcessFloorResponse(int64 RequestId, bool bSuccess, const FString& ResponseBody);

	/** Process a raw JSON response into the typed delegate for damage calc */
	void ProcessDamageCalcResponse(int64 RequestId, bool bSuccess, const FString& ResponseBody);

	/** Process a raw JSON response into the typed delegate for mastery */
	void ProcessMasteryResponse(int64 RequestId, bool bSuccess, const FString& ResponseBody);

	/** Process a raw JSON response into the typed delegate for wallet */
	void ProcessWalletResponse(int64 RequestId, bool bSuccess, const FString& ResponseBody);

	/** Process a raw JSON response into the typed delegate for loot */
	void ProcessLootResponse(int64 RequestId, bool bSuccess, const FString& ResponseBody);

	/** Periodic health check sent to /health on the Rust server */
	void PerformHealthCheck();

	/** Called when a health check response arrives */
	void OnHealthCheckResponse(bool bSuccess, const FString& ResponseBody);

	/** Update rolling average latency */
	void RecordLatency(int64 RequestId);

	/** Broadcast a generic failure and update stats */
	void HandleRequestFailure(int64 RequestId, int32 ErrorCode, const FString& ErrorMessage);

	/** Calculate exponential backoff delay for reconnection */
	float GetReconnectDelay() const;

	// ============ FFI Fallback ============

	/** Attempt to load the Rust DLL and resolve function pointers */
	bool TryLoadFFIBridge();

	/** Unload the FFI DLL handle */
	void UnloadFFIBridge();

	/** Check if FFI bridge is available */
	bool IsFFIAvailable() const { return FFIDllHandle != nullptr && FFIHealthCheck != nullptr; }

	// ============ JSON helpers ============

	/** Parse a JSON string into a shared JSON object. Returns nullptr on failure. */
	static TSharedPtr<FJsonObject> ParseJson(const FString& JsonString);

	/** Serialize a JSON object to a string */
	static FString SerializeJson(const TSharedPtr<FJsonObject>& JsonObject);

	/** Extract FSemanticTag array from a JSON array field */
	static TArray<FSemanticTag> ParseSemanticTags(const TArray<TSharedPtr<FJsonValue>>& JsonArray);
};
