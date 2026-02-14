#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "GuildWidget.generated.h"

class UTextBlock;
class UButton;
class UScrollBox;
class UVerticalBox;
class UProgressBar;
class UEditableTextBox;

/// Guild rank — mirrors Rust GuildRank
UENUM(BlueprintType)
enum class EGuildRank : uint8
{
    Recruit,
    Member,
    Officer,
    ViceLeader,
    Leader,
};

/// Guild member display
USTRUCT(BlueprintType)
struct FGuildMemberDisplay
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString UserId;
    UPROPERTY(BlueprintReadWrite) FString Name;
    UPROPERTY(BlueprintReadWrite) EGuildRank Rank = EGuildRank::Recruit;
    UPROPERTY(BlueprintReadWrite) int64 Contribution = 0;
    UPROPERTY(BlueprintReadWrite) bool bOnline = false;
    UPROPERTY(BlueprintReadWrite) FString LastOnline;
};

/// Guild display data
USTRUCT(BlueprintType)
struct FGuildDisplayData
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString GuildId;
    UPROPERTY(BlueprintReadWrite) FString Name;
    UPROPERTY(BlueprintReadWrite) FString Tag;
    UPROPERTY(BlueprintReadWrite) FString Description;
    UPROPERTY(BlueprintReadWrite) FString Faction;
    UPROPERTY(BlueprintReadWrite) int32 MemberCount = 0;
    UPROPERTY(BlueprintReadWrite) int32 MaxMembers = 50;
    UPROPERTY(BlueprintReadWrite) int32 GuildLevel = 1;
    UPROPERTY(BlueprintReadWrite) int64 GuildXP = 0;
    UPROPERTY(BlueprintReadWrite) int64 BankShards = 0;
    UPROPERTY(BlueprintReadWrite) FString MOTD;
    UPROPERTY(BlueprintReadWrite) TArray<FGuildMemberDisplay> Members;
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnGuildAction, const FString&, ActionType);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnMemberAction, const FString&, UserId);

/**
 * Guild management widget.
 * Member list, rank management, guild info, MOTD.
 * Mirrors Rust social::Guild struct.
 */
UCLASS()
class TOWERGAME_API UGuildWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;

    UFUNCTION(BlueprintCallable) void LoadFromJson(const FString& GuildJson);
    UFUNCTION(BlueprintCallable) void SetMyRank(EGuildRank Rank);
    UFUNCTION(BlueprintCallable) void SelectMember(const FString& UserId);
    UFUNCTION(BlueprintCallable) void InvitePlayer(const FString& PlayerName);
    UFUNCTION(BlueprintCallable) void KickMember(const FString& UserId);
    UFUNCTION(BlueprintCallable) void PromoteMember(const FString& UserId);
    UFUNCTION(BlueprintCallable) void UpdateMOTD(const FString& NewMOTD);
    UFUNCTION(BlueprintCallable) void LeaveGuild();

    UFUNCTION(BlueprintPure) FGuildDisplayData GetGuildData() const { return GuildData; }
    UFUNCTION(BlueprintPure) bool CanInvite() const;
    UFUNCTION(BlueprintPure) bool CanKick() const;
    UFUNCTION(BlueprintPure) bool CanPromote() const;

    UPROPERTY(BlueprintAssignable) FOnGuildAction OnGuildAction;
    UPROPERTY(BlueprintAssignable) FOnMemberAction OnMemberAction;

protected:
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* GuildNameText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* GuildTagText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* GuildLevelText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* MemberCountText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* MOTDText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* BankText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UScrollBox* MemberListBox = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* InviteButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* LeaveButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* KickButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* PromoteButton = nullptr;

    // Selected member detail
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* SelectedMemberName = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* SelectedMemberRank = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* SelectedMemberContrib = nullptr;

    FGuildDisplayData GuildData;
    EGuildRank MyRank = EGuildRank::Recruit;
    FString SelectedMemberId;

    void RebuildDisplay();
    FString GetRankName(EGuildRank Rank) const;
    FLinearColor GetRankColor(EGuildRank Rank) const;

    UFUNCTION() void OnInviteClicked();
    UFUNCTION() void OnLeaveClicked();
    UFUNCTION() void OnKickClicked();
    UFUNCTION() void OnPromoteClicked();
};
