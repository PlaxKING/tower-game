#include "ElementalVFXComponent.h"
#include "NiagaraFunctionLibrary.h"
#include "NiagaraComponent.h"

UElementalVFXComponent::UElementalVFXComponent()
{
    PrimaryComponentTick.bCanEverTick = false;
}

void UElementalVFXComponent::BeginPlay()
{
    Super::BeginPlay();

    // Auto-start aura if element is set
    if (Element != EElementType::None && AuraSystem)
    {
        StartAura();
    }
}

void UElementalVFXComponent::EndPlay(const EEndPlayReason::Type EndPlayReason)
{
    StopAura();
    Super::EndPlay(EndPlayReason);
}

void UElementalVFXComponent::SetElement(EElementType NewElement)
{
    Element = NewElement;

    // Update active aura if running
    if (AuraComponent && AuraComponent->IsActive())
    {
        ApplyElementParameters(AuraComponent, 1.0f);
    }
}

void UElementalVFXComponent::TriggerVFX(EVFXTrigger Trigger, FVector Location, float Scale)
{
    UNiagaraSystem* System = nullptr;

    switch (Trigger)
    {
    case EVFXTrigger::OnHit:
        System = HitSystem;
        break;
    case EVFXTrigger::OnDeath:
        System = DeathSystem;
        Scale *= 2.0f; // Death explosions are bigger
        break;
    case EVFXTrigger::OnDodge:
        System = DodgeSystem;
        break;
    case EVFXTrigger::OnComboFinish:
        System = ComboFinishSystem;
        Scale *= 1.5f;
        break;
    case EVFXTrigger::OnBreathShift:
        System = AuraSystem; // Reuse aura system with a burst
        Scale *= 3.0f;
        break;
    case EVFXTrigger::Ambient:
        // Ambient is handled by StartAura/StopAura
        return;
    }

    if (System && GetOwner())
    {
        SpawnElementalVFX(System, Location, Scale, true);
    }
}

void UElementalVFXComponent::StartAura()
{
    if (!AuraSystem || !GetOwner()) return;
    if (AuraComponent && AuraComponent->IsActive()) return;

    AuraComponent = SpawnElementalVFX(AuraSystem, GetOwner()->GetActorLocation(), 1.0f, false);
    if (AuraComponent)
    {
        AuraComponent->AttachToComponent(
            GetOwner()->GetRootComponent(),
            FAttachmentTransformRules::SnapToTargetNotIncludingScale
        );
    }
}

void UElementalVFXComponent::StopAura()
{
    if (AuraComponent)
    {
        AuraComponent->DeactivateImmediate();
        AuraComponent->DestroyComponent();
        AuraComponent = nullptr;
    }
}

FLinearColor UElementalVFXComponent::GetElementColor(EElementType InElement)
{
    switch (InElement)
    {
    case EElementType::Fire:       return FLinearColor(1.0f, 0.3f, 0.05f);   // Orange-red
    case EElementType::Water:      return FLinearColor(0.1f, 0.5f, 1.0f);    // Blue
    case EElementType::Earth:      return FLinearColor(0.6f, 0.4f, 0.15f);   // Brown
    case EElementType::Wind:       return FLinearColor(0.7f, 1.0f, 0.8f);    // Pale green
    case EElementType::Void:       return FLinearColor(0.3f, 0.0f, 0.5f);    // Deep purple
    case EElementType::Corruption: return FLinearColor(0.1f, 0.0f, 0.1f);    // Dark magenta
    default:                       return FLinearColor(0.8f, 0.8f, 0.8f);    // Neutral gray
    }
}

FLinearColor UElementalVFXComponent::GetElementSecondaryColor(EElementType InElement)
{
    switch (InElement)
    {
    case EElementType::Fire:       return FLinearColor(1.0f, 0.9f, 0.2f);    // Yellow
    case EElementType::Water:      return FLinearColor(0.6f, 0.9f, 1.0f);    // Cyan
    case EElementType::Earth:      return FLinearColor(0.3f, 0.7f, 0.2f);    // Green
    case EElementType::Wind:       return FLinearColor(1.0f, 1.0f, 1.0f);    // White
    case EElementType::Void:       return FLinearColor(0.1f, 0.0f, 0.3f);    // Dark blue
    case EElementType::Corruption: return FLinearColor(0.5f, 0.0f, 0.0f);    // Dark red
    default:                       return FLinearColor(0.5f, 0.5f, 0.5f);    // Gray
    }
}

UNiagaraComponent* UElementalVFXComponent::SpawnElementalVFX(
    UNiagaraSystem* System, FVector Location, float Scale, bool bAutoDestroy)
{
    if (!System || !GetOwner()) return nullptr;

    UNiagaraComponent* NiagaraComp = UNiagaraFunctionLibrary::SpawnSystemAtLocation(
        GetOwner(),
        System,
        Location,
        FRotator::ZeroRotator,
        FVector(ParticleScale * Scale),
        bAutoDestroy
    );

    if (NiagaraComp)
    {
        ApplyElementParameters(NiagaraComp, Scale);
    }

    return NiagaraComp;
}

void UElementalVFXComponent::ApplyElementParameters(UNiagaraComponent* NiagaraComp, float Scale)
{
    if (!NiagaraComp) return;

    FLinearColor PrimaryColor = GetElementColor(Element);
    FLinearColor SecColor = GetElementSecondaryColor(Element);

    // Blend secondary element if present
    if (SecondaryElement != EElementType::None)
    {
        FLinearColor SecElementColor = GetElementColor(SecondaryElement);
        PrimaryColor = FLinearColor::LerpUsingHSV(PrimaryColor, SecElementColor, 0.3f);
    }

    // Set Niagara user parameters
    NiagaraComp->SetColorParameter(FName("ElementColor"), PrimaryColor);
    NiagaraComp->SetColorParameter(FName("SecondaryColor"), SecColor);
    NiagaraComp->SetFloatParameter(FName("Intensity"), Intensity * Scale);
    NiagaraComp->SetFloatParameter(FName("ParticleScale"), ParticleScale * Scale);
    NiagaraComp->SetFloatParameter(FName("SpawnRate"), 20.0f * Intensity);
}
