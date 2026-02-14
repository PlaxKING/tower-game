#pragma once

#include "CoreMinimal.h"
#include "Components/ActorComponent.h"
#include "CelShadingComponent.generated.h"

class UPostProcessComponent;

/**
 * Anime-style cel-shading post-process component.
 *
 * Attach to the player camera or place in the level to apply:
 * - Toon shading with quantized light steps
 * - Edge detection outlines (Sobel-based)
 * - Specular highlight banding
 * - Optional Breath-of-Tower phase tinting
 *
 * For full anime look, pair with:
 * - Outline material (inverted hull or post-process)
 * - Niagara elemental VFX with cel-shaded sprites
 * - Custom shadow color (warm purple/blue instead of black)
 */
UCLASS(ClassGroup = (Rendering), meta = (BlueprintSpawnableComponent))
class TOWERGAME_API UCelShadingComponent : public UActorComponent
{
    GENERATED_BODY()

public:
    UCelShadingComponent();

    virtual void BeginPlay() override;

    // ============ Cel-Shading Parameters ============

    /** Number of light quantization steps (2 = classic toon, 4 = softer anime) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "CelShading", meta = (ClampMin = "2", ClampMax = "8"))
    int32 LightSteps = 3;

    /** Outline thickness in pixels */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "CelShading", meta = (ClampMin = "0.0", ClampMax = "4.0"))
    float OutlineThickness = 1.5f;

    /** Outline color */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "CelShading")
    FLinearColor OutlineColor = FLinearColor(0.05f, 0.02f, 0.08f, 1.0f);

    /** Shadow color tint (anime uses colored shadows, not pure black) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "CelShading")
    FLinearColor ShadowTint = FLinearColor(0.15f, 0.1f, 0.25f, 1.0f);

    /** Specular highlight sharpness */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "CelShading", meta = (ClampMin = "0.0", ClampMax = "1.0"))
    float SpecularSharpness = 0.7f;

    /** Enable Breath of Tower phase tinting */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "CelShading|Breath")
    bool bApplyBreathTint = true;

    /** Bloom intensity multiplier */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "CelShading", meta = (ClampMin = "0.0", ClampMax = "3.0"))
    float BloomIntensity = 0.8f;

    /** Saturation boost for anime-style vivid colors */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "CelShading", meta = (ClampMin = "0.5", ClampMax = "2.0"))
    float SaturationBoost = 1.2f;

    // ============ Runtime ============

    /** Apply current settings to the post-process volume */
    UFUNCTION(BlueprintCallable, Category = "CelShading")
    void ApplySettings();

    /** Set Breath phase tint (called from GameState) */
    UFUNCTION(BlueprintCallable, Category = "CelShading")
    void SetBreathPhaseTint(const FString& Phase);

private:
    UPROPERTY()
    UPostProcessComponent* PostProcessComp;

    void SetupPostProcess();
};
