#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "LoggingConfigWidget.generated.h"

class UButton;
class UTextBlock;
class UCheckBox;
class UScrollBox;
class UVerticalBox;
class UHorizontalBox;
class FProceduralCoreBridge;

/// Log level enum
UENUM(BlueprintType)
enum class ELogLevel : uint8
{
	Trace	UMETA(DisplayName = "Trace"),
	Debug	UMETA(DisplayName = "Debug"),
	Info	UMETA(DisplayName = "Info"),
	Warn	UMETA(DisplayName = "Warn"),
	Error	UMETA(DisplayName = "Error"),
};

/// Logging configuration data
USTRUCT(BlueprintType)
struct FLoggingConfig
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadWrite) ELogLevel DefaultLevel = ELogLevel::Info;
	UPROPERTY(BlueprintReadWrite) bool bShowTimestamps = true;
	UPROPERTY(BlueprintReadWrite) bool bShowThreadIds = false;
	UPROPERTY(BlueprintReadWrite) bool bShowTargets = true;
	UPROPERTY(BlueprintReadWrite) bool bShowFileLine = false;
	UPROPERTY(BlueprintReadWrite) TArray<FString> ModuleFilters;
};

/// Logging snapshot from Rust (read-only display info)
USTRUCT(BlueprintType)
struct FLoggingSnapshot
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadWrite) TArray<FString> AvailableLevels;
	UPROPERTY(BlueprintReadWrite) int32 ActiveFilterCount = 0;
	UPROPERTY(BlueprintReadWrite) FString CoreVersion;
	UPROPERTY(BlueprintReadWrite) bool bIsInitialized = false;
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnLoggingConfigApplied, const FLoggingConfig&, Config);

/**
 * Logging configuration widget for debug/development.
 * Manages structured logging configuration via ProceduralCoreBridge.
 *
 * Features:
 * - 5 log level buttons for setting default level
 * - Module filter list with add/remove buttons
 * - Checkboxes for output format options
 * - Apply button to send JSON config to Rust
 * - Display of logging snapshot and available levels
 *
 * Uses FFI functions:
 * - logging_get_default_config() -> JSON
 * - logging_init(config_json) -> void
 * - logging_get_snapshot() -> JSON
 * - logging_log_message(level, target, message) -> void
 */
UCLASS()
class TOWERGAME_API ULoggingConfigWidget : public UUserWidget
{
	GENERATED_BODY()

public:
	virtual void NativeConstruct() override;

	// --- Configuration Methods ---
	UFUNCTION(BlueprintCallable) void LoadFromJson(const FString& ConfigJson);
	UFUNCTION(BlueprintCallable) void ApplyConfiguration();
	UFUNCTION(BlueprintCallable) void ResetToDefaults();
	UFUNCTION(BlueprintCallable) void RefreshSnapshot();

	// --- Module Filter Management ---
	UFUNCTION(BlueprintCallable) void AddModuleFilter(const FString& ModuleName);
	UFUNCTION(BlueprintCallable) void RemoveModuleFilter(int32 FilterIndex);
	UFUNCTION(BlueprintCallable) void ClearAllFilters();

	// --- Level Selection ---
	UFUNCTION(BlueprintCallable) void SetDefaultLevel(ELogLevel Level);

	// --- Format Options ---
	UFUNCTION(BlueprintCallable) void SetShowTimestamps(bool bShow);
	UFUNCTION(BlueprintCallable) void SetShowThreadIds(bool bShow);
	UFUNCTION(BlueprintCallable) void SetShowTargets(bool bShow);
	UFUNCTION(BlueprintCallable) void SetShowFileLine(bool bShow);

	// --- Getters ---
	UFUNCTION(BlueprintPure) FLoggingConfig GetCurrentConfig() const { return CurrentConfig; }
	UFUNCTION(BlueprintPure) FLoggingSnapshot GetSnapshot() const { return CurrentSnapshot; }

