#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "LobbyWidget.generated.h"

class UScrollBox;
class UTextBlock;
class UButton;
class UEditableTextBox;

/**
 * Floor match entry data for the lobby list.
 */
USTRUCT(BlueprintType)
struct FFloorMatchEntry
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadOnly, Category = "Lobby")
    FString MatchId;

    UPROPERTY(BlueprintReadOnly, Category = "Lobby")
    int32 FloorLevel = 1;

    UPROPERTY(BlueprintReadOnly, Category = "Lobby")
    int32 PlayerCount = 0;

    UPROPERTY(BlueprintReadOnly, Category = "Lobby")
    int32 MaxPlayers = 50;

    UPROPERTY(BlueprintReadOnly, Category = "Lobby")
    FString HostName;

    UPROPERTY(BlueprintReadOnly, Category = "Lobby")
    float UptimeSeconds = 0.0f;

    /** Is this match joinable? */
    bool IsJoinable() const { return PlayerCount < MaxPlayers; }

    FString GetStatusText() const
    {
        return FString::Printf(TEXT("Floor %d | %d/%d players"),
            FloorLevel, PlayerCount, MaxPlayers);
    }
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnMatchJoinRequested, const FString&, MatchId);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnFloorCreateRequested, int32, FloorLevel);

/**
 * Matchmaking lobby widget.
 *
 * Layout:
 *   FLOOR LOBBY (title)
 *   [Floor Level Input] [Create Match]
 *   -----
 *   Active Matches:
 *   [Floor 1 | 3/50 players] [Join]
 *   [Floor 2 | 12/50 players] [Join]
 *   ...
 *   -----
 *   [Refresh]  [Solo Play]
 *
 * Shown before entering a floor. Player can:
 * - Join an existing match on their target floor
 * - Create a new match on a specific floor
 * - Play solo (no match connection)
 */
UCLASS()
class TOWERGAME_API ULobbyWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;

    // ============ Match List ============

    /** Populate the match list from JSON response */
    UFUNCTION(BlueprintCallable, Category = "Lobby")
    void PopulateMatchList(const FString& MatchesJson);

    /** Add a single match entry */
    UFUNCTION(BlueprintCallable, Category = "Lobby")
    void AddMatchEntry(const FFloorMatchEntry& Entry);

    /** Clear match list */
    UFUNCTION(BlueprintCallable, Category = "Lobby")
    void ClearMatchList();

    /** Get match entries */
    UFUNCTION(BlueprintPure, Category = "Lobby")
    const TArray<FFloorMatchEntry>& GetMatchEntries() const { return MatchEntries; }

    // ============ Events ============

    /** Fired when player clicks Join on a match */
    UPROPERTY(BlueprintAssignable, Category = "Lobby")
    FOnMatchJoinRequested OnJoinRequested;

    /** Fired when player clicks Create Match */
    UPROPERTY(BlueprintAssignable, Category = "Lobby")
    FOnFloorCreateRequested OnCreateRequested;

    // ============ Bound Widgets ============

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Lobby")
    UTextBlock* TitleText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Lobby")
    UEditableTextBox* FloorInput;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Lobby")
    UButton* CreateButton;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Lobby")
    UScrollBox* MatchListBox;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Lobby")
    UButton* RefreshButton;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Lobby")
    UButton* SoloButton;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Lobby")
    UTextBlock* StatusText;

    // ============ Config ============

    /** Auto-refresh interval (seconds, 0 = disabled) */
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Lobby")
    float AutoRefreshInterval = 10.0f;

protected:
    UFUNCTION()
    void OnCreateClicked();

    UFUNCTION()
    void OnRefreshClicked();

    UFUNCTION()
    void OnSoloClicked();

    void RebuildMatchList();

    /** Request match join (called from list item) */
    void RequestJoinMatch(const FString& MatchId);

private:
    UPROPERTY()
    TArray<FFloorMatchEntry> MatchEntries;

    int32 SelectedFloorLevel = 1;
};
