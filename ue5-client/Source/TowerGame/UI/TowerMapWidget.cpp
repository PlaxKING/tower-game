#include "TowerMapWidget.h"
#include "Components/ScrollBox.h"
#include "Components/TextBlock.h"
#include "Components/Button.h"
#include "Components/VerticalBox.h"
#include "Components/HorizontalBox.h"
#include "Components/ProgressBar.h"
#include "Components/Image.h"
#include "Components/ComboBoxString.h"
#include "Dom/JsonObject.h"
#include "Dom/JsonValue.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"
#include "Bridge/ProceduralCoreBridge.h"

// Forward declare the bridge (singleton)
static FProceduralCoreBridge* GetBridge()
{
	static FProceduralCoreBridge Bridge;
	static bool bInitialized = false;
	if (!bInitialized)
	{
		Bridge.Initialize(TEXT("tower_core.dll"));
		bInitialized = true;
	}
	return &Bridge;
}

void UTowerMapWidget::NativeConstruct()
{
	Super::NativeConstruct();

	// Initialize filter dropdown
	if (TierFilterBox)
	{
		TierFilterBox->AddOption(TEXT("All Tiers"));
		TierFilterBox->AddOption(TEXT("Echelon 1"));
		TierFilterBox->AddOption(TEXT("Echelon 2"));
		TierFilterBox->AddOption(TEXT("Echelon 3"));
		TierFilterBox->AddOption(TEXT("Echelon 4"));
		TierFilterBox->SetSelectedIndex(0);
		TierFilterBox->OnSelectionChanged.AddDynamic(this, &UTowerMapWidget::OnTierFilterChanged);
	}

	// Detail close button
	if (DetailCloseButton)
	{
		DetailCloseButton->OnClicked.AddDynamic(this, &UTowerMapWidget::OnDetailCloseClicked);
	}

	// Hide detail panel initially
	if (DetailPanel)
	{
		DetailPanel->SetVisibility(ESlateVisibility::Collapsed);
	}

	// Create empty map on init
	CreateEmptyMap();
}

void UTowerMapWidget::NativeDestruct()
{
	Super::NativeDestruct();
}

void UTowerMapWidget::CreateEmptyMap()
{
	FProceduralCoreBridge* Bridge = GetBridge();
	if (!Bridge || !Bridge->IsInitialized())
	{
		UE_LOG(LogTemp, Warning, TEXT("ProceduralCoreBridge not initialized"));
		return;
	}

	// Call towermap_create() via FFI
	FString MapJson = Bridge->GenerateFloor(0, 0); // Placeholder: need towermap_create wrapper
	// For now, create empty JSON
	CurrentMapJson = TEXT("{}");
	CachedFloors.Empty();
	CurrentOverview = FTowerMapOverview();

	if (bAutoRefreshUI)
	{
		RebuildOverviewPanel();
		RebuildFloorList();
	}

	OnMapUpdated.Broadcast();
}

void UTowerMapWidget::LoadMapFromJson(const FString& MapJson)
{
	CurrentMapJson = MapJson;
	ParseMapJson();

	if (bAutoRefreshUI)
	{
		RebuildOverviewPanel();
		RebuildFloorList();
	}

	OnMapUpdated.Broadcast();
}

