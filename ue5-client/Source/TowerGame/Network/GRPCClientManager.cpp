#include "GRPCClientManager.h"
#include "HttpModule.h"
#include "Interfaces/IHttpRequest.h"
#include "Interfaces/IHttpResponse.h"
#include "Dom/JsonObject.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"
#include "Serialization/JsonWriter.h"
#include "TimerManager.h"
#include "Engine/GameInstance.h"
#include "HAL/PlatformProcess.h"
#include "Misc/Paths.h"

DEFINE_LOG_CATEGORY_STATIC(LogGRPCClient, Log, All);

// ============================================================
// Subsystem lifecycle
// ============================================================

void UTowerGRPCClientManager::Initialize(FSubsystemCollectionBase& Collection)
{
	Super::Initialize(Collection);

	ConnectionState = EConnectionState::Disconnected;
	ActiveTransport = Config.TransportMode;
	NextRequestId = 1;
	ReconnectAttempts = 0;
	TotalRequestsSent = 0;
	TotalRequestsFailed = 0;
	ConsecutiveFailures = 0;
	AverageLatencyMs = 0.0f;

	UE_LOG(LogGRPCClient, Log,
		TEXT("GRPCClientManager initialized. Target: %s:%d  Transport: %d"),
		*Config.Host, Config.Port, static_cast<int32>(Config.TransportMode));
}

void UTowerGRPCClientManager::Deinitialize()
{
	Disconnect();
	UnloadFFIBridge();
	Super::Deinitialize();
}

// ============================================================
// Connection management
// ============================================================

void UTowerGRPCClientManager::Connect()
{
	if (ConnectionState == EConnectionState::Connected || ConnectionState == EConnectionState::Connecting)
	{
		UE_LOG(LogGRPCClient, Warning, TEXT("Connect() called while already %s"),
			ConnectionState == EConnectionState::Connected ? TEXT("connected") : TEXT("connecting"));
		return;
	}

	SetConnectionState(EConnectionState::Connecting);
	ReconnectAttempts = 0;

	// Attempt primary transport first — send a health check to verify connectivity
	PerformHealthCheck();
}

void UTowerGRPCClientManager::Disconnect()
{
	// Clear health check timer
	if (UGameInstance* GI = GetGameInstance())
	{
		if (UWorld* World = GI->GetWorld())
		{
			World->GetTimerManager().ClearTimer(HealthCheckTimerHandle);
			World->GetTimerManager().ClearTimer(ReconnectTimerHandle);
		}
	}

	InFlightRequests.Empty();
	SetConnectionState(EConnectionState::Disconnected);

	UE_LOG(LogGRPCClient, Log, TEXT("Disconnected from Rust procedural core"));
}

void UTowerGRPCClientManager::Reconnect()
{
	UE_LOG(LogGRPCClient, Log, TEXT("Reconnect requested (attempt %d)"), ReconnectAttempts + 1);

	// Clear existing timers
	if (UGameInstance* GI = GetGameInstance())
	{
		if (UWorld* World = GI->GetWorld())
		{
			World->GetTimerManager().ClearTimer(HealthCheckTimerHandle);
			World->GetTimerManager().ClearTimer(ReconnectTimerHandle);
		}
	}

	SetConnectionState(EConnectionState::Reconnecting);
	ReconnectAttempts++;

	float Delay = GetReconnectDelay();
	UE_LOG(LogGRPCClient, Log, TEXT("Reconnecting in %.1f seconds (attempt %d)"), Delay, ReconnectAttempts);

	if (UGameInstance* GI = GetGameInstance())
	{
		if (UWorld* World = GI->GetWorld())
		{
			World->GetTimerManager().SetTimer(
				ReconnectTimerHandle,
				[this]() { PerformHealthCheck(); },
				Delay,
				false
			);
		}
	}
}

