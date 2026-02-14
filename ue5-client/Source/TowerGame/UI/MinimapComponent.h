#pragma once

#include "CoreMinimal.h"
#include "Components/ActorComponent.h"
#include "MinimapComponent.generated.h"

class USceneCaptureComponent2D;
class UTextureRenderTarget2D;

/**
 * Top-down minimap that renders the floor layout.
 *
 * Uses a SceneCapture2D pointing downward from above the player.
 * Shows:
 * - Floor tiles (from FloorBuilder)
 * - Player position (centered)
 * - Monster positions (red dots)
 * - Echo positions (blue/green dots)
 * - Chest/Shrine markers
 * - Stairs marker (when unlocked)
 *
 * Attach to the player character. The render target can be
 * bound to a UMG Image widget for HUD display.
 */
UCLASS(ClassGroup = (UI), meta = (BlueprintSpawnableComponent))
class TOWERGAME_API UMinimapComponent : public UActorComponent
{
    GENERATED_BODY()

public:
    UMinimapComponent();

    virtual void BeginPlay() override;
    virtual void TickComponent(float DeltaTime, ELevelTick TickType, FActorComponentTickFunction* ThisTickFunction) override;

    /** Get the render target texture for binding to UMG */
    UFUNCTION(BlueprintPure, Category = "Minimap")
    UTextureRenderTarget2D* GetMinimapTexture() const { return RenderTarget; }

    // ============ Config ============

    /** Capture height above player */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Minimap")
    float CaptureHeight = 2000.0f;

    /** Orthographic width of the minimap view */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Minimap")
    float OrthoWidth = 3000.0f;

    /** Render target resolution */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Minimap")
    int32 TextureSize = 256;

    /** Update frequency (captures per second) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Minimap")
    float CaptureRate = 5.0f;

    /** Whether minimap rotates with player facing */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Minimap")
    bool bRotateWithPlayer = false;

    /** Zoom level (1.0 = default, 0.5 = zoomed in, 2.0 = zoomed out) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Minimap", meta = (ClampMin = "0.25", ClampMax = "4.0"))
    float ZoomLevel = 1.0f;

    // ============ Controls ============

    UFUNCTION(BlueprintCallable, Category = "Minimap")
    void ZoomIn();

    UFUNCTION(BlueprintCallable, Category = "Minimap")
    void ZoomOut();

    UFUNCTION(BlueprintCallable, Category = "Minimap")
    void ToggleRotation();

private:
    UPROPERTY()
    USceneCaptureComponent2D* CaptureComponent;

    UPROPERTY()
    UTextureRenderTarget2D* RenderTarget;

    float CaptureTimer = 0.0f;

    void SetupCapture();
    void UpdateCapturePosition();
};
