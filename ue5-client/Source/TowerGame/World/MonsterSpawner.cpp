#include "MonsterSpawner.h"
#include "Components/StaticMeshComponent.h"
#include "Materials/MaterialInstanceDynamic.h"
#include "UObject/ConstructorHelpers.h"
#include "Dom/JsonObject.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

// ============ AMonsterSpawner ============

AMonsterSpawner::AMonsterSpawner()
{
    PrimaryActorTick.bCanEverTick = false;
}

TArray<AActor*> AMonsterSpawner::SpawnMonstersFromJson(
    UWorld* World,
    const FString& MonstersJson,
    const TArray<FVector>& SpawnPoints,
    int32 FloorLevel)
{
    TArray<AActor*> SpawnedMonsters;

    if (!World || MonstersJson.IsEmpty())
    {
        return SpawnedMonsters;
    }

    // Parse the JSON array
    TArray<TSharedPtr<FJsonValue>> MonstersArray;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(MonstersJson);
    if (!FJsonSerializer::Deserialize(Reader, MonstersArray))
    {
        UE_LOG(LogTemp, Error, TEXT("Failed to parse monsters JSON"));
        return SpawnedMonsters;
    }

    UE_LOG(LogTemp, Log, TEXT("Spawning %d monsters for floor %d"), MonstersArray.Num(), FloorLevel);

    for (int32 i = 0; i < MonstersArray.Num(); i++)
    {
        TSharedPtr<FJsonObject> MonsterObj = MonstersArray[i]->AsObject();
        if (!MonsterObj.IsValid()) continue;

        // Extract data from Rust MonsterInfo JSON (flat structure)
        FString Name = MonsterObj->GetStringField(TEXT("name"));
        FString Size = MonsterObj->GetStringField(TEXT("size"));
        FString Element = MonsterObj->GetStringField(TEXT("element"));

        // Stats are flat fields in MonsterInfo from Rust
        float Hp = MonsterObj->GetNumberField(TEXT("max_hp"));
        float Atk = MonsterObj->GetNumberField(TEXT("damage"));
        float Def = MonsterObj->GetNumberField(TEXT("armor"));
        float Spd = MonsterObj->GetNumberField(TEXT("speed"));

        // Calculate spawn location
        FVector SpawnLoc;
        if (SpawnPoints.Num() > 0)
        {
            // Distribute among spawn points with some randomization
            int32 PointIdx = i % SpawnPoints.Num();
            SpawnLoc = SpawnPoints[PointIdx];
            // Add random offset within room (+-150 UU)
            SpawnLoc.X += FMath::FRandRange(-150.0f, 150.0f);
            SpawnLoc.Y += FMath::FRandRange(-150.0f, 150.0f);
        }
        else
        {
            // Fallback: spread in a circle
            float Angle = (float)i / (float)FMath::Max(1, MonstersArray.Num()) * 2.0f * PI;
            float Radius = 500.0f + FloorLevel * 50.0f;
            SpawnLoc = FVector(FMath::Cos(Angle) * Radius, FMath::Sin(Angle) * Radius, 50.0f);
        }

        FActorSpawnParameters SpawnParams;
        SpawnParams.SpawnCollisionHandlingOverride = ESpawnActorCollisionHandlingMethod::AdjustIfPossibleButAlwaysSpawn;

        ATowerMonster* Monster = World->SpawnActor<ATowerMonster>(
            ATowerMonster::StaticClass(), SpawnLoc, FRotator::ZeroRotator, SpawnParams);

        if (Monster)
        {
            Monster->InitFromData(Name, Size, Element, Hp, Atk, Def, Spd, FloorLevel);
            SpawnedMonsters.Add(Monster);
            UE_LOG(LogTemp, Verbose, TEXT("  Spawned: %s (HP=%.0f ATK=%.0f)"), *Name, Hp, Atk);
        }
    }

    return SpawnedMonsters;
}

// ============ ATowerMonster ============

ATowerMonster::ATowerMonster()
{
    PrimaryActorTick.bCanEverTick = true;

    MeshComponent = CreateDefaultSubobject<UStaticMeshComponent>(TEXT("MonsterMesh"));
    RootComponent = MeshComponent;

    static ConstructorHelpers::FObjectFinder<UStaticMesh> CubeMesh(
        TEXT("/Engine/BasicShapes/Cube.Cube"));
    if (CubeMesh.Succeeded())
    {
        MeshComponent->SetStaticMesh(CubeMesh.Object);
    }

    MeshComponent->SetCollisionEnabled(ECollisionEnabled::QueryAndPhysics);
    MeshComponent->SetCollisionResponseToAllChannels(ECR_Block);
    MeshComponent->SetCollisionObjectType(ECC_Pawn);
}