float UTowerGRPCClientManager::GetReconnectDelay() const
{
	// Exponential backoff: 1s, 2s, 4s, 8s, 16s, capped at 30s
	float Base = 1.0f;
	float Delay = Base * FMath::Pow(2.0f, static_cast<float>(FMath::Min(ReconnectAttempts - 1, 4)));
	return FMath::Min(Delay, 30.0f);
}

void UTowerGRPCClientManager::SetConnectionState(EConnectionState NewState)
{
	if (ConnectionState == NewState)
	{
		return;
	}

	EConnectionState OldState = ConnectionState;
	ConnectionState = NewState;

	UE_LOG(LogGRPCClient, Log, TEXT("Connection state: %d -> %d"), static_cast<int32>(OldState), static_cast<int32>(NewState));

	OnConnectionStateChanged.Broadcast(NewState, OldState);
}

// ============================================================
// Health check
// ============================================================

void UTowerGRPCClientManager::PerformHealthCheck()
{
	FString Url = FString::Printf(TEXT("%s/health"), *GetBaseUrl());

	TSharedRef<IHttpRequest, ESPMode::ThreadSafe> Request = FHttpModule::Get().CreateRequest();
	Request->SetURL(Url);
	Request->SetVerb(TEXT("GET"));
	Request->SetHeader(TEXT("Accept"), TEXT("application/json"));
	Request->SetTimeout(FMath::Min(Config.TimeoutSeconds, 5.0f));

	Request->OnProcessRequestComplete().BindLambda(
		[this](FHttpRequestPtr Req, FHttpResponsePtr Resp, bool bConnected)
		{
			bool bSuccess = false;
			FString Body;

			if (bConnected && Resp.IsValid())
			{
				int32 Code = Resp->GetResponseCode();
				Body = Resp->GetContentAsString();
				bSuccess = (Code >= 200 && Code < 300);
			}

			OnHealthCheckResponse(bSuccess, Body);
		});

	Request->ProcessRequest();
}

void UTowerGRPCClientManager::OnHealthCheckResponse(bool bSuccess, const FString& ResponseBody)
{
	if (bSuccess)
	{
		ConsecutiveFailures = 0;
		ReconnectAttempts = 0;

		if (ConnectionState != EConnectionState::Connected)
		{
			SetConnectionState(EConnectionState::Connected);

			UE_LOG(LogGRPCClient, Log,
				TEXT("Connected to Rust core at %s:%d via %s"),
				*Config.Host, Config.Port,
				ActiveTransport == ETransportMode::GRPC ? TEXT("gRPC-JSON") :
				ActiveTransport == ETransportMode::JSON ? TEXT("JSON") : TEXT("FFI"));

			// Start periodic health checks
			if (UGameInstance* GI = GetGameInstance())
			{
				if (UWorld* World = GI->GetWorld())
				{
					World->GetTimerManager().SetTimer(
						HealthCheckTimerHandle,
						[this]() { PerformHealthCheck(); },
						Config.HealthCheckIntervalSeconds,
						true  // looping
					);
				}
			}
		}
	}
	else
	{
		ConsecutiveFailures++;

		UE_LOG(LogGRPCClient, Warning,
			TEXT("Health check failed (consecutive: %d). Response: %s"),
			ConsecutiveFailures, *ResponseBody);

		// If we were connected, try to reconnect
		if (ConnectionState == EConnectionState::Connected)
		{
			Reconnect();
			return;
		}

		// If we were trying to connect/reconnect, attempt FFI fallback
		if (ConnectionState == EConnectionState::Connecting || ConnectionState == EConnectionState::Reconnecting)
		{
			if (ReconnectAttempts >= Config.MaxRetries)
			{
				// All HTTP retries exhausted — try FFI fallback
				if (TryLoadFFIBridge())
				{
					ActiveTransport = ETransportMode::FFI;
					SetConnectionState(EConnectionState::Connected);
					UE_LOG(LogGRPCClient, Log, TEXT("Fell back to FFI/DLL bridge"));
					return;
				}

				// No fallback available
				SetConnectionState(EConnectionState::Error);
				UE_LOG(LogGRPCClient, Error,
					TEXT("Failed to connect after %d attempts and FFI fallback unavailable"),
					Config.MaxRetries);
			}
			else
			{
				Reconnect();
			}
		}
	}
}

