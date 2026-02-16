#include "ProceduralCoreBridge.h"
#include "HAL/PlatformProcess.h"

#define LOAD_DLL_FUNC(FuncName, FuncType, ExportName) \
    Fn_##FuncName = (FuncType)FPlatformProcess::GetDllExport(DllHandle, TEXT(ExportName)); \
    if (!Fn_##FuncName) { UE_LOG(LogTemp, Warning, TEXT("Failed to load: %s"), TEXT(ExportName)); }

FProceduralCoreBridge::FProceduralCoreBridge()
{
}

FProceduralCoreBridge::~FProceduralCoreBridge()
{
    Shutdown();
}

bool FProceduralCoreBridge::Initialize(const FString& DllPath)
{
    if (DllHandle)
    {
        UE_LOG(LogTemp, Warning, TEXT("ProceduralCore DLL already loaded"));
        return true;
    }

    DllHandle = FPlatformProcess::GetDllHandle(*DllPath);
    if (!DllHandle)
    {
        UE_LOG(LogTemp, Error, TEXT("Failed to load ProceduralCore DLL: %s"), *DllPath);
        return false;
    }

    // ---- Core ----
    LOAD_DLL_FUNC(GetVersion, FnGetVersion, "get_version");
    LOAD_DLL_FUNC(FreeString, FnFreeString, "free_string");

    // ---- Floor Generation ----
    LOAD_DLL_FUNC(GenerateFloor, FnGenerateFloor, "generate_floor");
    LOAD_DLL_FUNC(GenerateFloorLayout, FnGenerateFloorLayout, "generate_floor_layout");
    LOAD_DLL_FUNC(GetFloorHash, FnGetFloorHash, "get_floor_hash");
    LOAD_DLL_FUNC(GetFloorTier, FnGetFloorTier, "get_floor_tier");

    // ---- Monster Generation ----
    LOAD_DLL_FUNC(GenerateMonster, FnGenerateMonster, "generate_monster");
    LOAD_DLL_FUNC(GenerateFloorMonsters, FnGenerateFloorMonsters, "generate_floor_monsters");

    // ---- Combat ----
    LOAD_DLL_FUNC(GetAngleMultiplier, FnGetAngleMultiplier, "get_angle_multiplier");
    LOAD_DLL_FUNC(CalculateCombat, FnCalculateCombat, "calculate_combat");

    // ---- Semantic ----
    LOAD_DLL_FUNC(SemanticSimilarity, FnSemanticSimilarity, "semantic_similarity");

    // ---- Loot ----
    LOAD_DLL_FUNC(GenerateLoot, FnGenerateLoot, "generate_loot");

    // ---- World ----
    LOAD_DLL_FUNC(GetBreathState, FnGetBreathState, "get_breath_state");

    // ---- Replication ----
    LOAD_DLL_FUNC(RecordDelta, FnRecordDelta, "record_delta");
    LOAD_DLL_FUNC(CreateFloorSnapshot, FnCreateFloorSnapshot, "create_floor_snapshot");

    // ---- Events ----
    LOAD_DLL_FUNC(EvaluateEventTrigger, FnEvaluateEventTrigger, "evaluate_event_trigger");

    // ---- Mastery ----
    LOAD_DLL_FUNC(MasteryCreateProfile, FnMasteryCreateProfile, "mastery_create_profile");
    LOAD_DLL_FUNC(MasteryGainXp, FnMasteryGainXp, "mastery_gain_xp");
    LOAD_DLL_FUNC(MasteryGetTier, FnMasteryGetTier, "mastery_get_tier");
    LOAD_DLL_FUNC(MasteryXpForAction, FnMasteryXpForAction, "mastery_xp_for_action");
    LOAD_DLL_FUNC(MasteryGetAllDomains, FnMasteryGetAllDomains, "mastery_get_all_domains");

    // ---- Specialization ----
    LOAD_DLL_FUNC(SpecGetAllBranches, FnSpecGetAllBranches, "spec_get_all_branches");
    LOAD_DLL_FUNC(SpecCreateProfile, FnSpecCreateProfile, "spec_create_profile");
    LOAD_DLL_FUNC(SpecChooseBranch, FnSpecChooseBranch, "spec_choose_branch");
    LOAD_DLL_FUNC(SpecFindSynergies, FnSpecFindSynergies, "spec_find_synergies");

    // ---- Abilities ----
    LOAD_DLL_FUNC(AbilityGetDefaults, FnAbilityGetDefaults, "ability_get_defaults");
    LOAD_DLL_FUNC(AbilityCreateLoadout, FnAbilityCreateLoadout, "ability_create_loadout");
    LOAD_DLL_FUNC(AbilityLearn, FnAbilityLearn, "ability_learn");
    LOAD_DLL_FUNC(AbilityEquip, FnAbilityEquip, "ability_equip");

    // ---- Sockets ----
    LOAD_DLL_FUNC(SocketGetStarterGems, FnSocketGetStarterGems, "socket_get_starter_gems");
    LOAD_DLL_FUNC(SocketGetStarterRunes, FnSocketGetStarterRunes, "socket_get_starter_runes");
    LOAD_DLL_FUNC(SocketCreateEquipment, FnSocketCreateEquipment, "socket_create_equipment");
    LOAD_DLL_FUNC(SocketInsertGem, FnSocketInsertGem, "socket_insert_gem");
    LOAD_DLL_FUNC(SocketInsertRune, FnSocketInsertRune, "socket_insert_rune");
    LOAD_DLL_FUNC(SocketCombineGems, FnSocketCombineGems, "socket_combine_gems");

    // ---- Cosmetics ----
    LOAD_DLL_FUNC(CosmeticGetAll, FnCosmeticGetAll, "cosmetic_get_all");
    LOAD_DLL_FUNC(CosmeticGetAllDyes, FnCosmeticGetAllDyes, "cosmetic_get_all_dyes");
    LOAD_DLL_FUNC(CosmeticCreateProfile, FnCosmeticCreateProfile, "cosmetic_create_profile");
    LOAD_DLL_FUNC(CosmeticUnlock, FnCosmeticUnlock, "cosmetic_unlock");
    LOAD_DLL_FUNC(CosmeticApplyTransmog, FnCosmeticApplyTransmog, "cosmetic_apply_transmog");
    LOAD_DLL_FUNC(CosmeticApplyDye, FnCosmeticApplyDye, "cosmetic_apply_dye");

    // ---- Tutorial ----
    LOAD_DLL_FUNC(TutorialGetSteps, FnTutorialGetSteps, "tutorial_get_steps");
    LOAD_DLL_FUNC(TutorialGetHints, FnTutorialGetHints, "tutorial_get_hints");
    LOAD_DLL_FUNC(TutorialCreateProgress, FnTutorialCreateProgress, "tutorial_create_progress");
    LOAD_DLL_FUNC(TutorialCompleteStep, FnTutorialCompleteStep, "tutorial_complete_step");
    LOAD_DLL_FUNC(TutorialCompletionPercent, FnTutorialCompletionPercent, "tutorial_completion_percent");

    // ---- Achievements ----
    LOAD_DLL_FUNC(AchievementCreateTracker, FnAchievementCreateTracker, "achievement_create_tracker");
    LOAD_DLL_FUNC(AchievementIncrement, FnAchievementIncrement, "achievement_increment");
    LOAD_DLL_FUNC(AchievementCheckAll, FnAchievementCheckAll, "achievement_check_all");
    LOAD_DLL_FUNC(AchievementCompletionPercent, FnAchievementCompletionPercent, "achievement_completion_percent");

    // ---- Season Pass ----
    LOAD_DLL_FUNC(SeasonCreatePass, FnSeasonCreatePass, "season_create_pass");
    LOAD_DLL_FUNC(SeasonAddXp, FnSeasonAddXp, "season_add_xp");
    LOAD_DLL_FUNC(SeasonGenerateDailies, FnSeasonGenerateDailies, "season_generate_dailies");
    LOAD_DLL_FUNC(SeasonGenerateWeeklies, FnSeasonGenerateWeeklies, "season_generate_weeklies");
    LOAD_DLL_FUNC(SeasonGetRewards, FnSeasonGetRewards, "season_get_rewards");

    // ---- Social - Guild ----
    LOAD_DLL_FUNC(SocialCreateGuild, FnSocialCreateGuild, "social_create_guild");
    LOAD_DLL_FUNC(SocialGuildAddMember, FnSocialGuildAddMember, "social_guild_add_member");

    // ---- Social - Party ----
    LOAD_DLL_FUNC(SocialCreateParty, FnSocialCreateParty, "social_create_party");
    LOAD_DLL_FUNC(SocialPartyAddMember, FnSocialPartyAddMember, "social_party_add_member");

    // ---- Social - Trade ----
    LOAD_DLL_FUNC(SocialCreateTrade, FnSocialCreateTrade, "social_create_trade");
    LOAD_DLL_FUNC(SocialTradeAddItem, FnSocialTradeAddItem, "social_trade_add_item");
    LOAD_DLL_FUNC(SocialTradeLock, FnSocialTradeLock, "social_trade_lock");
    LOAD_DLL_FUNC(SocialTradeConfirm, FnSocialTradeConfirm, "social_trade_confirm");
    LOAD_DLL_FUNC(SocialTradeExecute, FnSocialTradeExecute, "social_trade_execute");

    // ---- Hot-Reload (v0.6.0) ----
    LOAD_DLL_FUNC(HotReloadGetStatus, FnHotReloadGetStatus, "hotreload_get_status");
    LOAD_DLL_FUNC(HotReloadTriggerReload, FnHotReloadTriggerReload, "hotreload_trigger_reload");

    // ---- Analytics (v0.6.0) ----
    LOAD_DLL_FUNC(AnalyticsGetSnapshot, FnAnalyticsGetSnapshot, "analytics_get_snapshot");
    LOAD_DLL_FUNC(AnalyticsReset, FnAnalyticsReset, "analytics_reset");
    LOAD_DLL_FUNC(AnalyticsRecordDamage, FnAnalyticsRecordDamage, "analytics_record_damage");
    LOAD_DLL_FUNC(AnalyticsRecordFloorCleared, FnAnalyticsRecordFloorCleared, "analytics_record_floor_cleared");
    LOAD_DLL_FUNC(AnalyticsRecordGold, FnAnalyticsRecordGold, "analytics_record_gold");
    LOAD_DLL_FUNC(AnalyticsGetEventTypes, FnAnalyticsGetEventTypes, "analytics_get_event_types");

    // Verify critical functions
    if (!Fn_GenerateFloor || !Fn_FreeString || !Fn_GetVersion)
    {
        UE_LOG(LogTemp, Error, TEXT("Failed to resolve critical functions from ProceduralCore DLL"));
        Shutdown();
        return false;
    }

    // Log version
    FString Version = GetVersion();
    UE_LOG(LogTemp, Log, TEXT("ProceduralCore DLL loaded successfully. Version: %s"), *Version);

    return true;
}

void FProceduralCoreBridge::Shutdown()
{
    if (DllHandle)
    {
        FPlatformProcess::FreeDllHandle(DllHandle);
        DllHandle = nullptr;
        UE_LOG(LogTemp, Log, TEXT("ProceduralCore DLL unloaded"));
    }

    // Core
    Fn_GetVersion = nullptr;
    Fn_FreeString = nullptr;

    // Floor Generation
    Fn_GenerateFloor = nullptr;
    Fn_GenerateFloorLayout = nullptr;
    Fn_GetFloorHash = nullptr;
    Fn_GetFloorTier = nullptr;

    // Monster
    Fn_GenerateMonster = nullptr;
    Fn_GenerateFloorMonsters = nullptr;

    // Combat
    Fn_GetAngleMultiplier = nullptr;
    Fn_CalculateCombat = nullptr;

    // Semantic
    Fn_SemanticSimilarity = nullptr;

    // Loot
    Fn_GenerateLoot = nullptr;

    // World
    Fn_GetBreathState = nullptr;

    // Replication
    Fn_RecordDelta = nullptr;
    Fn_CreateFloorSnapshot = nullptr;

    // Events
    Fn_EvaluateEventTrigger = nullptr;

    // Mastery
    Fn_MasteryCreateProfile = nullptr;
    Fn_MasteryGainXp = nullptr;
    Fn_MasteryGetTier = nullptr;
    Fn_MasteryXpForAction = nullptr;
    Fn_MasteryGetAllDomains = nullptr;

    // Specialization
    Fn_SpecGetAllBranches = nullptr;
    Fn_SpecCreateProfile = nullptr;
    Fn_SpecChooseBranch = nullptr;
    Fn_SpecFindSynergies = nullptr;

    // Abilities
    Fn_AbilityGetDefaults = nullptr;
    Fn_AbilityCreateLoadout = nullptr;
    Fn_AbilityLearn = nullptr;
    Fn_AbilityEquip = nullptr;

    // Sockets
    Fn_SocketGetStarterGems = nullptr;
    Fn_SocketGetStarterRunes = nullptr;
    Fn_SocketCreateEquipment = nullptr;
    Fn_SocketInsertGem = nullptr;
    Fn_SocketInsertRune = nullptr;
    Fn_SocketCombineGems = nullptr;

    // Cosmetics
    Fn_CosmeticGetAll = nullptr;
    Fn_CosmeticGetAllDyes = nullptr;
    Fn_CosmeticCreateProfile = nullptr;
    Fn_CosmeticUnlock = nullptr;
    Fn_CosmeticApplyTransmog = nullptr;
    Fn_CosmeticApplyDye = nullptr;

    // Tutorial
    Fn_TutorialGetSteps = nullptr;
    Fn_TutorialGetHints = nullptr;
    Fn_TutorialCreateProgress = nullptr;
    Fn_TutorialCompleteStep = nullptr;
    Fn_TutorialCompletionPercent = nullptr;

    // Achievements
    Fn_AchievementCreateTracker = nullptr;
    Fn_AchievementIncrement = nullptr;
    Fn_AchievementCheckAll = nullptr;
    Fn_AchievementCompletionPercent = nullptr;

    // Season Pass
    Fn_SeasonCreatePass = nullptr;
    Fn_SeasonAddXp = nullptr;
    Fn_SeasonGenerateDailies = nullptr;
    Fn_SeasonGenerateWeeklies = nullptr;
    Fn_SeasonGetRewards = nullptr;

    // Social - Guild
    Fn_SocialCreateGuild = nullptr;
    Fn_SocialGuildAddMember = nullptr;

    // Social - Party
    Fn_SocialCreateParty = nullptr;
    Fn_SocialPartyAddMember = nullptr;

    // Social - Trade
    Fn_SocialCreateTrade = nullptr;
    Fn_SocialTradeAddItem = nullptr;
    Fn_SocialTradeLock = nullptr;
    Fn_SocialTradeConfirm = nullptr;
    Fn_SocialTradeExecute = nullptr;
}

// ============ Helper ============

static FString RustStringToFString(char* RustStr, FnFreeString FreeFn)
{
    if (!RustStr) return FString();
    FString Result = UTF8_TO_TCHAR(RustStr);
    if (FreeFn) FreeFn(RustStr);
    return Result;
}

// ============ Core ============

FString FProceduralCoreBridge::GetVersion()
{
    if (!Fn_GetVersion) return TEXT("unknown");
    return RustStringToFString(Fn_GetVersion(), Fn_FreeString);
}

void FProceduralCoreBridge::FreeRustString(char* Ptr)
{
    if (Fn_FreeString && Ptr)
    {
        Fn_FreeString(Ptr);
    }
}

// ============ Floor Generation ============

FString FProceduralCoreBridge::GenerateFloor(uint64 Seed, uint32 FloorId)
{
    if (!Fn_GenerateFloor) return FString();
    return RustStringToFString(Fn_GenerateFloor(Seed, FloorId), Fn_FreeString);
}

FString FProceduralCoreBridge::GenerateFloorLayout(uint64 Seed, uint32 FloorId)
{
    if (!Fn_GenerateFloorLayout) return FString();
    return RustStringToFString(Fn_GenerateFloorLayout(Seed, FloorId), Fn_FreeString);
}

uint64 FProceduralCoreBridge::GetFloorHash(uint64 Seed, uint32 FloorId)
{
    if (!Fn_GetFloorHash) return 0;
    return Fn_GetFloorHash(Seed, FloorId);
}

uint32 FProceduralCoreBridge::GetFloorTier(uint32 FloorId)
{
    if (!Fn_GetFloorTier) return 0;
    return Fn_GetFloorTier(FloorId);
}

// ============ Monster Generation ============

FString FProceduralCoreBridge::GenerateMonster(uint64 Hash, uint32 FloorLevel)
{
    if (!Fn_GenerateMonster) return FString();
    return RustStringToFString(Fn_GenerateMonster(Hash, FloorLevel), Fn_FreeString);
}

FString FProceduralCoreBridge::GenerateFloorMonsters(uint64 Seed, uint32 FloorId, uint32 Count)
{
    if (!Fn_GenerateFloorMonsters) return FString();
    return RustStringToFString(Fn_GenerateFloorMonsters(Seed, FloorId, Count), Fn_FreeString);
}

// ============ Combat ============

float FProceduralCoreBridge::GetAngleMultiplier(uint32 AngleId)
{
    if (!Fn_GetAngleMultiplier) return 1.0f;
    return Fn_GetAngleMultiplier(AngleId);
}

FString FProceduralCoreBridge::CalculateCombat(const FString& RequestJson)
{
    if (!Fn_CalculateCombat) return FString();
    FTCHARToUTF8 Utf8(*RequestJson);
    return RustStringToFString(Fn_CalculateCombat(Utf8.Get()), Fn_FreeString);
}

// ============ Semantic ============

float FProceduralCoreBridge::SemanticSimilarity(const FString& TagsA, const FString& TagsB)
{
    if (!Fn_SemanticSimilarity) return 0.0f;
    FTCHARToUTF8 Utf8A(*TagsA);
    FTCHARToUTF8 Utf8B(*TagsB);
    return Fn_SemanticSimilarity(Utf8A.Get(), Utf8B.Get());
}

// ============ Loot ============

FString FProceduralCoreBridge::GenerateLoot(const FString& SourceTagsJson, uint32 FloorLevel, uint64 DropHash)
{
    if (!Fn_GenerateLoot) return FString();
    FTCHARToUTF8 Utf8(*SourceTagsJson);
    return RustStringToFString(Fn_GenerateLoot(Utf8.Get(), FloorLevel, DropHash), Fn_FreeString);
}

// ============ World ============

FString FProceduralCoreBridge::GetBreathState(float ElapsedSeconds)
{
    if (!Fn_GetBreathState) return FString();
    return RustStringToFString(Fn_GetBreathState(ElapsedSeconds), Fn_FreeString);
}

// ============ Replication ============

FString FProceduralCoreBridge::RecordDelta(uint32 DeltaTypeId, uint32 FloorId, uint64 EntityHash,
                                           const FString& PlayerId, const FString& Payload, uint64 Tick)
{
    if (!Fn_RecordDelta) return FString();
    FTCHARToUTF8 Utf8Player(*PlayerId);
    FTCHARToUTF8 Utf8Payload(*Payload);
    return RustStringToFString(
        Fn_RecordDelta(DeltaTypeId, FloorId, EntityHash, Utf8Player.Get(), Utf8Payload.Get(), Tick),
        Fn_FreeString);
}

FString FProceduralCoreBridge::CreateFloorSnapshot(uint64 Seed, uint32 FloorId, const FString& DeltasJson)
{
    if (!Fn_CreateFloorSnapshot) return FString();
    FTCHARToUTF8 Utf8(*DeltasJson);
    return RustStringToFString(Fn_CreateFloorSnapshot(Seed, FloorId, Utf8.Get()), Fn_FreeString);
}

// ============ Events ============

FString FProceduralCoreBridge::EvaluateEventTrigger(uint32 TriggerTypeId, const FString& ContextJson)
{
    if (!Fn_EvaluateEventTrigger) return FString();
    FTCHARToUTF8 Utf8(*ContextJson);
    return RustStringToFString(Fn_EvaluateEventTrigger(TriggerTypeId, Utf8.Get()), Fn_FreeString);
}

// ============ Mastery ============

FString FProceduralCoreBridge::MasteryCreateProfile()
{
    if (!Fn_MasteryCreateProfile) return FString();
    return RustStringToFString(Fn_MasteryCreateProfile(), Fn_FreeString);
}

FString FProceduralCoreBridge::MasteryGainXp(const FString& ProfileJson, uint32 DomainId, uint64 Amount)
{
    if (!Fn_MasteryGainXp) return FString();
    FTCHARToUTF8 Utf8(*ProfileJson);
    return RustStringToFString(Fn_MasteryGainXp(Utf8.Get(), DomainId, Amount), Fn_FreeString);
}

int32 FProceduralCoreBridge::MasteryGetTier(const FString& ProfileJson, uint32 DomainId)
{
    if (!Fn_MasteryGetTier) return -1;
    FTCHARToUTF8 Utf8(*ProfileJson);
    return Fn_MasteryGetTier(Utf8.Get(), DomainId);
}

uint64 FProceduralCoreBridge::MasteryXpForAction(const FString& ActionName)
{
    if (!Fn_MasteryXpForAction) return 0;
    FTCHARToUTF8 Utf8(*ActionName);
    return Fn_MasteryXpForAction(Utf8.Get());
}

FString FProceduralCoreBridge::MasteryGetAllDomains()
{
    if (!Fn_MasteryGetAllDomains) return FString();
    return RustStringToFString(Fn_MasteryGetAllDomains(), Fn_FreeString);
}

// ============ Specialization ============

FString FProceduralCoreBridge::SpecGetAllBranches()
{
    if (!Fn_SpecGetAllBranches) return FString();
    return RustStringToFString(Fn_SpecGetAllBranches(), Fn_FreeString);
}

FString FProceduralCoreBridge::SpecCreateProfile()
{
    if (!Fn_SpecCreateProfile) return FString();
    return RustStringToFString(Fn_SpecCreateProfile(), Fn_FreeString);
}

FString FProceduralCoreBridge::SpecChooseBranch(const FString& ProfileJson, const FString& MasteryJson, const FString& BranchId)
{
    if (!Fn_SpecChooseBranch) return FString();
    FTCHARToUTF8 Utf8Profile(*ProfileJson);
    FTCHARToUTF8 Utf8Mastery(*MasteryJson);
    FTCHARToUTF8 Utf8Branch(*BranchId);
    return RustStringToFString(
        Fn_SpecChooseBranch(Utf8Profile.Get(), Utf8Mastery.Get(), Utf8Branch.Get()),
        Fn_FreeString);
}

FString FProceduralCoreBridge::SpecFindSynergies(const FString& BranchIdsJson)
{
    if (!Fn_SpecFindSynergies) return FString();
    FTCHARToUTF8 Utf8(*BranchIdsJson);
    return RustStringToFString(Fn_SpecFindSynergies(Utf8.Get()), Fn_FreeString);
}

// ============ Abilities ============

FString FProceduralCoreBridge::AbilityGetDefaults()
{
    if (!Fn_AbilityGetDefaults) return FString();
    return RustStringToFString(Fn_AbilityGetDefaults(), Fn_FreeString);
}

FString FProceduralCoreBridge::AbilityCreateLoadout()
{
    if (!Fn_AbilityCreateLoadout) return FString();
    return RustStringToFString(Fn_AbilityCreateLoadout(), Fn_FreeString);
}

FString FProceduralCoreBridge::AbilityLearn(const FString& LoadoutJson, const FString& AbilityId)
{
    if (!Fn_AbilityLearn) return FString();
    FTCHARToUTF8 Utf8Loadout(*LoadoutJson);
    FTCHARToUTF8 Utf8Id(*AbilityId);
    return RustStringToFString(Fn_AbilityLearn(Utf8Loadout.Get(), Utf8Id.Get()), Fn_FreeString);
}

FString FProceduralCoreBridge::AbilityEquip(const FString& LoadoutJson, uint32 Slot, const FString& AbilityId)
{
    if (!Fn_AbilityEquip) return FString();
    FTCHARToUTF8 Utf8Loadout(*LoadoutJson);
    FTCHARToUTF8 Utf8Id(*AbilityId);
    return RustStringToFString(Fn_AbilityEquip(Utf8Loadout.Get(), Slot, Utf8Id.Get()), Fn_FreeString);
}

// ============ Sockets ============

FString FProceduralCoreBridge::SocketGetStarterGems()
{
    if (!Fn_SocketGetStarterGems) return FString();
    return RustStringToFString(Fn_SocketGetStarterGems(), Fn_FreeString);
}

FString FProceduralCoreBridge::SocketGetStarterRunes()
{
    if (!Fn_SocketGetStarterRunes) return FString();
    return RustStringToFString(Fn_SocketGetStarterRunes(), Fn_FreeString);
}

FString FProceduralCoreBridge::SocketCreateEquipment(const FString& Name, const FString& ColorsJson)
{
    if (!Fn_SocketCreateEquipment) return FString();
    FTCHARToUTF8 Utf8Name(*Name);
    FTCHARToUTF8 Utf8Colors(*ColorsJson);
    return RustStringToFString(Fn_SocketCreateEquipment(Utf8Name.Get(), Utf8Colors.Get()), Fn_FreeString);
}

FString FProceduralCoreBridge::SocketInsertGem(const FString& EquipmentJson, uint32 Slot, const FString& GemJson)
{
    if (!Fn_SocketInsertGem) return FString();
    FTCHARToUTF8 Utf8Equip(*EquipmentJson);
    FTCHARToUTF8 Utf8Gem(*GemJson);
    return RustStringToFString(Fn_SocketInsertGem(Utf8Equip.Get(), Slot, Utf8Gem.Get()), Fn_FreeString);
}

FString FProceduralCoreBridge::SocketInsertRune(const FString& EquipmentJson, uint32 Slot, const FString& RuneJson)
{
    if (!Fn_SocketInsertRune) return FString();
    FTCHARToUTF8 Utf8Equip(*EquipmentJson);
    FTCHARToUTF8 Utf8Rune(*RuneJson);
    return RustStringToFString(Fn_SocketInsertRune(Utf8Equip.Get(), Slot, Utf8Rune.Get()), Fn_FreeString);
}

FString FProceduralCoreBridge::SocketCombineGems(const FString& GemsJson)
{
    if (!Fn_SocketCombineGems) return FString();
    FTCHARToUTF8 Utf8(*GemsJson);
    return RustStringToFString(Fn_SocketCombineGems(Utf8.Get()), Fn_FreeString);
}

// ============ Cosmetics ============

FString FProceduralCoreBridge::CosmeticGetAll()
{
    if (!Fn_CosmeticGetAll) return FString();
    return RustStringToFString(Fn_CosmeticGetAll(), Fn_FreeString);
}

FString FProceduralCoreBridge::CosmeticGetAllDyes()
{
    if (!Fn_CosmeticGetAllDyes) return FString();
    return RustStringToFString(Fn_CosmeticGetAllDyes(), Fn_FreeString);
}

FString FProceduralCoreBridge::CosmeticCreateProfile()
{
    if (!Fn_CosmeticCreateProfile) return FString();
    return RustStringToFString(Fn_CosmeticCreateProfile(), Fn_FreeString);
}

FString FProceduralCoreBridge::CosmeticUnlock(const FString& ProfileJson, const FString& CosmeticId)
{
    if (!Fn_CosmeticUnlock) return FString();
    FTCHARToUTF8 Utf8Profile(*ProfileJson);
    FTCHARToUTF8 Utf8Id(*CosmeticId);
    return RustStringToFString(Fn_CosmeticUnlock(Utf8Profile.Get(), Utf8Id.Get()), Fn_FreeString);
}

FString FProceduralCoreBridge::CosmeticApplyTransmog(const FString& ProfileJson, uint32 SlotId, const FString& CosmeticId)
{
    if (!Fn_CosmeticApplyTransmog) return FString();
    FTCHARToUTF8 Utf8Profile(*ProfileJson);
    FTCHARToUTF8 Utf8Id(*CosmeticId);
    return RustStringToFString(
        Fn_CosmeticApplyTransmog(Utf8Profile.Get(), SlotId, Utf8Id.Get()),
        Fn_FreeString);
}

FString FProceduralCoreBridge::CosmeticApplyDye(const FString& ProfileJson, uint32 SlotId, uint32 ChannelId, const FString& DyeId)
{
    if (!Fn_CosmeticApplyDye) return FString();
    FTCHARToUTF8 Utf8Profile(*ProfileJson);
    FTCHARToUTF8 Utf8Dye(*DyeId);
    return RustStringToFString(
        Fn_CosmeticApplyDye(Utf8Profile.Get(), SlotId, ChannelId, Utf8Dye.Get()),
        Fn_FreeString);
}

// ============ Tutorial ============

FString FProceduralCoreBridge::TutorialGetSteps()
{
    if (!Fn_TutorialGetSteps) return FString();
    return RustStringToFString(Fn_TutorialGetSteps(), Fn_FreeString);
}

FString FProceduralCoreBridge::TutorialGetHints()
{
    if (!Fn_TutorialGetHints) return FString();
    return RustStringToFString(Fn_TutorialGetHints(), Fn_FreeString);
}

FString FProceduralCoreBridge::TutorialCreateProgress()
{
    if (!Fn_TutorialCreateProgress) return FString();
    return RustStringToFString(Fn_TutorialCreateProgress(), Fn_FreeString);
}

FString FProceduralCoreBridge::TutorialCompleteStep(const FString& ProgressJson, const FString& StepId)
{
    if (!Fn_TutorialCompleteStep) return FString();
    FTCHARToUTF8 Utf8Progress(*ProgressJson);
    FTCHARToUTF8 Utf8Step(*StepId);
    return RustStringToFString(Fn_TutorialCompleteStep(Utf8Progress.Get(), Utf8Step.Get()), Fn_FreeString);
}

float FProceduralCoreBridge::TutorialCompletionPercent(const FString& ProgressJson)
{
    if (!Fn_TutorialCompletionPercent) return 0.0f;
    FTCHARToUTF8 Utf8(*ProgressJson);
    return Fn_TutorialCompletionPercent(Utf8.Get());
}

// ============ Achievements ============

FString FProceduralCoreBridge::AchievementCreateTracker()
{
    if (!Fn_AchievementCreateTracker) return FString();
    return RustStringToFString(Fn_AchievementCreateTracker(), Fn_FreeString);
}

FString FProceduralCoreBridge::AchievementIncrement(const FString& TrackerJson, const FString& AchievementId, uint64 Amount)
{
    if (!Fn_AchievementIncrement) return FString();
    FTCHARToUTF8 Utf8Tracker(*TrackerJson);
    FTCHARToUTF8 Utf8Id(*AchievementId);
    return RustStringToFString(Fn_AchievementIncrement(Utf8Tracker.Get(), Utf8Id.Get(), Amount), Fn_FreeString);
}

FString FProceduralCoreBridge::AchievementCheckAll(const FString& TrackerJson, uint64 CurrentTick)
{
    if (!Fn_AchievementCheckAll) return FString();
    FTCHARToUTF8 Utf8(*TrackerJson);
    return RustStringToFString(Fn_AchievementCheckAll(Utf8.Get(), CurrentTick), Fn_FreeString);
}

float FProceduralCoreBridge::AchievementCompletionPercent(const FString& TrackerJson)
{
    if (!Fn_AchievementCompletionPercent) return 0.0f;
    FTCHARToUTF8 Utf8(*TrackerJson);
    return Fn_AchievementCompletionPercent(Utf8.Get());
}

// ============ Season Pass ============

FString FProceduralCoreBridge::SeasonCreatePass(uint32 SeasonNumber, const FString& Name)
{
    if (!Fn_SeasonCreatePass) return FString();
    FTCHARToUTF8 Utf8(*Name);
    return RustStringToFString(Fn_SeasonCreatePass(SeasonNumber, Utf8.Get()), Fn_FreeString);
}

FString FProceduralCoreBridge::SeasonAddXp(const FString& PassJson, uint64 Amount)
{
    if (!Fn_SeasonAddXp) return FString();
    FTCHARToUTF8 Utf8(*PassJson);
    return RustStringToFString(Fn_SeasonAddXp(Utf8.Get(), Amount), Fn_FreeString);
}

FString FProceduralCoreBridge::SeasonGenerateDailies(uint64 DaySeed)
{
    if (!Fn_SeasonGenerateDailies) return FString();
    return RustStringToFString(Fn_SeasonGenerateDailies(DaySeed), Fn_FreeString);
}

FString FProceduralCoreBridge::SeasonGenerateWeeklies(uint64 WeekSeed)
{
    if (!Fn_SeasonGenerateWeeklies) return FString();
    return RustStringToFString(Fn_SeasonGenerateWeeklies(WeekSeed), Fn_FreeString);
}

FString FProceduralCoreBridge::SeasonGetRewards(uint32 SeasonNumber)
{
    if (!Fn_SeasonGetRewards) return FString();
    return RustStringToFString(Fn_SeasonGetRewards(SeasonNumber), Fn_FreeString);
}

// ============ Social - Guild ============

FString FProceduralCoreBridge::SocialCreateGuild(const FString& Name, const FString& Tag,
                                                  const FString& LeaderId, const FString& LeaderName, const FString& Faction)
{
    if (!Fn_SocialCreateGuild) return FString();
    FTCHARToUTF8 Utf8Name(*Name);
    FTCHARToUTF8 Utf8Tag(*Tag);
    FTCHARToUTF8 Utf8LeaderId(*LeaderId);
    FTCHARToUTF8 Utf8LeaderName(*LeaderName);
    FTCHARToUTF8 Utf8Faction(*Faction);
    return RustStringToFString(
        Fn_SocialCreateGuild(Utf8Name.Get(), Utf8Tag.Get(), Utf8LeaderId.Get(), Utf8LeaderName.Get(), Utf8Faction.Get()),
        Fn_FreeString);
}

FString FProceduralCoreBridge::SocialGuildAddMember(const FString& GuildJson, const FString& UserId, const FString& UserName)
{
    if (!Fn_SocialGuildAddMember) return FString();
    FTCHARToUTF8 Utf8Guild(*GuildJson);
    FTCHARToUTF8 Utf8Id(*UserId);
    FTCHARToUTF8 Utf8Name(*UserName);
    return RustStringToFString(
        Fn_SocialGuildAddMember(Utf8Guild.Get(), Utf8Id.Get(), Utf8Name.Get()),
        Fn_FreeString);
}

// ============ Social - Party ============

FString FProceduralCoreBridge::SocialCreateParty(const FString& LeaderId, const FString& LeaderName)
{
    if (!Fn_SocialCreateParty) return FString();
    FTCHARToUTF8 Utf8Id(*LeaderId);
    FTCHARToUTF8 Utf8Name(*LeaderName);
    return RustStringToFString(Fn_SocialCreateParty(Utf8Id.Get(), Utf8Name.Get()), Fn_FreeString);
}

FString FProceduralCoreBridge::SocialPartyAddMember(const FString& PartyJson, const FString& UserId,
                                                     const FString& UserName, uint32 RoleId)
{
    if (!Fn_SocialPartyAddMember) return FString();
    FTCHARToUTF8 Utf8Party(*PartyJson);
    FTCHARToUTF8 Utf8Id(*UserId);
    FTCHARToUTF8 Utf8Name(*UserName);
    return RustStringToFString(
        Fn_SocialPartyAddMember(Utf8Party.Get(), Utf8Id.Get(), Utf8Name.Get(), RoleId),
        Fn_FreeString);
}

// ============ Social - Trade ============

FString FProceduralCoreBridge::SocialCreateTrade(const FString& PlayerA, const FString& PlayerB)
{
    if (!Fn_SocialCreateTrade) return FString();
    FTCHARToUTF8 Utf8A(*PlayerA);
    FTCHARToUTF8 Utf8B(*PlayerB);
    return RustStringToFString(Fn_SocialCreateTrade(Utf8A.Get(), Utf8B.Get()), Fn_FreeString);
}

FString FProceduralCoreBridge::SocialTradeAddItem(const FString& TradeJson, const FString& PlayerId,
                                                   const FString& ItemName, uint32 Quantity, const FString& Rarity)
{
    if (!Fn_SocialTradeAddItem) return FString();
    FTCHARToUTF8 Utf8Trade(*TradeJson);
    FTCHARToUTF8 Utf8Player(*PlayerId);
    FTCHARToUTF8 Utf8Item(*ItemName);
    FTCHARToUTF8 Utf8Rarity(*Rarity);
    return RustStringToFString(
        Fn_SocialTradeAddItem(Utf8Trade.Get(), Utf8Player.Get(), Utf8Item.Get(), Quantity, Utf8Rarity.Get()),
        Fn_FreeString);
}

FString FProceduralCoreBridge::SocialTradeLock(const FString& TradeJson, const FString& PlayerId)
{
    if (!Fn_SocialTradeLock) return FString();
    FTCHARToUTF8 Utf8Trade(*TradeJson);
    FTCHARToUTF8 Utf8Player(*PlayerId);
    return RustStringToFString(Fn_SocialTradeLock(Utf8Trade.Get(), Utf8Player.Get()), Fn_FreeString);
}

FString FProceduralCoreBridge::SocialTradeConfirm(const FString& TradeJson, const FString& PlayerId)
{
    if (!Fn_SocialTradeConfirm) return FString();
    FTCHARToUTF8 Utf8Trade(*TradeJson);
    FTCHARToUTF8 Utf8Player(*PlayerId);
    return RustStringToFString(Fn_SocialTradeConfirm(Utf8Trade.Get(), Utf8Player.Get()), Fn_FreeString);
}

FString FProceduralCoreBridge::SocialTradeExecute(const FString& TradeJson)
{
    if (!Fn_SocialTradeExecute) return FString();
    FTCHARToUTF8 Utf8(*TradeJson);
    return RustStringToFString(Fn_SocialTradeExecute(Utf8.Get()), Fn_FreeString);
}

// ============ Hot-Reload (v0.6.0) ============

FString FProceduralCoreBridge::HotReloadGetStatus()
{
    if (!Fn_HotReloadGetStatus) return FString();
    return RustStringToFString(Fn_HotReloadGetStatus(), Fn_FreeString);
}

uint32 FProceduralCoreBridge::HotReloadTriggerReload()
{
    if (!Fn_HotReloadTriggerReload) return 0;
    return Fn_HotReloadTriggerReload();
}

// ============ Analytics (v0.6.0) ============

FString FProceduralCoreBridge::AnalyticsGetSnapshot()
{
    if (!Fn_AnalyticsGetSnapshot) return FString();
    return RustStringToFString(Fn_AnalyticsGetSnapshot(), Fn_FreeString);
}

void FProceduralCoreBridge::AnalyticsReset()
{
    if (Fn_AnalyticsReset)
    {
        Fn_AnalyticsReset();
    }
}

void FProceduralCoreBridge::AnalyticsRecordDamage(const FString& WeaponName, uint32 Amount)
{
    if (!Fn_AnalyticsRecordDamage) return;
    FTCHARToUTF8 Utf8Weapon(*WeaponName);
    Fn_AnalyticsRecordDamage(Utf8Weapon.Get(), Amount);
}

void FProceduralCoreBridge::AnalyticsRecordFloorCleared(uint32 FloorId, uint32 Tier, float TimeSecs)
{
    if (Fn_AnalyticsRecordFloorCleared)
    {
        Fn_AnalyticsRecordFloorCleared(FloorId, Tier, TimeSecs);
    }
}

void FProceduralCoreBridge::AnalyticsRecordGold(uint64 Amount)
{
    if (Fn_AnalyticsRecordGold)
    {
        Fn_AnalyticsRecordGold(Amount);
    }
}

FString FProceduralCoreBridge::AnalyticsGetEventTypes()
{
    if (!Fn_AnalyticsGetEventTypes) return FString();
    return RustStringToFString(Fn_AnalyticsGetEventTypes(), Fn_FreeString);
}
