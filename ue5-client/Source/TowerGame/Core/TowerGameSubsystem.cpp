#include "TowerGameSubsystem.h"
#include "TowerGame/Bridge/ProceduralCoreBridge.h"
#include "Misc/Paths.h"

void UTowerGameSubsystem::Initialize(FSubsystemCollectionBase& Collection)
{
    Super::Initialize(Collection);

    Bridge = MakeUnique<FProceduralCoreBridge>();

    FString DllPath = FindDllPath();
    if (Bridge->Initialize(DllPath))
    {
        UE_LOG(LogTemp, Log, TEXT("Tower Rust Core initialized. Version: %s"),
            *Bridge->GetVersion());
    }
    else
    {
        UE_LOG(LogTemp, Error, TEXT("Failed to initialize Tower Rust Core from: %s"), *DllPath);
    }
}

void UTowerGameSubsystem::Deinitialize()
{
    if (Bridge)
    {
        Bridge->Shutdown();
        Bridge.Reset();
    }

    Super::Deinitialize();
}

bool UTowerGameSubsystem::IsRustCoreReady() const
{
    return Bridge.IsValid() && Bridge->IsInitialized();
}

FString UTowerGameSubsystem::FindDllPath() const
{
    // Search order:
    // 1. Binaries/Win64/tower_core.dll (packaged game)
    // 2. ../../procedural-core/target/debug/tower_core.dll (dev)
    // 3. ../../procedural-core/target/release/tower_core.dll (dev release)

    TArray<FString> SearchPaths;
    SearchPaths.Add(FPaths::Combine(FPaths::ProjectDir(), TEXT("Binaries/Win64/tower_core.dll")));
    SearchPaths.Add(FPaths::Combine(FPaths::ProjectDir(), TEXT("ThirdParty/TowerCore/lib/tower_core.dll")));
    SearchPaths.Add(FPaths::Combine(FPaths::ProjectDir(), TEXT("../../procedural-core/target/release/tower_core.dll")));
    SearchPaths.Add(FPaths::Combine(FPaths::ProjectDir(), TEXT("../../procedural-core/target/debug/tower_core.dll")));

    for (const FString& Path : SearchPaths)
    {
        FString AbsPath = FPaths::ConvertRelativePathToFull(Path);
        if (FPaths::FileExists(AbsPath))
        {
            UE_LOG(LogTemp, Log, TEXT("Found Rust DLL at: %s"), *AbsPath);
            return AbsPath;
        }
    }

    UE_LOG(LogTemp, Warning, TEXT("tower_core.dll not found in any search path"));
    return TEXT("tower_core.dll");
}

// ============ High-Level API ============

FString UTowerGameSubsystem::RequestFloorLayout(int64 Seed, int32 FloorId)
{
    if (!IsRustCoreReady()) return FString();
    return Bridge->GenerateFloorLayout(static_cast<uint64>(Seed), static_cast<uint32>(FloorId));
}

FString UTowerGameSubsystem::RequestFloorMonsters(int64 Seed, int32 FloorId, int32 Count)
{
    if (!IsRustCoreReady()) return FString();
    return Bridge->GenerateFloorMonsters(
        static_cast<uint64>(Seed),
        static_cast<uint32>(FloorId),
        static_cast<uint32>(Count));
}

float UTowerGameSubsystem::CalculateDamage(float BaseDamage, int32 AngleId, int32 ComboStep)
{
    if (!IsRustCoreReady()) return BaseDamage;
    float AngleMult = Bridge->GetAngleMultiplier(static_cast<uint32>(AngleId));
    float ComboMult = 1.0f + static_cast<float>(ComboStep) * 0.15f;
    return BaseDamage * AngleMult * ComboMult;
}

float UTowerGameSubsystem::GetSemanticSimilarity(const FString& TagsA, const FString& TagsB)
{
    if (!IsRustCoreReady()) return 0.0f;
    return Bridge->SemanticSimilarity(TagsA, TagsB);
}

FString UTowerGameSubsystem::GetBreathState(float ElapsedSeconds)
{
    if (!IsRustCoreReady()) return FString();
    return Bridge->GetBreathState(ElapsedSeconds);
}

FString UTowerGameSubsystem::GetCoreVersion()
{
    if (!IsRustCoreReady()) return TEXT("not loaded");
    return Bridge->GetVersion();
}

// ============ Hot-Reload (v0.6.0) ============

FString UTowerGameSubsystem::GetHotReloadStatus()
{
    if (!IsRustCoreReady()) return TEXT("{\"enabled\":false}");
    return Bridge->HotReloadGetStatus();
}

int32 UTowerGameSubsystem::TriggerConfigReload()
{
    if (!IsRustCoreReady()) return 0;
    return static_cast<int32>(Bridge->HotReloadTriggerReload());
}

// ============ Analytics (v0.6.0) ============

FString UTowerGameSubsystem::GetAnalyticsSnapshot()
{
    if (!IsRustCoreReady()) return TEXT("{}");
    return Bridge->AnalyticsGetSnapshot();
}

void UTowerGameSubsystem::ResetAnalytics()
{
    if (IsRustCoreReady())
    {
        Bridge->AnalyticsReset();
    }
}

void UTowerGameSubsystem::RecordDamageDealt(const FString& WeaponName, int32 Amount)
{
    if (IsRustCoreReady())
    {
        Bridge->AnalyticsRecordDamage(WeaponName, static_cast<uint32>(Amount));
    }
}

void UTowerGameSubsystem::RecordFloorCleared(int32 FloorId, int32 Tier, float TimeSecs)
{
    if (IsRustCoreReady())
    {
        Bridge->AnalyticsRecordFloorCleared(
            static_cast<uint32>(FloorId),
            static_cast<uint32>(Tier),
            TimeSecs);
    }
}

void UTowerGameSubsystem::RecordGoldEarned(int64 Amount)
{
    if (IsRustCoreReady())
    {
        Bridge->AnalyticsRecordGold(static_cast<uint64>(Amount));
    }
}

FString UTowerGameSubsystem::GetAnalyticsEventTypes()
{
    if (!IsRustCoreReady()) return TEXT("[]");
    return Bridge->AnalyticsGetEventTypes();
}
