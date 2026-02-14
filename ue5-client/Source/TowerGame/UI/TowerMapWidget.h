#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "Components/ScrollBox.h"
#include "Components/TextBlock.h"
#include "Components/Button.h"
#include "Components/VerticalBox.h"
#include "Components/HorizontalBox.h"
#include "Components/ProgressBar.h"
#include "Components/Image.h"
#include "Components/ComboBoxString.h"
#include "TowerMapWidget.generated.h"

class UProceduralCoreBridge;

/// Floor tier enumeration matching Rust
UENUM(BlueprintType)
enum class ETowerTier : uint8
{
	Echelon1,
	Echelon2,
	Echelon3,
	Echelon4
};

/// Floor entry data from towermap module
USTRUCT(BlueprintType)
struct FTowerFloorEntry
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadWrite) uint32 FloorId = 0;
	UPROPERTY(BlueprintReadWrite) ETowerTier Tier = ETowerTier::Echelon1;
	UPROPERTY(BlueprintReadWrite) bool bDiscovered = false;
	UPROPERTY(BlueprintReadWrite) bool bCleared = false;
	UPROPERTY(BlueprintReadWrite) float CompletionPercent = 0.0f;
	UPROPERTY(BlueprintReadWrite) float BestClearTimeSecs = 0.0f;
	UPROPERTY(BlueprintReadWrite) uint32 DeathCount = 0;
	UPROPERTY(BlueprintReadWrite) uint32 DiscoveredRooms = 0;
	UPROPERTY(BlueprintReadWrite) uint32 TotalRooms = 0;
	UPROPERTY(BlueprintReadWrite) uint32 DiscoveredSecrets = 0;
	UPROPERTY(BlueprintReadWrite) uint32 TotalSecrets = 0;
	UPROPERTY(BlueprintReadWrite) uint32 MonstersKilled = 0;
	UPROPERTY(BlueprintReadWrite) uint32 TotalMonsters = 0;
	UPROPERTY(BlueprintReadWrite) uint32 ChestsOpened = 0;
	UPROPERTY(BlueprintReadWrite) uint32 TotalChests = 0;
	UPROPERTY(BlueprintReadWrite) FString ShrineFacton;
	UPROPERTY(BlueprintReadWrite) FString Notes;
};

/// Tower map overview statistics
USTRUCT(BlueprintType)
struct FTowerMapOverview
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadWrite) uint32 HighestFloor = 0;
	UPROPERTY(BlueprintReadWrite) uint32 TotalDiscovered = 0;
	UPROPERTY(BlueprintReadWrite) uint32 TotalCleared = 0;
	UPROPERTY(BlueprintReadWrite) uint32 TotalDeaths = 0;
	UPROPERTY(BlueprintReadWrite) float AverageCompletion = 0.0f;
	UPROPERTY(BlueprintReadWrite) float TotalPlaytimeHours = 0.0f;
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_One_Param(FOnFloorSelected, const FTowerFloorEntry&, FloorEntry);
DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnMapUpdated);

/**
 * Tower Map Widget: Visualizes player's exploration progress across tower floors.
 *
 * Features:
 * - Grid/list view of discovered floors with tier badges and colors
 * - Per-floor stats: completion %, rooms discovered, monsters killed, chests opened, secrets, best time
 * - Overall stats panel: highest floor, total discovered, total cleared, death count
 * - Detail view when clicking a floor entry
 * - Floor filtering by tier (Echelon1/2/3/4)
 * - Death count indicator with skull icon
 * - Integration with ProceduralCoreBridge for towermap_* FFI functions
 */
UCLASS()
class TOWERGAME_API UTowerMapWidget : public UUserWidget
{
	GENERATED_BODY()

public:
	virtual void NativeConstruct() override;
	virtual void NativeDestruct() override;

	// --- Data Loading & Management ---

	/// Load tower map from JSON (typically from persistent save data)
	UFUNCTION(BlueprintCallable, Category = "TowerMap")
	void LoadMapFromJson(const FString& MapJson);

	/// Create empty tower map
	UFUNCTION(BlueprintCallable, Category = "TowerMap")
	void CreateEmptyMap();

	/// Update single floor progress (called when floor is discovered/cleared/progressed)
	UFUNCTION(BlueprintCallable, Category = "TowerMap")
	void UpdateFloorProgress(uint32 FloorId);

	/// Discover a new floor (syncs with Rust backend)
	UFUNCTION(BlueprintCallable, Category = "TowerMap")
	void DiscoverFloor(uint32 FloorId, ETowerTier Tier, uint32 TotalRooms,
	                    uint32 TotalMonsters, uint32 TotalChests);

	/// Mark floor as cleared
	UFUNCTION(BlueprintCallable, Category = "TowerMap")
	void ClearFloor(uint32 FloorId, float ClearTimeSecs);

	/// Record a death on floor
	UFUNCTION(BlueprintCallable, Category = "TowerMap")
	void RecordDeath(uint32 FloorId);

	/// Record room discovered on floor
	UFUNCTION(BlueprintCallable, Category = "TowerMap")
	void DiscoverRoom(uint32 FloorId);

	/// Record monster killed on floor
	UFUNCTION(BlueprintCallable, Category = "TowerMap")
	void KillMonster(uint32 FloorId);

	/// Get current tower map as JSON (for saving)
	UFUNCTION(BlueprintCallable, Category = "TowerMap")
	FString GetMapAsJson() const;