// ============================================================
// Service requests
// ============================================================

int64 UTowerGRPCClientManager::RequestFloor(int64 TowerSeed, int32 FloorId)
{
	int64 ReqId = AllocateRequestId();

	TSharedPtr<FJsonObject> Payload = MakeShareable(new FJsonObject());
	Payload->SetNumberField(TEXT("tower_seed"), static_cast<double>(TowerSeed));
	Payload->SetNumberField(TEXT("floor_id"), static_cast<double>(FloorId));

	SendRequest(
		TEXT("/tower.GenerationService/GenerateFloor"),
		SerializeJson(Payload),
		ReqId,
		[this, ReqId](bool bSuccess, const FString& Body)
		{
			ProcessFloorResponse(ReqId, bSuccess, Body);
		});

	return ReqId;
}

int64 UTowerGRPCClientManager::RequestCombatCalc(int64 AttackerId, int64 DefenderId, const FString& WeaponId, const FString& AbilityId)
{
	int64 ReqId = AllocateRequestId();

	TSharedPtr<FJsonObject> Payload = MakeShareable(new FJsonObject());
	Payload->SetNumberField(TEXT("attacker_id"), static_cast<double>(AttackerId));
	Payload->SetNumberField(TEXT("defender_id"), static_cast<double>(DefenderId));
	Payload->SetStringField(TEXT("weapon_id"), WeaponId);
	Payload->SetStringField(TEXT("ability_id"), AbilityId);

	SendRequest(
		TEXT("/tower.CombatService/CalculateDamage"),
		SerializeJson(Payload),
		ReqId,
		[this, ReqId](bool bSuccess, const FString& Body)
		{
			ProcessDamageCalcResponse(ReqId, bSuccess, Body);
		});

	return ReqId;
}

int64 UTowerGRPCClientManager::RequestMasteryProgress(int64 PlayerId, const FString& Domain, const FString& ActionType, float XPAmount)
{
	int64 ReqId = AllocateRequestId();

	TSharedPtr<FJsonObject> Payload = MakeShareable(new FJsonObject());
	Payload->SetNumberField(TEXT("player_id"), static_cast<double>(PlayerId));
	Payload->SetStringField(TEXT("domain"), Domain);
	Payload->SetStringField(TEXT("action_type"), ActionType);
	Payload->SetNumberField(TEXT("xp_amount"), static_cast<double>(XPAmount));

	SendRequest(
		TEXT("/tower.MasteryService/TrackProgress"),
		SerializeJson(Payload),
		ReqId,
		[this, ReqId](bool bSuccess, const FString& Body)
		{
			ProcessMasteryResponse(ReqId, bSuccess, Body);
		});

	return ReqId;
}

int64 UTowerGRPCClientManager::RequestWallet(int64 PlayerId)
{
	int64 ReqId = AllocateRequestId();

	TSharedPtr<FJsonObject> Payload = MakeShareable(new FJsonObject());
	Payload->SetNumberField(TEXT("player_id"), static_cast<double>(PlayerId));

	SendRequest(
		TEXT("/tower.EconomyService/GetWallet"),
		SerializeJson(Payload),
		ReqId,
		[this, ReqId](bool bSuccess, const FString& Body)
		{
			ProcessWalletResponse(ReqId, bSuccess, Body);
		});

	return ReqId;
}

int64 UTowerGRPCClientManager::RequestLoot(int64 SourceEntityId, int64 PlayerId, float LuckModifier)
{
	int64 ReqId = AllocateRequestId();

	TSharedPtr<FJsonObject> Payload = MakeShareable(new FJsonObject());
	Payload->SetNumberField(TEXT("source_entity_id"), static_cast<double>(SourceEntityId));
	Payload->SetNumberField(TEXT("player_id"), static_cast<double>(PlayerId));
	Payload->SetNumberField(TEXT("luck_modifier"), static_cast<double>(LuckModifier));

	SendRequest(
		TEXT("/tower.GenerationService/GenerateLoot"),
		SerializeJson(Payload),
		ReqId,
		[this, ReqId](bool bSuccess, const FString& Body)
		{
			ProcessLootResponse(ReqId, bSuccess, Body);
		});

	return ReqId;
}

