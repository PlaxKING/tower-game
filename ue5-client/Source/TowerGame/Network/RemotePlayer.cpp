#include "RemotePlayer.h"
#include "Components/StaticMeshComponent.h"
#include "Components/CapsuleComponent.h"
#include "UObject/ConstructorHelpers.h"
#include "GameFramework/CharacterMovementComponent.h"

ARemotePlayer::ARemotePlayer()
{
    PrimaryActorTick.bCanEverTick = true;

    // Disable local movement — position is driven by network
    if (UCharacterMovementComponent* Movement = GetCharacterMovement())
    {
        Movement->GravityScale = 0.0f;
        Movement->MaxWalkSpeed = 0.0f;
    }

    // Nameplate indicator above head
    NameplateMesh = CreateDefaultSubobject<UStaticMeshComponent>(TEXT("NameplateMesh"));
    NameplateMesh->SetupAttachment(RootComponent);
    NameplateMesh->SetRelativeLocation(FVector(0.0f, 0.0f, 120.0f));
    NameplateMesh->SetCollisionEnabled(ECollisionEnabled::NoCollision);

    static ConstructorHelpers::FObjectFinder<UStaticMesh> PlaneMesh(
        TEXT("/Engine/BasicShapes/Plane.Plane"));
    if (PlaneMesh.Succeeded())
    {
        NameplateMesh->SetStaticMesh(PlaneMesh.Object);
    }
    NameplateMesh->SetWorldScale3D(FVector(0.3f, 0.1f, 1.0f));

    // No collision with local player
    GetCapsuleComponent()->SetCollisionResponseToChannel(ECC_Pawn, ECR_Ignore);
}

void ARemotePlayer::Tick(float DeltaTime)
{
    Super::Tick(DeltaTime);

    if (!bHasReceivedUpdate || bIsDead) return;

    TimeSinceLastUpdate += DeltaTime;

    // Interpolate position
    float Distance = FVector::Dist(GetActorLocation(), TargetPosition);

    if (Distance > TeleportThreshold)
    {
        // Too far — teleport
        SetActorLocation(TargetPosition);
        SetActorRotation(TargetRotation);
    }
    else
    {
        // Smooth interpolation
        FVector NewPos = FMath::VInterpTo(
            GetActorLocation(), TargetPosition, DeltaTime, InterpSpeed);
        FRotator NewRot = FMath::RInterpTo(
            GetActorRotation(), TargetRotation, DeltaTime, RotInterpSpeed);

        SetActorLocationAndRotation(NewPos, NewRot);
    }

    // Calculate speed for animation
    FVector Velocity = (GetActorLocation() - PreviousPosition) / FMath::Max(DeltaTime, 0.001f);
    Speed = Velocity.Size2D();
    PreviousPosition = GetActorLocation();

    // Reset attack state after brief display
    if (bIsAttacking && TimeSinceLastUpdate > 0.5f)
    {
        bIsAttacking = false;
        CurrentComboStep = 0;
    }
}

void ARemotePlayer::ApplyPositionUpdate(FVector NewPosition, FRotator NewRotation)
{
    PreviousPosition = GetActorLocation();
    TargetPosition = NewPosition;
    TargetRotation = NewRotation;
    TimeSinceLastUpdate = 0.0f;
    bHasReceivedUpdate = true;
}

void ARemotePlayer::PlayAttackAnimation(int32 ComboStep, int32 WeaponType)
{
    bIsAttacking = true;
    CurrentComboStep = ComboStep;
    TimeSinceLastUpdate = 0.0f;

    UE_LOG(LogTemp, Verbose, TEXT("Remote player %s attacks: combo=%d weapon=%d"),
        *DisplayName, ComboStep, WeaponType);
}

void ARemotePlayer::PlayDodgeAnimation()
{
    UE_LOG(LogTemp, Verbose, TEXT("Remote player %s dodges"), *DisplayName);
}

void ARemotePlayer::ShowDeath()
{
    bIsDead = true;
    bIsAttacking = false;

    UE_LOG(LogTemp, Log, TEXT("Remote player %s died"), *DisplayName);

    // Visual: collapse mesh
    if (GetMesh())
    {
        GetMesh()->SetSimulatePhysics(true);
    }
}

void ARemotePlayer::Respawn(FVector SpawnLocation)
{
    bIsDead = false;
    bIsAttacking = false;
    CurrentComboStep = 0;

    SetActorLocation(SpawnLocation);
    TargetPosition = SpawnLocation;

    if (GetMesh())
    {
        GetMesh()->SetSimulatePhysics(false);
    }

    UE_LOG(LogTemp, Log, TEXT("Remote player %s respawned"), *DisplayName);
}
