#include "TowerPlayerCharacter.h"
#include "TowerInputConfig.h"
#include "TowerGame/Core/TowerGameSubsystem.h"
#include "Camera/CameraComponent.h"
#include "GameFramework/SpringArmComponent.h"
#include "GameFramework/CharacterMovementComponent.h"
#include "EnhancedInputComponent.h"
#include "EnhancedInputSubsystems.h"
#include "Kismet/GameplayStatics.h"

ATowerPlayerCharacter::ATowerPlayerCharacter()
{
    PrimaryActorTick.bCanEverTick = true;

    // Don't rotate character when camera rotates
    bUseControllerRotationPitch = false;
    bUseControllerRotationYaw = false;
    bUseControllerRotationRoll = false;

    // Character movement
    GetCharacterMovement()->bOrientRotationToMovement = true;
    GetCharacterMovement()->RotationRate = FRotator(0.0f, 540.0f, 0.0f);
    GetCharacterMovement()->JumpZVelocity = 600.0f;
    GetCharacterMovement()->AirControl = 0.35f;
    GetCharacterMovement()->MaxWalkSpeed = 600.0f;

    // Camera boom
    CameraBoom = CreateDefaultSubobject<USpringArmComponent>(TEXT("CameraBoom"));
    CameraBoom->SetupAttachment(RootComponent);
    CameraBoom->TargetArmLength = 400.0f;
    CameraBoom->bUsePawnControlRotation = true;

    // Follow camera
    FollowCamera = CreateDefaultSubobject<UCameraComponent>(TEXT("FollowCamera"));
    FollowCamera->SetupAttachment(CameraBoom, USpringArmComponent::SocketName);
    FollowCamera->bUsePawnControlRotation = false;
}

void ATowerPlayerCharacter::BeginPlay()
{
    Super::BeginPlay();

    // Auto-create input config if not assigned via editor
    if (!DefaultMappingContext)
    {
        UTowerInputConfig* Config = UTowerInputConfig::CreateDefaultConfig(this);
        DefaultMappingContext = Config->DefaultContext;
        MoveAction = Config->IA_Move;
        LookAction = Config->IA_Look;
        JumpAction = Config->IA_Jump;
        AttackAction = Config->IA_Attack;
        DodgeAction = Config->IA_Dodge;
        InteractAction = Config->IA_Interact;
        UE_LOG(LogTemp, Log, TEXT("Auto-created Enhanced Input config (WASD+Mouse+LMB+Shift+E)"));
    }

    // Add input mapping context
    if (APlayerController* PC = Cast<APlayerController>(Controller))
    {
        if (UEnhancedInputLocalPlayerSubsystem* Subsystem =
            ULocalPlayer::GetSubsystem<UEnhancedInputLocalPlayerSubsystem>(PC->GetLocalPlayer()))
        {
            if (DefaultMappingContext)
            {
                Subsystem->AddMappingContext(DefaultMappingContext, 0);
            }
        }
    }
}

void ATowerPlayerCharacter::SetupPlayerInputComponent(UInputComponent* PlayerInputComponent)
{
    Super::SetupPlayerInputComponent(PlayerInputComponent);

    if (UEnhancedInputComponent* EnhancedInput = CastChecked<UEnhancedInputComponent>(PlayerInputComponent))
    {
        if (MoveAction)
            EnhancedInput->BindAction(MoveAction, ETriggerEvent::Triggered, this, &ATowerPlayerCharacter::Move);

        if (LookAction)
            EnhancedInput->BindAction(LookAction, ETriggerEvent::Triggered, this, &ATowerPlayerCharacter::Look);

        if (JumpAction)
        {
            EnhancedInput->BindAction(JumpAction, ETriggerEvent::Started, this, &ACharacter::Jump);
            EnhancedInput->BindAction(JumpAction, ETriggerEvent::Completed, this, &ACharacter::StopJumping);
        }

        if (AttackAction)
            EnhancedInput->BindAction(AttackAction, ETriggerEvent::Started, this, &ATowerPlayerCharacter::PerformAttack);

        if (DodgeAction)
            EnhancedInput->BindAction(DodgeAction, ETriggerEvent::Started, this, &ATowerPlayerCharacter::PerformDodge);

        if (InteractAction)
            EnhancedInput->BindAction(InteractAction, ETriggerEvent::Started, this, &ATowerPlayerCharacter::Interact);
    }
}

void ATowerPlayerCharacter::Tick(float DeltaTime)
{
    Super::Tick(DeltaTime);

    // Combo window decay
    if (ComboTimer > 0.0f)
    {
        ComboTimer -= DeltaTime;
        if (ComboTimer <= 0.0f)
        {
            ResetCombo();
        }
    }

    RegenerateResources(DeltaTime);
}