// ============================================================
// Core HTTP transport
// ============================================================

FString UTowerGRPCClientManager::GetBaseUrl() const
{
	return FString::Printf(TEXT("http://%s:%d"), *Config.Host, Config.Port);
}

int64 UTowerGRPCClientManager::AllocateRequestId()
{
	return NextRequestId++;
}

void UTowerGRPCClientManager::SendRequest(
	const FString& ServicePath,
	const FString& PayloadJson,
	int64 RequestId,
	TFunction<void(bool bSuccess, const FString& ResponseBody)> OnResponse)
{
	if (ConnectionState != EConnectionState::Connected && ConnectionState != EConnectionState::Connecting)
	{
		UE_LOG(LogGRPCClient, Warning,
			TEXT("SendRequest(%s) dropped — not connected (state: %d)"),
			*ServicePath, static_cast<int32>(ConnectionState));
		HandleRequestFailure(RequestId, -1, TEXT("Not connected"));
		return;
	}

	TotalRequestsSent++;
	InFlightRequests.Add(RequestId, FPlatformTime::Seconds());

	FString Url = GetBaseUrl() + ServicePath;

	UE_LOG(LogGRPCClient, Verbose, TEXT(">> [%lld] POST %s"), RequestId, *Url);

	TSharedRef<IHttpRequest, ESPMode::ThreadSafe> Request = FHttpModule::Get().CreateRequest();
	Request->SetURL(Url);
	Request->SetVerb(TEXT("POST"));
	Request->SetHeader(TEXT("Content-Type"), TEXT("application/json"));
	Request->SetHeader(TEXT("Accept"), TEXT("application/json"));
	// gRPC-Web style header so the Rust server knows this is a proto-JSON call
	Request->SetHeader(TEXT("X-Tower-Transport"), TEXT("grpc-json"));
	Request->SetHeader(TEXT("X-Tower-Request-Id"), FString::Printf(TEXT("%lld"), RequestId));
	Request->SetTimeout(Config.TimeoutSeconds);

	if (!PayloadJson.IsEmpty())
	{
		Request->SetContentAsString(PayloadJson);
	}

	// Capture OnResponse by value
	Request->OnProcessRequestComplete().BindLambda(
		[this, RequestId, OnResponse](FHttpRequestPtr Req, FHttpResponsePtr Resp, bool bConnected)
		{
			if (bConnected && Resp.IsValid())
			{
				int32 Code = Resp->GetResponseCode();
				FString Body = Resp->GetContentAsString();
				bool bOk = (Code >= 200 && Code < 300);

				RecordLatency(RequestId);
				InFlightRequests.Remove(RequestId);

				if (bOk)
				{
					ConsecutiveFailures = 0;
					UE_LOG(LogGRPCClient, Verbose, TEXT("<< [%lld] %d OK (%d bytes)"),
						RequestId, Code, Body.Len());
					OnRequestCompleted.Broadcast(RequestId, Body);
					OnResponse(true, Body);
				}
				else
				{
					UE_LOG(LogGRPCClient, Warning, TEXT("<< [%lld] HTTP %d: %s"),
						RequestId, Code, *Body.Left(512));
					HandleRequestFailure(RequestId, Code, Body);
					OnResponse(false, Body);
				}
			}
			else
			{
				InFlightRequests.Remove(RequestId);

				UE_LOG(LogGRPCClient, Error,
					TEXT("<< [%lld] Connection failed (no response)"), RequestId);
				HandleRequestFailure(RequestId, -1, TEXT("Connection failed"));
				OnResponse(false, TEXT("{\"error\":\"connection_failed\"}"));

				// Trigger reconnect if we thought we were connected
				if (ConnectionState == EConnectionState::Connected)
				{
					Reconnect();
				}
			}
		});

	Request->ProcessRequest();
}

