#pragma once

#include "CoreMinimal.h"

/**
 * C++ bridge for loading the Rust Procedural Core DLL (tower_core.dll).
 *
 * The Rust DLL v0.6.0 exposes 100 C-ABI functions covering:
 * - Floor generation (spec + full tile layout)
 * - Monster generation (templates + stats)
 * - Combat calculations (damage, angle, semantic bonuses)
 * - Loot generation (semantic-based drops)
 * - World state (Breath of Tower cycle)
 * - Semantic tag operations (similarity)
 * - Replication (delta recording, snapshots)
 * - Events (procedural event triggers)
 * - Mastery (XP tracking, tier progression, 21 domains)
 * - Specialization (branches, synergies, combat roles)
 * - Abilities (hotbar, cooldowns, learn/equip)
 * - Sockets (gems, runes, combining)
 * - Cosmetics (transmog, dyes, outfits)
 * - Tutorial (steps, hints, progress)
 * - Achievements (tracking, unlock, completion)
 * - Season Pass (XP, daily/weekly quests, rewards)
 * - Social (guild, party, trade)
 *
 * All returned strings must be freed with FreeRustString().
 */

// ============================================================
// Function pointer types matching Rust extern "C" exports
// ============================================================

// Core
typedef char* (*FnGetVersion)();
typedef void  (*FnFreeString)(char*);

// Floor Generation
typedef char*  (*FnGenerateFloor)(uint64, uint32);
typedef char*  (*FnGenerateFloorLayout)(uint64, uint32);
typedef uint64 (*FnGetFloorHash)(uint64, uint32);
typedef uint32 (*FnGetFloorTier)(uint32);

// Monster Generation
typedef char* (*FnGenerateMonster)(uint64, uint32);
typedef char* (*FnGenerateFloorMonsters)(uint64, uint32, uint32);

// Combat
typedef float (*FnGetAngleMultiplier)(uint32);
typedef char* (*FnCalculateCombat)(const char*);

// Semantic
typedef float (*FnSemanticSimilarity)(const char*, const char*);

// Loot
typedef char* (*FnGenerateLoot)(const char*, uint32, uint64);

// World
typedef char* (*FnGetBreathState)(float);

// Replication
typedef char* (*FnRecordDelta)(uint32, uint32, uint64, const char*, const char*, uint64);
typedef char* (*FnCreateFloorSnapshot)(uint64, uint32, const char*);

// Events
typedef char* (*FnEvaluateEventTrigger)(uint32, const char*);

// Mastery
typedef char*  (*FnMasteryCreateProfile)();
typedef char*  (*FnMasteryGainXp)(const char*, uint32, uint64);
typedef int32  (*FnMasteryGetTier)(const char*, uint32);
typedef uint64 (*FnMasteryXpForAction)(const char*);
typedef char*  (*FnMasteryGetAllDomains)();

// Specialization
typedef char* (*FnSpecGetAllBranches)();
typedef char* (*FnSpecCreateProfile)();
typedef char* (*FnSpecChooseBranch)(const char*, const char*, const char*);
typedef char* (*FnSpecFindSynergies)(const char*);

// Abilities
typedef char* (*FnAbilityGetDefaults)();
typedef char* (*FnAbilityCreateLoadout)();
typedef char* (*FnAbilityLearn)(const char*, const char*);
typedef char* (*FnAbilityEquip)(const char*, uint32, const char*);

// Sockets
typedef char* (*FnSocketGetStarterGems)();
typedef char* (*FnSocketGetStarterRunes)();
typedef char* (*FnSocketCreateEquipment)(const char*, const char*);
typedef char* (*FnSocketInsertGem)(const char*, uint32, const char*);
typedef char* (*FnSocketInsertRune)(const char*, uint32, const char*);
typedef char* (*FnSocketCombineGems)(const char*);

