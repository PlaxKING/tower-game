#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "TowerHUDWidget.generated.h"

class UProgressBar;
class UTextBlock;
class UVerticalBox;
class ATowerPlayerCharacter;
class ATowerGameState;

/**
 * Main HUD widget — bound to player character stats.
 * Can be subclassed in Blueprint for visual styling.
 *
 * Layout:
 *   [Top-Left]   Floor info + Breath phase
 *   [Top-Right]  Monsters remaining
 *   [Bottom-Left] HP bar + Resource bars
 *   [Bottom-Center] Combo counter
 */
UCLASS()
class TOWERGAME_API UTowerHUDWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;
    virtual void NativeTick(const FGeometry& MyGeometry, float InDeltaTime) override;

    // ============ Health ============

    UPROPERTY(meta = (BindWidget), BlueprintReadOnly, Category = "Tower|HUD")
    UProgressBar* HealthBar;

    UPROPERTY(meta = (BindWidget), BlueprintReadOnly, Category = "Tower|HUD")
    UTextBlock* HealthText;

    // ============ Resources ============

    UPROPERTY(meta = (BindWidget), BlueprintReadOnly, Category = "Tower|HUD")
    UProgressBar* KineticBar;

    UPROPERTY(meta = (BindWidget), BlueprintReadOnly, Category = "Tower|HUD")
    UProgressBar* ThermalBar;

    UPROPERTY(meta = (BindWidget), BlueprintReadOnly, Category = "Tower|HUD")
    UProgressBar* SemanticBar;

    // ============ Combat ============

    UPROPERTY(meta = (BindWidget), BlueprintReadOnly, Category = "Tower|HUD")
    UTextBlock* ComboText;

    // ============ World State ============

    UPROPERTY(meta = (BindWidget), BlueprintReadOnly, Category = "Tower|HUD")
    UTextBlock* FloorText;

    UPROPERTY(meta = (BindWidget), BlueprintReadOnly, Category = "Tower|HUD")
    UTextBlock* BreathText;

    UPROPERTY(meta = (BindWidget), BlueprintReadOnly, Category = "Tower|HUD")
    UTextBlock* MonstersText;

protected:
    /** Update all UI elements from current game state */
    void RefreshHUD();

    ATowerPlayerCharacter* GetPlayerCharacter() const;
    ATowerGameState* GetGameState() const;
};