void UTowerGRPCClientManager::HandleRequestFailure(int64 RequestId, int32 ErrorCode, const FString& ErrorMessage)
{
	TotalRequestsFailed++;
	ConsecutiveFailures++;

	OnRequestFailed.Broadcast(RequestId, ErrorCode, ErrorMessage);
}

void UTowerGRPCClientManager::RecordLatency(int64 RequestId)
{
	if (const double* StartTime = InFlightRequests.Find(RequestId))
	{
		double ElapsedMs = (FPlatformTime::Seconds() - *StartTime) * 1000.0;

		// Exponential moving average (alpha = 0.2)
		if (AverageLatencyMs <= 0.0f)
		{
			AverageLatencyMs = static_cast<float>(ElapsedMs);
		}
		else
		{
			AverageLatencyMs = AverageLatencyMs * 0.8f + static_cast<float>(ElapsedMs) * 0.2f;
		}
	}
}

// ============================================================
// Response processors
// ============================================================

void UTowerGRPCClientManager::ProcessFloorResponse(int64 RequestId, bool bSuccess, const FString& ResponseBody)
{
	if (!bSuccess)
	{
		return;
	}

	// Broadcast raw JSON — the floor layout is complex and callers (FloorManager, etc.)
	// will parse it according to their own needs. We also broadcast the typed delegate
	// with the full JSON for Blueprint consumers.
	OnFloorGenerated.Broadcast(RequestId, ResponseBody);
}

void UTowerGRPCClientManager::ProcessDamageCalcResponse(int64 RequestId, bool bSuccess, const FString& ResponseBody)
{
	if (!bSuccess)
	{
		return;
	}

	TSharedPtr<FJsonObject> Json = ParseJson(ResponseBody);
	if (!Json.IsValid())
	{
		HandleRequestFailure(RequestId, -2, TEXT("Invalid JSON in damage calc response"));
		return;
	}

	FDamageCalcResult Result;
	Result.BaseDamage = static_cast<float>(Json->GetNumberField(TEXT("base_damage")));
	Result.ModifiedDamage = static_cast<float>(Json->GetNumberField(TEXT("modified_damage")));
	Result.CritChance = static_cast<float>(Json->GetNumberField(TEXT("crit_chance")));
	Result.CritDamage = static_cast<float>(Json->GetNumberField(TEXT("crit_damage")));

	const TArray<TSharedPtr<FJsonValue>>* ModifiersArray = nullptr;
	if (Json->TryGetArrayField(TEXT("modifiers"), ModifiersArray) && ModifiersArray)
	{
		for (const TSharedPtr<FJsonValue>& ModVal : *ModifiersArray)
		{
			const TSharedPtr<FJsonObject>& ModObj = ModVal->AsObject();
			if (ModObj.IsValid())
			{
				FDamageModifierResult Mod;
				Mod.Source = ModObj->GetStringField(TEXT("source"));
				Mod.Multiplier = static_cast<float>(ModObj->GetNumberField(TEXT("multiplier")));
				Mod.Description = ModObj->GetStringField(TEXT("description"));
				Result.Modifiers.Add(Mod);
			}
		}
	}

	OnDamageCalculated.Broadcast(RequestId, Result);
}

