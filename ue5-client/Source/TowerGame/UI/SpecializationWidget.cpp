#include "SpecializationWidget.h"
#include "Components/TextBlock.h"
#include "Components/Button.h"
#include "Components/ScrollBox.h"
#include "Components/VerticalBox.h"
#include "Components/HorizontalBox.h"
#include "Components/ComboBoxString.h"
#include "Components/Image.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

void USpecializationWidget::NativeConstruct()
{
    Super::NativeConstruct();

    // Bind domain selector
    if (DomainSelectorCombo)
        DomainSelectorCombo->OnSelectionChanged.AddDynamic(this, &USpecializationWidget::OnDomainChanged);

    // Bind branch selection buttons
    if (LeftSelectButton)
        LeftSelectButton->OnClicked.AddDynamic(this, &USpecializationWidget::OnLeftSelectClicked);
    if (RightSelectButton)
        RightSelectButton->OnClicked.AddDynamic(this, &USpecializationWidget::OnRightSelectClicked);

    // Bind confirm / reset
    if (ConfirmButton)
    {
        ConfirmButton->OnClicked.AddDynamic(this, &USpecializationWidget::OnConfirmClicked);
        ConfirmButton->SetIsEnabled(false);
    }
    if (ResetButton)
        ResetButton->OnClicked.AddDynamic(this, &USpecializationWidget::OnResetClicked);

    RebuildBranchCards();
    UpdateRoleIndicators();
}

// ---------------------------------------------------------------------------
// Data Loading
// ---------------------------------------------------------------------------

void USpecializationWidget::LoadFromJson(const FString& SpecJson)
{
    DomainBranches.Empty();
    AllSynergies.Empty();
    PendingBranchId.Empty();

    TSharedPtr<FJsonValue> Parsed;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(SpecJson);
    if (!FJsonSerializer::Deserialize(Reader, Parsed)) return;

    const TSharedPtr<FJsonObject>& Root = Parsed->AsObject();
    if (!Root) return;

    // Parse branches grouped by domain
    const TArray<TSharedPtr<FJsonValue>>* BranchesArr;
    if (Root->TryGetArrayField(TEXT("branches"), BranchesArr))
    {
        for (const auto& BVal : *BranchesArr)
        {
            const TSharedPtr<FJsonObject>& BObj = BVal->AsObject();
            if (!BObj) continue;

            FSpecBranchDisplay Branch;
            Branch.Id = BObj->GetStringField(TEXT("id"));
            Branch.Name = BObj->GetStringField(TEXT("name"));
            Branch.Domain = ParseDomainString(BObj->GetStringField(TEXT("domain")));
            Branch.Description = BObj->HasField(TEXT("description")) ?
                BObj->GetStringField(TEXT("description")) : TEXT("");
            Branch.RoleAffinity = ParseRole(BObj->GetStringField(TEXT("role_affinity")));

            // Passives array
            const TArray<TSharedPtr<FJsonValue>>* PassivesArr;
            if (BObj->TryGetArrayField(TEXT("passives"), PassivesArr))
            {
                for (const auto& PVal : *PassivesArr)
                {
                    Branch.Passives.Add(PVal->AsString());
                }
            }

            // Ultimate
            Branch.bHasUltimate = BObj->HasField(TEXT("ultimate_name"));
            if (Branch.bHasUltimate)
            {
                Branch.UltimateName = BObj->GetStringField(TEXT("ultimate_name"));
                Branch.UltimateDescription = BObj->HasField(TEXT("ultimate_description")) ?
                    BObj->GetStringField(TEXT("ultimate_description")) : TEXT("");
            }

            // Selection state
            Branch.bIsChosen = BObj->HasField(TEXT("is_chosen")) && BObj->GetBoolField(TEXT("is_chosen"));
            Branch.bCanChoose = !BObj->HasField(TEXT("can_choose")) || BObj->GetBoolField(TEXT("can_choose"));

            // Add to domain map
            TArray<FSpecBranchDisplay>& DomainArr = DomainBranches.FindOrAdd(Branch.Domain);
            DomainArr.Add(Branch);
        }
    }

    // Parse synergies
    const TArray<TSharedPtr<FJsonValue>>* SynArr;
    if (Root->TryGetArrayField(TEXT("synergies"), SynArr))
    {
        for (const auto& SVal : *SynArr)
        {
            const TSharedPtr<FJsonObject>& SObj = SVal->AsObject();
            if (!SObj) continue;

            FSynergyDisplay Synergy;
            Synergy.Name = SObj->GetStringField(TEXT("name"));
            Synergy.Description = SObj->HasField(TEXT("description")) ?
                SObj->GetStringField(TEXT("description")) : TEXT("");
            Synergy.BranchA = SObj->GetStringField(TEXT("branch_a"));
            Synergy.BranchB = SObj->GetStringField(TEXT("branch_b"));
            Synergy.bIsActive = SObj->HasField(TEXT("is_active")) && SObj->GetBoolField(TEXT("is_active"));

            AllSynergies.Add(Synergy);
        }
    }

    UE_LOG(LogTemp, Log, TEXT("Loaded specializations: %d domains, %d synergies"),
        DomainBranches.Num(), AllSynergies.Num());

    PopulateDomainCombo();
    RebuildBranchCards();
    RebuildSynergyList();
    UpdateRoleIndicators();
    UpdateButtonStates();
}

