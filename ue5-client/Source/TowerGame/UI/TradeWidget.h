#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "TradeWidget.generated.h"

class UTextBlock;
class UButton;
class UVerticalBox;
class UHorizontalBox;
class UEditableTextBox;

/// Trade state — mirrors Rust TradeState
UENUM(BlueprintType)
enum class ETradeState : uint8
{
    Proposing,
    Locked,
    Confirmed,
    Completed,
    Cancelled,
};

/// Item in trade
USTRUCT(BlueprintType)
struct FTradeItemDisplay
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString ItemName;
    UPROPERTY(BlueprintReadWrite) int32 Quantity = 1;
    UPROPERTY(BlueprintReadWrite) FString Rarity;
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnTradeConfirmed);
DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnTradeCancelled);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnTradeItemAdded, const FTradeItemDisplay&, Item);

/**
 * Player-to-player trade window.
 * Two-sided item/shard exchange with lock→confirm→execute flow.
 * Mirrors Rust social::Trade struct.
 */
UCLASS()
class TOWERGAME_API UTradeWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;

    // --- Trade Flow ---
    UFUNCTION(BlueprintCallable) void StartTrade(const FString& OtherPlayerId, const FString& OtherPlayerName);
    UFUNCTION(BlueprintCallable) void AddMyItem(const FTradeItemDisplay& Item);
    UFUNCTION(BlueprintCallable) void RemoveMyItem(int32 Index);
    UFUNCTION(BlueprintCallable) void SetMyShards(int64 Amount);
    UFUNCTION(BlueprintCallable) void LockTrade();
    UFUNCTION(BlueprintCallable) void ConfirmTrade();
    UFUNCTION(BlueprintCallable) void CancelTrade();

    // --- Remote side updates ---
    UFUNCTION(BlueprintCallable) void UpdateRemoteItems(const TArray<FTradeItemDisplay>& Items);
    UFUNCTION(BlueprintCallable) void UpdateRemoteShards(int64 Amount);
    UFUNCTION(BlueprintCallable) void SetRemoteLocked(bool bLocked);
    UFUNCTION(BlueprintCallable) void SetRemoteConfirmed(bool bConfirmed);
    UFUNCTION(BlueprintCallable) void SetTradeState(ETradeState NewState);

    UFUNCTION(BlueprintPure) ETradeState GetTradeState() const { return CurrentState; }

    // --- Events ---
    UPROPERTY(BlueprintAssignable) FOnTradeConfirmed OnConfirmed;
    UPROPERTY(BlueprintAssignable) FOnTradeCancelled OnCancelled;
    UPROPERTY(BlueprintAssignable) FOnTradeItemAdded OnItemAdded;

protected:
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* MyNameText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* OtherNameText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UVerticalBox* MyItemsBox = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UVerticalBox* OtherItemsBox = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* MyShardsText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* OtherShardsText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* StateText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* LockButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* ConfirmButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* CancelButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* MyLockStatus = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* OtherLockStatus = nullptr;

    UPROPERTY(EditDefaultsOnly) int32 MaxTradeItems = 8;

    ETradeState CurrentState = ETradeState::Proposing;
    FString OtherPlayerId;
    FString OtherPlayerName;
    TArray<FTradeItemDisplay> MyItems;
    TArray<FTradeItemDisplay> RemoteItems;
    int64 MyShards = 0;
    int64 RemoteShards = 0;
    bool bMyLocked = false;
    bool bRemoteLocked = false;
    bool bMyConfirmed = false;
    bool bRemoteConfirmed = false;

    void RebuildDisplay();
    FLinearColor GetRarityColor(const FString& Rarity) const;
    FString GetStateText() const;

    UFUNCTION() void OnLockClicked();
    UFUNCTION() void OnConfirmClicked();
    UFUNCTION() void OnCancelClicked();
};
