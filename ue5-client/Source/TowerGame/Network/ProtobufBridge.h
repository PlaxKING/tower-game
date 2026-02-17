//  ProtobufBridge.h
//  Bridge between Protobuf-generated types and UE5 USTRUCT types
//  Session 28 - FFI Integration via Rust DLL

#pragma once

#include "CoreMinimal.h"
#include "Dom/JsonObject.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"
#include "ProtobufBridge.generated.h"

// Rust FFI functions (tower_bevy_server.dll)
extern "C"
{
    typedef char* (*ProtobufToJsonFunc)(const unsigned char*, unsigned int);
    typedef void (*FreeStringFunc)(char*);
    typedef char* (*GetChunkFieldFunc)(const unsigned char*, unsigned int, const char*);
}

/**
 * Vec3 - 3D Position (matches Protobuf schema)
 */
USTRUCT(BlueprintType)
struct TOWERGAME_API FProtoVec3
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite, Category = "Protobuf")
    float X = 0.0f;

    UPROPERTY(BlueprintReadWrite, Category = "Protobuf")
    float Y = 0.0f;

    UPROPERTY(BlueprintReadWrite, Category = "Protobuf")
    float Z = 0.0f;

    FProtoVec3() = default;
    FProtoVec3(float InX, float InY, float InZ) : X(InX), Y(InY), Z(InZ) {}

    // Convert to UE5 FVector (with coordinate system conversion)
    FVector ToUE5Vector() const
    {
        // Bevy (X, Y, Z) → UE5 (Z, X, Y), meters → centimeters
        return FVector(Z, X, Y) * 100.0f;
    }

    // Create from UE5 FVector (with coordinate system conversion)
    static FProtoVec3 FromUE5Vector(const FVector& UE5Vec)
    {
        // UE5 (X, Y, Z) → Bevy (Y, Z, X), centimeters → meters
        FVector Meters = UE5Vec / 100.0f;
        return FProtoVec3(Meters.Y, Meters.Z, Meters.X);
    }
};

/**
 * FloorTileData - Single floor tile (matches Protobuf schema)
 */
USTRUCT(BlueprintType)
struct TOWERGAME_API FProtoFloorTileData
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite, Category = "Protobuf")
    int32 TileType = 0;

    UPROPERTY(BlueprintReadWrite, Category = "Protobuf")
    int32 GridX = 0;

    UPROPERTY(BlueprintReadWrite, Category = "Protobuf")
    int32 GridY = 0;

    UPROPERTY(BlueprintReadWrite, Category = "Protobuf")
    int32 BiomeId = 0;

    UPROPERTY(BlueprintReadWrite, Category = "Protobuf")
    bool bIsWalkable = true;

    UPROPERTY(BlueprintReadWrite, Category = "Protobuf")
    bool bHasCollision = false;
};

/**
 * ChunkData - Procedural floor data (matches Protobuf schema)
 * This is sent from Rust server → UE5 client (only ~5KB per floor)
 */
USTRUCT(BlueprintType)
struct TOWERGAME_API FProtoChunkData
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite, Category = "Protobuf")
    int64 Seed = 0;

    UPROPERTY(BlueprintReadWrite, Category = "Protobuf")
    int32 FloorId = 0;

    UPROPERTY(BlueprintReadWrite, Category = "Protobuf")
    TArray<FProtoFloorTileData> Tiles;

    UPROPERTY(BlueprintReadWrite, Category = "Protobuf")
    TArray<uint8> ValidationHash;

    UPROPERTY(BlueprintReadWrite, Category = "Protobuf")
    int32 BiomeId = 0;

    UPROPERTY(BlueprintReadWrite, Category = "Protobuf")
    int32 Width = 0;

    UPROPERTY(BlueprintReadWrite, Category = "Protobuf")
    int32 Height = 0;

    UPROPERTY(BlueprintReadWrite, Category = "Protobuf")
    FProtoVec3 WorldOffset;

    // Serialize to JSON (fallback until Protobuf lib is integrated)
    FString ToJson() const;

    // Deserialize from JSON (fallback until Protobuf lib is integrated)
    static FProtoChunkData FromJson(const FString& JsonString);
};

/**
 * ProtobufBridge - Utility class for Protobuf ↔ UE5 conversion
 *
 * Usage:
 *   // Deserialize from server
 *   TArray<uint8> ReceivedData = ...;
 *   FProtoChunkData Chunk = UProtobufBridge::DeserializeChunkData(ReceivedData);
 *
 *   // Use in game
 *   for (const FProtoFloorTileData& Tile : Chunk.Tiles)
 *   {
 *       // Generate mesh at Tile.GridX, Tile.GridY
 *   }
 */
UCLASS()
class TOWERGAME_API UProtobufBridge : public UObject
{
    GENERATED_BODY()

public:
    /**
     * Deserialize ChunkData from binary Protobuf format
     * @param ProtobufBytes Raw bytes from Rust server
     * @return Deserialized ChunkData struct
     *
     * NOTE: Currently using JSON fallback. When Protobuf lib is linked, this will use:
     *   tower::game::ChunkData proto_chunk;
     *   proto_chunk.ParseFromArray(ProtobufBytes.GetData(), ProtobufBytes.Num());
     */
    UFUNCTION(BlueprintCallable, Category = "Protobuf")
    static FProtoChunkData DeserializeChunkData(const TArray<uint8>& ProtobufBytes);

    /**
     * Serialize ChunkData to binary Protobuf format
     * @param ChunkData UE5 chunk data struct
     * @return Binary Protobuf bytes ready to send to server
     */
    UFUNCTION(BlueprintCallable, Category = "Protobuf")
    static TArray<uint8> SerializeChunkData(const FProtoChunkData& ChunkData);

    /**
     * Validate chunk hash (anti-cheat)
     * @param ChunkData Chunk to validate
     * @param ExpectedHash Expected SHA-3 hash from server
     * @return True if hash matches
     */
    UFUNCTION(BlueprintCallable, Category = "Protobuf")
    static bool ValidateChunkHash(const FProtoChunkData& ChunkData, const TArray<uint8>& ExpectedHash);

    /**
     * Get bandwidth savings ratio
     * @param TileCount Number of tiles in floor
     * @return Estimated savings (e.g., 98.0 for 98x reduction)
     */
    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "Protobuf")
    static float GetBandwidthSavingsRatio(int32 TileCount);

private:
    // JSON fallback serialization (temporary until Protobuf lib linked)
    static FString ChunkDataToJson(const FProtoChunkData& ChunkData);
    static FProtoChunkData JsonToChunkData(const FString& JsonString);
};