// ---------------------------------------------------------------------------
// Branch Selection
// ---------------------------------------------------------------------------

void USpecializationWidget::SelectBranch(const FString& BranchId)
{
    PendingBranchId = BranchId;
    OnBranchSelected.Broadcast(SelectedDomain, BranchId);

    if (StatusText)
    {
        StatusText->SetText(FText::FromString(
            FString::Printf(TEXT("Selected: %s (not yet confirmed)"), *BranchId)));
        StatusText->SetColorAndOpacity(FSlateColor(FLinearColor(1.0f, 0.84f, 0.0f)));
    }

    UpdateButtonStates();
    RebuildBranchCards();
}

void USpecializationWidget::ConfirmSelection()
{
    if (PendingBranchId.IsEmpty()) return;

    // Find and mark the branch as chosen
    TArray<FSpecBranchDisplay>* Branches = DomainBranches.Find(SelectedDomain);
    if (!Branches) return;

    for (FSpecBranchDisplay& Branch : *Branches)
    {
        if (Branch.Id == PendingBranchId)
        {
            Branch.bIsChosen = true;
            Branch.bCanChoose = false;
        }
        else
        {
            // Only one branch per domain
            Branch.bIsChosen = false;
            Branch.bCanChoose = false;
        }
    }

    if (StatusText)
    {
        StatusText->SetText(FText::FromString(
            FString::Printf(TEXT("Confirmed specialization: %s"), *PendingBranchId)));
        StatusText->SetColorAndOpacity(FSlateColor(FLinearColor(0.3f, 1.0f, 0.5f)));
    }

    PendingBranchId.Empty();

    // Recompute synergy activation
    TArray<FString> ChosenBranches;
    for (const auto& DomainPair : DomainBranches)
    {
        for (const FSpecBranchDisplay& B : DomainPair.Value)
        {
            if (B.bIsChosen) ChosenBranches.Add(B.Id);
        }
    }
    for (FSynergyDisplay& Syn : AllSynergies)
    {
        Syn.bIsActive = ChosenBranches.Contains(Syn.BranchA) && ChosenBranches.Contains(Syn.BranchB);
    }

    RebuildBranchCards();
    RebuildSynergyList();
    UpdateRoleIndicators();
    UpdateButtonStates();
}

