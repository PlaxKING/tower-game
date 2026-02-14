#include "MatchConnection.h"
#include "WebSocketsModule.h"
#include "IWebSocket.h"
#include "Dom/JsonObject.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"
#include "Serialization/JsonWriter.h"
#include "Misc/Base64.h"

void UMatchConnection::Initialize(FSubsystemCollectionBase& Collection)
{
    Super::Initialize(Collection);

    // Ensure WebSockets module is loaded
    FModuleManager::Get().LoadModuleChecked<FWebSocketsModule>(TEXT("WebSockets"));

    UE_LOG(LogTemp, Log, TEXT("MatchConnection subsystem initialized"));
}

void UMatchConnection::Deinitialize()
{
    Disconnect();
    Super::Deinitialize();
}

// ============ Connection ============

FString UMatchConnection::GetWebSocketUrl() const
{
    return FString::Printf(TEXT("ws://%s:%d/ws?token=%s"),
        *ServerHost, ServerPort, *CurrentToken);
}

void UMatchConnection::Connect(const FString& MatchId, const FString& AuthToken)
{
    if (bConnected)
    {
        Disconnect();
    }

    CurrentMatchId = MatchId;
    CurrentToken = AuthToken;

    FString Url = GetWebSocketUrl();
    TArray<FString> Protocols;
    Protocols.Add(TEXT("json"));

    TMap<FString, FString> Headers;

    WebSocket = FWebSocketsModule::Get().CreateWebSocket(Url, Protocols, Headers);

    WebSocket->OnConnected().AddUObject(this, &UMatchConnection::OnWebSocketConnected);
    WebSocket->OnConnectionError().AddUObject(this, &UMatchConnection::OnWebSocketConnectionError);
    WebSocket->OnClosed().AddUObject(this, &UMatchConnection::OnWebSocketClosed);
    WebSocket->OnMessage().AddUObject(this, &UMatchConnection::OnWebSocketMessage);

    UE_LOG(LogTemp, Log, TEXT("Connecting to match %s..."), *MatchId);
    WebSocket->Connect();
}

void UMatchConnection::Disconnect()
{
    if (WebSocket.IsValid())
    {
        if (WebSocket->IsConnected())
        {
            WebSocket->Close();
        }
        WebSocket.Reset();
    }

    bConnected = false;
    CurrentMatchId.Empty();
}

// ============ WebSocket Callbacks ============

void UMatchConnection::OnWebSocketConnected()
{
    bConnected = true;
    UE_LOG(LogTemp, Log, TEXT("WebSocket connected to match %s"), *CurrentMatchId);

    // Send match join message
    TSharedPtr<FJsonObject> JoinMsg = MakeShareable(new FJsonObject());
    TSharedPtr<FJsonObject> MatchJoin = MakeShareable(new FJsonObject());
    MatchJoin->SetStringField(TEXT("match_id"), CurrentMatchId);
    JoinMsg->SetObjectField(TEXT("match_join"), MatchJoin);

    FString JoinJson;
    TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&JoinJson);
    FJsonSerializer::Serialize(JoinMsg.ToSharedRef(), Writer);

    WebSocket->Send(JoinJson);

    OnConnected.Broadcast();
}

void UMatchConnection::OnWebSocketConnectionError(const FString& Error)
{
    UE_LOG(LogTemp, Error, TEXT("WebSocket connection error: %s"), *Error);
    bConnected = false;
    OnDisconnected.Broadcast(Error);
}

void UMatchConnection::OnWebSocketClosed(int32 StatusCode, const FString& Reason, bool bWasClean)
{
    UE_LOG(LogTemp, Log, TEXT("WebSocket closed: %d %s (clean: %s)"),
        StatusCode, *Reason, bWasClean ? TEXT("yes") : TEXT("no"));
    bConnected = false;
    OnDisconnected.Broadcast(Reason);
}

void UMatchConnection::OnWebSocketMessage(const FString& Message)
{
    ParseMatchMessage(Message);
}

// ============ Send Data ============

void UMatchConnection::SendMatchData(EMatchOpCode OpCode, const FString& DataJson)
{
    if (!bConnected || !WebSocket.IsValid()) return;

    FString Encoded = EncodeMatchData(OpCode, DataJson);
    WebSocket->Send(Encoded);
}

