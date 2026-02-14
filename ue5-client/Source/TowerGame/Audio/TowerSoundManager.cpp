#include "TowerSoundManager.h"
#include "Sound/SoundCue.h"
#include "Components/AudioComponent.h"
#include "Kismet/GameplayStatics.h"

UTowerSoundManager::UTowerSoundManager()
{
    PrimaryComponentTick.bCanEverTick = false;
}

void UTowerSoundManager::BeginPlay()
{
    Super::BeginPlay();
}

void UTowerSoundManager::EndPlay(const EEndPlayReason::Type EndPlayReason)
{
    StopFloorAmbience();
    Super::EndPlay(EndPlayReason);
}

void UTowerSoundManager::PlaySound2D(ESoundCategory Category)
{
    USoundCue** CuePtr = SoundCues.Find(Category);
    if (!CuePtr || !*CuePtr) return;

    // Anti-spam check
    float CurrentTime = GetWorld() ? GetWorld()->GetTimeSeconds() : 0.0f;
    float* LastTime = LastPlayTimes.Find(Category);
    if (LastTime && (CurrentTime - *LastTime) < MinRepeatInterval)
    {
        return;
    }
    LastPlayTimes.Add(Category, CurrentTime);

    float Vol = GetEffectiveVolume(Category);
    float Pitch = 1.0f + FMath::FRandRange(-PitchVariation, PitchVariation);

    UGameplayStatics::PlaySound2D(GetOwner(), *CuePtr, Vol, Pitch);
}

void UTowerSoundManager::PlaySoundAtLocation(ESoundCategory Category, FVector Location)
{
    PlaySoundWithParams(Category, Location, 1.0f, 1.0f);
}

void UTowerSoundManager::PlaySoundWithParams(ESoundCategory Category, FVector Location,
    float PitchMultiplier, float VolumeMultiplier)
{
    USoundCue** CuePtr = SoundCues.Find(Category);
    if (!CuePtr || !*CuePtr) return;

    // Anti-spam check
    float CurrentTime = GetWorld() ? GetWorld()->GetTimeSeconds() : 0.0f;
    float* LastTime = LastPlayTimes.Find(Category);
    if (LastTime && (CurrentTime - *LastTime) < MinRepeatInterval)
    {
        return;
    }
    LastPlayTimes.Add(Category, CurrentTime);

    float Vol = GetEffectiveVolume(Category) * VolumeMultiplier;
    Vol *= (1.0f + FMath::FRandRange(-VolumeVariation, VolumeVariation));

    float Pitch = PitchMultiplier + FMath::FRandRange(-PitchVariation, PitchVariation);

    UGameplayStatics::PlaySoundAtLocation(GetOwner(), *CuePtr, Location, Vol, Pitch);
}

void UTowerSoundManager::StartFloorAmbience(int32 FloorLevel)
{
    StopFloorAmbience();

    if (!AmbientCue || !GetOwner()) return;

    AmbientComponent = UGameplayStatics::SpawnSoundAttached(
        AmbientCue,
        GetOwner()->GetRootComponent(),
        NAME_None,
        FVector::ZeroVector,
        EAttachLocation::KeepRelativeOffset,
        true, // bStopWhenOwnerDestroyed
        MusicVol * MasterVol,
        1.0f, // Pitch
        0.0f, // StartTime
        nullptr,
        nullptr,
        true // bAutoDestroy — no, we manage it
    );

    if (AmbientComponent)
    {
        AmbientComponent->bAutoDestroy = false;
        AmbientComponent->bIsUISound = false;

        // Pitch shifts slightly with floor level (higher floors = slightly higher pitch)
        float FloorPitch = 1.0f + (FloorLevel * 0.002f);
        AmbientComponent->SetPitchMultiplier(FMath::Clamp(FloorPitch, 0.8f, 1.5f));

        UE_LOG(LogTemp, Log, TEXT("Floor ambience started (floor %d, pitch=%.3f)"),
            FloorLevel, FloorPitch);
    }
}

void UTowerSoundManager::StopFloorAmbience()
{
    if (AmbientComponent)
    {
        AmbientComponent->FadeOut(2.0f, 0.0f);
        AmbientComponent = nullptr;
    }
}

void UTowerSoundManager::PlayBreathTransition(const FString& NewPhase)
{
    if (!BreathTransitionCue) return;

    // Different pitch for each phase
    float PhasePitch = 1.0f;
    if (NewPhase == TEXT("Inhale"))      PhasePitch = 0.8f;
    else if (NewPhase == TEXT("Hold"))   PhasePitch = 1.2f;
    else if (NewPhase == TEXT("Exhale")) PhasePitch = 1.0f;
    else if (NewPhase == TEXT("Pause"))  PhasePitch = 0.6f;

    float Vol = GetEffectiveVolume(ESoundCategory::BreathShift);
    UGameplayStatics::PlaySound2D(GetOwner(), BreathTransitionCue, Vol, PhasePitch);

    UE_LOG(LogTemp, Log, TEXT("Breath transition: %s (pitch=%.1f)"), *NewPhase, PhasePitch);
}

void UTowerSoundManager::SetMasterVolume(float Volume)
{
    MasterVol = FMath::Clamp(Volume, 0.0f, 1.0f);

    // Update ambient if playing
    if (AmbientComponent)
    {
        AmbientComponent->SetVolumeMultiplier(MusicVol * MasterVol);
    }
}

void UTowerSoundManager::SetSFXVolume(float Volume)
{
    SFXVol = FMath::Clamp(Volume, 0.0f, 1.0f);
}

void UTowerSoundManager::SetMusicVolume(float Volume)
{
    MusicVol = FMath::Clamp(Volume, 0.0f, 1.0f);

    // Update ambient if playing
    if (AmbientComponent)
    {
        AmbientComponent->SetVolumeMultiplier(MusicVol * MasterVol);
    }
}

float UTowerSoundManager::GetEffectiveVolume(ESoundCategory Category) const
{
    if (IsMusicCategory(Category))
    {
        return MusicVol * MasterVol;
    }
    return SFXVol * MasterVol;
}

bool UTowerSoundManager::IsMusicCategory(ESoundCategory Category) const
{
    return Category == ESoundCategory::BreathShift;
    // Floor ambience is handled separately via AmbientComponent
}