void UTowerMapWidget::ParseMapJson()
{
	CachedFloors.Empty();

	if (CurrentMapJson.IsEmpty())
	{
		CurrentOverview = FTowerMapOverview();
		return;
	}

	TSharedPtr<FJsonObject> Json;
	TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(CurrentMapJson);

	if (!FJsonSerializer::Deserialize(Reader, Json) || !Json.IsValid())
	{
		UE_LOG(LogTemp, Warning, TEXT("Failed to parse tower map JSON"));
		return;
	}

	// Parse floors array
	const TArray<TSharedPtr<FJsonValue>>* FloorsArray = nullptr;
	if (Json->TryGetArrayField(TEXT("floors"), FloorsArray))
	{
		for (const TSharedPtr<FJsonValue>& FloorValue : *FloorsArray)
		{
			const TSharedPtr<FJsonObject> FloorObj = FloorValue->AsObject();
			if (!FloorObj.IsValid()) continue;

			FTowerFloorEntry Entry;
			Entry.FloorId = FloorObj->GetIntegerField(TEXT("floor_id"));
			Entry.Tier = TierEnumToIndex(ETowerTier::Echelon1); // Parse from JSON
			Entry.bDiscovered = FloorObj->GetBoolField(TEXT("discovered"));
			Entry.bCleared = FloorObj->GetBoolField(TEXT("cleared"));
			Entry.CompletionPercent = FloorObj->GetNumberField(TEXT("completion_percent"));
			Entry.DeathCount = FloorObj->GetIntegerField(TEXT("death_count"));
			Entry.DiscoveredRooms = FloorObj->GetIntegerField(TEXT("discovered_rooms"));
			Entry.TotalRooms = FloorObj->GetIntegerField(TEXT("total_rooms"));
			Entry.DiscoveredSecrets = FloorObj->GetIntegerField(TEXT("discovered_secrets"));
			Entry.TotalSecrets = FloorObj->GetIntegerField(TEXT("total_secrets"));
			Entry.MonstersKilled = FloorObj->GetIntegerField(TEXT("monsters_killed"));
			Entry.TotalMonsters = FloorObj->GetIntegerField(TEXT("total_monsters"));
			Entry.ChestsOpened = FloorObj->GetIntegerField(TEXT("chests_opened"));
			Entry.TotalChests = FloorObj->GetIntegerField(TEXT("total_chests"));

			if (FloorObj->HasField(TEXT("best_clear_time_secs")))
			{
				Entry.BestClearTimeSecs = FloorObj->GetNumberField(TEXT("best_clear_time_secs"));
			}

			CachedFloors.Add(Entry.FloorId, Entry);
		}
	}

	// Parse overview stats
	if (Json->HasField(TEXT("highest_floor_reached")))
	{
		CurrentOverview.HighestFloor = Json->GetIntegerField(TEXT("highest_floor_reached"));
	}
	if (Json->HasField(TEXT("total_floors_discovered")))
	{
		CurrentOverview.TotalDiscovered = Json->GetIntegerField(TEXT("total_floors_discovered"));
	}
	if (Json->HasField(TEXT("total_floors_cleared")))
	{
		CurrentOverview.TotalCleared = Json->GetIntegerField(TEXT("total_floors_cleared"));
	}
	if (Json->HasField(TEXT("total_deaths")))
	{
		CurrentOverview.TotalDeaths = Json->GetIntegerField(TEXT("total_deaths"));
	}
	if (Json->HasField(TEXT("total_playtime_secs")))
	{
		float PlaytimeSecs = Json->GetNumberField(TEXT("total_playtime_secs"));
		CurrentOverview.TotalPlaytimeHours = PlaytimeSecs / 3600.0f;
	}

	// Calculate average completion
	if (CachedFloors.Num() > 0)
	{
		float TotalCompletion = 0.0f;
		for (const auto& Pair : CachedFloors)
		{
			TotalCompletion += Pair.Value.CompletionPercent;
		}
		CurrentOverview.AverageCompletion = TotalCompletion / CachedFloors.Num();
	}
}

