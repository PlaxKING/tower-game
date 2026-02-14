#include "TowerInputConfig.h"
#include "InputMappingContext.h"
#include "InputAction.h"
#include "InputModifiers.h"
#include "InputTriggers.h"

UTowerInputConfig* UTowerInputConfig::CreateDefaultConfig(UObject* Outer)
{
    UTowerInputConfig* Config = NewObject<UTowerInputConfig>(Outer);
    Config->SetupActions(Outer);
    Config->SetupMappingContext(Outer);
    return Config;
}

void UTowerInputConfig::SetupActions(UObject* Outer)
{
    // Move — 2D axis (WASD)
    IA_Move = NewObject<UInputAction>(Outer, TEXT("IA_Move"));
    IA_Move->ValueType = EInputActionValueType::Axis2D;

    // Look — 2D axis (Mouse)
    IA_Look = NewObject<UInputAction>(Outer, TEXT("IA_Look"));
    IA_Look->ValueType = EInputActionValueType::Axis2D;

    // Jump — digital (Space)
    IA_Jump = NewObject<UInputAction>(Outer, TEXT("IA_Jump"));
    IA_Jump->ValueType = EInputActionValueType::Boolean;

    // Attack — digital (LMB)
    IA_Attack = NewObject<UInputAction>(Outer, TEXT("IA_Attack"));
    IA_Attack->ValueType = EInputActionValueType::Boolean;

    // Dodge — digital (Shift)
    IA_Dodge = NewObject<UInputAction>(Outer, TEXT("IA_Dodge"));
    IA_Dodge->ValueType = EInputActionValueType::Boolean;

    // Interact — digital (E)
    IA_Interact = NewObject<UInputAction>(Outer, TEXT("IA_Interact"));
    IA_Interact->ValueType = EInputActionValueType::Boolean;

    // Inventory — digital (Tab)
    IA_Inventory = NewObject<UInputAction>(Outer, TEXT("IA_Inventory"));
    IA_Inventory->ValueType = EInputActionValueType::Boolean;

    // Pause — digital (Escape)
    IA_Pause = NewObject<UInputAction>(Outer, TEXT("IA_Pause"));
    IA_Pause->ValueType = EInputActionValueType::Boolean;
}

void UTowerInputConfig::SetupMappingContext(UObject* Outer)
{
    DefaultContext = NewObject<UInputMappingContext>(Outer, TEXT("IMC_TowerDefault"));

    // ===== Move: WASD =====
    {
        // W — Forward (+Y)
        FEnhancedActionKeyMapping& W = DefaultContext->MapKey(IA_Move, EKeys::W);
        UInputModifierSwizzleAxis* SwizzleW = NewObject<UInputModifierSwizzleAxis>(Outer);
        SwizzleW->Order = EInputAxisSwizzle::YXZ;
        W.Modifiers.Add(SwizzleW);

        // S — Backward (-Y)
        FEnhancedActionKeyMapping& S = DefaultContext->MapKey(IA_Move, EKeys::S);
        UInputModifierSwizzleAxis* SwizzleS = NewObject<UInputModifierSwizzleAxis>(Outer);
        SwizzleS->Order = EInputAxisSwizzle::YXZ;
        S.Modifiers.Add(SwizzleS);
        UInputModifierNegate* NegS = NewObject<UInputModifierNegate>(Outer);
        S.Modifiers.Add(NegS);

        // D — Right (+X)
        DefaultContext->MapKey(IA_Move, EKeys::D);

        // A — Left (-X)
        FEnhancedActionKeyMapping& A = DefaultContext->MapKey(IA_Move, EKeys::A);
        UInputModifierNegate* NegA = NewObject<UInputModifierNegate>(Outer);
        A.Modifiers.Add(NegA);
    }

    // ===== Look: Mouse XY =====
    {
        FEnhancedActionKeyMapping& MouseXY = DefaultContext->MapKey(IA_Look, EKeys::Mouse2D);
        UInputModifierNegate* NegLook = NewObject<UInputModifierNegate>(Outer);
        NegLook->bX = false;
        NegLook->bY = true;  // Invert Y for natural camera control
        NegLook->bZ = false;
        MouseXY.Modifiers.Add(NegLook);
    }

    // ===== Digital actions =====
    DefaultContext->MapKey(IA_Jump, EKeys::SpaceBar);
    DefaultContext->MapKey(IA_Attack, EKeys::LeftMouseButton);
    DefaultContext->MapKey(IA_Dodge, EKeys::LeftShift);
    DefaultContext->MapKey(IA_Interact, EKeys::E);
    DefaultContext->MapKey(IA_Inventory, EKeys::Tab);
    DefaultContext->MapKey(IA_Pause, EKeys::Escape);

    // Gamepad support
    {
        // Left stick — Move
        FEnhancedActionKeyMapping& LSY = DefaultContext->MapKey(IA_Move, EKeys::Gamepad_LeftY);
        UInputModifierSwizzleAxis* SwizzleGP = NewObject<UInputModifierSwizzleAxis>(Outer);
        SwizzleGP->Order = EInputAxisSwizzle::YXZ;
        LSY.Modifiers.Add(SwizzleGP);

        DefaultContext->MapKey(IA_Move, EKeys::Gamepad_LeftX);

        // Right stick — Look
        FEnhancedActionKeyMapping& RSY = DefaultContext->MapKey(IA_Look, EKeys::Gamepad_RightY);
        UInputModifierNegate* NegRSY = NewObject<UInputModifierNegate>(Outer);
        NegRSY->bX = false;
        NegRSY->bY = true;
        NegRSY->bZ = false;
        UInputModifierSwizzleAxis* SwizzleRS = NewObject<UInputModifierSwizzleAxis>(Outer);
        SwizzleRS->Order = EInputAxisSwizzle::YXZ;
        RSY.Modifiers.Add(SwizzleRS);
        RSY.Modifiers.Add(NegRSY);

        DefaultContext->MapKey(IA_Look, EKeys::Gamepad_RightX);

        // Buttons
        DefaultContext->MapKey(IA_Jump, EKeys::Gamepad_FaceButton_Bottom);      // A/Cross
        DefaultContext->MapKey(IA_Attack, EKeys::Gamepad_RightTrigger);          // RT/R2
        DefaultContext->MapKey(IA_Dodge, EKeys::Gamepad_FaceButton_Right);       // B/Circle
        DefaultContext->MapKey(IA_Interact, EKeys::Gamepad_FaceButton_Left);     // X/Square
        DefaultContext->MapKey(IA_Pause, EKeys::Gamepad_Special_Right);          // Start
    }
}
