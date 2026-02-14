#include "EchoGhost.h"
#include "Components/StaticMeshComponent.h"
#include "UObject/ConstructorHelpers.h"
#include "Materials/MaterialInstanceDynamic.h"
#include "Kismet/GameplayStatics.h"
#include "GameFramework/Character.h"

AEchoGhost::AEchoGhost()
{
    PrimaryActorTick.bCanEverTick = true;

    GhostMesh = CreateDefaultSubobject<UStaticMeshComponent>(TEXT("GhostMesh"));
    RootComponent = GhostMesh;

    // Use sphere as ghost shape
    static ConstructorHelpers::FObjectFinder<UStaticMesh> SphereMesh(
        TEXT("/Engine/BasicShapes/Sphere.Sphere"));
    if (SphereMesh.Succeeded())
    {
        GhostMesh->SetStaticMesh(SphereMesh.Object);
    }

    GhostMesh->SetCollisionProfileName(TEXT("OverlapAllDynamic"));
    GhostMesh->SetGenerateOverlapEvents(true);

    // Scale for humanoid silhouette
    SetActorScale3D(FVector(0.8f, 0.8f, 1.6f));
}

void AEchoGhost::BeginPlay()
{
    Super::BeginPlay();
    OriginalPosition = GetActorLocation();
    UpdateGhostMaterial();
}

void AEchoGhost::Tick(float DeltaTime)
{
    Super::Tick(DeltaTime);

    TimeAlive += DeltaTime;

    // Lifetime check
    if (TimeAlive >= LifetimeSeconds)
    {
        Destroy();
        return;
    }

    // Bobbing motion
    float BobOffset = FMath::Sin(TimeAlive * BobSpeed) * BobHeight;
    FVector NewPos = OriginalPosition;
    NewPos.Z += BobOffset;
    SetActorLocation(NewPos);

    // Slow rotation
    FRotator CurrentRot = GetActorRotation();
    CurrentRot.Yaw += 30.0f * DeltaTime;
    SetActorRotation(CurrentRot);

    // Pulsing opacity
    if (UMaterialInstanceDynamic* Mat = Cast<UMaterialInstanceDynamic>(GhostMesh->GetMaterial(0)))
    {
        float Pulse = 0.3f + 0.2f * FMath::Sin(TimeAlive * PulseSpeed);
        // Fade out in last 20% of lifetime
        float FadeProgress = TimeAlive / LifetimeSeconds;
        if (FadeProgress > 0.8f)
        {
            Pulse *= (1.0f - FadeProgress) / 0.2f;
        }
        Mat->SetScalarParameterValue(TEXT("Opacity"), Pulse);
    }

    // Apply echo type effects to nearby players
    ApplyEchoEffect(DeltaTime);
}

void AEchoGhost::InitFromData(const FString& PlayerName, EEchoType Type, FVector SpawnPosition)
{
    OriginalPlayerName = PlayerName;
    EchoType = Type;
    SetActorLocation(SpawnPosition);
    OriginalPosition = SpawnPosition;

    // Set scale based on echo type
    switch (EchoType)
    {
    case EEchoType::Aggressive:
        SetActorScale3D(FVector(1.0f, 1.0f, 1.8f));
        AggressiveDamage = 20.0f;
        break;
    case EEchoType::Helpful:
        SetActorScale3D(FVector(0.7f, 0.7f, 1.4f));
        HelpfulHealPerSecond = 8.0f;
        break;
    case EEchoType::Warning:
        PulseSpeed = 4.0f; // Fast pulsing
        break;
    default:
        break;
    }

    UpdateGhostMaterial();

    UE_LOG(LogTemp, Log, TEXT("Echo spawned: %s (%s) at (%.0f, %.0f, %.0f)"),
        *PlayerName,
        *UEnum::GetValueAsString(EchoType),
        SpawnPosition.X, SpawnPosition.Y, SpawnPosition.Z);
}

FLinearColor AEchoGhost::GetEchoColor() const
{
    switch (EchoType)
    {
    case EEchoType::Lingering:
        return FLinearColor(0.5f, 0.5f, 0.8f, 0.4f);   // Pale blue
    case EEchoType::Aggressive:
        return FLinearColor(0.9f, 0.2f, 0.15f, 0.5f);  // Red
    case EEchoType::Helpful:
        return FLinearColor(0.2f, 0.9f, 0.4f, 0.4f);   // Green
    case EEchoType::Warning:
        return FLinearColor(1.0f, 0.7f, 0.0f, 0.5f);   // Orange/yellow
    default:
        return FLinearColor(0.5f, 0.5f, 0.5f, 0.3f);
    }
}

void AEchoGhost::UpdateGhostMaterial()
{
    if (!GhostMesh) return;

    UMaterialInstanceDynamic* Mat = GhostMesh->CreateAndSetMaterialInstanceDynamic(0);
    if (Mat)
    {
        FLinearColor Color = GetEchoColor();
        Mat->SetVectorParameterValue(TEXT("BaseColor"), Color);
        Mat->SetScalarParameterValue(TEXT("Opacity"), Color.A);
        // Emissive glow
        Mat->SetVectorParameterValue(TEXT("EmissiveColor"), Color * 2.0f);
    }
}

void AEchoGhost::ApplyEchoEffect(float DeltaTime)
{
    // Only helpful and aggressive echoes interact with players
    if (EchoType != EEchoType::Aggressive && EchoType != EEchoType::Helpful)
        return;

    ACharacter* Player = UGameplayStatics::GetPlayerCharacter(this, 0);
    if (!Player) return;

    float Distance = FVector::Dist(GetActorLocation(), Player->GetActorLocation());
    if (Distance > EffectRadius) return;

    float Strength = 1.0f - (Distance / EffectRadius); // 1.0 at center, 0.0 at edge

    if (EchoType == EEchoType::Helpful)
    {
        // Heal nearby player
        // In production, call TowerPlayerCharacter::Heal()
        UE_LOG(LogTemp, Verbose, TEXT("Echo heal: +%.1f HP (strength: %.2f)"),
            HelpfulHealPerSecond * Strength * DeltaTime, Strength);
    }
    else if (EchoType == EEchoType::Aggressive)
    {
        // Damage nearby player (once per second approximation)
        static float DamageAccum = 0.0f;
        DamageAccum += DeltaTime;
        if (DamageAccum >= 1.0f)
        {
            DamageAccum = 0.0f;
            UE_LOG(LogTemp, Verbose, TEXT("Echo damage: %.1f (strength: %.2f)"),
                AggressiveDamage * Strength, Strength);
        }
    }
}
