#include "LeaderboardWidget.h"
#include "Components/ScrollBox.h"
#include "Components/TextBlock.h"
#include "Components/Button.h"
#include "Dom/JsonObject.h"
#include "Dom/JsonValue.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

void ULeaderboardWidget::NativeConstruct()
{
    Super::NativeConstruct();

    if (RefreshButton)
    {
        RefreshButton->OnClicked.AddDynamic(this, &ULeaderboardWidget::OnRefreshClicked);
    }
    if (TabHighestFloor)
    {
        TabHighestFloor->OnClicked.AddDynamic(this, &ULeaderboardWidget::OnTabHighestFloorClicked);
    }
    if (TabSpeed1)
    {
        TabSpeed1->OnClicked.AddDynamic(this, &ULeaderboardWidget::OnTabSpeed1Clicked);
    }
    if (TabSpeed5)
    {
        TabSpeed5->OnClicked.AddDynamic(this, &ULeaderboardWidget::OnTabSpeed5Clicked);
    }
    if (TabSpeed10)
    {
        TabSpeed10->OnClicked.AddDynamic(this, &ULeaderboardWidget::OnTabSpeed10Clicked);
    }

    RebuildDisplay();
}

void ULeaderboardWidget::PopulateFromJson(const FString& LeaderboardJson, ELeaderboardType Type)
{
    CurrentEntries.Empty();
    CurrentTab = Type;

    TSharedPtr<FJsonObject> Json;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(LeaderboardJson);
    if (!FJsonSerializer::Deserialize(Reader, Json) || !Json.IsValid()) return;

    const TArray<TSharedPtr<FJsonValue>>* Records;
    if (Json->TryGetArrayField(TEXT("records"), Records))
    {
        for (const TSharedPtr<FJsonValue>& RecVal : *Records)
        {
            const TSharedPtr<FJsonObject>& RecObj = RecVal->AsObject();
            if (!RecObj.IsValid()) continue;

            FLeaderboardEntry Entry;
            Entry.Rank = RecObj->GetIntegerField(TEXT("rank"));
            Entry.PlayerName = RecObj->HasField(TEXT("username")) ?
                RecObj->GetStringField(TEXT("username")) : TEXT("Unknown");
            Entry.Score = RecObj->GetIntegerField(TEXT("score"));
            Entry.PlayerId = RecObj->HasField(TEXT("owner_id")) ?
                RecObj->GetStringField(TEXT("owner_id")) : TEXT("");
            Entry.bIsLocalPlayer = (!LocalPlayerId.IsEmpty() && Entry.PlayerId == LocalPlayerId);

            CurrentEntries.Add(Entry);
        }
    }

    RebuildDisplay();
}

void ULeaderboardWidget::SetEntries(const TArray<FLeaderboardEntry>& Entries)
{
    CurrentEntries = Entries;
    RebuildDisplay();
}

void ULeaderboardWidget::SetActiveTab(ELeaderboardType Type)
{
    CurrentTab = Type;
    OnRefreshRequested.Broadcast(Type);
}

void ULeaderboardWidget::OnRefreshClicked()
{
    OnRefreshRequested.Broadcast(CurrentTab);
}

void ULeaderboardWidget::OnTabHighestFloorClicked()
{
    SetActiveTab(ELeaderboardType::HighestFloor);
}

void ULeaderboardWidget::OnTabSpeed1Clicked()
{
    SetActiveTab(ELeaderboardType::SpeedRunFloor1);
}

void ULeaderboardWidget::OnTabSpeed5Clicked()
{
    SetActiveTab(ELeaderboardType::SpeedRunFloor5);
}

void ULeaderboardWidget::OnTabSpeed10Clicked()
{
    SetActiveTab(ELeaderboardType::SpeedRunFloor10);
}