void UTowerMapWidget::RebuildOverviewPanel()
{
	// Highest Floor
	if (HighestFloorText)
	{
		HighestFloorText->SetText(FText::Format(
			FText::FromString(TEXT("Highest Floor: {0}")),
			FText::AsNumber(CurrentOverview.HighestFloor)
		));
	}

	// Total Discovered
	if (TotalDiscoveredText)
	{
		TotalDiscoveredText->SetText(FText::Format(
			FText::FromString(TEXT("Discovered: {0}")),
			FText::AsNumber(CurrentOverview.TotalDiscovered)
		));
	}

	// Total Cleared
	if (TotalClearedText)
	{
		TotalClearedText->SetText(FText::Format(
			FText::FromString(TEXT("Cleared: {0}")),
			FText::AsNumber(CurrentOverview.TotalCleared)
		));
	}

	// Total Deaths
	if (TotalDeathsText)
	{
		TotalDeathsText->SetText(FText::Format(
			FText::FromString(TEXT("Deaths: {0}")),
			FText::AsNumber(CurrentOverview.TotalDeaths)
		));
	}

	// Average Completion
	if (AverageCompletionText)
	{
		AverageCompletionText->SetText(FText::Format(
			FText::FromString(TEXT("Avg Completion: {0}%")),
			FText::AsNumber(static_cast<int32>(CurrentOverview.AverageCompletion * 100.0f))
		));
	}

	if (AverageCompletionBar)
	{
		AverageCompletionBar->SetPercent(CurrentOverview.AverageCompletion);
	}

	// Playtime
	if (PlaytimeText)
	{
		int32 Hours = static_cast<int32>(CurrentOverview.TotalPlaytimeHours);
		int32 Minutes = static_cast<int32>((CurrentOverview.TotalPlaytimeHours - Hours) * 60.0f);
		PlaytimeText->SetText(FText::Format(
			FText::FromString(TEXT("Playtime: {0}h {1}m")),
			FText::AsNumber(Hours),
			FText::AsNumber(Minutes)
		));
	}
}

void UTowerMapWidget::RebuildFloorList()
{
	if (!FloorListBox)
	{
		return;
	}

	FloorListBox->ClearChildren();

	// Filter and sort floors
	TArray<uint32> FloorIds;
	for (const auto& Pair : CachedFloors)
	{
		// Apply tier filter
		if (CurrentTierFilter > 0)
		{
			ETowerTier FilterTier = TierIndexToEnum(CurrentTierFilter);
			if (Pair.Value.Tier != FilterTier)
			{
				continue;
			}
		}

		// Only show discovered floors
		if (Pair.Value.bDiscovered)
		{
			FloorIds.Add(Pair.Key);
		}
	}

	// Sort by floor ID
	FloorIds.Sort();

	// Limit displayed floors
	if (FloorIds.Num() > MaxFloorsDisplayed)
	{
		FloorIds.SetNum(MaxFloorsDisplayed);
	}

	// Create floor entries
	for (uint32 FloorId : FloorIds)
	{
		const FTowerFloorEntry& Entry = CachedFloors[FloorId];

		// Create entry container
		UHorizontalBox* EntryBox = NewObject<UHorizontalBox>(this);
		if (!EntryBox) continue;

		// Floor ID text
		UTextBlock* IdText = NewObject<UTextBlock>(this);
		if (IdText)
		{
			IdText->SetText(FText::Format(FText::FromString(TEXT("Floor {0}")), FText::AsNumber(FloorId)));
			EntryBox->AddChild(IdText);
		}

		// Tier badge
		UTextBlock* TierText = NewObject<UTextBlock>(this);
		if (TierText)
		{
			TierText->SetText(FText::FromString(GetTierName(Entry.Tier)));
			TierText->SetColorAndOpacity(GetTierColor(Entry.Tier));
			EntryBox->AddChild(TierText);
		}

		// Cleared checkmark
		if (Entry.bCleared)
		{
			UTextBlock* CheckText = NewObject<UTextBlock>(this);
			if (CheckText)
			{
				CheckText->SetText(FText::FromString(TEXT("✓")));
				EntryBox->AddChild(CheckText);
			}
		}

		// Completion %
		UProgressBar* CompletionBar = NewObject<UProgressBar>(this);
		if (CompletionBar)
		{
			CompletionBar->SetPercent(Entry.CompletionPercent);
			EntryBox->AddChild(CompletionBar);
		}

		UTextBlock* CompletionText = NewObject<UTextBlock>(this);
		if (CompletionText)
		{
			CompletionText->SetText(FText::Format(
				FText::FromString(TEXT("{0}%")),
				FText::AsNumber(static_cast<int32>(Entry.CompletionPercent * 100.0f))
			));
			EntryBox->AddChild(CompletionText);
		}

		// Best time
		if (Entry.BestClearTimeSecs > 0.0f)
		{
			int32 Minutes = static_cast<int32>(Entry.BestClearTimeSecs) / 60;
			int32 Seconds = static_cast<int32>(Entry.BestClearTimeSecs) % 60;

			UTextBlock* TimeText = NewObject<UTextBlock>(this);
			if (TimeText)
			{
				TimeText->SetText(FText::Format(
					FText::FromString(TEXT("{0}:{1:02d}")),
					FText::AsNumber(Minutes),
					FText::AsNumber(Seconds)
				));
				EntryBox->AddChild(TimeText);
			}
		}

		// Deaths
		if (Entry.DeathCount > 0)
		{
			UTextBlock* DeathText = NewObject<UTextBlock>(this);
			if (DeathText)
			{
				DeathText->SetText(FText::Format(
					FText::FromString(TEXT("Deaths: {0}")),
					FText::AsNumber(Entry.DeathCount)
				));
				EntryBox->AddChild(DeathText);
			}
		}

		// Make clickable button
		UButton* FloorButton = NewObject<UButton>(this);
		if (FloorButton)
		{
			FloorButton->AddChild(EntryBox);
			// TODO: Bind click event - need to use FSimpleDelegate with FloorId capture
			FloorListBox->AddChild(FloorButton);
		}
		else
		{
			FloorListBox->AddChild(EntryBox);
		}
	}
}

