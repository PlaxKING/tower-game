#include "SkillTreeWidget.h"
#include "Components/TextBlock.h"
#include "Components/Button.h"
#include "Components/ProgressBar.h"
#include "Components/ScrollBox.h"
#include "Components/VerticalBox.h"
#include "Components/HorizontalBox.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

void USkillTreeWidget::NativeConstruct()
{
    Super::NativeConstruct();

    if (UnlockButton)
        UnlockButton->OnClicked.AddDynamic(this, &USkillTreeWidget::OnUnlockClicked);

    RebuildDisplay();
}

void USkillTreeWidget::LoadFromJson(const FString& MasteryJson)
{
    AllMasteries.Empty();

    TSharedPtr<FJsonValue> Parsed;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(MasteryJson);
    if (!FJsonSerializer::Deserialize(Reader, Parsed)) return;

    const TSharedPtr<FJsonObject>& Root = Parsed->AsObject();
    if (!Root) return;

    const TSharedPtr<FJsonObject>* MasteriesObj;
    if (!Root->TryGetObjectField(TEXT("masteries"), MasteriesObj)) return;

    for (const auto& Pair : (*MasteriesObj)->Values)
    {
        const TSharedPtr<FJsonObject>& MObj = Pair.Value->AsObject();
        if (!MObj) continue;

        FMasteryProgressDisplay Progress;
        Progress.Domain = ParseDomain(MObj->GetStringField(TEXT("domain")));
        Progress.DomainName = Pair.Key;
        Progress.XP = MObj->GetIntegerField(TEXT("xp"));
        Progress.Tier = ParseTier(MObj->GetStringField(TEXT("tier")));

        // Parse unlocked nodes
        const TArray<TSharedPtr<FJsonValue>>* UnlockedNodes;
        if (MObj->TryGetArrayField(TEXT("unlocked_nodes"), UnlockedNodes))
        {
            for (const auto& NodeVal : *UnlockedNodes)
            {
                FSkillNodeDisplay Node;
                Node.NodeId = NodeVal->AsString();
                Node.bUnlocked = true;
                Progress.Nodes.Add(Node);
            }
        }

        AllMasteries.Add(Progress.Domain, Progress);
    }

    UE_LOG(LogTemp, Log, TEXT("Loaded %d mastery domains"), AllMasteries.Num());
    RebuildDisplay();
}

void USkillTreeWidget::SelectDomain(EMasteryDomain Domain)
{
    CurrentDomain = Domain;
    SelectedNodeId = TEXT("");
    RebuildDisplay();
}

void USkillTreeWidget::SelectCategory(EMasteryCategory Category)
{
    // Select first domain in category
    for (const auto& Pair : AllMasteries)
    {
        if (GetDomainCategory(Pair.Key) == Category)
        {
            SelectDomain(Pair.Key);
            return;
        }
    }
}

void USkillTreeWidget::SelectNode(const FString& NodeId)
{
    SelectedNodeId = NodeId;
    OnNodeSelected.Broadcast(CurrentDomain, NodeId);

    // Update detail panel
    FMasteryProgressDisplay* Current = AllMasteries.Find(CurrentDomain);
    if (!Current) return;

    for (const auto& Node : Current->Nodes)
    {
        if (Node.NodeId == NodeId)
        {
            if (NodeNameText) NodeNameText->SetText(FText::FromString(Node.Name));
            if (NodeDescText) NodeDescText->SetText(FText::FromString(Node.Description));
            if (NodeRequirementText)
                NodeRequirementText->SetText(FText::FromString(
                    FString::Printf(TEXT("Requires: %s"), *GetTierName(Node.RequiredTier))));
            if (UnlockButton)
                UnlockButton->SetIsEnabled(Node.bCanUnlock && !Node.bUnlocked);
            break;
        }
    }
}

void USkillTreeWidget::UnlockSelectedNode()
{
    if (SelectedNodeId.IsEmpty()) return;
    OnNodeUnlocked.Broadcast(CurrentDomain, SelectedNodeId);
}

FMasteryProgressDisplay USkillTreeWidget::GetCurrentDomain() const
{
    const FMasteryProgressDisplay* Found = AllMasteries.Find(CurrentDomain);
    return Found ? *Found : FMasteryProgressDisplay();
}

TArray<FMasteryProgressDisplay> USkillTreeWidget::GetDomainsInCategory(EMasteryCategory Category) const
{
    TArray<FMasteryProgressDisplay> Result;
    for (const auto& Pair : AllMasteries)
    {
        if (GetDomainCategory(Pair.Key) == Category)
        {
            Result.Add(Pair.Value);
        }
    }
    return Result;
}

