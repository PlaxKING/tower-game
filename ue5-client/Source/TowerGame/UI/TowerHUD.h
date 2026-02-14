#pragma once

#include "CoreMinimal.h"
#include "GameFramework/HUD.h"
#include "TowerHUD.generated.h"

class UTowerHUDWidget;

/**
 * Tower HUD — creates and manages the main gameplay UI overlay.
 * Displays: HP bar, resource bars (Kinetic/Thermal/Semantic),
 * combo counter, breath state, floor info, and monster count.
 */
UCLASS()
class TOWERGAME_API ATowerHUD : public AHUD
{
    GENERATED_BODY()

public:
    ATowerHUD();

    virtual void BeginPlay() override;

    /** The main HUD widget class to spawn */
    UPROPERTY(EditDefaultsOnly, Category = "Tower|UI")
    TSubclassOf<UTowerHUDWidget> HUDWidgetClass;

    /** Reference to the spawned widget */
    UPROPERTY()
    UTowerHUDWidget* HUDWidget;
};