// Cosmetics
typedef char* (*FnCosmeticGetAll)();
typedef char* (*FnCosmeticGetAllDyes)();
typedef char* (*FnCosmeticCreateProfile)();
typedef char* (*FnCosmeticUnlock)(const char*, const char*);
typedef char* (*FnCosmeticApplyTransmog)(const char*, uint32, const char*);
typedef char* (*FnCosmeticApplyDye)(const char*, uint32, uint32, const char*);

// Tutorial
typedef char* (*FnTutorialGetSteps)();
typedef char* (*FnTutorialGetHints)();
typedef char* (*FnTutorialCreateProgress)();
typedef char* (*FnTutorialCompleteStep)(const char*, const char*);
typedef float (*FnTutorialCompletionPercent)(const char*);

// Achievements
typedef char* (*FnAchievementCreateTracker)();
typedef char* (*FnAchievementIncrement)(const char*, const char*, uint64);
typedef char* (*FnAchievementCheckAll)(const char*, uint64);
typedef float (*FnAchievementCompletionPercent)(const char*);

// Season Pass
typedef char* (*FnSeasonCreatePass)(uint32, const char*);
typedef char* (*FnSeasonAddXp)(const char*, uint64);
typedef char* (*FnSeasonGenerateDailies)(uint64);
typedef char* (*FnSeasonGenerateWeeklies)(uint64);
typedef char* (*FnSeasonGetRewards)(uint32);

// Social - Guild
typedef char* (*FnSocialCreateGuild)(const char*, const char*, const char*, const char*, const char*);
typedef char* (*FnSocialGuildAddMember)(const char*, const char*, const char*);

// Social - Party
typedef char* (*FnSocialCreateParty)(const char*, const char*);
typedef char* (*FnSocialPartyAddMember)(const char*, const char*, const char*, uint32);

// Social - Trade
typedef char* (*FnSocialCreateTrade)(const char*, const char*);
typedef char* (*FnSocialTradeAddItem)(const char*, const char*, const char*, uint32, const char*);
typedef char* (*FnSocialTradeLock)(const char*, const char*);
typedef char* (*FnSocialTradeConfirm)(const char*, const char*);
typedef char* (*FnSocialTradeExecute)(const char*);

// Hot-Reload (v0.6.0 - Session 22)
typedef char* (*FnHotReloadGetStatus)();
typedef uint32 (*FnHotReloadTriggerReload)();

// Analytics (v0.6.0 - Session 22)
typedef char* (*FnAnalyticsGetSnapshot)();
typedef void  (*FnAnalyticsReset)();
typedef void  (*FnAnalyticsRecordDamage)(const char*, uint32);
typedef void  (*FnAnalyticsRecordFloorCleared)(uint32, uint32, float);
typedef void  (*FnAnalyticsRecordGold)(uint64);
typedef char* (*FnAnalyticsGetEventTypes)();

// ============================================================
// Bridge class
// ============================================================

class TOWERGAME_API FProceduralCoreBridge
{
public:
    FProceduralCoreBridge();
    ~FProceduralCoreBridge();

    bool Initialize(const FString& DllPath);
    void Shutdown();
    bool IsInitialized() const { return DllHandle != nullptr; }

    // ============ Core ============
    FString GetVersion();
    void FreeRustString(char* Ptr);

    // ============ Floor Generation ============
    FString GenerateFloor(uint64 Seed, uint32 FloorId);
    FString GenerateFloorLayout(uint64 Seed, uint32 FloorId);
    uint64 GetFloorHash(uint64 Seed, uint32 FloorId);
    uint32 GetFloorTier(uint32 FloorId);

    // ============ Monster Generation ============
    FString GenerateMonster(uint64 Hash, uint32 FloorLevel);
    FString GenerateFloorMonsters(uint64 Seed, uint32 FloorId, uint32 Count);

    // ============ Combat ============
    float GetAngleMultiplier(uint32 AngleId);
    FString CalculateCombat(const FString& RequestJson);