void ATowerPlayerCharacter::Move(const FInputActionValue& Value)
{
    FVector2D MoveInput = Value.Get<FVector2D>();

    if (Controller != nullptr && !bIsAttacking)
    {
        const FRotator Rotation = Controller->GetControlRotation();
        const FRotator YawRotation(0, Rotation.Yaw, 0);

        const FVector ForwardDirection = FRotationMatrix(YawRotation).GetUnitAxis(EAxis::X);
        const FVector RightDirection = FRotationMatrix(YawRotation).GetUnitAxis(EAxis::Y);

        AddMovementInput(ForwardDirection, MoveInput.Y);
        AddMovementInput(RightDirection, MoveInput.X);
    }
}

void ATowerPlayerCharacter::Look(const FInputActionValue& Value)
{
    FVector2D LookInput = Value.Get<FVector2D>();

    if (Controller != nullptr)
    {
        AddControllerYawInput(LookInput.X);
        AddControllerPitchInput(LookInput.Y);
    }
}

void ATowerPlayerCharacter::PerformAttack()
{
    if (bIsAttacking || bIsDodging) return;
    if (KineticEnergy < 5.0f) return; // Not enough resource

    bIsAttacking = true;
    KineticEnergy -= 5.0f + ComboStep * 3.0f;

    // Calculate damage through Rust core
    float FinalDamage = BaseDamage;
    UTowerGameSubsystem* Sub = GetTowerSubsystem();
    if (Sub && Sub->IsRustCoreReady())
    {
        // AngleId: 0=Front, 1=Side, 2=Back (determine from target direction)
        FinalDamage = Sub->CalculateDamage(BaseDamage, 0, ComboStep);
    }
    else
    {
        // Fallback: simple combo multiplier
        FinalDamage = BaseDamage * (1.0f + ComboStep * 0.15f);
    }

    UE_LOG(LogTemp, Log, TEXT("Attack! Combo %d, Damage: %.1f, Kinetic: %.1f"),
        ComboStep, FinalDamage, KineticEnergy);

    // Advance combo
    ComboStep = (ComboStep + 1) % MaxCombo;
    ComboTimer = ComboWindow;

    // Simulate attack duration
    FTimerHandle AttackTimer;
    GetWorldTimerManager().SetTimer(AttackTimer, [this]()
    {
        bIsAttacking = false;
    }, 0.4f, false);
}

void ATowerPlayerCharacter::PerformDodge()
{
    if (bIsDodging || bIsAttacking) return;
    if (KineticEnergy < 15.0f) return;

    bIsDodging = true;
    KineticEnergy -= 15.0f;
    ResetCombo();

    // Dodge movement
    FVector DodgeDirection = GetLastMovementInputVector();
    if (DodgeDirection.IsNearlyZero())
    {
        DodgeDirection = GetActorForwardVector() * -1.0f; // Backstep
    }
    LaunchCharacter(DodgeDirection * 800.0f + FVector(0, 0, 100.0f), true, true);

    UE_LOG(LogTemp, Log, TEXT("Dodge! Kinetic: %.1f"), KineticEnergy);

    FTimerHandle DodgeTimer;
    GetWorldTimerManager().SetTimer(DodgeTimer, [this]()
    {
        bIsDodging = false;
    }, 0.5f, false);
}

void ATowerPlayerCharacter::Interact()
{
    UE_LOG(LogTemp, Log, TEXT("Interact pressed"));
    // Floor transition, NPC dialog, item pickup — handled by overlap checks
}

void ATowerPlayerCharacter::TakeCombatDamage(float Amount)
{
    if (bIsDodging) return; // I-frames during dodge

    float ActualDamage = FMath::Max(0.0f, Amount);
    CurrentHp -= ActualDamage;

    UE_LOG(LogTemp, Log, TEXT("Player takes %.1f damage. HP: %.0f/%.0f"),
        ActualDamage, CurrentHp, MaxHp);

    if (CurrentHp <= 0.0f)
    {
        CurrentHp = 0.0f;
        UE_LOG(LogTemp, Warning, TEXT("Player defeated!"));
        // Death/echo system will handle respawn
    }
}

UTowerGameSubsystem* ATowerPlayerCharacter::GetTowerSubsystem() const
{
    UGameInstance* GI = UGameplayStatics::GetGameInstance(this);
    if (!GI) return nullptr;
    return GI->GetSubsystem<UTowerGameSubsystem>();
}

void ATowerPlayerCharacter::ResetCombo()
{
    ComboStep = 0;
    ComboTimer = 0.0f;
}

void ATowerPlayerCharacter::RegenerateResources(float DeltaTime)
{
    // Kinetic: 5/s passive, more from movement
    float MovementBonus = GetVelocity().Size() > 50.0f ? 10.0f : 0.0f;
    KineticEnergy = FMath::Min(100.0f, KineticEnergy + (5.0f + MovementBonus) * DeltaTime);

    // Thermal: 3/s passive
    ThermalEnergy = FMath::Min(100.0f, ThermalEnergy + 3.0f * DeltaTime);

    // Semantic: 1/s passive (boosted by analyzing tags nearby)
    SemanticEnergy = FMath::Min(100.0f, SemanticEnergy + 1.0f * DeltaTime);
}