void UTowerGRPCClientManager::ProcessMasteryResponse(int64 RequestId, bool bSuccess, const FString& ResponseBody)
{
	if (!bSuccess)
	{
		return;
	}

	TSharedPtr<FJsonObject> Json = ParseJson(ResponseBody);
	if (!Json.IsValid())
	{
		HandleRequestFailure(RequestId, -2, TEXT("Invalid JSON in mastery response"));
		return;
	}

	FMasteryProgressResult Result;
	Result.Domain = Json->GetStringField(TEXT("domain"));
	Result.NewTier = static_cast<int32>(Json->GetNumberField(TEXT("new_tier")));
	Result.NewXP = static_cast<float>(Json->GetNumberField(TEXT("new_xp")));
	Result.XPToNext = static_cast<float>(Json->GetNumberField(TEXT("xp_to_next")));
	Result.bTierUp = Json->GetBoolField(TEXT("tier_up"));

	const TArray<TSharedPtr<FJsonValue>>* UnlockedArray = nullptr;
	if (Json->TryGetArrayField(TEXT("newly_unlocked"), UnlockedArray) && UnlockedArray)
	{
		for (const TSharedPtr<FJsonValue>& Val : *UnlockedArray)
		{
			Result.NewlyUnlocked.Add(Val->AsString());
		}
	}

	OnMasteryProgressReceived.Broadcast(RequestId, Result);
}

void UTowerGRPCClientManager::ProcessWalletResponse(int64 RequestId, bool bSuccess, const FString& ResponseBody)
{
	if (!bSuccess)
	{
		return;
	}

	TSharedPtr<FJsonObject> Json = ParseJson(ResponseBody);
	if (!Json.IsValid())
	{
		HandleRequestFailure(RequestId, -2, TEXT("Invalid JSON in wallet response"));
		return;
	}

	FWalletResult Result;
	Result.Gold = static_cast<int64>(Json->GetNumberField(TEXT("gold")));
	Result.PremiumCurrency = static_cast<int64>(Json->GetNumberField(TEXT("premium_currency")));
	Result.SeasonalCurrency = static_cast<int64>(Json->GetNumberField(TEXT("seasonal_currency")));

	OnWalletReceived.Broadcast(RequestId, Result);
}

void UTowerGRPCClientManager::ProcessLootResponse(int64 RequestId, bool bSuccess, const FString& ResponseBody)
{
	if (!bSuccess)
	{
		return;
	}

	TSharedPtr<FJsonObject> Json = ParseJson(ResponseBody);
	if (!Json.IsValid())
	{
		HandleRequestFailure(RequestId, -2, TEXT("Invalid JSON in loot response"));
		return;
	}

	TArray<FLootItemResult> Items;

	const TArray<TSharedPtr<FJsonValue>>* ItemsArray = nullptr;
	if (Json->TryGetArrayField(TEXT("items"), ItemsArray) && ItemsArray)
	{
		for (const TSharedPtr<FJsonValue>& ItemVal : *ItemsArray)
		{
			const TSharedPtr<FJsonObject>& ItemObj = ItemVal->AsObject();
			if (!ItemObj.IsValid())
			{
				continue;
			}

			FLootItemResult Item;
			Item.ItemName = ItemObj->GetStringField(TEXT("item_name"));
			Item.Rarity = static_cast<int32>(ItemObj->GetNumberField(TEXT("rarity")));
			Item.SocketCount = static_cast<int32>(ItemObj->GetNumberField(TEXT("socket_count")));

			const TArray<TSharedPtr<FJsonValue>>* TagsArray = nullptr;
			if (ItemObj->TryGetArrayField(TEXT("tags"), TagsArray) && TagsArray)
			{
				Item.Tags = ParseSemanticTags(*TagsArray);
			}

			Items.Add(Item);
		}
	}

	OnLootGenerated.Broadcast(RequestId, Items);
}

// ============================================================
// FFI Fallback
// ============================================================