    // ============ Semantic ============
    float SemanticSimilarity(const FString& TagsA, const FString& TagsB);

    // ============ Loot ============
    FString GenerateLoot(const FString& SourceTagsJson, uint32 FloorLevel, uint64 DropHash);

    // ============ World ============
    FString GetBreathState(float ElapsedSeconds);

    // ============ Replication ============
    FString RecordDelta(uint32 DeltaTypeId, uint32 FloorId, uint64 EntityHash,
                        const FString& PlayerId, const FString& Payload, uint64 Tick);
    FString CreateFloorSnapshot(uint64 Seed, uint32 FloorId, const FString& DeltasJson);

    // ============ Events ============
    FString EvaluateEventTrigger(uint32 TriggerTypeId, const FString& ContextJson);

    // ============ Mastery ============
    FString MasteryCreateProfile();
    FString MasteryGainXp(const FString& ProfileJson, uint32 DomainId, uint64 Amount);
    int32 MasteryGetTier(const FString& ProfileJson, uint32 DomainId);
    uint64 MasteryXpForAction(const FString& ActionName);
    FString MasteryGetAllDomains();

    // ============ Specialization ============
    FString SpecGetAllBranches();
    FString SpecCreateProfile();
    FString SpecChooseBranch(const FString& ProfileJson, const FString& MasteryJson, const FString& BranchId);
    FString SpecFindSynergies(const FString& BranchIdsJson);

    // ============ Abilities ============
    FString AbilityGetDefaults();
    FString AbilityCreateLoadout();
    FString AbilityLearn(const FString& LoadoutJson, const FString& AbilityId);
    FString AbilityEquip(const FString& LoadoutJson, uint32 Slot, const FString& AbilityId);

    // ============ Sockets ============
    FString SocketGetStarterGems();
    FString SocketGetStarterRunes();
    FString SocketCreateEquipment(const FString& Name, const FString& ColorsJson);
    FString SocketInsertGem(const FString& EquipmentJson, uint32 Slot, const FString& GemJson);
    FString SocketInsertRune(const FString& EquipmentJson, uint32 Slot, const FString& RuneJson);
    FString SocketCombineGems(const FString& GemsJson);

    // ============ Cosmetics ============
    FString CosmeticGetAll();
    FString CosmeticGetAllDyes();
    FString CosmeticCreateProfile();
    FString CosmeticUnlock(const FString& ProfileJson, const FString& CosmeticId);
    FString CosmeticApplyTransmog(const FString& ProfileJson, uint32 SlotId, const FString& CosmeticId);
    FString CosmeticApplyDye(const FString& ProfileJson, uint32 SlotId, uint32 ChannelId, const FString& DyeId);

    // ============ Tutorial ============
    FString TutorialGetSteps();
    FString TutorialGetHints();
    FString TutorialCreateProgress();
    FString TutorialCompleteStep(const FString& ProgressJson, const FString& StepId);
    float TutorialCompletionPercent(const FString& ProgressJson);

    // ============ Achievements ============
    FString AchievementCreateTracker();
    FString AchievementIncrement(const FString& TrackerJson, const FString& AchievementId, uint64 Amount);
    FString AchievementCheckAll(const FString& TrackerJson, uint64 CurrentTick);
    float AchievementCompletionPercent(const FString& TrackerJson);

    // ============ Season Pass ============
    FString SeasonCreatePass(uint32 SeasonNumber, const FString& Name);
    FString SeasonAddXp(const FString& PassJson, uint64 Amount);
    FString SeasonGenerateDailies(uint64 DaySeed);
    FString SeasonGenerateWeeklies(uint64 WeekSeed);
    FString SeasonGetRewards(uint32 SeasonNumber);

    // ============ Social - Guild ============
    FString SocialCreateGuild(const FString& Name, const FString& Tag,
                              const FString& LeaderId, const FString& LeaderName, const FString& Faction);
    FString SocialGuildAddMember(const FString& GuildJson, const FString& UserId, const FString& UserName);

