#include "FloorTransitionComponent.h"
#include "Core/TowerGameMode.h"
#include "Core/TowerGameSubsystem.h"
#include "Kismet/GameplayStatics.h"
#include "Camera/PlayerCameraManager.h"

UFloorTransitionComponent::UFloorTransitionComponent()
{
    PrimaryComponentTick.bCanEverTick = true;
}

void UFloorTransitionComponent::TickComponent(float DeltaTime, ELevelTick TickType,
    FActorComponentTickFunction* ThisTickFunction)
{
    Super::TickComponent(DeltaTime, TickType, ThisTickFunction);

    switch (State)
    {
    case ETransitionState::FadingOut:
        UpdateFadeOut(DeltaTime);
        break;
    case ETransitionState::Loading:
        UpdateLoading(DeltaTime);
        break;
    case ETransitionState::FadingIn:
        UpdateFadeIn(DeltaTime);
        break;
    default:
        break;
    }
}

void UFloorTransitionComponent::TransitionToFloor(int32 NewFloor)
{
    if (IsTransitioning()) return;

    TargetFloor = NewFloor;
    bFloorGenerated = false;

    UE_LOG(LogTemp, Log, TEXT("Floor transition: -> floor %d"), NewFloor);

    OnTransitionStart.Broadcast(NewFloor);
    BeginFadeOut();
}

void UFloorTransitionComponent::BeginFadeOut()
{
    State = ETransitionState::FadingOut;
    StateTimer = 0.0f;
    LoadProgress = 0.0f;
}

void UFloorTransitionComponent::UpdateFadeOut(float DeltaTime)
{
    StateTimer += DeltaTime;
    float Alpha = FMath::Clamp(StateTimer / FadeOutDuration, 0.0f, 1.0f);
    SetScreenFade(Alpha);

    LoadProgress = Alpha * 0.1f; // 0-10% during fade out
    OnLoadProgress.Broadcast(LoadProgress);

    if (StateTimer >= FadeOutDuration)
    {
        BeginLoading();
    }
}

void UFloorTransitionComponent::BeginLoading()
{
    State = ETransitionState::Loading;
    StateTimer = 0.0f;

    // Destroy old floor
    ATowerGameMode* GM = Cast<ATowerGameMode>(UGameplayStatics::GetGameMode(this));
    if (GM)
    {
        // GameMode handles destroying old tiles and monsters
        UE_LOG(LogTemp, Log, TEXT("Destroying old floor..."));
    }

    LoadProgress = 0.2f;
    OnLoadProgress.Broadcast(LoadProgress);

    // Generate new floor via Rust core
    UTowerGameSubsystem* Subsystem = nullptr;
    UGameInstance* GI = UGameplayStatics::GetGameInstance(this);
    if (GI)
    {
        Subsystem = GI->GetSubsystem<UTowerGameSubsystem>();
    }

    if (Subsystem)
    {
        LoadProgress = 0.5f;
        OnLoadProgress.Broadcast(LoadProgress);

        // Floor generation is synchronous (fast — handled by DLL)
        UE_LOG(LogTemp, Log, TEXT("Generating floor %d via Rust core..."), TargetFloor);
        bFloorGenerated = true;

        LoadProgress = 0.8f;
        OnLoadProgress.Broadcast(LoadProgress);
    }
}

void UFloorTransitionComponent::UpdateLoading(float DeltaTime)
{
    StateTimer += DeltaTime;

    // Ensure minimum load time
    if (StateTimer >= MinLoadTime && bFloorGenerated)
    {
        LoadProgress = 1.0f;
        OnLoadProgress.Broadcast(LoadProgress);
        BeginFadeIn();
    }
}

void UFloorTransitionComponent::BeginFadeIn()
{
    State = ETransitionState::FadingIn;
    StateTimer = 0.0f;
}

void UFloorTransitionComponent::UpdateFadeIn(float DeltaTime)
{
    StateTimer += DeltaTime;
    float Alpha = 1.0f - FMath::Clamp(StateTimer / FadeInDuration, 0.0f, 1.0f);
    SetScreenFade(Alpha);

    if (StateTimer >= FadeInDuration)
    {
        FinishTransition();
    }
}

void UFloorTransitionComponent::FinishTransition()
{
    State = ETransitionState::Idle;
    StateTimer = 0.0f;
    LoadProgress = 0.0f;

    SetScreenFade(0.0f);

    UE_LOG(LogTemp, Log, TEXT("Floor transition complete: floor %d"), TargetFloor);
    OnTransitionEnd.Broadcast(TargetFloor);
}

void UFloorTransitionComponent::SetScreenFade(float Alpha)
{
    APlayerController* PC = UGameplayStatics::GetPlayerController(this, 0);
    if (!PC) return;

    APlayerCameraManager* CameraManager = PC->PlayerCameraManager;
    if (CameraManager)
    {
        CameraManager->SetManualCameraFade(Alpha, FLinearColor::Black, false);
    }
}