	// --- Delegates ---
	UPROPERTY(BlueprintAssignable) FOnLoggingConfigApplied OnConfigApplied;

protected:
	// --- Level Selection Buttons ---
	UPROPERTY(meta = (BindWidgetOptional)) UButton* TraceLevelButton = nullptr;
	UPROPERTY(meta = (BindWidgetOptional)) UButton* DebugLevelButton = nullptr;
	UPROPERTY(meta = (BindWidgetOptional)) UButton* InfoLevelButton = nullptr;
	UPROPERTY(meta = (BindWidgetOptional)) UButton* WarnLevelButton = nullptr;
	UPROPERTY(meta = (BindWidgetOptional)) UButton* ErrorLevelButton = nullptr;

	// --- Current Level Display ---
	UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* CurrentLevelText = nullptr;

	// --- Format Checkboxes ---
	UPROPERTY(meta = (BindWidgetOptional)) UCheckBox* ShowTimestampsCheck = nullptr;
	UPROPERTY(meta = (BindWidgetOptional)) UCheckBox* ShowThreadIdsCheck = nullptr;
	UPROPERTY(meta = (BindWidgetOptional)) UCheckBox* ShowTargetsCheck = nullptr;
	UPROPERTY(meta = (BindWidgetOptional)) UCheckBox* ShowFileLineCheck = nullptr;

	// --- Module Filter UI ---
	UPROPERTY(meta = (BindWidgetOptional)) UScrollBox* ModuleFilterBox = nullptr;
	UPROPERTY(meta = (BindWidgetOptional)) UVerticalBox* ModuleListContainer = nullptr;
	UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* ModuleFilterCountText = nullptr;

	// --- Module Filter Add ---
	UPROPERTY(meta = (BindWidgetOptional)) UVerticalBox* AddFilterBox = nullptr;
	UPROPERTY(meta = (BindWidgetOptional)) UButton* AddFilterButton = nullptr;

	// --- Snapshot Display ---
	UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* AvailableLevelsText = nullptr;
	UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* CoreVersionText = nullptr;
	UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* InitStatusText = nullptr;

	// --- Action Buttons ---
	UPROPERTY(meta = (BindWidgetOptional)) UButton* ApplyButton = nullptr;
	UPROPERTY(meta = (BindWidgetOptional)) UButton* ResetButton = nullptr;
	UPROPERTY(meta = (BindWidgetOptional)) UButton* RefreshButton = nullptr;

	// --- Message Display ---
	UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* StatusMessageText = nullptr;

	// --- State ---
	FLoggingConfig CurrentConfig;
	FLoggingConfig PendingConfig;
	FLoggingSnapshot CurrentSnapshot;

	// --- Internal Methods ---
	void UpdateLevelButtonStates();
	void UpdateFormatCheckboxes();
	void UpdateModuleFilterDisplay();
	void UpdateSnapshotDisplay();
	void DisplayMessage(const FString& Message, bool bSuccess = true);
	FString SerializeConfigToJson() const;
	void DeserializeConfigFromJson(const FString& Json);
	ELogLevel StringToLogLevel(const FString& LevelStr) const;
	FString LogLevelToString(ELogLevel Level) const;
	FString LogLevelToDisplayString(ELogLevel Level) const;

	// --- Button Callbacks ---
	UFUNCTION() void OnTraceLevelClicked();
	UFUNCTION() void OnDebugLevelClicked();
	UFUNCTION() void OnInfoLevelClicked();
	UFUNCTION() void OnWarnLevelClicked();
	UFUNCTION() void OnErrorLevelClicked();

	UFUNCTION() void OnShowTimestampsChanged(bool bIsChecked);
	UFUNCTION() void OnShowThreadIdsChanged(bool bIsChecked);
	UFUNCTION() void OnShowTargetsChanged(bool bIsChecked);
	UFUNCTION() void OnShowFileLineChanged(bool bIsChecked);

	UFUNCTION() void OnAddFilterClicked();
	UFUNCTION() void OnApplyClicked();
	UFUNCTION() void OnResetClicked();
	UFUNCTION() void OnRefreshClicked();

	// --- Bridge Access ---
	FProceduralCoreBridge* GetBridge() const;
};
