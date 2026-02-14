#include "LootPickup.h"
#include "Components/StaticMeshComponent.h"
#include "Components/SphereComponent.h"
#include "Components/PointLightComponent.h"
#include "UObject/ConstructorHelpers.h"
#include "Materials/MaterialInstanceDynamic.h"
#include "Kismet/GameplayStatics.h"
#include "GameFramework/Character.h"
#include "Dom/JsonObject.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

ALootPickup::ALootPickup()
{
    PrimaryActorTick.bCanEverTick = true;

    // Collection sphere (trigger)
    CollectionSphere = CreateDefaultSubobject<USphereComponent>(TEXT("CollectionSphere"));
    CollectionSphere->SetSphereRadius(50.0f);
    CollectionSphere->SetCollisionProfileName(TEXT("OverlapAllDynamic"));
    CollectionSphere->SetGenerateOverlapEvents(true);
    RootComponent = CollectionSphere;

    // Visual mesh
    LootMesh = CreateDefaultSubobject<UStaticMeshComponent>(TEXT("LootMesh"));
    LootMesh->SetupAttachment(RootComponent);
    LootMesh->SetCollisionEnabled(ECollisionEnabled::NoCollision);

    static ConstructorHelpers::FObjectFinder<UStaticMesh> CubeMesh(
        TEXT("/Engine/BasicShapes/Cube.Cube"));
    if (CubeMesh.Succeeded())
    {
        LootMesh->SetStaticMesh(CubeMesh.Object);
    }
    LootMesh->SetWorldScale3D(FVector(0.3f));

    // Rarity glow
    RarityGlow = CreateDefaultSubobject<UPointLightComponent>(TEXT("RarityGlow"));
    RarityGlow->SetupAttachment(RootComponent);
    RarityGlow->SetIntensity(500.0f);
    RarityGlow->SetAttenuationRadius(100.0f);
    RarityGlow->SetCastShadows(false);
}

void ALootPickup::BeginPlay()
{
    Super::BeginPlay();

    SpawnPosition = GetActorLocation();

    // Setup overlap
    CollectionSphere->OnComponentBeginOverlap.AddDynamic(this, &ALootPickup::OnOverlapBegin);

    // Apply rarity visuals
    FLinearColor Color = GetRarityColor();

    UMaterialInstanceDynamic* Mat = LootMesh->CreateAndSetMaterialInstanceDynamic(0);
    if (Mat)
    {
        Mat->SetVectorParameterValue(TEXT("BaseColor"), Color);
        Mat->SetVectorParameterValue(TEXT("EmissiveColor"), Color * 3.0f);
    }

    LootMesh->SetWorldScale3D(FVector(GetRarityScale() * 0.3f));
    RarityGlow->SetLightColor(Color.ToFColor(true));
    RarityGlow->SetIntensity(GetRarityGlowIntensity());
}

void ALootPickup::Tick(float DeltaTime)
{
    Super::Tick(DeltaTime);

    if (bCollected) return;

    TimeAlive += DeltaTime;

    // Despawn check
    if (TimeAlive >= DespawnTime)
    {
        Destroy();
        return;
    }

    // Bobbing
    float BobOffset = FMath::Sin(TimeAlive * 3.0f) * BobHeight;
    FVector NewPos = SpawnPosition;
    NewPos.Z += BobOffset + 30.0f; // Float above ground

    // Rotation
    FRotator NewRot = GetActorRotation();
    NewRot.Yaw += RotateSpeed * DeltaTime;

    SetActorLocationAndRotation(NewPos, NewRot);

    // Magnet: pull toward nearby player
    ACharacter* Player = UGameplayStatics::GetPlayerCharacter(this, 0);
    if (Player)
    {
        float Dist = FVector::Dist(GetActorLocation(), Player->GetActorLocation());
        if (Dist < MagnetRadius && Dist > 10.0f)
        {
            FVector Dir = (Player->GetActorLocation() - GetActorLocation()).GetSafeNormal();
            float MagnetStrength = 1.0f - (Dist / MagnetRadius);
            SpawnPosition += Dir * MagnetSpeed * MagnetStrength * DeltaTime;
        }
    }

    // Fade out glow in last 20% of lifetime
    float LifeProgress = TimeAlive / DespawnTime;
    if (LifeProgress > 0.8f && RarityGlow)
    {
        float Fade = (1.0f - LifeProgress) / 0.2f;
        RarityGlow->SetIntensity(GetRarityGlowIntensity() * Fade);
    }
}

