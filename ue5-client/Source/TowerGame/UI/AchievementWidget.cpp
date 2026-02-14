#include "AchievementWidget.h"
#include "Components/TextBlock.h"
#include "Components/ProgressBar.h"
#include "Components/ScrollBox.h"
#include "Components/HorizontalBox.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

void UAchievementWidget::NativeConstruct()
{
    Super::NativeConstruct();

    bFilterActive = false;
    RebuildList();
}

void UAchievementWidget::LoadFromJson(const FString& AchievementsJson)
{
    AllAchievements.Empty();

    TSharedPtr<FJsonValue> Parsed;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(AchievementsJson);
    if (!FJsonSerializer::Deserialize(Reader, Parsed)) return;

    const TArray<TSharedPtr<FJsonValue>>* Items;
    if (!Parsed->TryGetArray(Items)) return;

    for (const auto& Val : *Items)
    {
        const TSharedPtr<FJsonObject>& Obj = Val->AsObject();
        if (!Obj) continue;

        FAchievementDisplayData Data;
        Data.Id = Obj->GetStringField(TEXT("id"));
        Data.Name = Obj->GetStringField(TEXT("name"));
        Data.Description = Obj->GetStringField(TEXT("description"));
        Data.Category = ParseCategory(Obj->GetStringField(TEXT("category")));
        Data.Tier = ParseTier(Obj->GetStringField(TEXT("tier")));
        Data.Progress = Obj->GetNumberField(TEXT("progress"));
        Data.bUnlocked = Obj->GetBoolField(TEXT("unlocked"));
        Data.bHidden = Obj->GetBoolField(TEXT("hidden"));
        Data.ShardReward = Obj->GetIntegerField(TEXT("shard_reward"));

        AllAchievements.Add(Data);
    }

    UE_LOG(LogTemp, Log, TEXT("Loaded %d achievements"), AllAchievements.Num());
    RebuildList();
}

void UAchievementWidget::AddAchievement(const FAchievementDisplayData& Data)
{
    AllAchievements.Add(Data);
    RebuildList();
}

void UAchievementWidget::UpdateProgress(const FString& AchievementId, float NewProgress)
{
    for (auto& Ach : AllAchievements)
    {
        if (Ach.Id == AchievementId)
        {
            Ach.Progress = FMath::Clamp(NewProgress, 0.0f, 1.0f);
            if (NewProgress >= 1.0f && !Ach.bUnlocked)
            {
                Ach.bUnlocked = true;
                ShowUnlockToast(AchievementId);
            }
            break;
        }
    }
    RebuildList();
}

void UAchievementWidget::MarkUnlocked(const FString& AchievementId)
{
    for (auto& Ach : AllAchievements)
    {
        if (Ach.Id == AchievementId)
        {
            Ach.bUnlocked = true;
            Ach.Progress = 1.0f;
            break;
        }
    }
    RebuildList();
}

void UAchievementWidget::FilterByCategory(EAchievementCategory Category)
{
    CurrentFilter = Category;
    bFilterActive = true;
    if (CategoryFilterText)
        CategoryFilterText->SetText(FText::FromString(GetCategoryName(Category)));
    RebuildList();
}

void UAchievementWidget::ShowAll()
{
    bFilterActive = false;
    if (CategoryFilterText)
        CategoryFilterText->SetText(FText::FromString(TEXT("All")));
    RebuildList();
}

void UAchievementWidget::ToggleHidden(bool bShow)
{
    bShowHidden = bShow;
    RebuildList();
}

int32 UAchievementWidget::GetUnlockedCount() const
{
    int32 Count = 0;
    for (const auto& Ach : AllAchievements)
    {
        if (Ach.bUnlocked) Count++;
    }
    return Count;
}

float UAchievementWidget::GetOverallProgress() const
{
    if (AllAchievements.Num() == 0) return 0.0f;
    float Sum = 0.0f;
    for (const auto& Ach : AllAchievements)
    {
        Sum += Ach.Progress;
    }
    return Sum / AllAchievements.Num();
}

TArray<FAchievementCategoryTab> UAchievementWidget::GetCategoryTabs() const
{
    TArray<FAchievementCategoryTab> Tabs;

    // All 8 categories
    for (uint8 i = 0; i <= static_cast<uint8>(EAchievementCategory::Tower); i++)
    {
        FAchievementCategoryTab Tab;
        Tab.Category = static_cast<EAchievementCategory>(i);
        Tab.Total = 0;
        Tab.Unlocked = 0;

        for (const auto& Ach : AllAchievements)
        {
            if (Ach.Category == Tab.Category)
            {
                Tab.Total++;
                if (Ach.bUnlocked) Tab.Unlocked++;
            }
        }

        Tabs.Add(Tab);
    }

    return Tabs;
}

void UAchievementWidget::ShowUnlockToast(const FString& AchievementId)
{
    for (const auto& Ach : AllAchievements)
    {
        if (Ach.Id == AchievementId)
        {
            if (ToastTitle)
                ToastTitle->SetText(FText::FromString(
                    FString::Printf(TEXT("Achievement Unlocked!"))));
            if (ToastDesc)
                ToastDesc->SetText(FText::FromString(
                    FString::Printf(TEXT("%s — %d Shards"), *Ach.Name, Ach.ShardReward)));

            UE_LOG(LogTemp, Log, TEXT("Achievement unlocked: %s (+%d shards)"),
                *Ach.Name, Ach.ShardReward);
            break;
        }
    }
}

