#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "LeaderboardWidget.generated.h"

class UScrollBox;
class UTextBlock;
class UButton;

/**
 * Leaderboard entry data.
 */
USTRUCT(BlueprintType)
struct FLeaderboardEntry
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadOnly, Category = "Leaderboard")
    int32 Rank = 0;

    UPROPERTY(BlueprintReadOnly, Category = "Leaderboard")
    FString PlayerName;

    UPROPERTY(BlueprintReadOnly, Category = "Leaderboard")
    int64 Score = 0;

    UPROPERTY(BlueprintReadOnly, Category = "Leaderboard")
    FString PlayerId;

    UPROPERTY(BlueprintReadOnly, Category = "Leaderboard")
    bool bIsLocalPlayer = false;
};

/**
 * Leaderboard categories matching Nakama server setup.
 */
UENUM(BlueprintType)
enum class ELeaderboardType : uint8
{
    HighestFloor    UMETA(DisplayName = "Highest Floor"),
    SpeedRunFloor1  UMETA(DisplayName = "Floor 1 Speed"),
    SpeedRunFloor5  UMETA(DisplayName = "Floor 5 Speed"),
    SpeedRunFloor10 UMETA(DisplayName = "Floor 10 Speed"),
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnLeaderboardRefresh, ELeaderboardType, Type);

/**
 * Leaderboard widget — shows ranked player scores.
 *
 * Layout:
 *   LEADERBOARD (title)
 *   [Highest Floor] [Floor 1 Speed] [Floor 5 Speed] [Floor 10 Speed] (tabs)
 *   -----
 *   #1  PlayerName   Score
 *   #2  PlayerName   Score
 *   #3  PlayerName   Score
 *   ...
 *   -----
 *   Your rank: #N (Score)
 *   [Refresh]
 *
 * Top 3 entries are gold/silver/bronze colored.
 * Local player entry is highlighted.
 */
UCLASS()
class TOWERGAME_API ULeaderboardWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;

    // ============ API ============

    /** Populate from Nakama leaderboard JSON */
    UFUNCTION(BlueprintCallable, Category = "Leaderboard")
    void PopulateFromJson(const FString& LeaderboardJson, ELeaderboardType Type);

    /** Set entries directly */
    UFUNCTION(BlueprintCallable, Category = "Leaderboard")
    void SetEntries(const TArray<FLeaderboardEntry>& Entries);

    /** Set current leaderboard tab */
    UFUNCTION(BlueprintCallable, Category = "Leaderboard")
    void SetActiveTab(ELeaderboardType Type);

    /** Get current tab */
    UFUNCTION(BlueprintPure, Category = "Leaderboard")
    ELeaderboardType GetActiveTab() const { return CurrentTab; }

    // ============ Config ============

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Leaderboard")
    int32 MaxDisplayEntries = 20;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Leaderboard")
    FString LocalPlayerId;

    // ============ Events ============

    UPROPERTY(BlueprintAssignable, Category = "Leaderboard")
    FOnLeaderboardRefresh OnRefreshRequested;

    // ============ Bound Widgets ============

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Leaderboard")
    UTextBlock* TitleText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Leaderboard")
    UScrollBox* EntryScrollBox;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Leaderboard")
    UTextBlock* PlayerRankText;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Leaderboard")
    UButton* RefreshButton;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Leaderboard")
    UButton* TabHighestFloor;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Leaderboard")
    UButton* TabSpeed1;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Leaderboard")
    UButton* TabSpeed5;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Leaderboard")
    UButton* TabSpeed10;

protected:
    UFUNCTION()
    void OnRefreshClicked();

    UFUNCTION()
    void OnTabHighestFloorClicked();

    UFUNCTION()
    void OnTabSpeed1Clicked();

    UFUNCTION()
    void OnTabSpeed5Clicked();

    UFUNCTION()
    void OnTabSpeed10Clicked();

    void RebuildDisplay();
    FLinearColor GetRankColor(int32 Rank) const;
    FString FormatScore(int64 Score, ELeaderboardType Type) const;

private:
    UPROPERTY()
    TArray<FLeaderboardEntry> CurrentEntries;

    ELeaderboardType CurrentTab = ELeaderboardType::HighestFloor;
};
