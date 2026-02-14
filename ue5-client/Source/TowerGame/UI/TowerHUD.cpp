#include "TowerHUD.h"
#include "TowerHUDWidget.h"
#include "Blueprint/UserWidget.h"

ATowerHUD::ATowerHUD()
{
}

void ATowerHUD::BeginPlay()
{
    Super::BeginPlay();

    if (HUDWidgetClass)
    {
        HUDWidget = CreateWidget<UTowerHUDWidget>(GetOwningPlayerController(), HUDWidgetClass);
        if (HUDWidget)
        {
            HUDWidget->AddToViewport();
            UE_LOG(LogTemp, Log, TEXT("Tower HUD widget added to viewport"));
        }
    }
    else
    {
        UE_LOG(LogTemp, Warning, TEXT("No HUD widget class assigned — using C++ fallback"));
    }
}