void UAchievementWidget::RebuildList()
{
    // Update overall progress
    if (OverallProgressBar)
        OverallProgressBar->SetPercent(GetOverallProgress());

    int32 Unlocked = GetUnlockedCount();
    int32 Total = AllAchievements.Num();
    if (TotalProgressText)
        TotalProgressText->SetText(FText::FromString(
            FString::Printf(TEXT("%d / %d"), Unlocked, Total)));

    // List is rebuilt by Blueprint (UMG ScrollBox items)
    // This code provides data; visual entry creation uses UMG patterns
}

FLinearColor UAchievementWidget::GetCategoryColor(EAchievementCategory Category) const
{
    switch (Category)
    {
    case EAchievementCategory::Combat:      return FLinearColor(1.0f, 0.2f, 0.2f);  // red
    case EAchievementCategory::Exploration: return FLinearColor(0.2f, 0.8f, 0.4f);  // green
    case EAchievementCategory::Semantic:    return FLinearColor(0.6f, 0.2f, 1.0f);  // purple
    case EAchievementCategory::Social:      return FLinearColor(0.2f, 0.6f, 1.0f);  // blue
    case EAchievementCategory::Crafting:    return FLinearColor(1.0f, 0.7f, 0.2f);  // orange
    case EAchievementCategory::Survival:    return FLinearColor(0.5f, 0.5f, 0.5f);  // gray
    case EAchievementCategory::Mastery:     return FLinearColor(1.0f, 0.85f, 0.0f); // gold
    case EAchievementCategory::Tower:       return FLinearColor(0.0f, 0.8f, 0.8f);  // cyan
    default:                                return FLinearColor::White;
    }
}

FLinearColor UAchievementWidget::GetTierColor(EAchievementTier Tier) const
{
    switch (Tier)
    {
    case EAchievementTier::Bronze:   return FLinearColor(0.8f, 0.5f, 0.2f);
    case EAchievementTier::Silver:   return FLinearColor(0.75f, 0.75f, 0.8f);
    case EAchievementTier::Gold:     return FLinearColor(1.0f, 0.84f, 0.0f);
    case EAchievementTier::Platinum: return FLinearColor(0.7f, 0.9f, 1.0f);
    case EAchievementTier::Mythic:   return FLinearColor(1.0f, 0.4f, 0.8f);
    default:                         return FLinearColor::White;
    }
}

FString UAchievementWidget::GetCategoryName(EAchievementCategory Category) const
{
    switch (Category)
    {
    case EAchievementCategory::Combat:      return TEXT("Combat");
    case EAchievementCategory::Exploration: return TEXT("Exploration");
    case EAchievementCategory::Semantic:    return TEXT("Semantic");
    case EAchievementCategory::Social:      return TEXT("Social");
    case EAchievementCategory::Crafting:    return TEXT("Crafting");
    case EAchievementCategory::Survival:    return TEXT("Survival");
    case EAchievementCategory::Mastery:     return TEXT("Mastery");
    case EAchievementCategory::Tower:       return TEXT("Tower");
    default:                                return TEXT("Unknown");
    }
}

FString UAchievementWidget::GetTierName(EAchievementTier Tier) const
{
    switch (Tier)
    {
    case EAchievementTier::Bronze:   return TEXT("Bronze");
    case EAchievementTier::Silver:   return TEXT("Silver");
    case EAchievementTier::Gold:     return TEXT("Gold");
    case EAchievementTier::Platinum: return TEXT("Platinum");
    case EAchievementTier::Mythic:   return TEXT("Mythic");
    default:                         return TEXT("Unknown");
    }
}

FString UAchievementWidget::GetCategoryIcon(EAchievementCategory Category) const
{
    switch (Category)
    {
    case EAchievementCategory::Combat:      return TEXT("[Sword]");
    case EAchievementCategory::Exploration: return TEXT("[Compass]");
    case EAchievementCategory::Semantic:    return TEXT("[Star]");
    case EAchievementCategory::Social:      return TEXT("[People]");
    case EAchievementCategory::Crafting:    return TEXT("[Anvil]");
    case EAchievementCategory::Survival:    return TEXT("[Skull]");
    case EAchievementCategory::Mastery:     return TEXT("[Crown]");
    case EAchievementCategory::Tower:       return TEXT("[Tower]");
    default:                                return TEXT("[?]");
    }
}

EAchievementCategory UAchievementWidget::ParseCategory(const FString& Str) const
{
    if (Str == TEXT("Combat"))      return EAchievementCategory::Combat;
    if (Str == TEXT("Exploration")) return EAchievementCategory::Exploration;
    if (Str == TEXT("Semantic"))    return EAchievementCategory::Semantic;
    if (Str == TEXT("Social"))      return EAchievementCategory::Social;
    if (Str == TEXT("Crafting"))    return EAchievementCategory::Crafting;
    if (Str == TEXT("Survival"))    return EAchievementCategory::Survival;
    if (Str == TEXT("Mastery"))     return EAchievementCategory::Mastery;
    if (Str == TEXT("Tower"))       return EAchievementCategory::Tower;
    return EAchievementCategory::Combat;
}

EAchievementTier UAchievementWidget::ParseTier(const FString& Str) const
{
    if (Str == TEXT("Bronze"))   return EAchievementTier::Bronze;
    if (Str == TEXT("Silver"))   return EAchievementTier::Silver;
    if (Str == TEXT("Gold"))     return EAchievementTier::Gold;
    if (Str == TEXT("Platinum")) return EAchievementTier::Platinum;
    if (Str == TEXT("Mythic"))   return EAchievementTier::Mythic;
    return EAchievementTier::Bronze;
}
