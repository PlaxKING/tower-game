#pragma once

#include "CoreMinimal.h"
#include "Subsystems/GameInstanceSubsystem.h"
#include "Interfaces/IHttpRequest.h"
#include "Interfaces/IHttpResponse.h"
#include "NakamaSubsystem.generated.h"

DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnNakamaResponse, bool, bSuccess, const FString&, ResponseJson);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnNakamaAuthenticated, bool, bSuccess);

/**
 * Nakama Server communication subsystem.
 * Handles authentication, RPC calls, and state sync with the Nakama backend.
 *
 * RPC endpoints:
 * - get_tower_seed: Fetch shared tower seed
 * - request_floor: Validate + enter a floor
 * - report_floor_clear: Report floor completion
 * - report_death: Report player death + create echo
 * - get_floor_echoes: Fetch death echoes for a floor
 * - update_faction: Update faction standing
 * - get_player_state: Fetch full player state
 * - health_check: Server health/version check
 */
UCLASS()
class TOWERGAME_API UNakamaSubsystem : public UGameInstanceSubsystem
{
    GENERATED_BODY()

public:
    virtual void Initialize(FSubsystemCollectionBase& Collection) override;
    virtual void Deinitialize() override;

    // ============ Configuration ============

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Nakama|Config")
    FString ServerHost = TEXT("127.0.0.1");

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Nakama|Config")
    int32 ServerPort = 7350;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Nakama|Config")
    FString ServerKey = TEXT("defaultkey");

    // ============ Authentication ============

    /** Authenticate with device ID (anonymous login) */
    UFUNCTION(BlueprintCallable, Category = "Nakama|Auth")
    void AuthenticateDevice(const FString& DeviceId);

    /** Authenticate with email/password */
    UFUNCTION(BlueprintCallable, Category = "Nakama|Auth")
    void AuthenticateEmail(const FString& Email, const FString& Password, bool bCreate = true);

    /** Is the player authenticated? */
    UFUNCTION(BlueprintPure, Category = "Nakama|Auth")
    bool IsAuthenticated() const { return !AuthToken.IsEmpty(); }

    /** Get current session token */
    UFUNCTION(BlueprintPure, Category = "Nakama|Auth")
    FString GetAuthToken() const { return AuthToken; }

    UPROPERTY(BlueprintAssignable, Category = "Nakama|Auth")
    FOnNakamaAuthenticated OnAuthenticated;

    // ============ Tower RPC Calls ============

    /** Fetch the shared tower seed from server */
    UFUNCTION(BlueprintCallable, Category = "Nakama|Tower")
    void FetchTowerSeed();

    /** Request to enter a floor (validates access) */
    UFUNCTION(BlueprintCallable, Category = "Nakama|Tower")
    void RequestFloor(int32 FloorId);

    /** Report floor cleared with stats */
    UFUNCTION(BlueprintCallable, Category = "Nakama|Tower")
    void ReportFloorClear(int32 FloorId, int32 Kills, float ClearTimeSeconds);

    /** Report player death and create echo */
    UFUNCTION(BlueprintCallable, Category = "Nakama|Tower")
    void ReportDeath(int32 FloorId, const FString& EchoType, FVector Position);

    /** Fetch echoes for a floor */
    UFUNCTION(BlueprintCallable, Category = "Nakama|Tower")
    void FetchFloorEchoes(int32 FloorId);

    /** Update faction standing */
    UFUNCTION(BlueprintCallable, Category = "Nakama|Tower")
    void UpdateFaction(const FString& Faction, int32 Delta);

    /** Fetch full player state */
    UFUNCTION(BlueprintCallable, Category = "Nakama|Tower")
    void FetchPlayerState();

    /** Server health check */
    UFUNCTION(BlueprintCallable, Category = "Nakama|Tower")
    void HealthCheck();

    /** Join or create a floor match instance */
    UFUNCTION(BlueprintCallable, Category = "Nakama|Match")
    void JoinFloorMatch(int32 FloorId);

    /** List active floor matches */
    UFUNCTION(BlueprintCallable, Category = "Nakama|Match")
    void ListActiveMatches();

    // ============ Response Delegates ============

    UPROPERTY(BlueprintAssignable, Category = "Nakama|Events")
    FOnNakamaResponse OnTowerSeedReceived;

    UPROPERTY(BlueprintAssignable, Category = "Nakama|Events")
    FOnNakamaResponse OnFloorRequested;

    UPROPERTY(BlueprintAssignable, Category = "Nakama|Events")
    FOnNakamaResponse OnFloorCleared;

    UPROPERTY(BlueprintAssignable, Category = "Nakama|Events")
    FOnNakamaResponse OnDeathReported;

    UPROPERTY(BlueprintAssignable, Category = "Nakama|Events")
    FOnNakamaResponse OnEchoesReceived;

    UPROPERTY(BlueprintAssignable, Category = "Nakama|Events")
    FOnNakamaResponse OnFactionUpdated;

    UPROPERTY(BlueprintAssignable, Category = "Nakama|Events")
    FOnNakamaResponse OnPlayerStateReceived;

    UPROPERTY(BlueprintAssignable, Category = "Nakama|Events")
    FOnNakamaResponse OnHealthCheckReceived;

    UPROPERTY(BlueprintAssignable, Category = "Nakama|Events")
    FOnNakamaResponse OnFloorMatchJoined;

    UPROPERTY(BlueprintAssignable, Category = "Nakama|Events")
    FOnNakamaResponse OnActiveMatchesReceived;

    // ============ Cached State ============

    /** Server-provided tower seed */
    UPROPERTY(BlueprintReadOnly, Category = "Nakama|State")
    int64 ServerTowerSeed = 0;

    /** Player's highest floor from server */
    UPROPERTY(BlueprintReadOnly, Category = "Nakama|State")
    int32 ServerHighestFloor = 1;

    /** Current match ID (if in a floor match) */
    UPROPERTY(BlueprintReadOnly, Category = "Nakama|State")
    FString CurrentMatchId;

    /** User ID from authentication */
    UPROPERTY(BlueprintReadOnly, Category = "Nakama|State")
    FString UserId;

    /** Username from authentication */
    UPROPERTY(BlueprintReadOnly, Category = "Nakama|State")
    FString Username;

private:
    FString AuthToken;

    /** Build base URL for Nakama API */
    FString GetBaseUrl() const;

    /** Send an RPC call to Nakama */
    void CallRpc(const FString& RpcId, const FString& PayloadJson, FOnNakamaResponse& ResponseDelegate);

    /** Generic HTTP request helper */
    void SendHttpRequest(
        const FString& Url,
        const FString& Verb,
        const FString& ContentJson,
        TFunction<void(bool, const FString&)> Callback
    );

    /** Handle auth response */
    void ProcessAuthResponse(bool bSuccess, const FString& ResponseJson);
};