void UTowerMapWidget::UpdateDetailPanel()
{
	if (!DetailPanel || SelectedFloorId == 0)
	{
		return;
	}

	const FTowerFloorEntry* Entry = CachedFloors.Find(SelectedFloorId);
	if (!Entry)
	{
		return;
	}

	// Update detail fields
	if (DetailFloorIdText)
	{
		DetailFloorIdText->SetText(FText::Format(
			FText::FromString(TEXT("Floor {0}")),
			FText::AsNumber(Entry->FloorId)
		));
	}

	if (DetailTierText)
	{
		DetailTierText->SetText(FText::FromString(GetTierName(Entry->Tier)));
		DetailTierText->SetColorAndOpacity(GetTierColor(Entry->Tier));
	}

	if (DetailCompletionBar)
	{
		DetailCompletionBar->SetPercent(Entry->CompletionPercent);
	}

	if (DetailCompletionText)
	{
		DetailCompletionText->SetText(FText::Format(
			FText::FromString(TEXT("Completion: {0}%")),
			FText::AsNumber(static_cast<int32>(Entry->CompletionPercent * 100.0f))
		));
	}

	// Rooms
	if (DetailRoomsText)
	{
		DetailRoomsText->SetText(FText::Format(
			FText::FromString(TEXT("Rooms: {0}/{1}")),
			FText::AsNumber(Entry->DiscoveredRooms),
			FText::AsNumber(Entry->TotalRooms)
		));
	}

	// Monsters
	if (DetailMonstersText)
	{
		DetailMonstersText->SetText(FText::Format(
			FText::FromString(TEXT("Monsters: {0}/{1}")),
			FText::AsNumber(Entry->MonstersKilled),
			FText::AsNumber(Entry->TotalMonsters)
		));
	}

	// Chests
	if (DetailChestsText)
	{
		DetailChestsText->SetText(FText::Format(
			FText::FromString(TEXT("Chests: {0}/{1}")),
			FText::AsNumber(Entry->ChestsOpened),
			FText::AsNumber(Entry->TotalChests)
		));
	}

	// Secrets
	if (DetailSecretsText)
	{
		DetailSecretsText->SetText(FText::Format(
			FText::FromString(TEXT("Secrets: {0}/{1}")),
			FText::AsNumber(Entry->DiscoveredSecrets),
			FText::AsNumber(Entry->TotalSecrets)
		));
	}

	// Deaths
	if (DetailDeathsText)
	{
		DetailDeathsText->SetText(FText::Format(
			FText::FromString(TEXT("Deaths: {0}")),
			FText::AsNumber(Entry->DeathCount)
		));
	}

	// Best time
	if (DetailBestTimeText)
	{
		if (Entry->BestClearTimeSecs > 0.0f)
		{
			int32 Minutes = static_cast<int32>(Entry->BestClearTimeSecs) / 60;
			int32 Seconds = static_cast<int32>(Entry->BestClearTimeSecs) % 60;
			DetailBestTimeText->SetText(FText::Format(
				FText::FromString(TEXT("Best Time: {0}:{1:02d}")),
				FText::AsNumber(Minutes),
				FText::AsNumber(Seconds)
			));
		}
		else
		{
			DetailBestTimeText->SetText(FText::FromString(TEXT("Best Time: Not cleared")));
		}
	}
}

