// Copyright Epic Games, Inc. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "BincodeSerializer.generated.h"

/**
 * Simplified Bincode deserializer for Rust data structures
 * Supports: u8, u16, u32, u64, f32, f64, Vec3, arrays, strings
 *
 * Bincode format (little-endian):
 * - Integers: raw bytes in little-endian
 * - Floats: IEEE 754 little-endian
 * - Strings: length (u64) + UTF-8 bytes
 * - Arrays: length (u64) + elements
 */
class TOWERGAME_API FBincodeReader
{
public:
    FBincodeReader(const TArray<uint8>& InData);
    FBincodeReader(const uint8* InData, int32 InSize);

    // Primitive types
    uint8 ReadU8();
    int16 ReadU16();
    int32 ReadU32();
    int64 ReadU64();

    int8 ReadI8();
    int16 ReadI16();
    int32 ReadI32();
    int64 ReadI64();

    float ReadF32();
    double ReadF64();

    bool ReadBool();

    // Compound types
    FString ReadString();
    FVector ReadVec3();  // Reads [f32; 3] in Bevy coordinates
    FVector ReadBevyVec3();  // Reads [f32; 3] and converts Bevy → UE5

    template<typename T>
    TArray<T> ReadArray(TFunction<T(FBincodeReader&)> ReadElement);

    // State
    bool IsValid() const { return bIsValid && Position < DataSize; }
    bool HasError() const { return !bIsValid; }
    int32 GetPosition() const { return Position; }
    int32 GetRemainingBytes() const { return DataSize - Position; }

    // Coordinate system conversion utilities
    // Bevy/Rust: Y-up (X-right, Y-up, Z-forward)
    // UE5: Z-up (X-forward, Y-right, Z-up)
    static FORCEINLINE FVector BevyToUE5(const FVector& BevyPos)
    {
        // Bevy (X, Y, Z) → UE5 (Z, X, Y)
        return FVector(BevyPos.Z, BevyPos.X, BevyPos.Y) * 100.0f; // Also convert meters to cm
    }

    static FORCEINLINE FVector UE5ToBevy(const FVector& UE5Pos)
    {
        // UE5 (X, Y, Z) → Bevy (Y, Z, X)
        FVector Meters = UE5Pos / 100.0f; // Convert cm to meters
        return FVector(Meters.Y, Meters.Z, Meters.X);
    }

private:
    const uint8* Data;
    int32 DataSize;
    int32 Position;
    bool bIsValid;

    void SetError();
    bool CanRead(int32 Bytes) const;
};

/**
 * Helper structs matching Rust types
 */
USTRUCT(BlueprintType)
struct TOWERGAME_API FPlayerData
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadOnly)
    int64 Id = 0;

    UPROPERTY(BlueprintReadOnly)
    FVector Position = FVector::ZeroVector;

    UPROPERTY(BlueprintReadOnly)
    float Health = 0.0f;

    UPROPERTY(BlueprintReadOnly)
    int32 CurrentFloor = 0;

    // Deserialize from bincode
    static FPlayerData FromBincode(FBincodeReader& Reader);
};

USTRUCT(BlueprintType)
struct TOWERGAME_API FMonsterData
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadOnly)
    FString MonsterType;

    UPROPERTY(BlueprintReadOnly)
    FVector Position = FVector::ZeroVector;

    UPROPERTY(BlueprintReadOnly)
    float Health = 0.0f;

    UPROPERTY(BlueprintReadOnly)
    float MaxHealth = 0.0f;

    static FMonsterData FromBincode(FBincodeReader& Reader);
};

USTRUCT(BlueprintType)
struct TOWERGAME_API FFloorTileData
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadOnly)
    uint8 TileType = 0;

    UPROPERTY(BlueprintReadOnly)
    int32 GridX = 0;

    UPROPERTY(BlueprintReadOnly)
    int32 GridY = 0;

    static FFloorTileData FromBincode(FBincodeReader& Reader);
};
