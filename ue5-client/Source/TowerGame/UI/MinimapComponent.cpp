#include "MinimapComponent.h"
#include "Components/SceneCaptureComponent2D.h"
#include "Engine/TextureRenderTarget2D.h"
#include "GameFramework/Character.h"

UMinimapComponent::UMinimapComponent()
{
    PrimaryComponentTick.bCanEverTick = true;
}

void UMinimapComponent::BeginPlay()
{
    Super::BeginPlay();
    SetupCapture();
}

void UMinimapComponent::SetupCapture()
{
    AActor* Owner = GetOwner();
    if (!Owner) return;

    // Create render target
    RenderTarget = NewObject<UTextureRenderTarget2D>(this);
    RenderTarget->InitAutoFormat(TextureSize, TextureSize);
    RenderTarget->UpdateResourceImmediate(true);

    // Create scene capture component
    CaptureComponent = NewObject<USceneCaptureComponent2D>(Owner);
    CaptureComponent->RegisterComponent();
    CaptureComponent->AttachToComponent(Owner->GetRootComponent(), FAttachmentTransformRules::KeepRelativeTransform);

    // Configure for top-down orthographic view
    CaptureComponent->ProjectionType = ECameraProjectionMode::Orthographic;
    CaptureComponent->OrthoWidth = OrthoWidth * ZoomLevel;
    CaptureComponent->TextureTarget = RenderTarget;
    CaptureComponent->bCaptureEveryFrame = false;
    CaptureComponent->bCaptureOnMovement = false;
    CaptureComponent->CaptureSource = ESceneCaptureSource::SCS_FinalColorLDR;

    // Look straight down
    CaptureComponent->SetRelativeLocation(FVector(0, 0, CaptureHeight));
    CaptureComponent->SetRelativeRotation(FRotator(-90.0f, 0.0f, 0.0f));

    // Hide certain show flags for clean minimap
    CaptureComponent->ShowFlags.SetFog(false);
    CaptureComponent->ShowFlags.SetAtmosphere(false);
    CaptureComponent->ShowFlags.SetBloom(false);
    CaptureComponent->ShowFlags.SetMotionBlur(false);

    Owner->AddInstanceComponent(CaptureComponent);

    UE_LOG(LogTemp, Log, TEXT("Minimap initialized: %dx%d, ortho %.0f, height %.0f"),
        TextureSize, TextureSize, OrthoWidth, CaptureHeight);
}

void UMinimapComponent::TickComponent(float DeltaTime, ELevelTick TickType, FActorComponentTickFunction* ThisTickFunction)
{
    Super::TickComponent(DeltaTime, TickType, ThisTickFunction);

    CaptureTimer += DeltaTime;
    float CaptureInterval = 1.0f / FMath::Max(CaptureRate, 1.0f);

    if (CaptureTimer >= CaptureInterval)
    {
        CaptureTimer = 0.0f;
        UpdateCapturePosition();

        if (CaptureComponent)
        {
            CaptureComponent->CaptureScene();
        }
    }
}

void UMinimapComponent::UpdateCapturePosition()
{
    if (!CaptureComponent) return;

    AActor* Owner = GetOwner();
    if (!Owner) return;

    // Update orthographic width for zoom
    CaptureComponent->OrthoWidth = OrthoWidth * ZoomLevel;

    // Position above player
    FVector OwnerLoc = Owner->GetActorLocation();
    CaptureComponent->SetWorldLocation(FVector(OwnerLoc.X, OwnerLoc.Y, OwnerLoc.Z + CaptureHeight));

    // Rotation
    if (bRotateWithPlayer)
    {
        float PlayerYaw = Owner->GetActorRotation().Yaw;
        CaptureComponent->SetWorldRotation(FRotator(-90.0f, PlayerYaw, 0.0f));
    }
    else
    {
        CaptureComponent->SetWorldRotation(FRotator(-90.0f, 0.0f, 0.0f));
    }
}

void UMinimapComponent::ZoomIn()
{
    ZoomLevel = FMath::Max(0.25f, ZoomLevel * 0.75f);
}

void UMinimapComponent::ZoomOut()
{
    ZoomLevel = FMath::Min(4.0f, ZoomLevel * 1.33f);
}

void UMinimapComponent::ToggleRotation()
{
    bRotateWithPlayer = !bRotateWithPlayer;
}