void ALootPickup::InitFromJson(const FString& LootJson)
{
    LootDataJson = LootJson;

    TSharedPtr<FJsonObject> Json;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(LootJson);
    if (FJsonSerializer::Deserialize(Reader, Json) && Json.IsValid())
    {
        ItemName = Json->GetStringField(TEXT("name"));

        FString RarityStr = Json->GetStringField(TEXT("rarity"));
        if (RarityStr == TEXT("Uncommon")) Rarity = ELootRarity::Uncommon;
        else if (RarityStr == TEXT("Rare")) Rarity = ELootRarity::Rare;
        else if (RarityStr == TEXT("Epic")) Rarity = ELootRarity::Epic;
        else if (RarityStr == TEXT("Legendary")) Rarity = ELootRarity::Legendary;
        else if (RarityStr == TEXT("Mythic")) Rarity = ELootRarity::Mythic;
        else Rarity = ELootRarity::Common;
    }
}

void ALootPickup::OnOverlapBegin(UPrimitiveComponent* OverlappedComp, AActor* OtherActor,
    UPrimitiveComponent* OtherComp, int32 OtherBodyIndex,
    bool bFromSweep, const FHitResult& SweepResult)
{
    if (bCollected) return;

    ACharacter* Player = Cast<ACharacter>(OtherActor);
    if (!Player) return;

    bCollected = true;

    UE_LOG(LogTemp, Log, TEXT("Loot collected: %s (%s)"),
        *ItemName, *UEnum::GetValueAsString(Rarity));

    OnLootCollected.Broadcast(OtherActor, LootDataJson);

    // Despawn after brief delay (for pickup VFX)
    SetLifeSpan(0.2f);
}

FLinearColor ALootPickup::GetRarityColor() const
{
    switch (Rarity)
    {
    case ELootRarity::Common:    return FLinearColor(0.8f, 0.8f, 0.8f, 1.0f);     // White
    case ELootRarity::Uncommon:  return FLinearColor(0.2f, 0.9f, 0.3f, 1.0f);     // Green
    case ELootRarity::Rare:      return FLinearColor(0.2f, 0.4f, 1.0f, 1.0f);     // Blue
    case ELootRarity::Epic:      return FLinearColor(0.7f, 0.2f, 0.9f, 1.0f);     // Purple
    case ELootRarity::Legendary: return FLinearColor(1.0f, 0.65f, 0.0f, 1.0f);    // Orange
    case ELootRarity::Mythic:    return FLinearColor(1.0f, 0.15f, 0.15f, 1.0f);   // Red
    default:                     return FLinearColor::White;
    }
}

float ALootPickup::GetRarityScale() const
{
    switch (Rarity)
    {
    case ELootRarity::Common:    return 1.0f;
    case ELootRarity::Uncommon:  return 1.1f;
    case ELootRarity::Rare:      return 1.2f;
    case ELootRarity::Epic:      return 1.3f;
    case ELootRarity::Legendary: return 1.5f;
    case ELootRarity::Mythic:    return 1.7f;
    default:                     return 1.0f;
    }
}

float ALootPickup::GetRarityGlowIntensity() const
{
    switch (Rarity)
    {
    case ELootRarity::Common:    return 200.0f;
    case ELootRarity::Uncommon:  return 400.0f;
    case ELootRarity::Rare:      return 600.0f;
    case ELootRarity::Epic:      return 1000.0f;
    case ELootRarity::Legendary: return 2000.0f;
    case ELootRarity::Mythic:    return 3000.0f;
    default:                     return 200.0f;
    }
}
