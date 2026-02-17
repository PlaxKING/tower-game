// ProtobufBridge.cpp
// Implementation of Protobuf ↔ UE5 bridge via Rust FFI
// Session 28 - FFI Integration

#include "ProtobufBridge.h"
#include "Misc/Base64.h"
#include "Dom/JsonObject.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonWriter.h"
#include "Serialization/JsonSerializer.h"
#include "HAL/PlatformProcess.h"

// DLL Handle and function pointers
static void* BevyDllHandle = nullptr;
static ProtobufToJsonFunc ProtobufToJson = nullptr;
static FreeStringFunc FreeString = nullptr;
static GetChunkFieldFunc GetChunkField = nullptr;

// Load Rust FFI DLL
static bool LoadBevyDll()
{
    if (BevyDllHandle != nullptr)
        return true; // Already loaded

    FString DllPath = FPaths::Combine(FPaths::ProjectPluginsDir(), TEXT("../../ThirdParty/TowerBevy/lib/tower_bevy_server.dll"));
    DllPath = FPaths::ConvertRelativePathToFull(DllPath);

    BevyDllHandle = FPlatformProcess::GetDllHandle(*DllPath);
    if (BevyDllHandle == nullptr)
    {
        UE_LOG(LogTemp, Warning, TEXT("Failed to load tower_bevy_server.dll from: %s"), *DllPath);
        return false;
    }

    ProtobufToJson = (ProtobufToJsonFunc)FPlatformProcess::GetDllExport(BevyDllHandle, TEXT("protobuf_to_json"));
    FreeString = (FreeStringFunc)FPlatformProcess::GetDllExport(BevyDllHandle, TEXT("free_string"));
    GetChunkField = (GetChunkFieldFunc)FPlatformProcess::GetDllExport(BevyDllHandle, TEXT("get_chunk_field"));

    if (ProtobufToJson == nullptr || FreeString == nullptr)
    {
        UE_LOG(LogTemp, Error, TEXT("Failed to load FFI functions from tower_bevy_server.dll"));
        FPlatformProcess::FreeDllHandle(BevyDllHandle);
        BevyDllHandle = nullptr;
        return false;
    }

    UE_LOG(LogTemp, Log, TEXT("✅ Loaded tower_bevy_server.dll successfully"));
    return true;
}

// ============================================================================
// FProtoChunkData Serialization (JSON Fallback)
// ============================================================================

FString FProtoChunkData::ToJson() const
{
    TSharedPtr<FJsonObject> JsonObject = MakeShareable(new FJsonObject());

    JsonObject->SetNumberField(TEXT("seed"), Seed);
    JsonObject->SetNumberField(TEXT("floor_id"), FloorId);
    JsonObject->SetNumberField(TEXT("biome_id"), BiomeId);
    JsonObject->SetNumberField(TEXT("width"), Width);
    JsonObject->SetNumberField(TEXT("height"), Height);

    // Validation hash (base64 encoded)
    FString HashBase64 = FBase64::Encode(ValidationHash);
    JsonObject->SetStringField(TEXT("validation_hash"), HashBase64);

    // World offset
    TSharedPtr<FJsonObject> OffsetJson = MakeShareable(new FJsonObject());
    OffsetJson->SetNumberField(TEXT("x"), WorldOffset.X);
    OffsetJson->SetNumberField(TEXT("y"), WorldOffset.Y);
    OffsetJson->SetNumberField(TEXT("z"), WorldOffset.Z);
    JsonObject->SetObjectField(TEXT("world_offset"), OffsetJson);

    // Tiles array
    TArray<TSharedPtr<FJsonValue>> TilesArray;
    for (const FProtoFloorTileData& Tile : Tiles)
    {
        TSharedPtr<FJsonObject> TileJson = MakeShareable(new FJsonObject());
        TileJson->SetNumberField(TEXT("tile_type"), Tile.TileType);
        TileJson->SetNumberField(TEXT("grid_x"), Tile.GridX);
        TileJson->SetNumberField(TEXT("grid_y"), Tile.GridY);
        TileJson->SetNumberField(TEXT("biome_id"), Tile.BiomeId);
        TileJson->SetBoolField(TEXT("is_walkable"), Tile.bIsWalkable);
        TileJson->SetBoolField(TEXT("has_collision"), Tile.bHasCollision);

        TilesArray.Add(MakeShareable(new FJsonValueObject(TileJson)));
    }
    JsonObject->SetArrayField(TEXT("tiles"), TilesArray);

    // Serialize to string
    FString OutputString;
    TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&OutputString);
    FJsonSerializer::Serialize(JsonObject.ToSharedRef(), Writer);

    return OutputString;
}

