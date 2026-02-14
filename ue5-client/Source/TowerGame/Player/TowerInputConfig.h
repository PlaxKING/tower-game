#pragma once

#include "CoreMinimal.h"
#include "InputMappingContext.h"
#include "InputAction.h"
#include "TowerInputConfig.generated.h"

/**
 * Code-driven input configuration for Tower Game.
 * Creates Enhanced Input actions and mapping context programmatically
 * so the game works without manual .uasset creation.
 *
 * Bindings:
 *   WASD       — Move (2D Axis)
 *   Mouse XY   — Look (2D Axis)
 *   Space      — Jump (Digital)
 *   LMB        — Attack (Digital)
 *   Shift      — Dodge (Digital)
 *   E          — Interact (Digital)
 *   Tab        — Inventory (Digital)
 *   Escape     — Pause (Digital)
 */
UCLASS(Blueprintable)
class TOWERGAME_API UTowerInputConfig : public UObject
{
    GENERATED_BODY()

public:
    /** Create and return all input assets. Call once at game startup. */
    static UTowerInputConfig* CreateDefaultConfig(UObject* Outer);

    UPROPERTY()
    UInputMappingContext* DefaultContext;

    UPROPERTY()
    UInputAction* IA_Move;

    UPROPERTY()
    UInputAction* IA_Look;

    UPROPERTY()
    UInputAction* IA_Jump;

    UPROPERTY()
    UInputAction* IA_Attack;

    UPROPERTY()
    UInputAction* IA_Dodge;

    UPROPERTY()
    UInputAction* IA_Interact;

    UPROPERTY()
    UInputAction* IA_Inventory;

    UPROPERTY()
    UInputAction* IA_Pause;

private:
    void SetupActions(UObject* Outer);
    void SetupMappingContext(UObject* Outer);
};
