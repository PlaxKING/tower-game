#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "SkillTreeWidget.h"
#include "SpecializationWidget.generated.h"

class UTextBlock;
class UButton;
class UScrollBox;
class UVerticalBox;
class UHorizontalBox;
class UComboBoxString;
class UImage;

/// Combat role — mirrors Rust CombatRole
UENUM(BlueprintType)
enum class ECombatRole : uint8
{
    Vanguard,
    Striker,
    Support,
    Sentinel,
    Specialist,
};

/// Passive ability display data
USTRUCT(BlueprintType)
struct FSpecPassiveDisplay
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString Name;
    UPROPERTY(BlueprintReadWrite) float Value = 0.0f;
    UPROPERTY(BlueprintReadWrite) FString Description;
};

/// Specialization branch display data — mirrors Rust SpecBranch
USTRUCT(BlueprintType)
struct FSpecBranchDisplay
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString Id;
    UPROPERTY(BlueprintReadWrite) FString Name;
    UPROPERTY(BlueprintReadWrite) EMasteryDomain Domain = EMasteryDomain::SwordMastery;
    UPROPERTY(BlueprintReadWrite) FString Description;
    UPROPERTY(BlueprintReadWrite) ECombatRole RoleAffinity = ECombatRole::Vanguard;
    UPROPERTY(BlueprintReadWrite) TArray<FString> Passives;
    UPROPERTY(BlueprintReadWrite) bool bHasUltimate = false;
    UPROPERTY(BlueprintReadWrite) FString UltimateName;
    UPROPERTY(BlueprintReadWrite) FString UltimateDescription;
    UPROPERTY(BlueprintReadWrite) bool bIsChosen = false;
    UPROPERTY(BlueprintReadWrite) bool bCanChoose = true;
};

/// Synergy between two branches
USTRUCT(BlueprintType)
struct FSynergyDisplay
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString Name;
    UPROPERTY(BlueprintReadWrite) FString Description;
    UPROPERTY(BlueprintReadWrite) FString BranchA;
    UPROPERTY(BlueprintReadWrite) FString BranchB;
    UPROPERTY(BlueprintReadWrite) bool bIsActive = false;
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnBranchSelected, EMasteryDomain, Domain, const FString&, BranchId);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnBranchReset, EMasteryDomain, Domain);

/**
 * Specialization / role selection widget.
 * Displays two spec branches per mastery domain, passive lists, ultimate previews,
 * synergies, and role indicators. Mirrors Rust specialization module.
 */
UCLASS()
class TOWERGAME_API USpecializationWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;

    // --- Data Loading ---
    UFUNCTION(BlueprintCallable, Category = "Specialization")
    void LoadFromJson(const FString& SpecJson);

    // --- Branch Selection ---
    UFUNCTION(BlueprintCallable, Category = "Specialization")
    void SelectBranch(const FString& BranchId);

    UFUNCTION(BlueprintCallable, Category = "Specialization")
    void ConfirmSelection();

    UFUNCTION(BlueprintCallable, Category = "Specialization")
    void ResetBranch(EMasteryDomain Domain);

    // --- Queries ---
    UFUNCTION(BlueprintPure, Category = "Specialization")
    ECombatRole GetPrimaryRole() const;

    UFUNCTION(BlueprintPure, Category = "Specialization")
    ECombatRole GetSecondaryRole() const;

    UFUNCTION(BlueprintPure, Category = "Specialization")
    FString GetRoleName(ECombatRole Role) const;

    UFUNCTION(BlueprintPure, Category = "Specialization")
    FLinearColor GetRoleColor(ECombatRole Role) const;

    UFUNCTION(BlueprintPure, Category = "Specialization")
    TArray<FSynergyDisplay> GetActiveSynergies() const;

    // --- Events ---
    UPROPERTY(BlueprintAssignable, Category = "Specialization")
    FOnBranchSelected OnBranchSelected;

    UPROPERTY(BlueprintAssignable, Category = "Specialization")
    FOnBranchReset OnBranchReset;

protected:
    // --- Domain Selector ---
    UPROPERTY(meta = (BindWidgetOptional)) UComboBoxString* DomainSelectorCombo = nullptr;

    // --- Left Branch Card ---
    UPROPERTY(meta = (BindWidgetOptional)) UVerticalBox* LeftBranchCard = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* LeftBranchNameText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* LeftBranchDescText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* LeftBranchRoleText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UScrollBox* LeftPassiveListBox = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* LeftUltimateNameText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* LeftUltimateDescText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* LeftSelectButton = nullptr;

    // --- Right Branch Card ---
    UPROPERTY(meta = (BindWidgetOptional)) UVerticalBox* RightBranchCard = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* RightBranchNameText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* RightBranchDescText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* RightBranchRoleText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UScrollBox* RightPassiveListBox = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* RightUltimateNameText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* RightUltimateDescText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* RightSelectButton = nullptr;

    // --- Role Indicator ---
    UPROPERTY(meta = (BindWidgetOptional)) UImage* PrimaryRoleIcon = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* PrimaryRoleText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UImage* SecondaryRoleIcon = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* SecondaryRoleText = nullptr;

    // --- Synergy List ---
    UPROPERTY(meta = (BindWidgetOptional)) UScrollBox* SynergyListBox = nullptr;

    // --- Confirm / Reset ---
    UPROPERTY(meta = (BindWidgetOptional)) UButton* ConfirmButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* ResetButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* StatusText = nullptr;

    // --- State ---
    TMap<EMasteryDomain, TArray<FSpecBranchDisplay>> DomainBranches;
    TArray<FSynergyDisplay> AllSynergies;
    EMasteryDomain SelectedDomain = EMasteryDomain::SwordMastery;
    FString PendingBranchId;

    // --- Internal ---
    void PopulateDomainCombo();
    void RebuildBranchCards();
    void RebuildPassiveList(UScrollBox* ListBox, const FSpecBranchDisplay& Branch);
    void RebuildSynergyList();
    void UpdateRoleIndicators();
    void UpdateButtonStates();

    EMasteryDomain ParseDomainString(const FString& Str) const;
    ECombatRole ParseRole(const FString& Str) const;
    ECombatRole ComputeRole(bool bPrimary) const;

    UFUNCTION() void OnDomainChanged(FString SelectedItem, ESelectInfo::Type SelectionType);
    UFUNCTION() void OnLeftSelectClicked();
    UFUNCTION() void OnRightSelectClicked();
    UFUNCTION() void OnConfirmClicked();
    UFUNCTION() void OnResetClicked();
};
