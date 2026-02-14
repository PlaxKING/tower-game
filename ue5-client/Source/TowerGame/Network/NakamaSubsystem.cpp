#include "NakamaSubsystem.h"
#include "HttpModule.h"
#include "Interfaces/IHttpRequest.h"
#include "Interfaces/IHttpResponse.h"
#include "Dom/JsonObject.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"
#include "Serialization/JsonWriter.h"
#include "Misc/Base64.h"

void UNakamaSubsystem::Initialize(FSubsystemCollectionBase& Collection)
{
    Super::Initialize(Collection);
    UE_LOG(LogTemp, Log, TEXT("NakamaSubsystem initialized. Server: %s:%d"), *ServerHost, ServerPort);
}

void UNakamaSubsystem::Deinitialize()
{
    AuthToken.Empty();
    Super::Deinitialize();
}

FString UNakamaSubsystem::GetBaseUrl() const
{
    return FString::Printf(TEXT("http://%s:%d"), *ServerHost, ServerPort);
}

// ============ HTTP Helper ============

void UNakamaSubsystem::SendHttpRequest(
    const FString& Url,
    const FString& Verb,
    const FString& ContentJson,
    TFunction<void(bool, const FString&)> Callback)
{
    TSharedRef<IHttpRequest, ESPMode::ThreadSafe> Request = FHttpModule::Get().CreateRequest();
    Request->SetURL(Url);
    Request->SetVerb(Verb);
    Request->SetHeader(TEXT("Content-Type"), TEXT("application/json"));
    Request->SetHeader(TEXT("Accept"), TEXT("application/json"));

    // Auth header
    if (!AuthToken.IsEmpty())
    {
        Request->SetHeader(TEXT("Authorization"), FString::Printf(TEXT("Bearer %s"), *AuthToken));
    }
    else
    {
        // Basic auth with server key for unauthenticated requests
        FString BasicAuth = FBase64::Encode(FString::Printf(TEXT("%s:"), *ServerKey));
        Request->SetHeader(TEXT("Authorization"), FString::Printf(TEXT("Basic %s"), *BasicAuth));
    }

    if (!ContentJson.IsEmpty())
    {
        Request->SetContentAsString(ContentJson);
    }

    // Capture callback by value
    Request->OnProcessRequestComplete().BindLambda(
        [Callback](FHttpRequestPtr Req, FHttpResponsePtr Resp, bool bConnected)
        {
            if (bConnected && Resp.IsValid())
            {
                int32 Code = Resp->GetResponseCode();
                FString Body = Resp->GetContentAsString();
                bool bOk = (Code >= 200 && Code < 300);
                if (!bOk)
                {
                    UE_LOG(LogTemp, Warning, TEXT("Nakama HTTP %d: %s"), Code, *Body);
                }
                Callback(bOk, Body);
            }
            else
            {
                UE_LOG(LogTemp, Error, TEXT("Nakama HTTP request failed (no connection)"));
                Callback(false, TEXT("{\"error\":\"connection_failed\"}"));
            }
        });

    Request->ProcessRequest();
}

// ============ Authentication ============

void UNakamaSubsystem::AuthenticateDevice(const FString& DeviceId)
{
    FString Url = FString::Printf(TEXT("%s/v2/account/authenticate/device?create=true"), *GetBaseUrl());

    TSharedPtr<FJsonObject> Body = MakeShareable(new FJsonObject());
    Body->SetStringField(TEXT("id"), DeviceId);

    FString BodyJson;
    TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&BodyJson);
    FJsonSerializer::Serialize(Body.ToSharedRef(), Writer);

    SendHttpRequest(Url, TEXT("POST"), BodyJson,
        [this](bool bSuccess, const FString& Response)
        {
            ProcessAuthResponse(bSuccess, Response);
        });
}

void UNakamaSubsystem::AuthenticateEmail(const FString& Email, const FString& Password, bool bCreate)
{
    FString Url = FString::Printf(TEXT("%s/v2/account/authenticate/email?create=%s"),
        *GetBaseUrl(), bCreate ? TEXT("true") : TEXT("false"));

    TSharedPtr<FJsonObject> Body = MakeShareable(new FJsonObject());
    Body->SetStringField(TEXT("email"), Email);
    Body->SetStringField(TEXT("password"), Password);

    FString BodyJson;
    TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&BodyJson);
    FJsonSerializer::Serialize(Body.ToSharedRef(), Writer);

    SendHttpRequest(Url, TEXT("POST"), BodyJson,
        [this](bool bSuccess, const FString& Response)
        {
            ProcessAuthResponse(bSuccess, Response);
        });
}

