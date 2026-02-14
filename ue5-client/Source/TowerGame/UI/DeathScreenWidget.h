#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "DeathScreenWidget.generated.h"

class UTextBlock;
class UButton;
class UProgressBar;

DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnRespawnRequested);
DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnReturnToLobby);

/**
 * Death screen widget — shown when the player dies.
 *
 * Layout:
 *   YOU DIED (title, fade in)
 *   "Your echo lingers on floor {N}..." (flavor text)
 *   Echo type: {type} (what echo was left behind)
 *   -----
 *   Floor reached: {N}
 *   Monsters slain: {N}
 *   Time survived: {MM:SS}
 *   -----
 *   [Respawn] (enabled after cooldown)
 *   [Return to Lobby]
 *
 * Respawn has a configurable cooldown (3s default) with progress bar.
 * Background fades to dark red.
 */
UCLASS()
class TOWERGAME_API UDeathScreenWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;
    virtual void NativeTick(const FGeometry& MyGeometry, float InDeltaTime) override;

    // ============ API ============

    /** Show the death screen with stats */
    UFUNCTION(BlueprintCallable, Category = "DeathScreen")
    void ShowDeathScreen(int32 FloorReached, int32 MonstersSlain,
        float TimeSurvived, const FString& EchoType);

    /** Hide death screen */
    UFUNCTION(BlueprintCallable, Category = "DeathScreen")
    void HideDeathScreen();

    // ============ Config ============

    /** Time before respawn button is enabled */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "DeathScreen")
    float RespawnCooldown = 3.0f;

    // ============ Events ============

    UPROPERTY(BlueprintAssignable, Category = "DeathScreen")
    FOnRespawnRequested OnRespawnRequested;

    UPROPERTY(BlueprintAssignable, Category = "DeathScreen")
    FOnReturnToLobby OnReturnToLobby;

    // ============ Bound Widgets ============

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "DeathScreen")
    UTextBlock* TitleText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "DeathScreen")
    UTextBlock* FlavorText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "DeathScreen")
    UTextBlock* EchoTypeText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "DeathScreen")
    UTextBlock* FloorText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "DeathScreen")
    UTextBlock* MonstersText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "DeathScreen")
    UTextBlock* TimeText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "DeathScreen")
    UButton* RespawnButton;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "DeathScreen")
    UButton* LobbyButton;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "DeathScreen")
    UProgressBar* RespawnCooldownBar;

protected:
    UFUNCTION()
    void OnRespawnClicked();

    UFUNCTION()
    void OnLobbyClicked();

private:
    float CooldownTimer = 0.0f;
    float FadeInTimer = 0.0f;
    bool bShowing = false;
};