	/// Get tower map overview statistics
	UFUNCTION(BlueprintCallable, Category = "TowerMap")
	FTowerMapOverview GetOverview() const;

	/// Get specific floor entry by ID
	UFUNCTION(BlueprintCallable, Category = "TowerMap")
	bool GetFloorEntry(uint32 FloorId, FTowerFloorEntry& OutEntry) const;

	// --- UI Interactions ---

	/// Set filter tier (0=All, 1=Echelon1, 2=Echelon2, etc)
	UFUNCTION(BlueprintCallable, Category = "TowerMap|UI")
	void SetTierFilter(int32 TierFilter);

	/// Refresh the floor list display
	UFUNCTION(BlueprintCallable, Category = "TowerMap|UI")
	void RefreshFloorList();

	/// Show detail view for floor
	UFUNCTION(BlueprintCallable, Category = "TowerMap|UI")
	void ShowFloorDetail(uint32 FloorId);

	/// Hide detail view
	UFUNCTION(BlueprintCallable, Category = "TowerMap|UI")
	void HideFloorDetail();

	// --- Events ---
	UPROPERTY(BlueprintAssignable, Category = "TowerMap|Events")
	FOnFloorSelected OnFloorSelected;

	UPROPERTY(BlueprintAssignable, Category = "TowerMap|Events")
	FOnMapUpdated OnMapUpdated;

protected:
	// --- Bound Widgets (named in UMG Designer) ---

	// Overview Panel
	UPROPERTY(meta = (BindWidgetOptional))
	UTextBlock* HighestFloorText = nullptr;

	UPROPERTY(meta = (BindWidgetOptional))
	UTextBlock* TotalDiscoveredText = nullptr;

	UPROPERTY(meta = (BindWidgetOptional))
	UTextBlock* TotalClearedText = nullptr;

	UPROPERTY(meta = (BindWidgetOptional))
	UTextBlock* TotalDeathsText = nullptr;

	UPROPERTY(meta = (BindWidgetOptional))
	UImage* DeathSkullIcon = nullptr;

	UPROPERTY(meta = (BindWidgetOptional))
	UTextBlock* AverageCompletionText = nullptr;

	UPROPERTY(meta = (BindWidgetOptional))
	UProgressBar* AverageCompletionBar = nullptr;

	UPROPERTY(meta = (BindWidgetOptional))
	UTextBlock* PlaytimeText = nullptr;

	// Filter Panel
	UPROPERTY(meta = (BindWidgetOptional))
	UComboBoxString* TierFilterBox = nullptr;

	// Floor List
	UPROPERTY(meta = (BindWidgetOptional))
	UScrollBox* FloorListBox = nullptr;

	// Detail View
	UPROPERTY(meta = (BindWidgetOptional))
	UVerticalBox* DetailPanel = nullptr;

	UPROPERTY(meta = (BindWidgetOptional))
	UTextBlock* DetailFloorIdText = nullptr;

	UPROPERTY(meta = (BindWidgetOptional))
	UTextBlock* DetailTierText = nullptr;

	UPROPERTY(meta = (BindWidgetOptional))
	UProgressBar* DetailCompletionBar = nullptr;

	UPROPERTY(meta = (BindWidgetOptional))
	UTextBlock* DetailCompletionText = nullptr;

	UPROPERTY(meta = (BindWidgetOptional))
	UTextBlock* DetailRoomsText = nullptr;

	UPROPERTY(meta = (BindWidgetOptional))
	UTextBlock* DetailMonstersText = nullptr;

	UPROPERTY(meta = (BindWidgetOptional))
	UTextBlock* DetailChestsText = nullptr;

	UPROPERTY(meta = (BindWidgetOptional))
	UTextBlock* DetailSecretsText = nullptr;

	UPROPERTY(meta = (BindWidgetOptional))
	UTextBlock* DetailDeathsText = nullptr;

	UPROPERTY(meta = (BindWidgetOptional))
	UTextBlock* DetailBestTimeText = nullptr;

	UPROPERTY(meta = (BindWidgetOptional))
	UButton* DetailCloseButton = nullptr;

	// --- State ---
	UPROPERTY()
	FString CurrentMapJson;

	UPROPERTY()
	TMap<uint32, FTowerFloorEntry> CachedFloors;

	UPROPERTY()
	FTowerMapOverview CurrentOverview;

	int32 CurrentTierFilter = 0; // 0 = All, 1 = Echelon1, etc

	uint32 SelectedFloorId = 0;

	// --- Configuration ---
	UPROPERTY(EditDefaultsOnly, BlueprintReadOnly, Category = "TowerMap|Config")
	bool bAutoRefreshUI = true;

	UPROPERTY(EditDefaultsOnly, BlueprintReadOnly, Category = "TowerMap|Config")
	int32 MaxFloorsDisplayed = 100;

	// --- Internal Methods ---
	void ParseMapJson();
	void RebuildOverviewPanel();
	void RebuildFloorList();
	void UpdateDetailPanel();
	FLinearColor GetTierColor(ETowerTier Tier) const;
	FString GetTierName(ETowerTier Tier) const;
	ETowerTier TierIndexToEnum(int32 Index) const;
	int32 TierEnumToIndex(ETowerTier Tier) const;

	UFUNCTION()
	void OnTierFilterChanged(FString SelectedItem, ESelectInfo::Type SelectInfo);

	UFUNCTION()
	void OnDetailCloseClicked();

	UFUNCTION()
	void OnFloorListItemClicked(uint32 FloorId);
};