FLinearColor ULeaderboardWidget::GetRankColor(int32 Rank) const
{
    switch (Rank)
    {
    case 1:  return FLinearColor(1.0f, 0.84f, 0.0f);  // Gold
    case 2:  return FLinearColor(0.75f, 0.75f, 0.75f); // Silver
    case 3:  return FLinearColor(0.8f, 0.5f, 0.2f);    // Bronze
    default: return FLinearColor(0.7f, 0.7f, 0.7f);    // Normal
    }
}

FString ULeaderboardWidget::FormatScore(int64 Score, ELeaderboardType Type) const
{
    switch (Type)
    {
    case ELeaderboardType::HighestFloor:
        return FString::Printf(TEXT("Floor %lld"), Score);

    case ELeaderboardType::SpeedRunFloor1:
    case ELeaderboardType::SpeedRunFloor5:
    case ELeaderboardType::SpeedRunFloor10:
    {
        // Score is time in milliseconds (lower is better)
        int32 TotalSecs = Score / 1000;
        int32 Mins = TotalSecs / 60;
        int32 Secs = TotalSecs % 60;
        int32 Ms = Score % 1000;
        return FString::Printf(TEXT("%02d:%02d.%03d"), Mins, Secs, Ms);
    }

    default:
        return FString::Printf(TEXT("%lld"), Score);
    }
}

void ULeaderboardWidget::RebuildDisplay()
{
    if (!EntryScrollBox) return;

    EntryScrollBox->ClearChildren();

    // Title update
    if (TitleText)
    {
        FString TabName;
        switch (CurrentTab)
        {
        case ELeaderboardType::HighestFloor:   TabName = TEXT("Highest Floor"); break;
        case ELeaderboardType::SpeedRunFloor1: TabName = TEXT("Floor 1 Speed"); break;
        case ELeaderboardType::SpeedRunFloor5: TabName = TEXT("Floor 5 Speed"); break;
        case ELeaderboardType::SpeedRunFloor10:TabName = TEXT("Floor 10 Speed"); break;
        }
        TitleText->SetText(FText::FromString(FString::Printf(TEXT("LEADERBOARD - %s"), *TabName)));
    }

    int32 LocalRank = -1;
    int64 LocalScore = 0;

    int32 DisplayCount = FMath::Min(CurrentEntries.Num(), MaxDisplayEntries);
    for (int32 i = 0; i < DisplayCount; i++)
    {
        const FLeaderboardEntry& Entry = CurrentEntries[i];

        UTextBlock* EntryText = NewObject<UTextBlock>(this);

        FString ScoreStr = FormatScore(Entry.Score, CurrentTab);
        FString DisplayText = FString::Printf(TEXT("#%-3d  %-20s  %s"),
            Entry.Rank, *Entry.PlayerName, *ScoreStr);

        EntryText->SetText(FText::FromString(DisplayText));

        FLinearColor EntryColor;
        if (Entry.bIsLocalPlayer)
        {
            EntryColor = FLinearColor(0.3f, 1.0f, 0.5f); // Highlighted green
            LocalRank = Entry.Rank;
            LocalScore = Entry.Score;
        }
        else
        {
            EntryColor = GetRankColor(Entry.Rank);
        }
        EntryText->SetColorAndOpacity(FSlateColor(EntryColor));

        FSlateFontInfo Font = EntryText->GetFont();
        Font.Size = Entry.Rank <= 3 ? 14 : 12;
        EntryText->SetFont(Font);

        EntryScrollBox->AddChild(EntryText);
    }

    // Player rank footer
    if (PlayerRankText)
    {
        if (LocalRank > 0)
        {
            FString ScoreStr = FormatScore(LocalScore, CurrentTab);
            PlayerRankText->SetText(FText::FromString(
                FString::Printf(TEXT("Your rank: #%d (%s)"), LocalRank, *ScoreStr)));
            PlayerRankText->SetColorAndOpacity(FSlateColor(FLinearColor(0.3f, 1.0f, 0.5f)));
        }
        else
        {
            PlayerRankText->SetText(FText::FromString(TEXT("Not ranked")));
            PlayerRankText->SetColorAndOpacity(FSlateColor(FLinearColor(0.5f, 0.5f, 0.5f)));
        }
    }
}