FLinearColor UTowerMapWidget::GetTierColor(ETowerTier Tier) const
{
	switch (Tier)
	{
		case ETowerTier::Echelon1:
			return FLinearColor::Green;
		case ETowerTier::Echelon2:
			return FLinearColor::Yellow;
		case ETowerTier::Echelon3:
			return FLinearColor(1.0f, 0.5f, 0.0f, 1.0f); // Orange
		case ETowerTier::Echelon4:
			return FLinearColor::Red;
		default:
			return FLinearColor::White;
	}
}

FString UTowerMapWidget::GetTierName(ETowerTier Tier) const
{
	switch (Tier)
	{
		case ETowerTier::Echelon1:
			return TEXT("Echelon I");
		case ETowerTier::Echelon2:
			return TEXT("Echelon II");
		case ETowerTier::Echelon3:
			return TEXT("Echelon III");
		case ETowerTier::Echelon4:
			return TEXT("Echelon IV");
		default:
			return TEXT("Unknown");
	}
}

ETowerTier UTowerMapWidget::TierIndexToEnum(int32 Index) const
{
	switch (Index)
	{
		case 0:
			return ETowerTier::Echelon1;
		case 1:
			return ETowerTier::Echelon2;
		case 2:
			return ETowerTier::Echelon3;
		case 3:
			return ETowerTier::Echelon4;
		default:
			return ETowerTier::Echelon1;
	}
}

int32 UTowerMapWidget::TierEnumToIndex(ETowerTier Tier) const
{
	return static_cast<int32>(Tier);
}

void UTowerMapWidget::UpdateFloorProgress(uint32 FloorId)
{
	FProceduralCoreBridge* Bridge = GetBridge();
	if (!Bridge || !Bridge->IsInitialized())
	{
		return;
	}

	// Parse current map, update floor, refresh
	FTowerFloorEntry* Entry = CachedFloors.Find(FloorId);
	if (Entry)
	{
		// In real implementation, would call towermap_get_floor to get latest
		if (bAutoRefreshUI)
		{
			RebuildFloorList();
			if (SelectedFloorId == FloorId)
			{
				UpdateDetailPanel();
			}
		}
		OnMapUpdated.Broadcast();
	}
}

void UTowerMapWidget::DiscoverFloor(uint32 FloorId, ETowerTier Tier, uint32 TotalRooms,
                                     uint32 TotalMonsters, uint32 TotalChests)
{
	FProceduralCoreBridge* Bridge = GetBridge();
	if (!Bridge || !Bridge->IsInitialized())
	{
		return;
	}

	// Call towermap_discover_floor via FFI
	// FString UpdatedMapJson = Bridge->TowermapDiscoverFloor(
	//     CurrentMapJson, FloorId, static_cast<uint32>(Tier), TotalRooms, TotalMonsters, TotalChests);
	// LoadMapFromJson(UpdatedMapJson);

	// Fallback: update locally
	FTowerFloorEntry& Entry = CachedFloors.FindOrAdd(FloorId);
	Entry.FloorId = FloorId;
	Entry.Tier = Tier;
	Entry.bDiscovered = true;
	Entry.TotalRooms = TotalRooms;
	Entry.TotalMonsters = TotalMonsters;
	Entry.TotalChests = TotalChests;

	UpdateFloorProgress(FloorId);
}

void UTowerMapWidget::ClearFloor(uint32 FloorId, float ClearTimeSecs)
{
	FTowerFloorEntry* Entry = CachedFloors.Find(FloorId);
	if (Entry)
	{
		Entry->bCleared = true;
		Entry->BestClearTimeSecs = ClearTimeSecs;
		Entry->CompletionPercent = 1.0f;
		UpdateFloorProgress(FloorId);
	}
}

