// Copyright Epic Games, Inc. All Rights Reserved.

#include "BincodeSerializer.h"

FBincodeReader::FBincodeReader(const TArray<uint8>& InData)
    : Data(InData.GetData())
    , DataSize(InData.Num())
    , Position(0)
    , bIsValid(true)
{
}

FBincodeReader::FBincodeReader(const uint8* InData, int32 InSize)
    : Data(InData)
    , DataSize(InSize)
    , Position(0)
    , bIsValid(true)
{
}

void FBincodeReader::SetError()
{
    bIsValid = false;
    UE_LOG(LogTemp, Error, TEXT("BincodeReader: Read error at position %d (size %d)"), Position, DataSize);
}

bool FBincodeReader::CanRead(int32 Bytes) const
{
    return bIsValid && (Position + Bytes <= DataSize);
}

uint8 FBincodeReader::ReadU8()
{
    if (!CanRead(1))
    {
        const_cast<FBincodeReader*>(this)->SetError();
        return 0;
    }
    return Data[Position++];
}

int16 FBincodeReader::ReadU16()
{
    if (!CanRead(2))
    {
        SetError();
        return 0;
    }

    int16 Value;
    FMemory::Memcpy(&Value, &Data[Position], 2);
    Position += 2;

    // Bincode uses little-endian
    #if PLATFORM_LITTLE_ENDIAN
        return Value;
    #else
        return ((Value & 0xFF) << 8) | ((Value >> 8) & 0xFF);
    #endif
}

int32 FBincodeReader::ReadU32()
{
    if (!CanRead(4))
    {
        SetError();
        return 0;
    }

    int32 Value;
    FMemory::Memcpy(&Value, &Data[Position], 4);
    Position += 4;

    #if PLATFORM_LITTLE_ENDIAN
        return Value;
    #else
        return ((Value & 0xFF) << 24) |
               ((Value & 0xFF00) << 8) |
               ((Value & 0xFF0000) >> 8) |
               ((Value >> 24) & 0xFF);
    #endif
}

int64 FBincodeReader::ReadU64()
{
    if (!CanRead(8))
    {
        SetError();
        return 0;
    }

    int64 Value;
    FMemory::Memcpy(&Value, &Data[Position], 8);
    Position += 8;

    #if PLATFORM_LITTLE_ENDIAN
        return Value;
    #else
        // Byte swap for big-endian platforms
        return ((Value & 0xFF) << 56) |
               ((Value & 0xFF00) << 40) |
               ((Value & 0xFF0000) << 24) |
               ((Value & 0xFF000000) << 8) |
               ((Value >> 8) & 0xFF000000) |
               ((Value >> 24) & 0xFF0000) |
               ((Value >> 40) & 0xFF00) |
               ((Value >> 56) & 0xFF);
    #endif
}

int8 FBincodeReader::ReadI8()
{
    return static_cast<int8>(ReadU8());
}

int16 FBincodeReader::ReadI16()
{
    return static_cast<int16>(ReadU16());
}

int32 FBincodeReader::ReadI32()
{
    return static_cast<int32>(ReadU32());
}

int64 FBincodeReader::ReadI64()
{
    return static_cast<int64>(ReadU64());
}

float FBincodeReader::ReadF32()
{
    int32 IntValue = ReadU32();
    float FloatValue;
    FMemory::Memcpy(&FloatValue, &IntValue, 4);
    return FloatValue;
}

double FBincodeReader::ReadF64()
{
    int64 IntValue = ReadU64();
    double DoubleValue;
    FMemory::Memcpy(&DoubleValue, &IntValue, 8);
    return DoubleValue;
}

bool FBincodeReader::ReadBool()
{
    return ReadU8() != 0;
}

FString FBincodeReader::ReadString()
{
    // Bincode strings: length (u64) + UTF-8 bytes
    int64 Length = ReadU64();

    if (Length > static_cast<int64>(INT32_MAX))
    {
        SetError();
        return FString();
    }

    int32 StringLength = static_cast<int32>(Length);
    if (!CanRead(StringLength))
    {
        SetError();
        return FString();
    }

    // Convert UTF-8 to FString
    FUTF8ToTCHAR Converter(reinterpret_cast<const ANSICHAR*>(&Data[Position]), StringLength);
    FString Result(Converter.Length(), Converter.Get());

    Position += StringLength;
    return Result;
}

FVector FBincodeReader::ReadVec3()
{
    // Rust [f32; 3] -> UE FVector (raw, no conversion)
    float X = ReadF32();
    float Y = ReadF32();
    float Z = ReadF32();

    return FVector(X, Y, Z);
}

FVector FBincodeReader::ReadBevyVec3()
{
    // Rust [f32; 3] -> UE FVector with coordinate system conversion
    FVector BevyPos = ReadVec3();
    return BevyToUE5(BevyPos);
}

// Player data deserialization
FPlayerData FPlayerData::FromBincode(FBincodeReader& Reader)
{
    FPlayerData Result;

    Result.Id = Reader.ReadU64();
    Result.Position = Reader.ReadBevyVec3();  // Convert Bevy Y-up → UE5 Z-up
    Result.Health = Reader.ReadF32();
    Result.CurrentFloor = Reader.ReadU32();

    if (!Reader.IsValid())
    {
        UE_LOG(LogTemp, Error, TEXT("Failed to deserialize PlayerData"));
    }

    return Result;
}

// Monster data deserialization
FMonsterData FMonsterData::FromBincode(FBincodeReader& Reader)
{
    FMonsterData Result;

    Result.MonsterType = Reader.ReadString();
    Result.Position = Reader.ReadBevyVec3();  // Convert Bevy Y-up → UE5 Z-up
    Result.Health = Reader.ReadF32();
    Result.MaxHealth = Reader.ReadF32();

    if (!Reader.IsValid())
    {
        UE_LOG(LogTemp, Error, TEXT("Failed to deserialize MonsterData"));
    }

    return Result;
}

// Floor tile data deserialization
FFloorTileData FFloorTileData::FromBincode(FBincodeReader& Reader)
{
    FFloorTileData Result;

    Result.TileType = Reader.ReadU8();
    Result.GridX = Reader.ReadI32();
    Result.GridY = Reader.ReadI32();

    if (!Reader.IsValid())
    {
        UE_LOG(LogTemp, Error, TEXT("Failed to deserialize FloorTileData"));
    }

    return Result;
}
