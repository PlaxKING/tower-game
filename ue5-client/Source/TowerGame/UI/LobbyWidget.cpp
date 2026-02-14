#include "LobbyWidget.h"
#include "Components/ScrollBox.h"
#include "Components/TextBlock.h"
#include "Components/Button.h"
#include "Components/EditableTextBox.h"
#include "Dom/JsonObject.h"
#include "Dom/JsonValue.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

void ULobbyWidget::NativeConstruct()
{
    Super::NativeConstruct();

    if (CreateButton)
    {
        CreateButton->OnClicked.AddDynamic(this, &ULobbyWidget::OnCreateClicked);
    }
    if (RefreshButton)
    {
        RefreshButton->OnClicked.AddDynamic(this, &ULobbyWidget::OnRefreshClicked);
    }
    if (SoloButton)
    {
        SoloButton->OnClicked.AddDynamic(this, &ULobbyWidget::OnSoloClicked);
    }

    if (StatusText)
    {
        StatusText->SetText(FText::FromString(TEXT("Ready")));
    }
}

void ULobbyWidget::PopulateMatchList(const FString& MatchesJson)
{
    ClearMatchList();

    TSharedPtr<FJsonObject> Json;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(MatchesJson);

    if (!FJsonSerializer::Deserialize(Reader, Json) || !Json.IsValid())
    {
        UE_LOG(LogTemp, Warning, TEXT("Lobby: failed to parse matches JSON"));
        return;
    }

    const TArray<TSharedPtr<FJsonValue>>* Matches;
    if (Json->TryGetArrayField(TEXT("matches"), Matches))
    {
        for (const TSharedPtr<FJsonValue>& MatchVal : *Matches)
        {
            const TSharedPtr<FJsonObject>& MatchObj = MatchVal->AsObject();
            if (!MatchObj.IsValid()) continue;

            FFloorMatchEntry Entry;
            Entry.MatchId = MatchObj->GetStringField(TEXT("match_id"));
            Entry.FloorLevel = MatchObj->GetIntegerField(TEXT("floor_level"));
            Entry.PlayerCount = MatchObj->GetIntegerField(TEXT("player_count"));
            Entry.MaxPlayers = MatchObj->HasField(TEXT("max_players")) ?
                MatchObj->GetIntegerField(TEXT("max_players")) : 50;
            Entry.HostName = MatchObj->HasField(TEXT("host")) ?
                MatchObj->GetStringField(TEXT("host")) : TEXT("Unknown");

            MatchEntries.Add(Entry);
        }
    }

    RebuildMatchList();

    if (StatusText)
    {
        StatusText->SetText(FText::FromString(
            FString::Printf(TEXT("%d matches found"), MatchEntries.Num())));
    }
}

void ULobbyWidget::AddMatchEntry(const FFloorMatchEntry& Entry)
{
    MatchEntries.Add(Entry);
    RebuildMatchList();
}

void ULobbyWidget::ClearMatchList()
{
    MatchEntries.Empty();
    if (MatchListBox)
    {
        MatchListBox->ClearChildren();
    }
}

void ULobbyWidget::OnCreateClicked()
{
    if (FloorInput)
    {
        FString FloorStr = FloorInput->GetText().ToString();
        SelectedFloorLevel = FCString::Atoi(*FloorStr);
        if (SelectedFloorLevel < 1) SelectedFloorLevel = 1;
        if (SelectedFloorLevel > 1000) SelectedFloorLevel = 1000;
    }

    OnCreateRequested.Broadcast(SelectedFloorLevel);

    if (StatusText)
    {
        StatusText->SetText(FText::FromString(
            FString::Printf(TEXT("Creating match for floor %d..."), SelectedFloorLevel)));
    }
}

void ULobbyWidget::OnRefreshClicked()
{
    if (StatusText)
    {
        StatusText->SetText(FText::FromString(TEXT("Refreshing...")));
    }

    // The actual refresh is triggered by the owning code (GameMode/PlayerController)
    // which calls NakamaSubsystem->ListActiveMatches() and then PopulateMatchList()
}

void ULobbyWidget::OnSoloClicked()
{
    if (StatusText)
    {
        StatusText->SetText(FText::FromString(TEXT("Starting solo...")));
    }

    // Solo play means no match connection — just load the floor locally
    // Broadcast with empty match ID to signal solo mode
    OnJoinRequested.Broadcast(TEXT(""));
}

void ULobbyWidget::RequestJoinMatch(const FString& MatchId)
{
    OnJoinRequested.Broadcast(MatchId);

    if (StatusText)
    {
        StatusText->SetText(FText::FromString(TEXT("Joining match...")));
    }
}

void ULobbyWidget::RebuildMatchList()
{
    if (!MatchListBox) return;

    MatchListBox->ClearChildren();

    // Sort by floor level
    MatchEntries.Sort([](const FFloorMatchEntry& A, const FFloorMatchEntry& B) {
        return A.FloorLevel < B.FloorLevel;
    });

    for (int32 i = 0; i < MatchEntries.Num(); i++)
    {
        const FFloorMatchEntry& Entry = MatchEntries[i];

        UTextBlock* EntryText = NewObject<UTextBlock>(this);

        FString DisplayText = FString::Printf(TEXT("Floor %d | %d/%d players | %s"),
            Entry.FloorLevel, Entry.PlayerCount, Entry.MaxPlayers, *Entry.HostName);

        if (!Entry.IsJoinable())
        {
            DisplayText += TEXT(" [FULL]");
        }

        EntryText->SetText(FText::FromString(DisplayText));

        // Color based on capacity
        float Fullness = (float)Entry.PlayerCount / (float)FMath::Max(Entry.MaxPlayers, 1);
        FLinearColor EntryColor;
        if (!Entry.IsJoinable())
        {
            EntryColor = FLinearColor(0.5f, 0.3f, 0.3f); // Red-gray for full
        }
        else if (Fullness > 0.7f)
        {
            EntryColor = FLinearColor(1.0f, 0.7f, 0.3f); // Orange for almost full
        }
        else if (Fullness > 0.3f)
        {
            EntryColor = FLinearColor(0.3f, 1.0f, 0.4f); // Green for moderate
        }
        else
        {
            EntryColor = FLinearColor(0.8f, 0.8f, 0.8f); // White for low
        }
        EntryText->SetColorAndOpacity(FSlateColor(EntryColor));

        FSlateFontInfo Font = EntryText->GetFont();
        Font.Size = 12;
        EntryText->SetFont(Font);

        MatchListBox->AddChild(EntryText);
    }

    if (MatchEntries.Num() == 0)
    {
        UTextBlock* EmptyText = NewObject<UTextBlock>(this);
        EmptyText->SetText(FText::FromString(TEXT("No active matches. Create one or play solo.")));
        EmptyText->SetColorAndOpacity(FSlateColor(FLinearColor(0.5f, 0.5f, 0.5f)));
        MatchListBox->AddChild(EmptyText);
    }
}
