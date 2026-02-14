#include "TradeWidget.h"
#include "Components/TextBlock.h"
#include "Components/Button.h"
#include "Components/VerticalBox.h"

void UTradeWidget::NativeConstruct()
{
    Super::NativeConstruct();

    if (LockButton)
        LockButton->OnClicked.AddDynamic(this, &UTradeWidget::OnLockClicked);
    if (ConfirmButton)
        ConfirmButton->OnClicked.AddDynamic(this, &UTradeWidget::OnConfirmClicked);
    if (CancelButton)
        CancelButton->OnClicked.AddDynamic(this, &UTradeWidget::OnCancelClicked);

    RebuildDisplay();
}

void UTradeWidget::StartTrade(const FString& InOtherPlayerId, const FString& InOtherPlayerName)
{
    OtherPlayerId = InOtherPlayerId;
    OtherPlayerName = InOtherPlayerName;
    CurrentState = ETradeState::Proposing;
    MyItems.Empty();
    RemoteItems.Empty();
    MyShards = 0;
    RemoteShards = 0;
    bMyLocked = false;
    bRemoteLocked = false;
    bMyConfirmed = false;
    bRemoteConfirmed = false;

    if (OtherNameText)
        OtherNameText->SetText(FText::FromString(OtherPlayerName));

    RebuildDisplay();
    UE_LOG(LogTemp, Log, TEXT("Trade started with %s"), *OtherPlayerName);
}

void UTradeWidget::AddMyItem(const FTradeItemDisplay& Item)
{
    if (CurrentState != ETradeState::Proposing) return;
    if (MyItems.Num() >= MaxTradeItems) return;

    MyItems.Add(Item);
    OnItemAdded.Broadcast(Item);
    RebuildDisplay();
}

void UTradeWidget::RemoveMyItem(int32 Index)
{
    if (CurrentState != ETradeState::Proposing) return;
    if (!MyItems.IsValidIndex(Index)) return;

    MyItems.RemoveAt(Index);
    RebuildDisplay();
}

void UTradeWidget::SetMyShards(int64 Amount)
{
    if (CurrentState != ETradeState::Proposing) return;
    MyShards = FMath::Max(Amount, (int64)0);
    RebuildDisplay();
}

void UTradeWidget::LockTrade()
{
    if (CurrentState != ETradeState::Proposing) return;
    bMyLocked = true;
    if (bMyLocked && bRemoteLocked)
        CurrentState = ETradeState::Locked;
    RebuildDisplay();
}

void UTradeWidget::ConfirmTrade()
{
    if (CurrentState != ETradeState::Locked) return;
    bMyConfirmed = true;
    if (bMyConfirmed && bRemoteConfirmed)
    {
        CurrentState = ETradeState::Confirmed;
        OnConfirmed.Broadcast();
    }
    RebuildDisplay();
}

void UTradeWidget::CancelTrade()
{
    CurrentState = ETradeState::Cancelled;
    OnCancelled.Broadcast();
    RebuildDisplay();
}

void UTradeWidget::UpdateRemoteItems(const TArray<FTradeItemDisplay>& Items)
{
    RemoteItems = Items;
    RebuildDisplay();
}

void UTradeWidget::UpdateRemoteShards(int64 Amount)
{
    RemoteShards = Amount;
    RebuildDisplay();
}

void UTradeWidget::SetRemoteLocked(bool bLocked)
{
    bRemoteLocked = bLocked;
    if (bMyLocked && bRemoteLocked && CurrentState == ETradeState::Proposing)
        CurrentState = ETradeState::Locked;
    RebuildDisplay();
}

void UTradeWidget::SetRemoteConfirmed(bool bConfirmed)
{
    bRemoteConfirmed = bConfirmed;
    if (bMyConfirmed && bRemoteConfirmed && CurrentState == ETradeState::Locked)
    {
        CurrentState = ETradeState::Confirmed;
        OnConfirmed.Broadcast();
    }
    RebuildDisplay();
}

void UTradeWidget::SetTradeState(ETradeState NewState)
{
    CurrentState = NewState;
    RebuildDisplay();
}

void UTradeWidget::RebuildDisplay()
{
    // My items count
    if (MyShardsText)
        MyShardsText->SetText(FText::FromString(FString::Printf(TEXT("%lld Shards"), MyShards)));
    if (OtherShardsText)
        OtherShardsText->SetText(FText::FromString(FString::Printf(TEXT("%lld Shards"), RemoteShards)));

    // State text
    if (StateText)
        StateText->SetText(FText::FromString(GetStateText()));

    // Lock status
    if (MyLockStatus)
        MyLockStatus->SetText(FText::FromString(bMyLocked ? TEXT("LOCKED") : TEXT("Not Locked")));
    if (OtherLockStatus)
        OtherLockStatus->SetText(FText::FromString(bRemoteLocked ? TEXT("LOCKED") : TEXT("Not Locked")));

    // Button states
    if (LockButton)
        LockButton->SetIsEnabled(CurrentState == ETradeState::Proposing && !bMyLocked);
    if (ConfirmButton)
        ConfirmButton->SetIsEnabled(CurrentState == ETradeState::Locked && !bMyConfirmed);
    if (CancelButton)
        CancelButton->SetIsEnabled(CurrentState != ETradeState::Completed && CurrentState != ETradeState::Cancelled);
}

FLinearColor UTradeWidget::GetRarityColor(const FString& Rarity) const
{
    if (Rarity == TEXT("Common"))    return FLinearColor::White;
    if (Rarity == TEXT("Uncommon"))  return FLinearColor(0.2f, 0.8f, 0.2f);
    if (Rarity == TEXT("Rare"))      return FLinearColor(0.3f, 0.5f, 1.0f);
    if (Rarity == TEXT("Epic"))      return FLinearColor(0.6f, 0.2f, 1.0f);
    if (Rarity == TEXT("Legendary")) return FLinearColor(1.0f, 0.7f, 0.0f);
    if (Rarity == TEXT("Mythic"))    return FLinearColor(1.0f, 0.2f, 0.2f);
    return FLinearColor::White;
}

FString UTradeWidget::GetStateText() const
{
    switch (CurrentState)
    {
    case ETradeState::Proposing: return TEXT("Adding Items...");
    case ETradeState::Locked:    return TEXT("Review & Confirm");
    case ETradeState::Confirmed: return TEXT("Trade Confirmed!");
    case ETradeState::Completed: return TEXT("Trade Complete");
    case ETradeState::Cancelled: return TEXT("Trade Cancelled");
    default:                     return TEXT("Unknown");
    }
}

void UTradeWidget::OnLockClicked()    { LockTrade(); }
void UTradeWidget::OnConfirmClicked() { ConfirmTrade(); }
void UTradeWidget::OnCancelClicked()  { CancelTrade(); }
