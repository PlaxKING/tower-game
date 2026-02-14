#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "CharacterSelectWidget.generated.h"

class UTextBlock;
class UButton;
class UImage;
class UVerticalBox;
class UHorizontalBox;
class UProgressBar;

/// Weapon type — mirrors Rust WeaponType enum
UENUM(BlueprintType)
enum class EStartingWeapon : uint8
{
    Sword,
    Greatsword,
    DualDaggers,
    Spear,
    Gauntlets,
    Staff,
};

/// Starting element affinity
UENUM(BlueprintType)
enum class EStartingElement : uint8
{
    Fire,
    Water,
    Earth,
    Wind,
    Void,
    Neutral,
};

/// Character build preview data
USTRUCT(BlueprintType)
struct FCharacterBuildPreview
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString CharacterName;
    UPROPERTY(BlueprintReadWrite) EStartingWeapon Weapon = EStartingWeapon::Sword;
    UPROPERTY(BlueprintReadWrite) EStartingElement Element = EStartingElement::Neutral;

    // Base stat allocation (total = 20 at start)
    UPROPERTY(BlueprintReadWrite) int32 Strength = 4;
    UPROPERTY(BlueprintReadWrite) int32 Agility = 4;
    UPROPERTY(BlueprintReadWrite) int32 Vitality = 4;
    UPROPERTY(BlueprintReadWrite) int32 Mind = 4;
    UPROPERTY(BlueprintReadWrite) int32 Spirit = 4;

    UPROPERTY(BlueprintReadWrite) int32 RemainingPoints = 0;
};

/// Weapon info for display
USTRUCT(BlueprintType)
struct FWeaponPreviewInfo
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) EStartingWeapon Type = EStartingWeapon::Sword;
    UPROPERTY(BlueprintReadWrite) FString Name;
    UPROPERTY(BlueprintReadWrite) FString Description;
    UPROPERTY(BlueprintReadWrite) int32 ComboLength = 3;
    UPROPERTY(BlueprintReadWrite) float AttackSpeed = 1.0f;   // relative
    UPROPERTY(BlueprintReadWrite) float DamageRating = 1.0f;  // relative
    UPROPERTY(BlueprintReadWrite) float RangeRating = 1.0f;   // relative
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnCharacterConfirmed, const FCharacterBuildPreview&, Build);

/**
 * Character creation / selection screen.
 * Lets player pick weapon, element, allocate starting stats.
 * Mirrors Rust player module stat structure and combat::weapons types.
 */
UCLASS()
class TOWERGAME_API UCharacterSelectWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;

    // --- Weapon Selection ---

    UFUNCTION(BlueprintCallable, Category = "CharSelect")
    void SelectWeapon(EStartingWeapon Weapon);

    UFUNCTION(BlueprintCallable, Category = "CharSelect")
    void CycleWeapon(bool bForward);

    // --- Element Selection ---

    UFUNCTION(BlueprintCallable, Category = "CharSelect")
    void SelectElement(EStartingElement Element);

    UFUNCTION(BlueprintCallable, Category = "CharSelect")
    void CycleElement(bool bForward);

    // --- Stat Allocation ---

    UFUNCTION(BlueprintCallable, Category = "CharSelect")
    bool IncreaseStat(const FString& StatName);

    UFUNCTION(BlueprintCallable, Category = "CharSelect")
    bool DecreaseStat(const FString& StatName);

    UFUNCTION(BlueprintCallable, Category = "CharSelect")
    void ResetStats();

    // --- Name ---

    UFUNCTION(BlueprintCallable, Category = "CharSelect")
    void SetCharacterName(const FString& Name);

    // --- Confirm ---

    UFUNCTION(BlueprintCallable, Category = "CharSelect")
    void ConfirmCharacter();

    UFUNCTION(BlueprintPure, Category = "CharSelect")
    bool IsValid() const;

    UFUNCTION(BlueprintPure, Category = "CharSelect")
    FCharacterBuildPreview GetCurrentBuild() const { return CurrentBuild; }

    // --- Events ---

    UPROPERTY(BlueprintAssignable, Category = "CharSelect")
    FOnCharacterConfirmed OnConfirmed;

protected:
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* CharacterNameText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* WeaponNameText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* WeaponDescText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* ElementNameText = nullptr;

    // Stat display
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* StrengthText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* AgilityText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* VitalityText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* MindText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* SpiritText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* RemainingPointsText = nullptr;

    // Weapon stat bars
    UPROPERTY(meta = (BindWidgetOptional)) UProgressBar* SpeedBar = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UProgressBar* DamageBar = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UProgressBar* RangeBar = nullptr;

    // Buttons
    UPROPERTY(meta = (BindWidgetOptional)) UButton* ConfirmButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* WeaponLeftBtn = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* WeaponRightBtn = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* ElementLeftBtn = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* ElementRightBtn = nullptr;

    UPROPERTY(EditDefaultsOnly, Category = "CharSelect")
    int32 TotalStartingPoints = 20;

    UPROPERTY(EditDefaultsOnly, Category = "CharSelect")
    int32 MaxSingleStat = 10;

    UPROPERTY(EditDefaultsOnly, Category = "CharSelect")
    int32 MinSingleStat = 1;

    FCharacterBuildPreview CurrentBuild;
    TArray<FWeaponPreviewInfo> WeaponInfos;

    void InitWeaponInfos();
    void UpdateDisplay();
    FWeaponPreviewInfo GetWeaponInfo(EStartingWeapon Weapon) const;
    FString GetElementName(EStartingElement Element) const;
    FLinearColor GetElementColor(EStartingElement Element) const;

    int32* GetStatPtr(const FString& StatName);

    UFUNCTION() void OnConfirmClicked();
    UFUNCTION() void OnWeaponLeftClicked();
    UFUNCTION() void OnWeaponRightClicked();
    UFUNCTION() void OnElementLeftClicked();
    UFUNCTION() void OnElementRightClicked();
};