void USkillTreeWidget::RebuildDisplay()
{
    FMasteryProgressDisplay* Current = AllMasteries.Find(CurrentDomain);
    if (!Current) return;

    if (DomainNameText)
        DomainNameText->SetText(FText::FromString(Current->DomainName));
    if (TierText)
    {
        FString TierStr = GetTierName(Current->Tier);
        TierText->SetText(FText::FromString(TierStr));
        TierText->SetColorAndOpacity(FSlateColor(GetTierColor(Current->Tier)));
    }
    if (TierProgressBar)
        TierProgressBar->SetPercent(Current->TierProgress);
    if (XPText)
        XPText->SetText(FText::FromString(FString::Printf(TEXT("%lld XP"), Current->XP)));
}

FString USkillTreeWidget::GetTierName(EMasteryTier Tier) const
{
    switch (Tier)
    {
    case EMasteryTier::Novice:      return TEXT("Novice");
    case EMasteryTier::Apprentice:  return TEXT("Apprentice");
    case EMasteryTier::Journeyman:  return TEXT("Journeyman");
    case EMasteryTier::Expert:      return TEXT("Expert");
    case EMasteryTier::Master:      return TEXT("Master");
    case EMasteryTier::Grandmaster: return TEXT("Grandmaster");
    default:                        return TEXT("Unknown");
    }
}

FLinearColor USkillTreeWidget::GetTierColor(EMasteryTier Tier) const
{
    switch (Tier)
    {
    case EMasteryTier::Novice:      return FLinearColor(0.6f, 0.6f, 0.6f);   // gray
    case EMasteryTier::Apprentice:  return FLinearColor(0.3f, 0.8f, 0.3f);   // green
    case EMasteryTier::Journeyman:  return FLinearColor(0.2f, 0.6f, 1.0f);   // blue
    case EMasteryTier::Expert:      return FLinearColor(0.7f, 0.3f, 1.0f);   // purple
    case EMasteryTier::Master:      return FLinearColor(1.0f, 0.7f, 0.0f);   // gold
    case EMasteryTier::Grandmaster: return FLinearColor(1.0f, 0.2f, 0.2f);   // red
    default:                        return FLinearColor::White;
    }
}

FString USkillTreeWidget::GetCategoryName(EMasteryCategory Category) const
{
    switch (Category)
    {
    case EMasteryCategory::Weapon:          return TEXT("Weapons");
    case EMasteryCategory::CombatTechnique: return TEXT("Combat");
    case EMasteryCategory::Crafting:        return TEXT("Crafting");
    case EMasteryCategory::Gathering:       return TEXT("Gathering");
    case EMasteryCategory::Other:           return TEXT("Other");
    default:                                return TEXT("Unknown");
    }
}

EMasteryCategory USkillTreeWidget::GetDomainCategory(EMasteryDomain Domain) const
{
    switch (Domain)
    {
    case EMasteryDomain::SwordMastery:
    case EMasteryDomain::GreatswordMastery:
    case EMasteryDomain::DaggerMastery:
    case EMasteryDomain::SpearMastery:
    case EMasteryDomain::GauntletMastery:
    case EMasteryDomain::StaffMastery:
        return EMasteryCategory::Weapon;
    case EMasteryDomain::ParryMastery:
    case EMasteryDomain::DodgeMastery:
    case EMasteryDomain::BlockMastery:
    case EMasteryDomain::AerialMastery:
        return EMasteryCategory::CombatTechnique;
    case EMasteryDomain::Blacksmithing:
    case EMasteryDomain::Alchemy:
    case EMasteryDomain::Enchanting:
    case EMasteryDomain::Tailoring:
    case EMasteryDomain::Cooking:
        return EMasteryCategory::Crafting;
    case EMasteryDomain::Mining:
    case EMasteryDomain::Herbalism:
    case EMasteryDomain::Salvaging:
        return EMasteryCategory::Gathering;
    default:
        return EMasteryCategory::Other;
    }
}

EMasteryDomain USkillTreeWidget::ParseDomain(const FString& Str) const
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

EMasteryTier USkillTreeWidget::ParseTier(const FString& Str) const
{
    if (Str == TEXT("Novice")) return EMasteryTier::Novice;
    if (Str == TEXT("Apprentice")) return EMasteryTier::Apprentice;
    if (Str == TEXT("Journeyman")) return EMasteryTier::Journeyman;
    if (Str == TEXT("Expert")) return EMasteryTier::Expert;
    if (Str == TEXT("Master")) return EMasteryTier::Master;
    if (Str == TEXT("Grandmaster")) return EMasteryTier::Grandmaster;
    return EMasteryTier::Novice;
}

void USkillTreeWidget::OnUnlockClicked()
{
    UnlockSelectedNode();
}
