#include "GuildWidget.h"
#include "Components/TextBlock.h"
#include "Components/Button.h"
#include "Components/ScrollBox.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

void UGuildWidget::NativeConstruct()
{
    Super::NativeConstruct();

    if (InviteButton)
        InviteButton->OnClicked.AddDynamic(this, &UGuildWidget::OnInviteClicked);
    if (LeaveButton)
        LeaveButton->OnClicked.AddDynamic(this, &UGuildWidget::OnLeaveClicked);
    if (KickButton)
        KickButton->OnClicked.AddDynamic(this, &UGuildWidget::OnKickClicked);
    if (PromoteButton)
        PromoteButton->OnClicked.AddDynamic(this, &UGuildWidget::OnPromoteClicked);

    RebuildDisplay();
}

void UGuildWidget::LoadFromJson(const FString& GuildJson)
{
    TSharedPtr<FJsonValue> Parsed;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(GuildJson);
    if (!FJsonSerializer::Deserialize(Reader, Parsed)) return;

    const TSharedPtr<FJsonObject>& Obj = Parsed->AsObject();
    if (!Obj) return;

    GuildData.GuildId = Obj->GetStringField(TEXT("id"));
    GuildData.Name = Obj->GetStringField(TEXT("name"));
    GuildData.Tag = Obj->GetStringField(TEXT("tag"));
    GuildData.Description = Obj->GetStringField(TEXT("description"));
    GuildData.GuildLevel = Obj->GetIntegerField(TEXT("guild_level"));
    GuildData.GuildXP = Obj->GetIntegerField(TEXT("guild_xp"));
    GuildData.BankShards = Obj->GetIntegerField(TEXT("bank_shards"));
    GuildData.MaxMembers = Obj->GetIntegerField(TEXT("max_members"));

    // Parse MOTD from settings
    const TSharedPtr<FJsonObject>* Settings;
    if (Obj->TryGetObjectField(TEXT("settings"), Settings))
    {
        GuildData.MOTD = (*Settings)->GetStringField(TEXT("motd"));
    }

    // Parse members
    GuildData.Members.Empty();
    const TArray<TSharedPtr<FJsonValue>>* Members;
    if (Obj->TryGetArrayField(TEXT("members"), Members))
    {
        for (const auto& MVal : *Members)
        {
            const TSharedPtr<FJsonObject>& MObj = MVal->AsObject();
            if (!MObj) continue;

            FGuildMemberDisplay Member;
            Member.UserId = MObj->GetStringField(TEXT("user_id"));
            Member.Name = MObj->GetStringField(TEXT("name"));
            Member.Contribution = MObj->GetIntegerField(TEXT("contribution"));
            Member.bOnline = MObj->GetBoolField(TEXT("online"));

            FString RankStr = MObj->GetStringField(TEXT("rank"));
            if (RankStr == TEXT("Leader")) Member.Rank = EGuildRank::Leader;
            else if (RankStr == TEXT("ViceLeader")) Member.Rank = EGuildRank::ViceLeader;
            else if (RankStr == TEXT("Officer")) Member.Rank = EGuildRank::Officer;
            else if (RankStr == TEXT("Member")) Member.Rank = EGuildRank::Member;
            else Member.Rank = EGuildRank::Recruit;

            GuildData.Members.Add(Member);
        }
    }

    GuildData.MemberCount = GuildData.Members.Num();

    UE_LOG(LogTemp, Log, TEXT("Loaded guild: %s [%s] (%d members)"),
        *GuildData.Name, *GuildData.Tag, GuildData.MemberCount);

    RebuildDisplay();
}

void UGuildWidget::SetMyRank(EGuildRank Rank)
{
    MyRank = Rank;
    RebuildDisplay();
}

void UGuildWidget::SelectMember(const FString& UserId)
{
    SelectedMemberId = UserId;

    for (const auto& Member : GuildData.Members)
    {
        if (Member.UserId == UserId)
        {
            if (SelectedMemberName)
                SelectedMemberName->SetText(FText::FromString(Member.Name));
            if (SelectedMemberRank)
            {
                SelectedMemberRank->SetText(FText::FromString(GetRankName(Member.Rank)));
                SelectedMemberRank->SetColorAndOpacity(FSlateColor(GetRankColor(Member.Rank)));
            }
            if (SelectedMemberContrib)
                SelectedMemberContrib->SetText(FText::FromString(
                    FString::Printf(TEXT("Contribution: %lld"), Member.Contribution)));
            break;
        }
    }

    // Update button states based on my rank vs selected member's rank
    if (KickButton) KickButton->SetIsEnabled(CanKick());
    if (PromoteButton) PromoteButton->SetIsEnabled(CanPromote());
}