void ATowerMonster::InitFromData(
    const FString& InName,
    const FString& InSize,
    const FString& InElement,
    float InHp,
    float InAttack,
    float InDefense,
    float InSpeed,
    int32 InFloorLevel)
{
    MonsterName = InName;
    Size = InSize;
    Element = InElement;
    MaxHp = InHp;
    CurrentHp = InHp;
    Attack = InAttack;
    Defense = InDefense;
    Speed = InSpeed;
    FloorLevel = InFloorLevel;

    // Scale based on size
    float Scale = GetSizeScale(InSize);
    MeshComponent->SetWorldScale3D(FVector(Scale));

    // Raise to sit on ground
    FVector Loc = GetActorLocation();
    Loc.Z = Scale * 50.0f; // Half height
    SetActorLocation(Loc);

    // Color based on element
    UMaterialInterface* BaseMat = MeshComponent->GetMaterial(0);
    if (BaseMat)
    {
        UMaterialInstanceDynamic* DynMat = UMaterialInstanceDynamic::Create(BaseMat, this);
        FLinearColor Color = GetElementColor(InElement);
        DynMat->SetVectorParameterValue(TEXT("BaseColor"), Color);
        MeshComponent->SetMaterial(0, DynMat);
    }

#if WITH_EDITOR
    SetActorLabel(FString::Printf(TEXT("Monster_%s"), *InName));
#endif

    UE_LOG(LogTemp, Log, TEXT("Monster initialized: %s [%s/%s] HP=%.0f ATK=%.0f DEF=%.0f SPD=%.0f"),
        *MonsterName, *Size, *Element, MaxHp, Attack, Defense, Speed);
}

void ATowerMonster::TakeDamageFromPlayer(float DamageAmount)
{
    if (!bIsAlive) return;

    float ActualDamage = FMath::Max(0.0f, DamageAmount - Defense * 0.3f);
    CurrentHp -= ActualDamage;

    UE_LOG(LogTemp, Log, TEXT("%s takes %.1f damage (%.1f mitigated). HP: %.0f/%.0f"),
        *MonsterName, ActualDamage, DamageAmount - ActualDamage, CurrentHp, MaxHp);

    if (CurrentHp <= 0.0f)
    {
        CurrentHp = 0.0f;
        bIsAlive = false;
        OnMonsterDeath.Broadcast(this);
        UE_LOG(LogTemp, Log, TEXT("%s defeated!"), *MonsterName);
    }
}

FLinearColor ATowerMonster::GetElementColor(const FString& InElement)
{
    if (InElement == TEXT("Fire"))       return FLinearColor(1.0f, 0.3f, 0.1f);
    if (InElement == TEXT("Ice"))        return FLinearColor(0.3f, 0.7f, 1.0f);
    if (InElement == TEXT("Lightning"))  return FLinearColor(1.0f, 1.0f, 0.2f);
    if (InElement == TEXT("Poison"))     return FLinearColor(0.3f, 0.9f, 0.2f);
    if (InElement == TEXT("Void"))       return FLinearColor(0.4f, 0.1f, 0.6f);
    if (InElement == TEXT("Stone"))      return FLinearColor(0.5f, 0.45f, 0.4f);
    if (InElement == TEXT("Wind"))       return FLinearColor(0.7f, 0.9f, 0.7f);
    if (InElement == TEXT("Arcane"))     return FLinearColor(0.6f, 0.3f, 0.9f);
    // Neutral
    return FLinearColor(0.6f, 0.6f, 0.6f);
}

float ATowerMonster::GetSizeScale(const FString& InSize)
{
    if (InSize == TEXT("Tiny"))      return 0.5f;
    if (InSize == TEXT("Small"))     return 0.8f;
    if (InSize == TEXT("Medium"))    return 1.2f;
    if (InSize == TEXT("Large"))     return 1.8f;
    if (InSize == TEXT("Huge"))      return 2.5f;
    if (InSize == TEXT("Colossal"))  return 3.5f;
    return 1.0f;
}
