// Copyright Epic Games, Inc. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "Sockets.h"
#include "SocketSubsystem.h"
#include "IPAddress.h"
#include "Containers/Queue.h"
#include "NetcodeClient.generated.h"

/**
 * Low-level UDP client for renet netcode protocol
 * Connects to Bevy server and handles packet transmission
 */
UCLASS(BlueprintType)
class TOWERGAME_API UNetcodeClient : public UObject
{
    GENERATED_BODY()

public:
    UNetcodeClient();
    virtual ~UNetcodeClient();

    // Connection management
    UFUNCTION(BlueprintCallable, Category = "Netcode")
    bool Connect(const FString& ServerIP, int32 Port);

    UFUNCTION(BlueprintCallable, Category = "Netcode")
    void Disconnect();

    UFUNCTION(BlueprintPure, Category = "Netcode")
    bool IsConnected() const { return bIsConnected; }

    UFUNCTION(BlueprintPure, Category = "Netcode")
    int64 GetClientId() const { return ClientId; }

    // Packet sending/receiving
    bool SendPacket(const TArray<uint8>& Data);
    bool ReceivePackets(TArray<TArray<uint8>>& OutPackets);

    // Called every frame to process network
    void Tick(float DeltaTime);

private:
    // Socket
    FSocket* UdpSocket;
    ISocketSubsystem* SocketSubsystem;
    TSharedPtr<FInternetAddr> ServerAddress;

    // Connection state
    bool bIsConnected;
    int64 ClientId;
    int64 ProtocolId;

    // Timing
    double LastPacketTime;
    double ConnectionTime;
    float TickAccumulator;

    // Buffers
    TArray<uint8> ReceiveBuffer;
    TQueue<TArray<uint8>> OutgoingQueue;

    // Internal methods
    bool CreateSocket();
    void CloseSocket();
    bool SendHandshake();
    void ProcessIncomingData();

    // Netcode protocol constants
    static constexpr int32 MAX_PACKET_SIZE = 1200;
    static constexpr float TICK_RATE = 0.05f; // 20 Hz
};