void UGuildWidget::InvitePlayer(const FString& PlayerName)
{
    if (!CanInvite()) return;
    OnGuildAction.Broadcast(TEXT("invite"));
    OnMemberAction.Broadcast(PlayerName);
}

void UGuildWidget::KickMember(const FString& UserId)
{
    if (!CanKick()) return;
    OnGuildAction.Broadcast(TEXT("kick"));
    OnMemberAction.Broadcast(UserId);
}

void UGuildWidget::PromoteMember(const FString& UserId)
{
    if (!CanPromote()) return;
    OnGuildAction.Broadcast(TEXT("promote"));
    OnMemberAction.Broadcast(UserId);
}

void UGuildWidget::UpdateMOTD(const FString& NewMOTD)
{
    GuildData.MOTD = NewMOTD;
    if (MOTDText)
        MOTDText->SetText(FText::FromString(NewMOTD));
    OnGuildAction.Broadcast(TEXT("update_motd"));
}

void UGuildWidget::LeaveGuild()
{
    OnGuildAction.Broadcast(TEXT("leave"));
}

bool UGuildWidget::CanInvite() const  { return MyRank >= EGuildRank::Officer; }
bool UGuildWidget::CanKick() const    { return MyRank >= EGuildRank::Officer; }
bool UGuildWidget::CanPromote() const { return MyRank >= EGuildRank::ViceLeader; }

void UGuildWidget::RebuildDisplay()
{
    if (GuildNameText)
        GuildNameText->SetText(FText::FromString(GuildData.Name));
    if (GuildTagText)
        GuildTagText->SetText(FText::FromString(FString::Printf(TEXT("[%s]"), *GuildData.Tag)));
    if (GuildLevelText)
        GuildLevelText->SetText(FText::FromString(FString::Printf(TEXT("Level %d"), GuildData.GuildLevel)));
    if (MemberCountText)
        MemberCountText->SetText(FText::FromString(
            FString::Printf(TEXT("%d / %d"), GuildData.MemberCount, GuildData.MaxMembers)));
    if (MOTDText)
        MOTDText->SetText(FText::FromString(GuildData.MOTD));
    if (BankText)
        BankText->SetText(FText::FromString(FString::Printf(TEXT("%lld Shards"), GuildData.BankShards)));

    // Button permissions
    if (InviteButton) InviteButton->SetIsEnabled(CanInvite());
    if (KickButton) KickButton->SetIsEnabled(CanKick());
    if (PromoteButton) PromoteButton->SetIsEnabled(CanPromote());
}

FString UGuildWidget::GetRankName(EGuildRank Rank) const
{
    switch (Rank)
    {
    case EGuildRank::Recruit:    return TEXT("Recruit");
    case EGuildRank::Member:     return TEXT("Member");
    case EGuildRank::Officer:    return TEXT("Officer");
    case EGuildRank::ViceLeader: return TEXT("Vice Leader");
    case EGuildRank::Leader:     return TEXT("Guild Leader");
    default:                     return TEXT("Unknown");
    }
}

FLinearColor UGuildWidget::GetRankColor(EGuildRank Rank) const
{
    switch (Rank)
    {
    case EGuildRank::Recruit:    return FLinearColor(0.6f, 0.6f, 0.6f);  // gray
    case EGuildRank::Member:     return FLinearColor::White;
    case EGuildRank::Officer:    return FLinearColor(0.3f, 0.7f, 1.0f);  // blue
    case EGuildRank::ViceLeader: return FLinearColor(0.7f, 0.3f, 1.0f);  // purple
    case EGuildRank::Leader:     return FLinearColor(1.0f, 0.7f, 0.0f);  // gold
    default:                     return FLinearColor::White;
    }
}

void UGuildWidget::OnInviteClicked()  { OnGuildAction.Broadcast(TEXT("invite_dialog")); }
void UGuildWidget::OnLeaveClicked()   { LeaveGuild(); }
void UGuildWidget::OnKickClicked()    { if (!SelectedMemberId.IsEmpty()) KickMember(SelectedMemberId); }
void UGuildWidget::OnPromoteClicked() { if (!SelectedMemberId.IsEmpty()) PromoteMember(SelectedMemberId); }
