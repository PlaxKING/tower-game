#include "PlayerSyncComponent.h"
#include "RemotePlayer.h"
#include "Player/TowerPlayerCharacter.h"
#include "Kismet/GameplayStatics.h"
#include "Dom/JsonObject.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"
#include "Serialization/JsonWriter.h"
#include "Engine/World.h"

UPlayerSyncComponent::UPlayerSyncComponent()
{
    PrimaryComponentTick.bCanEverTick = true;
    PrimaryComponentTick.TickInterval = 0.0f; // Every frame for smooth interp
}

void UPlayerSyncComponent::BeginPlay()
{
    Super::BeginPlay();

    // Bind to match connection events
    UMatchConnection* Match = GetMatchConnection();
    if (Match)
    {
        Match->OnMatchData.AddDynamic(this, &UPlayerSyncComponent::OnMatchDataReceived);
    }
}

void UPlayerSyncComponent::EndPlay(const EEndPlayReason::Type EndPlayReason)
{
    StopSync();

    UMatchConnection* Match = GetMatchConnection();
    if (Match)
    {
        Match->OnMatchData.RemoveDynamic(this, &UPlayerSyncComponent::OnMatchDataReceived);
    }

    Super::EndPlay(EndPlayReason);
}

void UPlayerSyncComponent::TickComponent(float DeltaTime, ELevelTick TickType,
    FActorComponentTickFunction* ThisTickFunction)
{
    Super::TickComponent(DeltaTime, TickType, ThisTickFunction);

    if (!bSyncing) return;

    // Send local position at configured rate
    SendTimer += DeltaTime;
    float SendInterval = 1.0f / FMath::Max(SendRate, 1.0f);

    if (SendTimer >= SendInterval)
    {
        SendTimer = 0.0f;
        BroadcastLocalPosition();
    }
}

void UPlayerSyncComponent::StartSync()
{
    bSyncing = true;
    SendTimer = 0.0f;
    UE_LOG(LogTemp, Log, TEXT("PlayerSync: started"));
}

void UPlayerSyncComponent::StopSync()
{
    bSyncing = false;
    DespawnAllRemotePlayers();
    UE_LOG(LogTemp, Log, TEXT("PlayerSync: stopped"));
}

TArray<ARemotePlayer*> UPlayerSyncComponent::GetRemotePlayers() const
{
    TArray<ARemotePlayer*> Result;
    RemotePlayers.GenerateValueArray(Result);
    return Result;
}

ARemotePlayer* UPlayerSyncComponent::GetRemotePlayerById(const FString& UserId) const
{
    const ARemotePlayer* const* Found = RemotePlayers.Find(UserId);
    return Found ? const_cast<ARemotePlayer*>(*Found) : nullptr;
}

UMatchConnection* UPlayerSyncComponent::GetMatchConnection() const
{
    UGameInstance* GI = GetOwner() ? GetOwner()->GetGameInstance() : nullptr;
    return GI ? GI->GetSubsystem<UMatchConnection>() : nullptr;
}

void UPlayerSyncComponent::OnMatchDataReceived(EMatchOpCode OpCode, const FString& DataJson)
{
    // Parse sender user ID from the data
    TSharedPtr<FJsonObject> Json;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(DataJson);

    if (!FJsonSerializer::Deserialize(Reader, Json) || !Json.IsValid()) return;

    FString UserId = Json->GetStringField(TEXT("user_id"));
    if (UserId.IsEmpty()) return;

    // Skip messages from self
    // (In production, the server would filter these)

    switch (OpCode)
    {
    case EMatchOpCode::PlayerPosition:
        HandlePlayerPosition(UserId, DataJson);
        break;
    case EMatchOpCode::PlayerAttack:
        HandlePlayerAttack(UserId, DataJson);
        break;
    case EMatchOpCode::PlayerDeath:
        HandlePlayerDeath(UserId, DataJson);
        break;
    case EMatchOpCode::PlayerJoined:
        HandlePlayerJoined(UserId, DataJson);
        break;
    case EMatchOpCode::PlayerLeft:
        HandlePlayerLeft(UserId);
        break;
    case EMatchOpCode::ChatMessage:
        HandleChat(UserId, DataJson);
        break;
    default:
        break; // Other op codes handled elsewhere (GameMode, etc.)
    }
}

void UPlayerSyncComponent::HandlePlayerPosition(const FString& UserId, const FString& DataJson)
{
    TSharedPtr<FJsonObject> Json;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(DataJson);
    if (!FJsonSerializer::Deserialize(Reader, Json)) return;

    FVector Position;
    Position.X = Json->GetNumberField(TEXT("x"));
    Position.Y = Json->GetNumberField(TEXT("y"));
    Position.Z = Json->GetNumberField(TEXT("z"));

    FRotator Rotation;
    Rotation.Yaw = Json->GetNumberField(TEXT("yaw"));

    ARemotePlayer* Remote = GetRemotePlayerById(UserId);
    if (!Remote)
    {
        // Auto-spawn if we get position before join message
        FString Name = Json->HasField(TEXT("name")) ? Json->GetStringField(TEXT("name")) : UserId;
        Remote = SpawnRemotePlayer(UserId, Name);
    }

    if (Remote)
    {
        Remote->ApplyPositionUpdate(Position, Rotation);
    }
}