void UMatchConnection::SendPosition(FVector Position, FRotator Rotation)
{
    FString Json = FString::Printf(
        TEXT("{\"position\":{\"x\":%.1f,\"y\":%.1f,\"z\":%.1f},\"rotation\":{\"yaw\":%.1f}}"),
        Position.X, Position.Y, Position.Z, Rotation.Yaw);
    SendMatchData(EMatchOpCode::PlayerPosition, Json);
}

void UMatchConnection::SendAttack(int32 TargetMonsterId, float Damage, int32 AngleId, int32 ComboStep)
{
    FString Json = FString::Printf(
        TEXT("{\"target_id\":%d,\"damage\":%.1f,\"angle_id\":%d,\"combo_step\":%d}"),
        TargetMonsterId, Damage, AngleId, ComboStep);
    SendMatchData(EMatchOpCode::PlayerAttack, Json);
}

void UMatchConnection::SendDeath(const FString& EchoType, FVector Position)
{
    FString Json = FString::Printf(
        TEXT("{\"echo_type\":\"%s\",\"position\":{\"x\":%.1f,\"y\":%.1f,\"z\":%.1f}}"),
        *EchoType, Position.X, Position.Y, Position.Z);
    SendMatchData(EMatchOpCode::PlayerDeath, Json);
}

void UMatchConnection::SendChat(const FString& Message)
{
    // Sanitize message
    FString SafeMsg = Message.Left(200).Replace(TEXT("\""), TEXT("\\\""));
    FString Json = FString::Printf(TEXT("{\"message\":\"%s\"}"), *SafeMsg);
    SendMatchData(EMatchOpCode::ChatMessage, Json);
}

void UMatchConnection::SendInteract(const FString& InteractType, const FString& TargetId)
{
    FString Json = FString::Printf(TEXT("{\"type\":\"%s\",\"target_id\":\"%s\"}"),
        *InteractType, *TargetId);
    SendMatchData(EMatchOpCode::PlayerInteract, Json);
}

// ============ Wire Format ============

FString UMatchConnection::EncodeMatchData(EMatchOpCode OpCode, const FString& DataJson)
{
    // Nakama match_data_send format
    TSharedPtr<FJsonObject> Msg = MakeShareable(new FJsonObject());
    TSharedPtr<FJsonObject> MatchData = MakeShareable(new FJsonObject());

    MatchData->SetStringField(TEXT("match_id"), CurrentMatchId);
    MatchData->SetNumberField(TEXT("op_code"), static_cast<int32>(OpCode));

    // Base64 encode the data payload
    FString DataBase64 = FBase64::Encode(DataJson);
    MatchData->SetStringField(TEXT("data"), DataBase64);

    Msg->SetObjectField(TEXT("match_data_send"), MatchData);

    FString Result;
    TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&Result);
    FJsonSerializer::Serialize(Msg.ToSharedRef(), Writer);

    return Result;
}

void UMatchConnection::ParseMatchMessage(const FString& Message)
{
    TSharedPtr<FJsonObject> Json;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(Message);
    if (!FJsonSerializer::Deserialize(Reader, Json) || !Json.IsValid())
    {
        return;
    }

    // Check for match_data (incoming match state)
    const TSharedPtr<FJsonObject>* MatchDataPtr;
    if (Json->TryGetObjectField(TEXT("match_data"), MatchDataPtr))
    {
        const TSharedPtr<FJsonObject>& MatchData = *MatchDataPtr;

        int32 OpCodeInt = static_cast<int32>(MatchData->GetNumberField(TEXT("op_code")));
        EMatchOpCode OpCode = static_cast<EMatchOpCode>(OpCodeInt);

        // Decode base64 data
        FString DataBase64 = MatchData->GetStringField(TEXT("data"));
        FString DataJson;
        FBase64::Decode(DataBase64, DataJson);

        // Broadcast to listeners
        OnMatchData.Broadcast(OpCode, DataJson);
    }

    // Check for match_presence_event (join/leave)
    const TSharedPtr<FJsonObject>* PresencePtr;
    if (Json->TryGetObjectField(TEXT("match_presence_event"), PresencePtr))
    {
        // Already handled by match handler sending OpCode 8/9
    }

    // Check for match error
    const TSharedPtr<FJsonObject>* ErrorPtr;
    if (Json->TryGetObjectField(TEXT("error"), ErrorPtr))
    {
        FString ErrorMsg = (*ErrorPtr)->GetStringField(TEXT("message"));
        UE_LOG(LogTemp, Error, TEXT("Match error: %s"), *ErrorMsg);
    }
}
