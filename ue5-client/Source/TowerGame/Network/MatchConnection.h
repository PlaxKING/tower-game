#pragma once

#include "CoreMinimal.h"
#include "Subsystems/GameInstanceSubsystem.h"
#include "IWebSocket.h"
#include "MatchConnection.generated.h"

/**
 * Match OpCodes — must match tower_match.lua
 */
UENUM(BlueprintType)
enum class EMatchOpCode : uint8
{
    PlayerPosition  = 1,
    PlayerAttack    = 2,
    MonsterDamage   = 3,
    MonsterDefeated = 4,
    PlayerDeath     = 5,
    FloorClear      = 6,
    BreathSync      = 7,
    PlayerJoined    = 8,
    PlayerLeft      = 9,
    ChatMessage     = 10,
    LootDropped     = 11,
    PlayerInteract  = 12,
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnMatchData, EMatchOpCode, OpCode, const FString&, DataJson);
DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnMatchConnected);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnMatchDisconnected, const FString&, Reason);

/**
 * WebSocket connection to a Nakama match instance.
 *
 * Handles real-time bidirectional communication for:
 * - Player position sync (5Hz)
 * - Combat events (attacks, damage, deaths)
 * - Monster state sync
 * - Floor clear notifications
 * - Breath of Tower phase sync
 * - Chat messages
 *
 * Usage:
 * 1. Call Connect() with match_id from NakamaSubsystem::JoinFloorMatch response
 * 2. Bind to OnMatchData for incoming events
 * 3. Call SendMatchData() to broadcast player actions
 * 4. Call Disconnect() when leaving the floor
 */
UCLASS()
class TOWERGAME_API UMatchConnection : public UGameInstanceSubsystem
{
    GENERATED_BODY()

public:
    virtual void Initialize(FSubsystemCollectionBase& Collection) override;
    virtual void Deinitialize() override;

    // ============ Connection ============

    /** Connect to a match via WebSocket */
    UFUNCTION(BlueprintCallable, Category = "Match")
    void Connect(const FString& MatchId, const FString& AuthToken);

    /** Disconnect from current match */
    UFUNCTION(BlueprintCallable, Category = "Match")
    void Disconnect();

    /** Is WebSocket connected? */
    UFUNCTION(BlueprintPure, Category = "Match")
    bool IsConnected() const { return bConnected; }

    /** Get current match ID */
    UFUNCTION(BlueprintPure, Category = "Match")
    FString GetMatchId() const { return CurrentMatchId; }

    // ============ Send Data ============

    /** Send match data with op code */
    UFUNCTION(BlueprintCallable, Category = "Match")
    void SendMatchData(EMatchOpCode OpCode, const FString& DataJson);

    /** Send player position update */
    UFUNCTION(BlueprintCallable, Category = "Match")
    void SendPosition(FVector Position, FRotator Rotation);

    /** Send attack event */
    UFUNCTION(BlueprintCallable, Category = "Match")
    void SendAttack(int32 TargetMonsterId, float Damage, int32 AngleId, int32 ComboStep);

    /** Send death event */
    UFUNCTION(BlueprintCallable, Category = "Match")
    void SendDeath(const FString& EchoType, FVector Position);

    /** Send chat message */
    UFUNCTION(BlueprintCallable, Category = "Match")
    void SendChat(const FString& Message);

    /** Send interact event */
    UFUNCTION(BlueprintCallable, Category = "Match")
    void SendInteract(const FString& InteractType, const FString& TargetId);

    // ============ Events ============

    UPROPERTY(BlueprintAssignable, Category = "Match|Events")
    FOnMatchConnected OnConnected;

    UPROPERTY(BlueprintAssignable, Category = "Match|Events")
    FOnMatchDisconnected OnDisconnected;

    UPROPERTY(BlueprintAssignable, Category = "Match|Events")
    FOnMatchData OnMatchData;

    // ============ Config ============

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Match|Config")
    FString ServerHost = TEXT("127.0.0.1");

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Match|Config")
    int32 ServerPort = 7350;

    /** Position update send rate (Hz) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Match|Config")
    float PositionSendRate = 5.0f;

private:
    TSharedPtr<IWebSocket> WebSocket;
    FString CurrentMatchId;
    FString CurrentToken;
    bool bConnected = false;
    float PositionSendTimer = 0.0f;

    void OnWebSocketConnected();
    void OnWebSocketConnectionError(const FString& Error);
    void OnWebSocketClosed(int32 StatusCode, const FString& Reason, bool bWasClean);
    void OnWebSocketMessage(const FString& Message);

    /** Build WebSocket URL */
    FString GetWebSocketUrl() const;

    /** Encode match data for Nakama wire format */
    FString EncodeMatchData(EMatchOpCode OpCode, const FString& DataJson);

    /** Parse incoming Nakama match data */
    void ParseMatchMessage(const FString& Message);
};