bool UTowerGRPCClientManager::TryLoadFFIBridge()
{
	if (FFIDllHandle != nullptr)
	{
		return true;  // Already loaded
	}

	// Resolve DLL path relative to the project binaries
	FString DllPath = FPaths::Combine(
		FPaths::ProjectDir(),
		TEXT("Binaries"),
		FPlatformProcess::GetBinariesSubdirectory(),
		Config.FFIDllPath
	);

	if (!FPaths::FileExists(DllPath))
	{
		UE_LOG(LogGRPCClient, Warning, TEXT("FFI DLL not found at: %s"), *DllPath);
		return false;
	}

	FFIDllHandle = FPlatformProcess::GetDllHandle(*DllPath);
	if (FFIDllHandle == nullptr)
	{
		UE_LOG(LogGRPCClient, Error, TEXT("Failed to load FFI DLL: %s"), *DllPath);
		return false;
	}

	// Resolve function pointers
	FFIHealthCheck = reinterpret_cast<FFIHealthCheckFn>(
		FPlatformProcess::GetDllExport(FFIDllHandle, TEXT("tower_health_check")));
	FFIGenerateFloor = reinterpret_cast<FFIGenerateFloorFn>(
		FPlatformProcess::GetDllExport(FFIDllHandle, TEXT("tower_generate_floor")));
	FFICalculateDamage = reinterpret_cast<FFICalculateDamageFn>(
		FPlatformProcess::GetDllExport(FFIDllHandle, TEXT("tower_calculate_damage")));
	FFIFreeString = reinterpret_cast<FFIFreeStringFn>(
		FPlatformProcess::GetDllExport(FFIDllHandle, TEXT("tower_free_string")));

	if (FFIHealthCheck == nullptr)
	{
		UE_LOG(LogGRPCClient, Error,
			TEXT("FFI DLL loaded but missing required export 'tower_health_check'"));
		UnloadFFIBridge();
		return false;
	}

	// Verify the DLL is alive
	int32 HealthResult = FFIHealthCheck();
	if (HealthResult != 0)
	{
		UE_LOG(LogGRPCClient, Error,
			TEXT("FFI health check returned non-zero: %d"), HealthResult);
		UnloadFFIBridge();
		return false;
	}

	UE_LOG(LogGRPCClient, Log, TEXT("FFI DLL bridge loaded: %s"), *DllPath);
	return true;
}

void UTowerGRPCClientManager::UnloadFFIBridge()
{
	if (FFIDllHandle != nullptr)
	{
		FPlatformProcess::FreeDllHandle(FFIDllHandle);
		FFIDllHandle = nullptr;
		FFIHealthCheck = nullptr;
		FFIGenerateFloor = nullptr;
		FFICalculateDamage = nullptr;
		FFIFreeString = nullptr;

		UE_LOG(LogGRPCClient, Log, TEXT("FFI DLL bridge unloaded"));
	}
}

// ============================================================
// JSON helpers
// ============================================================

TSharedPtr<FJsonObject> UTowerGRPCClientManager::ParseJson(const FString& JsonString)
{
	TSharedPtr<FJsonObject> Result;
	TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(JsonString);

	if (!FJsonSerializer::Deserialize(Reader, Result) || !Result.IsValid())
	{
		UE_LOG(LogGRPCClient, Warning, TEXT("Failed to parse JSON: %s"), *JsonString.Left(256));
		return nullptr;
	}

	return Result;
}

FString UTowerGRPCClientManager::SerializeJson(const TSharedPtr<FJsonObject>& JsonObject)
{
	if (!JsonObject.IsValid())
	{
		return TEXT("{}");
	}

	FString Output;
	TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&Output);
	FJsonSerializer::Serialize(JsonObject.ToSharedRef(), Writer);
	return Output;
}

TArray<FSemanticTag> UTowerGRPCClientManager::ParseSemanticTags(const TArray<TSharedPtr<FJsonValue>>& JsonArray)
{
	TArray<FSemanticTag> Tags;
	Tags.Reserve(JsonArray.Num());

	for (const TSharedPtr<FJsonValue>& Val : JsonArray)
	{
		const TSharedPtr<FJsonObject>& TagObj = Val->AsObject();
		if (TagObj.IsValid())
		{
			FSemanticTag Tag;
			Tag.Name = TagObj->GetStringField(TEXT("name"));
			Tag.Value = static_cast<float>(TagObj->GetNumberField(TEXT("value")));
			Tags.Add(Tag);
		}
	}

	return Tags;
}