FProtoChunkData FProtoChunkData::FromJson(const FString& JsonString)
{
    FProtoChunkData Result;

    TSharedPtr<FJsonObject> JsonObject;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(JsonString);

    if (!FJsonSerializer::Deserialize(Reader, JsonObject) || !JsonObject.IsValid())
    {
        UE_LOG(LogTemp, Error, TEXT("Failed to parse ChunkData JSON"));
        return Result;
    }

    Result.Seed = static_cast<int64>(JsonObject->GetNumberField(TEXT("seed")));
    Result.FloorId = static_cast<int32>(JsonObject->GetNumberField(TEXT("floor_id")));
    Result.BiomeId = static_cast<int32>(JsonObject->GetNumberField(TEXT("biome_id")));
    Result.Width = static_cast<int32>(JsonObject->GetNumberField(TEXT("width")));
    Result.Height = static_cast<int32>(JsonObject->GetNumberField(TEXT("height")));

    // Validation hash (base64 decode)
    FString HashBase64 = JsonObject->GetStringField(TEXT("validation_hash"));
    FBase64::Decode(HashBase64, Result.ValidationHash);

    // World offset
    const TSharedPtr<FJsonObject>* OffsetJson;
    if (JsonObject->TryGetObjectField(TEXT("world_offset"), OffsetJson))
    {
        Result.WorldOffset.X = static_cast<float>((*OffsetJson)->GetNumberField(TEXT("x")));
        Result.WorldOffset.Y = static_cast<float>((*OffsetJson)->GetNumberField(TEXT("y")));
        Result.WorldOffset.Z = static_cast<float>((*OffsetJson)->GetNumberField(TEXT("z")));
    }

    // Tiles array
    const TArray<TSharedPtr<FJsonValue>>* TilesArray;
    if (JsonObject->TryGetArrayField(TEXT("tiles"), TilesArray))
    {
        for (const TSharedPtr<FJsonValue>& TileValue : *TilesArray)
        {
            const TSharedPtr<FJsonObject>& TileJson = TileValue->AsObject();
            if (!TileJson.IsValid())
                continue;

            FProtoFloorTileData Tile;
            Tile.TileType = static_cast<int32>(TileJson->GetNumberField(TEXT("tile_type")));
            Tile.GridX = static_cast<int32>(TileJson->GetNumberField(TEXT("grid_x")));
            Tile.GridY = static_cast<int32>(TileJson->GetNumberField(TEXT("grid_y")));
            Tile.BiomeId = static_cast<int32>(TileJson->GetNumberField(TEXT("biome_id")));
            Tile.bIsWalkable = TileJson->GetBoolField(TEXT("is_walkable"));
            Tile.bHasCollision = TileJson->GetBoolField(TEXT("has_collision"));

            Result.Tiles.Add(Tile);
        }
    }

    return Result;
}

// ============================================================================
// UProtobufBridge Implementation
// ============================================================================

FProtoChunkData UProtobufBridge::DeserializeChunkData(const TArray<uint8>& ProtobufBytes)
{
    // Use Rust FFI for Protobuf deserialization (no libprotobuf.lib needed!)
    if (!LoadBevyDll())
    {
        UE_LOG(LogTemp, Error, TEXT("tower_bevy_server.dll not loaded, falling back to JSON"));
        // JSON fallback
        FString JsonString;
        FFileHelper::BufferToString(JsonString, ProtobufBytes.GetData(), ProtobufBytes.Num());
        return FProtoChunkData::FromJson(JsonString);
    }

    // Call Rust FFI: protobuf_to_json()
    char* JsonCStr = ProtobufToJson(ProtobufBytes.GetData(), ProtobufBytes.Num());
    if (JsonCStr == nullptr)
    {
        UE_LOG(LogTemp, Error, TEXT("Rust FFI protobuf_to_json() returned null"));
        return FProtoChunkData();
    }

    // Convert to FString and deserialize
    FString JsonString(UTF8_TO_TCHAR(JsonCStr));
    FreeString(JsonCStr); // Free Rust-allocated string

    FProtoChunkData Result = FProtoChunkData::FromJson(JsonString);
    UE_LOG(LogTemp, Log, TEXT("✅ Deserialized ChunkData via Rust FFI: floor_id=%d, tiles=%d"), Result.FloorId, Result.Tiles.Num());

    return Result;
}

TArray<uint8> UProtobufBridge::SerializeChunkData(const FProtoChunkData& ChunkData)
{
    // TODO: When Protobuf C++ library is linked, use:
    //   tower::game::ChunkData proto_chunk = ConvertToProtoChunk(ChunkData);
    //   std::string serialized = proto_chunk.SerializeAsString();
    //   TArray<uint8> Result;
    //   Result.Append(reinterpret_cast<const uint8*>(serialized.data()), serialized.size());
    //   return Result;

    // JSON fallback (temporary)
    FString JsonString = ChunkData.ToJson();
    TArray<uint8> Result;
    Result.Append(reinterpret_cast<const uint8*>(TCHAR_TO_UTF8(*JsonString)), JsonString.Len());
    return Result;
}

bool UProtobufBridge::ValidateChunkHash(const FProtoChunkData& ChunkData, const TArray<uint8>& ExpectedHash)
{
    // Simple validation - check if hashes match
    if (ChunkData.ValidationHash.Num() != ExpectedHash.Num())
    {
        UE_LOG(LogTemp, Warning, TEXT("Chunk hash length mismatch: %d vs %d"),
            ChunkData.ValidationHash.Num(), ExpectedHash.Num());
        return false;
    }

    for (int32 i = 0; i < ChunkData.ValidationHash.Num(); ++i)
    {
        if (ChunkData.ValidationHash[i] != ExpectedHash[i])
        {
            UE_LOG(LogTemp, Warning, TEXT("Chunk hash mismatch at byte %d"), i);
            return false;
        }
    }

    UE_LOG(LogTemp, Log, TEXT("Chunk hash validation PASSED"));
    return true;
}

float UProtobufBridge::GetBandwidthSavingsRatio(int32 TileCount)
{
    // Estimate based on benchmarks:
    // Full mesh: ~200 bytes per tile
    // Procedural: ~2 bytes per tile + 240 bytes overhead
    const float FullMeshSize = TileCount * 200.0f;
    const float ProceduralSize = TileCount * 2.0f + 240.0f;

    if (ProceduralSize <= 0.0f)
        return 1.0f;

    return FullMeshSize / ProceduralSize;
}

// ============================================================================
// Helper Functions (TODO: Move to separate file if this grows)
// ============================================================================

FString UProtobufBridge::ChunkDataToJson(const FProtoChunkData& ChunkData)
{
    return ChunkData.ToJson();
}

FProtoChunkData UProtobufBridge::JsonToChunkData(const FString& JsonString)
{
    return FProtoChunkData::FromJson(JsonString);
}