void UTowerMapWidget::RecordDeath(uint32 FloorId)
{
	FTowerFloorEntry* Entry = CachedFloors.Find(FloorId);
	if (Entry)
	{
		Entry->DeathCount++;
		CurrentOverview.TotalDeaths++;
		UpdateFloorProgress(FloorId);
	}
}

void UTowerMapWidget::DiscoverRoom(uint32 FloorId)
{
	FTowerFloorEntry* Entry = CachedFloors.Find(FloorId);
	if (Entry && Entry->DiscoveredRooms < Entry->TotalRooms)
	{
		Entry->DiscoveredRooms++;
		Entry->CompletionPercent = (Entry->DiscoveredRooms / static_cast<float>(Entry->TotalRooms)) * 0.3f +
		                            (Entry->MonstersKilled / static_cast<float>(Entry->TotalMonsters)) * 0.4f +
		                            (Entry->ChestsOpened / static_cast<float>(Entry->TotalChests)) * 0.2f;
		UpdateFloorProgress(FloorId);
	}
}

void UTowerMapWidget::KillMonster(uint32 FloorId)
{
	FTowerFloorEntry* Entry = CachedFloors.Find(FloorId);
	if (Entry && Entry->MonstersKilled < Entry->TotalMonsters)
	{
		Entry->MonstersKilled++;
		Entry->CompletionPercent = (Entry->DiscoveredRooms / static_cast<float>(Entry->TotalRooms)) * 0.3f +
		                            (Entry->MonstersKilled / static_cast<float>(Entry->TotalMonsters)) * 0.4f +
		                            (Entry->ChestsOpened / static_cast<float>(Entry->TotalChests)) * 0.2f;
		UpdateFloorProgress(FloorId);
	}
}

FString UTowerMapWidget::GetMapAsJson() const
{
	return CurrentMapJson;
}

FTowerMapOverview UTowerMapWidget::GetOverview() const
{
	return CurrentOverview;
}

bool UTowerMapWidget::GetFloorEntry(uint32 FloorId, FTowerFloorEntry& OutEntry) const
{
	const FTowerFloorEntry* Entry = CachedFloors.Find(FloorId);
	if (Entry)
	{
		OutEntry = *Entry;
		return true;
	}
	return false;
}

void UTowerMapWidget::SetTierFilter(int32 TierFilter)
{
	if (CurrentTierFilter != TierFilter)
	{
		CurrentTierFilter = TierFilter;
		RebuildFloorList();
	}
}

void UTowerMapWidget::RefreshFloorList()
{
	RebuildFloorList();
}

void UTowerMapWidget::ShowFloorDetail(uint32 FloorId)
{
	SelectedFloorId = FloorId;

	if (DetailPanel)
	{
		DetailPanel->SetVisibility(ESlateVisibility::Visible);
	}

	UpdateDetailPanel();

	FTowerFloorEntry* Entry = CachedFloors.Find(FloorId);
	if (Entry)
	{
		OnFloorSelected.Broadcast(*Entry);
	}
}

void UTowerMapWidget::HideFloorDetail()
{
	if (DetailPanel)
	{
		DetailPanel->SetVisibility(ESlateVisibility::Collapsed);
	}
	SelectedFloorId = 0;
}

void UTowerMapWidget::OnTierFilterChanged(FString SelectedItem, ESelectInfo::Type SelectInfo)
{
	// Find index from selected item
	int32 FilterIndex = 0;
	if (SelectedItem == TEXT("Echelon 1"))
		FilterIndex = 1;
	else if (SelectedItem == TEXT("Echelon 2"))
		FilterIndex = 2;
	else if (SelectedItem == TEXT("Echelon 3"))
		FilterIndex = 3;
	else if (SelectedItem == TEXT("Echelon 4"))
		FilterIndex = 4;

	SetTierFilter(FilterIndex);
}

void UTowerMapWidget::OnDetailCloseClicked()
{
	HideFloorDetail();
}

void UTowerMapWidget::OnFloorListItemClicked(uint32 FloorId)
{
	ShowFloorDetail(FloorId);
}