void USpecializationWidget::ResetBranch(EMasteryDomain Domain)
{
    TArray<FSpecBranchDisplay>* Branches = DomainBranches.Find(Domain);
    if (!Branches) return;

    for (FSpecBranchDisplay& Branch : *Branches)
    {
        Branch.bIsChosen = false;
        Branch.bCanChoose = true;
    }

    if (Domain == SelectedDomain)
    {
        PendingBranchId.Empty();
    }

    // Recompute synergy activation
    TArray<FString> ChosenBranches;
    for (const auto& DomainPair : DomainBranches)
    {
        for (const FSpecBranchDisplay& B : DomainPair.Value)
        {
            if (B.bIsChosen) ChosenBranches.Add(B.Id);
        }
    }
    for (FSynergyDisplay& Syn : AllSynergies)
    {
        Syn.bIsActive = ChosenBranches.Contains(Syn.BranchA) && ChosenBranches.Contains(Syn.BranchB);
    }

    OnBranchReset.Broadcast(Domain);

    if (StatusText)
    {
        StatusText->SetText(FText::FromString(TEXT("Branch reset. Choose a new specialization.")));
        StatusText->SetColorAndOpacity(FSlateColor(FLinearColor(0.7f, 0.7f, 0.7f)));
    }

    RebuildBranchCards();
    RebuildSynergyList();
    UpdateRoleIndicators();
    UpdateButtonStates();
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

ECombatRole USpecializationWidget::GetPrimaryRole() const
{
    return ComputeRole(true);
}

ECombatRole USpecializationWidget::GetSecondaryRole() const
{
    return ComputeRole(false);
}

FString USpecializationWidget::GetRoleName(ECombatRole Role) const
{
    switch (Role)
    {
    case ECombatRole::Vanguard:   return TEXT("Vanguard");
    case ECombatRole::Striker:    return TEXT("Striker");
    case ECombatRole::Support:    return TEXT("Support");
    case ECombatRole::Sentinel:   return TEXT("Sentinel");
    case ECombatRole::Specialist: return TEXT("Specialist");
    default:                      return TEXT("Unknown");
    }
}

FLinearColor USpecializationWidget::GetRoleColor(ECombatRole Role) const
{
    switch (Role)
    {
    case ECombatRole::Vanguard:   return FLinearColor(0.2f, 0.5f, 1.0f);   // blue
    case ECombatRole::Striker:    return FLinearColor(1.0f, 0.25f, 0.25f);  // red
    case ECombatRole::Support:    return FLinearColor(1.0f, 0.84f, 0.0f);   // yellow
    case ECombatRole::Sentinel:   return FLinearColor(0.2f, 0.85f, 0.3f);   // green
    case ECombatRole::Specialist: return FLinearColor(0.7f, 0.3f, 1.0f);    // purple
    default:                      return FLinearColor(0.5f, 0.5f, 0.5f);
    }
}

TArray<FSynergyDisplay> USpecializationWidget::GetActiveSynergies() const
{
    TArray<FSynergyDisplay> Active;
    for (const FSynergyDisplay& Syn : AllSynergies)
    {
        if (Syn.bIsActive) Active.Add(Syn);
    }
    return Active;
}

// ---------------------------------------------------------------------------
// Domain Combo
// ---------------------------------------------------------------------------

void USpecializationWidget::PopulateDomainCombo()
{
    if (!DomainSelectorCombo) return;
    DomainSelectorCombo->ClearOptions();

    for (const auto& Pair : DomainBranches)
    {
        FString DomainName = UEnum::GetValueAsString(Pair.Key);
        // Strip enum class prefix (e.g. "EMasteryDomain::")
        DomainName.RemoveFromStart(TEXT("EMasteryDomain::"));
        DomainSelectorCombo->AddOption(DomainName);
    }

    // Select current domain
    FString Current = UEnum::GetValueAsString(SelectedDomain);
    Current.RemoveFromStart(TEXT("EMasteryDomain::"));
    DomainSelectorCombo->SetSelectedOption(Current);
}

// ---------------------------------------------------------------------------
// Branch Card Display
// ---------------------------------------------------------------------------

void USpecializationWidget::RebuildBranchCards()
{
    const TArray<FSpecBranchDisplay>* Branches = DomainBranches.Find(SelectedDomain);

    // --- Left branch (index 0) ---
    if (Branches && Branches->Num() > 0)
    {
        const FSpecBranchDisplay& Left = (*Branches)[0];
        FLinearColor RoleColor = GetRoleColor(Left.RoleAffinity);
        bool bIsPending = (PendingBranchId == Left.Id);

        if (LeftBranchNameText)
        {
            FString Prefix = Left.bIsChosen ? TEXT("[ACTIVE] ") : (bIsPending ? TEXT("[PENDING] ") : TEXT(""));
            LeftBranchNameText->SetText(FText::FromString(Prefix + Left.Name));
            LeftBranchNameText->SetColorAndOpacity(FSlateColor(
                Left.bIsChosen ? FLinearColor(0.3f, 1.0f, 0.5f) : (bIsPending ? FLinearColor(1.0f, 0.84f, 0.0f) : FLinearColor::White)));
        }

        if (LeftBranchDescText)
            LeftBranchDescText->SetText(FText::FromString(Left.Description));

        if (LeftBranchRoleText)
        {
            LeftBranchRoleText->SetText(FText::FromString(
                FString::Printf(TEXT("Role: %s"), *GetRoleName(Left.RoleAffinity))));
            LeftBranchRoleText->SetColorAndOpacity(FSlateColor(RoleColor));
        }

        // Ultimate preview
        if (LeftUltimateNameText)
        {
            if (Left.bHasUltimate)
            {
                LeftUltimateNameText->SetText(FText::FromString(
                    FString::Printf(TEXT("Ultimate: %s"), *Left.UltimateName)));
                LeftUltimateNameText->SetColorAndOpacity(FSlateColor(FLinearColor(1.0f, 0.84f, 0.0f)));
            }
            else
            {
                LeftUltimateNameText->SetText(FText::FromString(TEXT("No Ultimate")));
                LeftUltimateNameText->SetColorAndOpacity(FSlateColor(FLinearColor(0.4f, 0.4f, 0.4f)));
            }
        }

        if (LeftUltimateDescText)
        {
            LeftUltimateDescText->SetText(FText::FromString(
                Left.bHasUltimate ? Left.UltimateDescription : TEXT("")));
        }

        // Passive list
        RebuildPassiveList(LeftPassiveListBox, Left);

        // Select button state
        if (LeftSelectButton)
        {
            LeftSelectButton->SetIsEnabled(Left.bCanChoose && !Left.bIsChosen);
        }
    }
    else
    {
        // No left branch available
        if (LeftBranchNameText) LeftBranchNameText->SetText(FText::FromString(TEXT("No branch available")));
        if (LeftBranchDescText) LeftBranchDescText->SetText(FText::GetEmpty());
        if (LeftBranchRoleText) LeftBranchRoleText->SetText(FText::GetEmpty());
        if (LeftUltimateNameText) LeftUltimateNameText->SetText(FText::GetEmpty());
        if (LeftUltimateDescText) LeftUltimateDescText->SetText(FText::GetEmpty());
        if (LeftPassiveListBox) LeftPassiveListBox->ClearChildren();
        if (LeftSelectButton) LeftSelectButton->SetIsEnabled(false);
    }

    // --- Right branch (index 1) ---
    if (Branches && Branches->Num() > 1)
    {
        const FSpecBranchDisplay& Right = (*Branches)[1];
        FLinearColor RoleColor = GetRoleColor(Right.RoleAffinity);
        bool bIsPending = (PendingBranchId == Right.Id);

        if (RightBranchNameText)
        {
            FString Prefix = Right.bIsChosen ? TEXT("[ACTIVE] ") : (bIsPending ? TEXT("[PENDING] ") : TEXT(""));
            RightBranchNameText->SetText(FText::FromString(Prefix + Right.Name));
            RightBranchNameText->SetColorAndOpacity(FSlateColor(
                Right.bIsChosen ? FLinearColor(0.3f, 1.0f, 0.5f) : (bIsPending ? FLinearColor(1.0f, 0.84f, 0.0f) : FLinearColor::White)));
        }

        if (RightBranchDescText)
            RightBranchDescText->SetText(FText::FromString(Right.Description));

        if (RightBranchRoleText)
        {
            RightBranchRoleText->SetText(FText::FromString(
                FString::Printf(TEXT("Role: %s"), *GetRoleName(Right.RoleAffinity))));
            RightBranchRoleText->SetColorAndOpacity(FSlateColor(RoleColor));
        }

        // Ultimate preview
        if (RightUltimateNameText)
        {
            if (Right.bHasUltimate)
            {
                RightUltimateNameText->SetText(FText::FromString(
                    FString::Printf(TEXT("Ultimate: %s"), *Right.UltimateName)));
                RightUltimateNameText->SetColorAndOpacity(FSlateColor(FLinearColor(1.0f, 0.84f, 0.0f)));
            }
            else
            {
                RightUltimateNameText->SetText(FText::FromString(TEXT("No Ultimate")));
                RightUltimateNameText->SetColorAndOpacity(FSlateColor(FLinearColor(0.4f, 0.4f, 0.4f)));
            }
        }

        if (RightUltimateDescText)
        {
            RightUltimateDescText->SetText(FText::FromString(
                Right.bHasUltimate ? Right.UltimateDescription : TEXT("")));
        }

        // Passive list
        RebuildPassiveList(RightPassiveListBox, Right);

        // Select button state
        if (RightSelectButton)
        {
            RightSelectButton->SetIsEnabled(Right.bCanChoose && !Right.bIsChosen);
        }
    }
    else
    {
        // No right branch available
        if (RightBranchNameText) RightBranchNameText->SetText(FText::FromString(TEXT("No branch available")));
        if (RightBranchDescText) RightBranchDescText->SetText(FText::GetEmpty());
        if (RightBranchRoleText) RightBranchRoleText->SetText(FText::GetEmpty());
        if (RightUltimateNameText) RightUltimateNameText->SetText(FText::GetEmpty());
        if (RightUltimateDescText) RightUltimateDescText->SetText(FText::GetEmpty());
        if (RightPassiveListBox) RightPassiveListBox->ClearChildren();
        if (RightSelectButton) RightSelectButton->SetIsEnabled(false);
    }
}

void USpecializationWidget::RebuildPassiveList(UScrollBox* ListBox, const FSpecBranchDisplay& Branch)
{
    if (!ListBox) return;
    ListBox->ClearChildren();

    for (const FString& Passive : Branch.Passives)
    {
        UTextBlock* PassiveText = NewObject<UTextBlock>(this);
        PassiveText->SetText(FText::FromString(FString::Printf(TEXT("  - %s"), *Passive)));

        FLinearColor Color = Branch.bIsChosen ?
            FLinearColor(0.3f, 1.0f, 0.5f) : FLinearColor(0.8f, 0.8f, 0.8f);
        PassiveText->SetColorAndOpacity(FSlateColor(Color));

        FSlateFontInfo Font = PassiveText->GetFont();
        Font.Size = 11;
        PassiveText->SetFont(Font);

        ListBox->AddChild(PassiveText);
    }
}

// ---------------------------------------------------------------------------
// Synergy List
// ---------------------------------------------------------------------------

void USpecializationWidget::RebuildSynergyList()
{
    if (!SynergyListBox) return;
    SynergyListBox->ClearChildren();

    for (const FSynergyDisplay& Syn : AllSynergies)
    {
        UTextBlock* SynText = NewObject<UTextBlock>(this);

        FString Display = FString::Printf(TEXT("%s%s: %s (%s + %s)"),
            Syn.bIsActive ? TEXT("[ACTIVE] ") : TEXT(""),
            *Syn.Name, *Syn.Description, *Syn.BranchA, *Syn.BranchB);
        SynText->SetText(FText::FromString(Display));

        FLinearColor Color = Syn.bIsActive ?
            FLinearColor(0.3f, 1.0f, 0.5f) : FLinearColor(0.4f, 0.4f, 0.4f);
        SynText->SetColorAndOpacity(FSlateColor(Color));

        FSlateFontInfo Font = SynText->GetFont();
        Font.Size = 11;
        SynText->SetFont(Font);

        SynergyListBox->AddChild(SynText);
    }
}

// ---------------------------------------------------------------------------
// Role Indicators
// ---------------------------------------------------------------------------

void USpecializationWidget::UpdateRoleIndicators()
{
    ECombatRole Primary = GetPrimaryRole();
    ECombatRole Secondary = GetSecondaryRole();

    if (PrimaryRoleText)
    {
        PrimaryRoleText->SetText(FText::FromString(
            FString::Printf(TEXT("Primary: %s"), *GetRoleName(Primary))));
        PrimaryRoleText->SetColorAndOpacity(FSlateColor(GetRoleColor(Primary)));
    }

    if (PrimaryRoleIcon)
    {
        PrimaryRoleIcon->SetColorAndOpacity(GetRoleColor(Primary));
    }

    if (SecondaryRoleText)
    {
        SecondaryRoleText->SetText(FText::FromString(
            FString::Printf(TEXT("Secondary: %s"), *GetRoleName(Secondary))));
        SecondaryRoleText->SetColorAndOpacity(FSlateColor(GetRoleColor(Secondary)));
    }

    if (SecondaryRoleIcon)
    {
        SecondaryRoleIcon->SetColorAndOpacity(GetRoleColor(Secondary));
    }
}

// ---------------------------------------------------------------------------
// Button State
// ---------------------------------------------------------------------------

void USpecializationWidget::UpdateButtonStates()
{
    // Confirm: enabled only if a branch is pending and domain not already specialized
    bool bCanConfirm = !PendingBranchId.IsEmpty();
    if (bCanConfirm)
    {
        const TArray<FSpecBranchDisplay>* Branches = DomainBranches.Find(SelectedDomain);
        if (Branches)
        {
            for (const FSpecBranchDisplay& B : *Branches)
            {
                if (B.bIsChosen)
                {
                    bCanConfirm = false; // Already specialized in this domain
                    break;
                }
            }
        }
    }

    if (ConfirmButton)
        ConfirmButton->SetIsEnabled(bCanConfirm);

    // Reset: enabled only if domain has a chosen branch
    bool bCanReset = false;
    const TArray<FSpecBranchDisplay>* Branches = DomainBranches.Find(SelectedDomain);
    if (Branches)
    {
        for (const FSpecBranchDisplay& B : *Branches)
        {
            if (B.bIsChosen) { bCanReset = true; break; }
        }
    }

    if (ResetButton)
        ResetButton->SetIsEnabled(bCanReset);
}

// ---------------------------------------------------------------------------
// Parsing Helpers
// ---------------------------------------------------------------------------

EMasteryDomain USpecializationWidget::ParseDomainString(const FString& Str) const
{
    if (Str == TEXT("SwordMastery")) return EMasteryDomain::SwordMastery;
    if (Str == TEXT("GreatswordMastery")) return EMasteryDomain::GreatswordMastery;
    if (Str == TEXT("DaggerMastery")) return EMasteryDomain::DaggerMastery;
    if (Str == TEXT("SpearMastery")) return EMasteryDomain::SpearMastery;
    if (Str == TEXT("GauntletMastery")) return EMasteryDomain::GauntletMastery;
    if (Str == TEXT("StaffMastery")) return EMasteryDomain::StaffMastery;
    if (Str == TEXT("ParryMastery")) return EMasteryDomain::ParryMastery;
    if (Str == TEXT("DodgeMastery")) return EMasteryDomain::DodgeMastery;
    if (Str == TEXT("BlockMastery")) return EMasteryDomain::BlockMastery;
    if (Str == TEXT("AerialMastery")) return EMasteryDomain::AerialMastery;
    if (Str == TEXT("Blacksmithing")) return EMasteryDomain::Blacksmithing;
    if (Str == TEXT("Alchemy")) return EMasteryDomain::Alchemy;
    if (Str == TEXT("Enchanting")) return EMasteryDomain::Enchanting;
    if (Str == TEXT("Tailoring")) return EMasteryDomain::Tailoring;
    if (Str == TEXT("Cooking")) return EMasteryDomain::Cooking;
    if (Str == TEXT("Mining")) return EMasteryDomain::Mining;
    if (Str == TEXT("Herbalism")) return EMasteryDomain::Herbalism;
    if (Str == TEXT("Salvaging")) return EMasteryDomain::Salvaging;
    if (Str == TEXT("Trading")) return EMasteryDomain::Trading;
    if (Str == TEXT("Exploration")) return EMasteryDomain::Exploration;
    if (Str == TEXT("SemanticAttunement")) return EMasteryDomain::SemanticAttunement;
    return EMasteryDomain::SwordMastery;
}

ECombatRole USpecializationWidget::ParseRole(const FString& Str) const
{
    if (Str == TEXT("Vanguard")) return ECombatRole::Vanguard;
    if (Str == TEXT("Striker")) return ECombatRole::Striker;
    if (Str == TEXT("Support")) return ECombatRole::Support;
    if (Str == TEXT("Sentinel")) return ECombatRole::Sentinel;
    if (Str == TEXT("Specialist")) return ECombatRole::Specialist;
    return ECombatRole::Vanguard;
}

ECombatRole USpecializationWidget::ComputeRole(bool bPrimary) const
{
    // Count role affinities from all chosen branches
    TMap<ECombatRole, int32> RoleCounts;
    RoleCounts.Add(ECombatRole::Vanguard, 0);
    RoleCounts.Add(ECombatRole::Striker, 0);
    RoleCounts.Add(ECombatRole::Support, 0);
    RoleCounts.Add(ECombatRole::Sentinel, 0);
    RoleCounts.Add(ECombatRole::Specialist, 0);

    for (const auto& DomainPair : DomainBranches)
    {
        for (const FSpecBranchDisplay& Branch : DomainPair.Value)
        {
            if (Branch.bIsChosen)
            {
                int32& Count = RoleCounts.FindOrAdd(Branch.RoleAffinity);
                Count++;
            }
        }
    }

    // Sort by count descending
    TArray<TPair<ECombatRole, int32>> Sorted;
    for (const auto& Pair : RoleCounts)
    {
        Sorted.Add(TPair<ECombatRole, int32>(Pair.Key, Pair.Value));
    }
    Sorted.Sort([](const TPair<ECombatRole, int32>& A, const TPair<ECombatRole, int32>& B) {
        return A.Value > B.Value;
    });

    if (Sorted.Num() == 0) return ECombatRole::Vanguard;

    if (bPrimary)
    {
        return Sorted[0].Key;
    }
    else
    {
        // Secondary: second highest count (or same as primary if only one)
        return Sorted.Num() > 1 ? Sorted[1].Key : Sorted[0].Key;
    }
}

// ---------------------------------------------------------------------------
// Callbacks
// ---------------------------------------------------------------------------

void USpecializationWidget::OnDomainChanged(FString SelectedItem, ESelectInfo::Type SelectionType)
{
    SelectedDomain = ParseDomainString(SelectedItem);
    PendingBranchId.Empty();

    RebuildBranchCards();
    UpdateButtonStates();
}

void USpecializationWidget::OnLeftSelectClicked()
{
    const TArray<FSpecBranchDisplay>* Branches = DomainBranches.Find(SelectedDomain);
    if (Branches && Branches->Num() > 0)
    {
        SelectBranch((*Branches)[0].Id);
    }
}

void USpecializationWidget::OnRightSelectClicked()
{
    const TArray<FSpecBranchDisplay>* Branches = DomainBranches.Find(SelectedDomain);
    if (Branches && Branches->Num() > 1)
    {
        SelectBranch((*Branches)[1].Id);
    }
}

void USpecializationWidget::OnConfirmClicked()
{
    ConfirmSelection();
}

void USpecializationWidget::OnResetClicked()
{
    ResetBranch(SelectedDomain);
}