void UNakamaSubsystem::ProcessAuthResponse(bool bSuccess, const FString& ResponseJson)
{
    if (bSuccess)
    {
        TSharedPtr<FJsonObject> Json;
        TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(ResponseJson);
        if (FJsonSerializer::Deserialize(Reader, Json) && Json.IsValid())
        {
            AuthToken = Json->GetStringField(TEXT("token"));
            // Decode token to get user_id (JWT payload)
            // For simplicity, we fetch account info separately
            UE_LOG(LogTemp, Log, TEXT("Nakama authenticated successfully"));
        }
    }
    else
    {
        UE_LOG(LogTemp, Error, TEXT("Nakama authentication failed: %s"), *ResponseJson);
    }

    OnAuthenticated.Broadcast(bSuccess);
}

// ============ RPC Calls ============

void UNakamaSubsystem::CallRpc(const FString& RpcId, const FString& PayloadJson, FOnNakamaResponse& ResponseDelegate)
{
    if (!IsAuthenticated())
    {
        UE_LOG(LogTemp, Warning, TEXT("Cannot call RPC '%s': not authenticated"), *RpcId);
        ResponseDelegate.Broadcast(false, TEXT("{\"error\":\"not_authenticated\"}"));
        return;
    }

    FString Url = FString::Printf(TEXT("%s/v2/rpc/%s"), *GetBaseUrl(), *RpcId);

    // Wrap payload in Nakama RPC format
    TSharedPtr<FJsonObject> RpcBody = MakeShareable(new FJsonObject());
    RpcBody->SetStringField(TEXT("payload"), PayloadJson);

    FString BodyJson;
    TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&BodyJson);
    FJsonSerializer::Serialize(RpcBody.ToSharedRef(), Writer);

    SendHttpRequest(Url, TEXT("POST"), BodyJson,
        [&ResponseDelegate](bool bSuccess, const FString& Response)
        {
            // Extract payload from Nakama RPC response
            FString ResultPayload = Response;
            if (bSuccess)
            {
                TSharedPtr<FJsonObject> Json;
                TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(Response);
                if (FJsonSerializer::Deserialize(Reader, Json) && Json.IsValid())
                {
                    ResultPayload = Json->GetStringField(TEXT("payload"));
                }
            }
            ResponseDelegate.Broadcast(bSuccess, ResultPayload);
        });
}

void UNakamaSubsystem::FetchTowerSeed()
{
    CallRpc(TEXT("get_tower_seed"), TEXT("{}"), OnTowerSeedReceived);
}

void UNakamaSubsystem::RequestFloor(int32 FloorId)
{
    FString Payload = FString::Printf(TEXT("{\"floor_id\":%d}"), FloorId);
    CallRpc(TEXT("request_floor"), Payload, OnFloorRequested);
}

void UNakamaSubsystem::ReportFloorClear(int32 FloorId, int32 Kills, float ClearTimeSeconds)
{
    FString Payload = FString::Printf(TEXT("{\"floor_id\":%d,\"kills\":%d,\"clear_time_seconds\":%.2f}"),
        FloorId, Kills, ClearTimeSeconds);
    CallRpc(TEXT("report_floor_clear"), Payload, OnFloorCleared);
}

void UNakamaSubsystem::ReportDeath(int32 FloorId, const FString& EchoType, FVector Position)
{
    FString Payload = FString::Printf(
        TEXT("{\"floor_id\":%d,\"echo_type\":\"%s\",\"position\":{\"x\":%.1f,\"y\":%.1f,\"z\":%.1f}}"),
        FloorId, *EchoType, Position.X, Position.Y, Position.Z);
    CallRpc(TEXT("report_death"), Payload, OnDeathReported);
}

void UNakamaSubsystem::FetchFloorEchoes(int32 FloorId)
{
    FString Payload = FString::Printf(TEXT("{\"floor_id\":%d}"), FloorId);
    CallRpc(TEXT("get_floor_echoes"), Payload, OnEchoesReceived);
}

void UNakamaSubsystem::UpdateFaction(const FString& Faction, int32 Delta)
{
    FString Payload = FString::Printf(TEXT("{\"faction\":\"%s\",\"delta\":%d}"), *Faction, Delta);
    CallRpc(TEXT("update_faction"), Payload, OnFactionUpdated);
}

void UNakamaSubsystem::FetchPlayerState()
{
    CallRpc(TEXT("get_player_state"), TEXT("{}"), OnPlayerStateReceived);
}

void UNakamaSubsystem::HealthCheck()
{
    CallRpc(TEXT("health_check"), TEXT("{}"), OnHealthCheckReceived);
}

void UNakamaSubsystem::JoinFloorMatch(int32 FloorId)
{
    FString Payload = FString::Printf(TEXT("{\"floor_id\":%d}"), FloorId);
    CallRpc(TEXT("join_floor_match"), Payload, OnFloorMatchJoined);
}

void UNakamaSubsystem::ListActiveMatches()
{
    CallRpc(TEXT("list_active_matches"), TEXT("{}"), OnActiveMatchesReceived);
}