    // ============ Social - Party ============
    FString SocialCreateParty(const FString& LeaderId, const FString& LeaderName);
    FString SocialPartyAddMember(const FString& PartyJson, const FString& UserId,
                                 const FString& UserName, uint32 RoleId);

    // ============ Social - Trade ============
    FString SocialCreateTrade(const FString& PlayerA, const FString& PlayerB);
    FString SocialTradeAddItem(const FString& TradeJson, const FString& PlayerId,
                               const FString& ItemName, uint32 Quantity, const FString& Rarity);
    FString SocialTradeLock(const FString& TradeJson, const FString& PlayerId);
    FString SocialTradeConfirm(const FString& TradeJson, const FString& PlayerId);
    FString SocialTradeExecute(const FString& TradeJson);

    // ============ Hot-Reload (v0.6.0) ============
    FString HotReloadGetStatus();
    uint32 HotReloadTriggerReload();

    // ============ Analytics (v0.6.0) ============
    FString AnalyticsGetSnapshot();
    void AnalyticsReset();
    void AnalyticsRecordDamage(const FString& WeaponName, uint32 Amount);
    void AnalyticsRecordFloorCleared(uint32 FloorId, uint32 Tier, float TimeSecs);
    void AnalyticsRecordGold(uint64 Amount);
    FString AnalyticsGetEventTypes();

private:
    void* DllHandle = nullptr;

    // Core
    FnGetVersion Fn_GetVersion = nullptr;
    FnFreeString Fn_FreeString = nullptr;

    // Floor Generation
    FnGenerateFloor Fn_GenerateFloor = nullptr;
    FnGenerateFloorLayout Fn_GenerateFloorLayout = nullptr;
    FnGetFloorHash Fn_GetFloorHash = nullptr;
    FnGetFloorTier Fn_GetFloorTier = nullptr;

    // Monster
    FnGenerateMonster Fn_GenerateMonster = nullptr;
    FnGenerateFloorMonsters Fn_GenerateFloorMonsters = nullptr;

    // Combat
    FnGetAngleMultiplier Fn_GetAngleMultiplier = nullptr;
    FnCalculateCombat Fn_CalculateCombat = nullptr;

    // Semantic
    FnSemanticSimilarity Fn_SemanticSimilarity = nullptr;

    // Loot
    FnGenerateLoot Fn_GenerateLoot = nullptr;

    // World
    FnGetBreathState Fn_GetBreathState = nullptr;

    // Replication
    FnRecordDelta Fn_RecordDelta = nullptr;
    FnCreateFloorSnapshot Fn_CreateFloorSnapshot = nullptr;

    // Events
    FnEvaluateEventTrigger Fn_EvaluateEventTrigger = nullptr;

    // Mastery
    FnMasteryCreateProfile Fn_MasteryCreateProfile = nullptr;
    FnMasteryGainXp Fn_MasteryGainXp = nullptr;
    FnMasteryGetTier Fn_MasteryGetTier = nullptr;
    FnMasteryXpForAction Fn_MasteryXpForAction = nullptr;
    FnMasteryGetAllDomains Fn_MasteryGetAllDomains = nullptr;

    // Specialization
    FnSpecGetAllBranches Fn_SpecGetAllBranches = nullptr;
    FnSpecCreateProfile Fn_SpecCreateProfile = nullptr;
    FnSpecChooseBranch Fn_SpecChooseBranch = nullptr;
    FnSpecFindSynergies Fn_SpecFindSynergies = nullptr;

    // Abilities
    FnAbilityGetDefaults Fn_AbilityGetDefaults = nullptr;
    FnAbilityCreateLoadout Fn_AbilityCreateLoadout = nullptr;
    FnAbilityLearn Fn_AbilityLearn = nullptr;
    FnAbilityEquip Fn_AbilityEquip = nullptr;

