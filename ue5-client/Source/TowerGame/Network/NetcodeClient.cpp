// Copyright Epic Games, Inc. All Rights Reserved.

#include "NetcodeClient.h"
#include "Sockets.h"
#include "SocketSubsystem.h"
#include "Misc/DateTime.h"

UNetcodeClient::UNetcodeClient()
    : UdpSocket(nullptr)
    , SocketSubsystem(nullptr)
    , bIsConnected(false)
    , ClientId(0)
    , ProtocolId(0)
    , LastPacketTime(0.0)
    , ConnectionTime(0.0)
    , TickAccumulator(0.0f)
{
    ReceiveBuffer.SetNum(MAX_PACKET_SIZE);
}

UNetcodeClient::~UNetcodeClient()
{
    Disconnect();
}

bool UNetcodeClient::Connect(const FString& ServerIP, int32 Port)
{
    UE_LOG(LogTemp, Log, TEXT("NetcodeClient: Connecting to %s:%d"), *ServerIP, Port);

    // Get socket subsystem
    SocketSubsystem = ISocketSubsystem::Get(PLATFORM_SOCKETSUBSYSTEM);
    if (!SocketSubsystem)
    {
        UE_LOG(LogTemp, Error, TEXT("NetcodeClient: Failed to get socket subsystem"));
        return false;
    }

    // Create socket
    if (!CreateSocket())
    {
        return false;
    }

    // Resolve server address
    TSharedPtr<FInternetAddr> ResolvedAddress = SocketSubsystem->CreateInternetAddr();
    bool bIsValid = false;
    ResolvedAddress->SetIp(*ServerIP, bIsValid);
    ResolvedAddress->SetPort(Port);

    if (!bIsValid)
    {
        UE_LOG(LogTemp, Error, TEXT("NetcodeClient: Invalid server IP: %s"), *ServerIP);
        CloseSocket();
        return false;
    }

    ServerAddress = ResolvedAddress;

    // Generate client ID (timestamp-based, similar to Bevy client)
    ClientId = static_cast<int64>(FDateTime::Now().ToUnixTimestamp() * 1000);

    // Send handshake
    if (!SendHandshake())
    {
        UE_LOG(LogTemp, Error, TEXT("NetcodeClient: Failed to send handshake"));
        CloseSocket();
        return false;
    }

    bIsConnected = true;
    ConnectionTime = FPlatformTime::Seconds();

    UE_LOG(LogTemp, Log, TEXT("NetcodeClient: Connected! Client ID: %llu"), ClientId);
    return true;
}

void UNetcodeClient::Disconnect()
{
    if (bIsConnected)
    {
        UE_LOG(LogTemp, Log, TEXT("NetcodeClient: Disconnecting..."));
        bIsConnected = false;
    }

    CloseSocket();
}

bool UNetcodeClient::CreateSocket()
{
    // Create UDP socket
    UdpSocket = SocketSubsystem->CreateSocket(NAME_DGram, TEXT("NetcodeClient"), false);

    if (!UdpSocket)
    {
        UE_LOG(LogTemp, Error, TEXT("NetcodeClient: Failed to create UDP socket"));
        return false;
    }

    // Set socket to non-blocking
    UdpSocket->SetNonBlocking(true);

    // Set buffer sizes
    int32 SendSize = 2 * 1024 * 1024; // 2MB
    int32 ReceiveSize = 2 * 1024 * 1024; // 2MB
    UdpSocket->SetSendBufferSize(SendSize, SendSize);
    UdpSocket->SetReceiveBufferSize(ReceiveSize, ReceiveSize);

    UE_LOG(LogTemp, Log, TEXT("NetcodeClient: UDP socket created"));
    return true;
}

void UNetcodeClient::CloseSocket()
{
    if (UdpSocket)
    {
        UdpSocket->Close();
        SocketSubsystem->DestroySocket(UdpSocket);
        UdpSocket = nullptr;
    }
}

bool UNetcodeClient::SendHandshake()
{
    // Simple handshake: just send client ID
    // In full renet implementation, this would be a proper connect token
    TArray<uint8> HandshakeData;
    HandshakeData.SetNum(8);

    // Write client ID in little-endian
    FMemory::Memcpy(HandshakeData.GetData(), &ClientId, sizeof(int64));

    int32 BytesSent = 0;
    bool bSuccess = UdpSocket->SendTo(HandshakeData.GetData(), HandshakeData.Num(), BytesSent, *ServerAddress);

    if (bSuccess && BytesSent == HandshakeData.Num())
    {
        UE_LOG(LogTemp, Log, TEXT("NetcodeClient: Handshake sent (%d bytes)"), BytesSent);
        return true;
    }

    UE_LOG(LogTemp, Error, TEXT("NetcodeClient: Failed to send handshake"));
    return false;
}

bool UNetcodeClient::SendPacket(const TArray<uint8>& Data)
{
    if (!bIsConnected || !UdpSocket)
    {
        return false;
    }

    int32 BytesSent = 0;
    bool bSuccess = UdpSocket->SendTo(Data.GetData(), Data.Num(), BytesSent, *ServerAddress);

    if (bSuccess)
    {
        UE_LOG(LogTemp, VeryVerbose, TEXT("NetcodeClient: Sent %d bytes"), BytesSent);
    }

    return bSuccess && BytesSent == Data.Num();
}

bool UNetcodeClient::ReceivePackets(TArray<TArray<uint8>>& OutPackets)
{
    if (!bIsConnected || !UdpSocket)
    {
        return false;
    }

    OutPackets.Empty();

    // Receive all available packets
    while (true)
    {
        int32 BytesRead = 0;
        TSharedRef<FInternetAddr> Sender = SocketSubsystem->CreateInternetAddr();

        if (!UdpSocket->RecvFrom(ReceiveBuffer.GetData(), ReceiveBuffer.Num(), BytesRead, *Sender))
        {
            break; // No more packets
        }

        if (BytesRead > 0)
        {
            // Copy received data
            TArray<uint8> PacketData;
            PacketData.SetNum(BytesRead);
            FMemory::Memcpy(PacketData.GetData(), ReceiveBuffer.GetData(), BytesRead);

            OutPackets.Add(PacketData);
            LastPacketTime = FPlatformTime::Seconds();

            UE_LOG(LogTemp, VeryVerbose, TEXT("NetcodeClient: Received %d bytes"), BytesRead);
        }
    }

    return OutPackets.Num() > 0;
}

void UNetcodeClient::Tick(float DeltaTime)
{
    if (!bIsConnected)
    {
        return;
    }

    TickAccumulator += DeltaTime;

    // Send keepalive every tick interval
    if (TickAccumulator >= TICK_RATE)
    {
        TickAccumulator = 0.0f;

        // Send empty packet as keepalive
        TArray<uint8> KeepaliveData;
        KeepaliveData.Add(0x00); // Keepalive packet type
        SendPacket(KeepaliveData);
    }

    // Check for timeout (5 seconds without packets)
    double TimeSinceLastPacket = FPlatformTime::Seconds() - LastPacketTime;
    if (TimeSinceLastPacket > 5.0)
    {
        UE_LOG(LogTemp, Warning, TEXT("NetcodeClient: Connection timeout (%.1fs since last packet)"), TimeSinceLastPacket);
        // Could trigger disconnect here
    }
}
