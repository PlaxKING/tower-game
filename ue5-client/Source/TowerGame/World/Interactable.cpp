#include "Interactable.h"
#include "Components/StaticMeshComponent.h"
#include "Components/SphereComponent.h"
#include "UObject/ConstructorHelpers.h"
#include "Materials/MaterialInstanceDynamic.h"
#include "GameFramework/Character.h"
#include "Kismet/GameplayStatics.h"

// ============ Base Interactable ============

AInteractable::AInteractable()
{
    PrimaryActorTick.bCanEverTick = true;

    // Interaction zone
    InteractionZone = CreateDefaultSubobject<USphereComponent>(TEXT("InteractionZone"));
    InteractionZone->SetSphereRadius(InteractionRadius);
    InteractionZone->SetCollisionProfileName(TEXT("OverlapAllDynamic"));
    InteractionZone->SetGenerateOverlapEvents(true);
    RootComponent = InteractionZone;

    // Visual mesh
    BaseMesh = CreateDefaultSubobject<UStaticMeshComponent>(TEXT("BaseMesh"));
    BaseMesh->SetupAttachment(RootComponent);
    BaseMesh->SetCollisionProfileName(TEXT("BlockAllDynamic"));
}

void AInteractable::BeginPlay()
{
    Super::BeginPlay();

    InteractionZone->SetSphereRadius(InteractionRadius);
    InteractionZone->OnComponentBeginOverlap.AddDynamic(this, &AInteractable::OnOverlapBegin);
    InteractionZone->OnComponentEndOverlap.AddDynamic(this, &AInteractable::OnOverlapEnd);
}

void AInteractable::Tick(float DeltaTime)
{
    Super::Tick(DeltaTime);

    if (CooldownTimer > 0.0f)
    {
        CooldownTimer -= DeltaTime;
    }
}

bool AInteractable::TryInteract(AActor* Interactor)
{
    if (!bPlayerInRange) return false;
    if (CooldownTimer > 0.0f) return false;
    if (bSingleUse && bUsed) return false;

    CooldownTimer = CooldownSeconds;
    bUsed = true;

    ExecuteInteraction(Interactor);
    OnInteracted.Broadcast(Interactor, this);

    return true;
}

void AInteractable::ExecuteInteraction(AActor* Interactor)
{
    UE_LOG(LogTemp, Log, TEXT("Interactable '%s' used by %s"),
        *GetName(), *Interactor->GetName());
}

void AInteractable::OnOverlapBegin(UPrimitiveComponent* OverlappedComp, AActor* OtherActor,
    UPrimitiveComponent* OtherComp, int32 OtherBodyIndex,
    bool bFromSweep, const FHitResult& SweepResult)
{
    if (Cast<ACharacter>(OtherActor))
    {
        bPlayerInRange = true;

        // Visual highlight
        if (UMaterialInstanceDynamic* Mat = Cast<UMaterialInstanceDynamic>(BaseMesh->GetMaterial(0)))
        {
            Mat->SetScalarParameterValue(TEXT("Highlight"), 1.0f);
        }
    }
}

void AInteractable::OnOverlapEnd(UPrimitiveComponent* OverlappedComp, AActor* OtherActor,
    UPrimitiveComponent* OtherComp, int32 OtherBodyIndex)
{
    if (Cast<ACharacter>(OtherActor))
    {
        bPlayerInRange = false;

        if (UMaterialInstanceDynamic* Mat = Cast<UMaterialInstanceDynamic>(BaseMesh->GetMaterial(0)))
        {
            Mat->SetScalarParameterValue(TEXT("Highlight"), 0.0f);
        }
    }
}

// ============ Chest ============

ATowerChest::ATowerChest()
{
    InteractionPrompt = TEXT("Open Chest");
    bSingleUse = true;

    static ConstructorHelpers::FObjectFinder<UStaticMesh> CubeMesh(
        TEXT("/Engine/BasicShapes/Cube.Cube"));
    if (CubeMesh.Succeeded())
    {
        BaseMesh->SetStaticMesh(CubeMesh.Object);
    }
    BaseMesh->SetWorldScale3D(FVector(0.8f, 0.6f, 0.5f));
}

void ATowerChest::ExecuteInteraction(AActor* Interactor)
{
    if (bOpened) return;
    bOpened = true;

    UE_LOG(LogTemp, Log, TEXT("Chest opened on floor %d by %s"),
        FloorLevel, *Interactor->GetName());

    // Generate loot via Rust core (done through GameMode)
    // The GameMode listens to OnInteracted and calls GenerateLoot

    // Visual: open chest lid
    BaseMesh->SetWorldScale3D(FVector(0.8f, 0.6f, 0.2f));
}

// ============ Shrine ============

ATowerShrine::ATowerShrine()
{
    InteractionPrompt = TEXT("Pray at Shrine");
    bSingleUse = false;
    CooldownSeconds = 30.0f; // Once per 30 seconds

    static ConstructorHelpers::FObjectFinder<UStaticMesh> CylinderMesh(
        TEXT("/Engine/BasicShapes/Cylinder.Cylinder"));
    if (CylinderMesh.Succeeded())
    {
        BaseMesh->SetStaticMesh(CylinderMesh.Object);
    }
    BaseMesh->SetWorldScale3D(FVector(0.5f, 0.5f, 1.5f));
}

void ATowerShrine::ExecuteInteraction(AActor* Interactor)
{
    UE_LOG(LogTemp, Log, TEXT("Shrine '%s' prayed at by %s (+%d standing)"),
        *FactionName, *Interactor->GetName(), StandingReward);

    // Faction standing update happens through Nakama RPC
    // The GameMode listens and calls NakamaSubsystem->UpdateFaction()
}

// ============ Stairs ============

ATowerStairs::ATowerStairs()
{
    InteractionPrompt = TEXT("Ascend");
    bSingleUse = false;
    bRequiresFloorClear = true;

    static ConstructorHelpers::FObjectFinder<UStaticMesh> ConeMesh(
        TEXT("/Engine/BasicShapes/Cone.Cone"));
    if (ConeMesh.Succeeded())
    {
        BaseMesh->SetStaticMesh(ConeMesh.Object);
    }
    BaseMesh->SetWorldScale3D(FVector(1.0f, 1.0f, 1.5f));
}

void ATowerStairs::ExecuteInteraction(AActor* Interactor)
{
    FString Direction = bGoingUp ? TEXT("up") : TEXT("down");
    UE_LOG(LogTemp, Log, TEXT("Stairs %s used by %s"), *Direction, *Interactor->GetName());

    // Floor transition handled by GameMode::GoToNextFloor/GoToPreviousFloor
}