    // Sockets
    FnSocketGetStarterGems Fn_SocketGetStarterGems = nullptr;
    FnSocketGetStarterRunes Fn_SocketGetStarterRunes = nullptr;
    FnSocketCreateEquipment Fn_SocketCreateEquipment = nullptr;
    FnSocketInsertGem Fn_SocketInsertGem = nullptr;
    FnSocketInsertRune Fn_SocketInsertRune = nullptr;
    FnSocketCombineGems Fn_SocketCombineGems = nullptr;

    // Cosmetics
    FnCosmeticGetAll Fn_CosmeticGetAll = nullptr;
    FnCosmeticGetAllDyes Fn_CosmeticGetAllDyes = nullptr;
    FnCosmeticCreateProfile Fn_CosmeticCreateProfile = nullptr;
    FnCosmeticUnlock Fn_CosmeticUnlock = nullptr;
    FnCosmeticApplyTransmog Fn_CosmeticApplyTransmog = nullptr;
    FnCosmeticApplyDye Fn_CosmeticApplyDye = nullptr;

    // Tutorial
    FnTutorialGetSteps Fn_TutorialGetSteps = nullptr;
    FnTutorialGetHints Fn_TutorialGetHints = nullptr;
    FnTutorialCreateProgress Fn_TutorialCreateProgress = nullptr;
    FnTutorialCompleteStep Fn_TutorialCompleteStep = nullptr;
    FnTutorialCompletionPercent Fn_TutorialCompletionPercent = nullptr;

    // Achievements
    FnAchievementCreateTracker Fn_AchievementCreateTracker = nullptr;
    FnAchievementIncrement Fn_AchievementIncrement = nullptr;
    FnAchievementCheckAll Fn_AchievementCheckAll = nullptr;
    FnAchievementCompletionPercent Fn_AchievementCompletionPercent = nullptr;

    // Season Pass
    FnSeasonCreatePass Fn_SeasonCreatePass = nullptr;
    FnSeasonAddXp Fn_SeasonAddXp = nullptr;
    FnSeasonGenerateDailies Fn_SeasonGenerateDailies = nullptr;
    FnSeasonGenerateWeeklies Fn_SeasonGenerateWeeklies = nullptr;
    FnSeasonGetRewards Fn_SeasonGetRewards = nullptr;

    // Social - Guild
    FnSocialCreateGuild Fn_SocialCreateGuild = nullptr;
    FnSocialGuildAddMember Fn_SocialGuildAddMember = nullptr;

    // Social - Party
    FnSocialCreateParty Fn_SocialCreateParty = nullptr;
    FnSocialPartyAddMember Fn_SocialPartyAddMember = nullptr;

    // Social - Trade
    FnSocialCreateTrade Fn_SocialCreateTrade = nullptr;
    FnSocialTradeAddItem Fn_SocialTradeAddItem = nullptr;
    FnSocialTradeLock Fn_SocialTradeLock = nullptr;
    FnSocialTradeConfirm Fn_SocialTradeConfirm = nullptr;
    FnSocialTradeExecute Fn_SocialTradeExecute = nullptr;

    // Hot-Reload (v0.6.0)
    FnHotReloadGetStatus Fn_HotReloadGetStatus = nullptr;
    FnHotReloadTriggerReload Fn_HotReloadTriggerReload = nullptr;

    // Analytics (v0.6.0)
    FnAnalyticsGetSnapshot Fn_AnalyticsGetSnapshot = nullptr;
    FnAnalyticsReset Fn_AnalyticsReset = nullptr;
    FnAnalyticsRecordDamage Fn_AnalyticsRecordDamage = nullptr;
    FnAnalyticsRecordFloorCleared Fn_AnalyticsRecordFloorCleared = nullptr;
    FnAnalyticsRecordGold Fn_AnalyticsRecordGold = nullptr;
    FnAnalyticsGetEventTypes Fn_AnalyticsGetEventTypes = nullptr;
};
