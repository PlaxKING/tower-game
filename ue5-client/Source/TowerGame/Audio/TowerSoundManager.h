#pragma once

#include "CoreMinimal.h"
#include "Components/ActorComponent.h"
#include "TowerSoundManager.generated.h"

class USoundCue;
class UAudioComponent;

/**
 * Sound categories for the Tower game.
 */
UENUM(BlueprintType)
enum class ESoundCategory : uint8
{
    // Combat
    SwordSwing      UMETA(DisplayName = "Sword Swing"),
    GreatswordSlam  UMETA(DisplayName = "Greatsword Slam"),
    DaggerSlash     UMETA(DisplayName = "Dagger Slash"),
    SpearThrust     UMETA(DisplayName = "Spear Thrust"),
    GauntletPunch   UMETA(DisplayName = "Gauntlet Punch"),
    StaffCast       UMETA(DisplayName = "Staff Cast"),
    HitFlesh        UMETA(DisplayName = "Hit Flesh"),
    HitArmor        UMETA(DisplayName = "Hit Armor"),
    ParryClang      UMETA(DisplayName = "Parry Clang"),
    DodgeWhoosh     UMETA(DisplayName = "Dodge Whoosh"),
    BlockImpact     UMETA(DisplayName = "Block Impact"),
    CriticalHit     UMETA(DisplayName = "Critical Hit"),

    // Elements
    FireBurst       UMETA(DisplayName = "Fire Burst"),
    IceCrack        UMETA(DisplayName = "Ice Crack"),
    WindGust        UMETA(DisplayName = "Wind Gust"),
    EarthRumble     UMETA(DisplayName = "Earth Rumble"),
    VoidPulse       UMETA(DisplayName = "Void Pulse"),
    CorruptionHiss  UMETA(DisplayName = "Corruption Hiss"),

    // Player
    Footstep        UMETA(DisplayName = "Footstep"),
    Jump            UMETA(DisplayName = "Jump"),
    Land            UMETA(DisplayName = "Land"),
    Death           UMETA(DisplayName = "Death"),
    LevelUp         UMETA(DisplayName = "Level Up"),
    HealPulse       UMETA(DisplayName = "Heal Pulse"),

    // World
    LootPickup      UMETA(DisplayName = "Loot Pickup"),
    LootDrop        UMETA(DisplayName = "Loot Drop"),
    ChestOpen       UMETA(DisplayName = "Chest Open"),
    ShrineActivate  UMETA(DisplayName = "Shrine Activate"),
    StairsAscend    UMETA(DisplayName = "Stairs Ascend"),
    BreathShift     UMETA(DisplayName = "Breath Shift"),
    EchoAppear      UMETA(DisplayName = "Echo Appear"),
    EchoDisappear   UMETA(DisplayName = "Echo Disappear"),

    // UI
    MenuOpen        UMETA(DisplayName = "Menu Open"),
    MenuClose       UMETA(DisplayName = "Menu Close"),
    ButtonClick     UMETA(DisplayName = "Button Click"),
    QuestComplete   UMETA(DisplayName = "Quest Complete"),
};

/**
 * Sound manager component — centralized audio playback for the Tower.
 *
 * Attach to the player character. Provides:
 * - Categorized sound playback (combat, elements, world, UI)
 * - Volume control per category (inherits from PauseMenuWidget settings)
 * - Spatial 3D sound for world events
 * - Ambient loop management (floor ambience, Breath of Tower)
 * - Pitch/volume variation for repeated sounds (anti-repetition)
 *
 * Sounds are played via SoundCue references set in Blueprint or
 * loaded at runtime. If no SoundCue is assigned for a category,
 * playback silently skips.
 */
UCLASS(ClassGroup = (Audio), meta = (BlueprintSpawnableComponent))
class TOWERGAME_API UTowerSoundManager : public UActorComponent
{
    GENERATED_BODY()

public:
    UTowerSoundManager();

    virtual void BeginPlay() override;
    virtual void EndPlay(const EEndPlayReason::Type EndPlayReason) override;

    // ============ Playback ============

    /** Play a 2D sound (UI, player-local) */
    UFUNCTION(BlueprintCallable, Category = "Sound")
    void PlaySound2D(ESoundCategory Category);

    /** Play a 3D sound at a location */
    UFUNCTION(BlueprintCallable, Category = "Sound")
    void PlaySoundAtLocation(ESoundCategory Category, FVector Location);

    /** Play with custom pitch/volume override */
    UFUNCTION(BlueprintCallable, Category = "Sound")
    void PlaySoundWithParams(ESoundCategory Category, FVector Location, float PitchMultiplier, float VolumeMultiplier);

    // ============ Ambient ============

    /** Start ambient loop for the current floor */
    UFUNCTION(BlueprintCallable, Category = "Sound|Ambient")
    void StartFloorAmbience(int32 FloorLevel);

    /** Stop ambient loop */
    UFUNCTION(BlueprintCallable, Category = "Sound|Ambient")
    void StopFloorAmbience();

    /** Play breath phase transition sound */
    UFUNCTION(BlueprintCallable, Category = "Sound|Ambient")
    void PlayBreathTransition(const FString& NewPhase);

    // ============ Volume ============

    /** Set master volume (0.0 - 1.0) */
    UFUNCTION(BlueprintCallable, Category = "Sound|Volume")
    void SetMasterVolume(float Volume);

    /** Set SFX volume (0.0 - 1.0) */
    UFUNCTION(BlueprintCallable, Category = "Sound|Volume")
    void SetSFXVolume(float Volume);

    /** Set music volume (0.0 - 1.0) */
    UFUNCTION(BlueprintCallable, Category = "Sound|Volume")
    void SetMusicVolume(float Volume);

    // ============ Sound Cue Assignments ============

    /** Map of sound categories to sound cues */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Sound|Cues")
    TMap<ESoundCategory, USoundCue*> SoundCues;

    /** Ambient loop cue (per floor tier) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Sound|Cues")
    USoundCue* AmbientCue;

    /** Breath transition cue */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Sound|Cues")
    USoundCue* BreathTransitionCue;

    // ============ Config ============

    /** Random pitch variation range (+/-) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Sound|Config")
    float PitchVariation = 0.1f;

    /** Random volume variation range (+/-) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Sound|Config")
    float VolumeVariation = 0.05f;

    /** Minimum time between same sound category (anti-spam) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Sound|Config")
    float MinRepeatInterval = 0.05f;

private:
    UPROPERTY()
    UAudioComponent* AmbientComponent;

    float MasterVol = 1.0f;
    float SFXVol = 0.8f;
    float MusicVol = 0.6f;

    /** Last play time for each category (anti-spam) */
    TMap<ESoundCategory, float> LastPlayTimes;

    /** Get effective volume for a category */
    float GetEffectiveVolume(ESoundCategory Category) const;

    /** Is this a music/ambient category? */
    bool IsMusicCategory(ESoundCategory Category) const;
};