void UPlayerSyncComponent::HandlePlayerAttack(const FString& UserId, const FString& DataJson)
{
    ARemotePlayer* Remote = GetRemotePlayerById(UserId);
    if (!Remote) return;

    TSharedPtr<FJsonObject> Json;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(DataJson);
    if (!FJsonSerializer::Deserialize(Reader, Json)) return;

    int32 ComboStep = Json->GetIntegerField(TEXT("combo_step"));
    int32 WeaponType = Json->HasField(TEXT("weapon_type")) ? Json->GetIntegerField(TEXT("weapon_type")) : 0;

    Remote->PlayAttackAnimation(ComboStep, WeaponType);
}

void UPlayerSyncComponent::HandlePlayerDeath(const FString& UserId, const FString& DataJson)
{
    ARemotePlayer* Remote = GetRemotePlayerById(UserId);
    if (Remote)
    {
        Remote->ShowDeath();
    }
}

void UPlayerSyncComponent::HandlePlayerJoined(const FString& UserId, const FString& DataJson)
{
    if (RemotePlayers.Contains(UserId)) return; // Already tracked

    TSharedPtr<FJsonObject> Json;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(DataJson);

    FString DisplayName = UserId;
    if (FJsonSerializer::Deserialize(Reader, Json) && Json.IsValid())
    {
        DisplayName = Json->GetStringField(TEXT("name"));
    }

    SpawnRemotePlayer(UserId, DisplayName);
    UE_LOG(LogTemp, Log, TEXT("Player joined: %s (%s)"), *DisplayName, *UserId);
}

void UPlayerSyncComponent::HandlePlayerLeft(const FString& UserId)
{
    UE_LOG(LogTemp, Log, TEXT("Player left: %s"), *UserId);
    DespawnRemotePlayer(UserId);
}

void UPlayerSyncComponent::HandleChat(const FString& UserId, const FString& DataJson)
{
    TSharedPtr<FJsonObject> Json;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(DataJson);
    if (!FJsonSerializer::Deserialize(Reader, Json)) return;

    FString Message = Json->GetStringField(TEXT("message"));
    FString Name = Json->HasField(TEXT("name")) ? Json->GetStringField(TEXT("name")) : UserId;

    UE_LOG(LogTemp, Log, TEXT("[Chat] %s: %s"), *Name, *Message);
    // TODO: Route to chat widget when implemented
}

ARemotePlayer* UPlayerSyncComponent::SpawnRemotePlayer(const FString& UserId, const FString& DisplayName)
{
    if (RemotePlayers.Contains(UserId))
    {
        return RemotePlayers[UserId];
    }

    UWorld* World = GetWorld();
    if (!World) return nullptr;

    FActorSpawnParameters SpawnParams;
    SpawnParams.SpawnCollisionHandlingOverride = ESpawnActorCollisionHandlingMethod::AlwaysSpawn;

    ARemotePlayer* Remote = nullptr;
    if (RemotePlayerClass)
    {
        Remote = World->SpawnActor<ARemotePlayer>(RemotePlayerClass, FVector::ZeroVector, FRotator::ZeroRotator, SpawnParams);
    }
    else
    {
        Remote = World->SpawnActor<ARemotePlayer>(ARemotePlayer::StaticClass(), FVector::ZeroVector, FRotator::ZeroRotator, SpawnParams);
    }

    if (Remote)
    {
        Remote->UserId = UserId;
        Remote->DisplayName = DisplayName;
        RemotePlayers.Add(UserId, Remote);
    }

    return Remote;
}

void UPlayerSyncComponent::DespawnRemotePlayer(const FString& UserId)
{
    ARemotePlayer** Found = RemotePlayers.Find(UserId);
    if (Found && *Found)
    {
        (*Found)->Destroy();
    }
    RemotePlayers.Remove(UserId);
}

void UPlayerSyncComponent::DespawnAllRemotePlayers()
{
    for (auto& Pair : RemotePlayers)
    {
        if (Pair.Value)
        {
            Pair.Value->Destroy();
        }
    }
    RemotePlayers.Empty();
}

void UPlayerSyncComponent::BroadcastLocalPosition()
{
    UMatchConnection* Match = GetMatchConnection();
    if (!Match || !Match->IsConnected()) return;

    AActor* Owner = GetOwner();
    if (!Owner) return;

    Match->SendPosition(Owner->GetActorLocation(), Owner->GetActorRotation());
}
