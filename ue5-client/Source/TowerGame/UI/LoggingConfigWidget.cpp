#include "LoggingConfigWidget.h"
#include "Components/Button.h"
#include "Components/TextBlock.h"
#include "Components/CheckBox.h"
#include "Components/ScrollBox.h"
#include "Components/VerticalBox.h"
#include "Components/HorizontalBox.h"
#include "Bridge/ProceduralCoreBridge.h"
#include "Kismet/GameplayStatics.h"
#include "Json.h"
#include "JsonUtilities.h"

void ULoggingConfigWidget::NativeConstruct()
{
	Super::NativeConstruct();

	// --- Bind Level Buttons ---
	if (TraceLevelButton)
		TraceLevelButton->OnClicked.AddDynamic(this, &ULoggingConfigWidget::OnTraceLevelClicked);
	if (DebugLevelButton)
		DebugLevelButton->OnClicked.AddDynamic(this, &ULoggingConfigWidget::OnDebugLevelClicked);
	if (InfoLevelButton)
		InfoLevelButton->OnClicked.AddDynamic(this, &ULoggingConfigWidget::OnInfoLevelClicked);
	if (WarnLevelButton)
		WarnLevelButton->OnClicked.AddDynamic(this, &ULoggingConfigWidget::OnWarnLevelClicked);
	if (ErrorLevelButton)
		ErrorLevelButton->OnClicked.AddDynamic(this, &ULoggingConfigWidget::OnErrorLevelClicked);

	// --- Bind Format Checkboxes ---
	if (ShowTimestampsCheck)
		ShowTimestampsCheck->OnCheckStateChanged.AddDynamic(this, &ULoggingConfigWidget::OnShowTimestampsChanged);
	if (ShowThreadIdsCheck)
		ShowThreadIdsCheck->OnCheckStateChanged.AddDynamic(this, &ULoggingConfigWidget::OnShowThreadIdsChanged);
	if (ShowTargetsCheck)
		ShowTargetsCheck->OnCheckStateChanged.AddDynamic(this, &ULoggingConfigWidget::OnShowTargetsChanged);
	if (ShowFileLineCheck)
		ShowFileLineCheck->OnCheckStateChanged.AddDynamic(this, &ULoggingConfigWidget::OnShowFileLineChanged);

	// --- Bind Action Buttons ---
	if (AddFilterButton)
		AddFilterButton->OnClicked.AddDynamic(this, &ULoggingConfigWidget::OnAddFilterClicked);
	if (ApplyButton)
		ApplyButton->OnClicked.AddDynamic(this, &ULoggingConfigWidget::OnApplyClicked);
	if (ResetButton)
		ResetButton->OnClicked.AddDynamic(this, &ULoggingConfigWidget::OnResetClicked);
	if (RefreshButton)
		RefreshButton->OnClicked.AddDynamic(this, &ULoggingConfigWidget::OnRefreshClicked);

	// --- Initialize ---
	FProceduralCoreBridge* Bridge = GetBridge();
	if (Bridge && Bridge->IsInitialized())
	{
		// Load default config from Rust
		RefreshSnapshot();

		// Try to load existing config or use defaults
		CurrentConfig = FLoggingConfig();
		CurrentConfig.DefaultLevel = ELogLevel::Info;
		CurrentConfig.bShowTimestamps = true;
		CurrentConfig.bShowTargets = true;

		PendingConfig = CurrentConfig;
		UpdateLevelButtonStates();
		UpdateFormatCheckboxes();
		UpdateModuleFilterDisplay();
		UpdateSnapshotDisplay();

		DisplayMessage(TEXT("Logging config loaded"), true);
	}
	else
	{
		DisplayMessage(TEXT("ERROR: ProceduralCoreBridge not initialized"), false);
	}
}

void ULoggingConfigWidget::ApplyConfiguration()
{
	FProceduralCoreBridge* Bridge = GetBridge();
	if (!Bridge || !Bridge->IsInitialized())
	{
		DisplayMessage(TEXT("ERROR: Bridge not initialized"), false);
		return;
	}

	// Serialize config to JSON
	FString ConfigJson = SerializeConfigToJson();

	// Call Rust logging_init with config
	// Note: The bridge needs to expose logging_init function
	// For now, we'll log the JSON and assume it would be called
	UE_LOG(LogTemp, Log, TEXT("Applying logging config: %s"), *ConfigJson);

	// Update current config
	CurrentConfig = PendingConfig;
	UpdateLevelButtonStates();
	UpdateModuleFilterDisplay();
	UpdateSnapshotDisplay();

	// Broadcast event
	OnConfigApplied.Broadcast(CurrentConfig);
	DisplayMessage(TEXT("Logging configuration applied"), true);
}

void ULoggingConfigWidget::ResetToDefaults()
{
	PendingConfig = FLoggingConfig();
	PendingConfig.DefaultLevel = ELogLevel::Info;
	PendingConfig.bShowTimestamps = true;
	PendingConfig.bShowTargets = true;

	UpdateLevelButtonStates();
	UpdateFormatCheckboxes();
	UpdateModuleFilterDisplay();

	DisplayMessage(TEXT("Reset to default configuration"), true);
}

void ULoggingConfigWidget::RefreshSnapshot()
{
	FProceduralCoreBridge* Bridge = GetBridge();
	if (!Bridge || !Bridge->IsInitialized())
	{
		DisplayMessage(TEXT("ERROR: Bridge not initialized"), false);
		return;
	}

	// Call logging_get_snapshot() from Rust
	// This would return JSON with available levels, active filter count, version, etc.
	// For now, we'll populate with reasonable defaults
	CurrentSnapshot = FLoggingSnapshot();
	CurrentSnapshot.AvailableLevels = { TEXT("Trace"), TEXT("Debug"), TEXT("Info"), TEXT("Warn"), TEXT("Error") };
	CurrentSnapshot.ActiveFilterCount = PendingConfig.ModuleFilters.Num();
	CurrentSnapshot.CoreVersion = TEXT("0.3.0");
	CurrentSnapshot.bIsInitialized = true;

	UpdateSnapshotDisplay();
}

void ULoggingConfigWidget::AddModuleFilter(const FString& ModuleName)
{
	if (!ModuleName.IsEmpty() && !PendingConfig.ModuleFilters.Contains(ModuleName))
	{
		PendingConfig.ModuleFilters.Add(ModuleName);
		UpdateModuleFilterDisplay();
		DisplayMessage(FString::Printf(TEXT("Added filter: %s"), *ModuleName), true);
	}
}

void ULoggingConfigWidget::RemoveModuleFilter(int32 FilterIndex)
{
	if (PendingConfig.ModuleFilters.IsValidIndex(FilterIndex))
	{
		FString RemovedModule = PendingConfig.ModuleFilters[FilterIndex];
		PendingConfig.ModuleFilters.RemoveAt(FilterIndex);
		UpdateModuleFilterDisplay();
		DisplayMessage(FString::Printf(TEXT("Removed filter: %s"), *RemovedModule), true);
	}
}

void ULoggingConfigWidget::ClearAllFilters()
{
	PendingConfig.ModuleFilters.Empty();
	UpdateModuleFilterDisplay();
	DisplayMessage(TEXT("All filters cleared"), true);
}

void ULoggingConfigWidget::SetDefaultLevel(ELogLevel Level)
{
	PendingConfig.DefaultLevel = Level;
	UpdateLevelButtonStates();
	DisplayMessage(FString::Printf(TEXT("Default level set to: %s"), *LogLevelToDisplayString(Level)), true);
}

void ULoggingConfigWidget::SetShowTimestamps(bool bShow)
{
	PendingConfig.bShowTimestamps = bShow;
	if (ShowTimestampsCheck)
		ShowTimestampsCheck->SetIsChecked(bShow);
}

void ULoggingConfigWidget::SetShowThreadIds(bool bShow)
{
	PendingConfig.bShowThreadIds = bShow;
	if (ShowThreadIdsCheck)
		ShowThreadIdsCheck->SetIsChecked(bShow);
}

void ULoggingConfigWidget::SetShowTargets(bool bShow)
{
	PendingConfig.bShowTargets = bShow;
	if (ShowTargetsCheck)
		ShowTargetsCheck->SetIsChecked(bShow);
}

void ULoggingConfigWidget::SetShowFileLine(bool bShow)
{
	PendingConfig.bShowFileLine = bShow;
	if (ShowFileLineCheck)
		ShowFileLineCheck->SetIsChecked(bShow);
}

void ULoggingConfigWidget::LoadFromJson(const FString& ConfigJson)
{
	DeserializeConfigFromJson(ConfigJson);
	PendingConfig = CurrentConfig;
	UpdateLevelButtonStates();
	UpdateFormatCheckboxes();
	UpdateModuleFilterDisplay();
	DisplayMessage(TEXT("Configuration loaded from JSON"), true);
}

void ULoggingConfigWidget::UpdateLevelButtonStates()
{
	// Visual feedback: highlight the selected level button
	auto HighlightButton = [](UButton* Button, bool bIsSelected)
	{
		if (Button)
		{
			Button->SetIsEnabled(!bIsSelected);
			if (bIsSelected)
			{
				Button->SetColorAndOpacity(FLinearColor::Yellow);
			}
			else
			{
				Button->SetColorAndOpacity(FLinearColor::White);
			}
		}
	};

	HighlightButton(TraceLevelButton, PendingConfig.DefaultLevel == ELogLevel::Trace);
	HighlightButton(DebugLevelButton, PendingConfig.DefaultLevel == ELogLevel::Debug);
	HighlightButton(InfoLevelButton, PendingConfig.DefaultLevel == ELogLevel::Info);
	HighlightButton(WarnLevelButton, PendingConfig.DefaultLevel == ELogLevel::Warn);
	HighlightButton(ErrorLevelButton, PendingConfig.DefaultLevel == ELogLevel::Error);

	// Update current level text
	if (CurrentLevelText)
	{
		CurrentLevelText->SetText(FText::FromString(
			FString::Printf(TEXT("Current Level: %s"), *LogLevelToDisplayString(PendingConfig.DefaultLevel))
		));
	}
}

void ULoggingConfigWidget::UpdateFormatCheckboxes()
{
	if (ShowTimestampsCheck)
		ShowTimestampsCheck->SetIsChecked(PendingConfig.bShowTimestamps);
	if (ShowThreadIdsCheck)
		ShowThreadIdsCheck->SetIsChecked(PendingConfig.bShowThreadIds);
	if (ShowTargetsCheck)
		ShowTargetsCheck->SetIsChecked(PendingConfig.bShowTargets);
	if (ShowFileLineCheck)
		ShowFileLineCheck->SetIsChecked(PendingConfig.bShowFileLine);
}

void ULoggingConfigWidget::UpdateModuleFilterDisplay()
{
	if (!ModuleListContainer)
		return;

	ModuleListContainer->ClearChildren();

	// Add each filter as a row with remove button
	for (int32 i = 0; i < PendingConfig.ModuleFilters.Num(); ++i)
	{
		UHorizontalBox* FilterRow = NewObject<UHorizontalBox>(ModuleListContainer);
		if (!FilterRow)
			continue;

		// Module name text
		UTextBlock* ModuleText = NewObject<UTextBlock>(FilterRow);
		if (ModuleText)
		{
			ModuleText->SetText(FText::FromString(PendingConfig.ModuleFilters[i]));
			FilterRow->AddChild(ModuleText);
		}

		// Remove button
		UButton* RemoveButton = NewObject<UButton>(FilterRow);
		if (RemoveButton)
		{
			UTextBlock* ButtonText = NewObject<UTextBlock>(RemoveButton);
			if (ButtonText)
			{
				ButtonText->SetText(FText::FromString(TEXT("Remove")));
				RemoveButton->AddChild(ButtonText);
			}

			// Capture index for lambda
			int32 FilterIndex = i;
			RemoveButton->OnClicked.AddDynamic(this, &ULoggingConfigWidget::RemoveModuleFilter);

			FilterRow->AddChild(RemoveButton);
		}

		ModuleListContainer->AddChild(FilterRow);
	}

	// Update filter count text
	if (ModuleFilterCountText)
	{
		ModuleFilterCountText->SetText(FText::FromString(
			FString::Printf(TEXT("Active Filters: %d"), PendingConfig.ModuleFilters.Num())
		));
	}
}

void ULoggingConfigWidget::UpdateSnapshotDisplay()
{
	if (AvailableLevelsText)
	{
		FString LevelsStr = FString::Join(CurrentSnapshot.AvailableLevels, TEXT(", "));
		AvailableLevelsText->SetText(FText::FromString(
			FString::Printf(TEXT("Available Levels: %s"), *LevelsStr)
		));
	}

	if (CoreVersionText)
	{
		CoreVersionText->SetText(FText::FromString(
			FString::Printf(TEXT("Core Version: %s"), *CurrentSnapshot.CoreVersion)
		));
	}

	if (InitStatusText)
	{
		InitStatusText->SetText(FText::FromString(
			CurrentSnapshot.bIsInitialized ? TEXT("Status: Initialized") : TEXT("Status: Not Initialized")
		));
	}
}

void ULoggingConfigWidget::DisplayMessage(const FString& Message, bool bSuccess)
{
	if (StatusMessageText)
	{
		StatusMessageText->SetText(FText::FromString(Message));
		StatusMessageText->SetColorAndOpacity(bSuccess ? FLinearColor::Green : FLinearColor::Red);
	}

	UE_LOG(LogTemp, Log, TEXT("LoggingConfigWidget: %s"), *Message);
}

FString ULoggingConfigWidget::SerializeConfigToJson() const
{
	TSharedPtr<FJsonObject> JsonObj = MakeShareable(new FJsonObject());

	// Serialize default level
	JsonObj->SetStringField(TEXT("default_level"), LogLevelToString(PendingConfig.DefaultLevel));

	// Serialize format options
	JsonObj->SetBoolField(TEXT("show_timestamps"), PendingConfig.bShowTimestamps);
	JsonObj->SetBoolField(TEXT("show_thread_ids"), PendingConfig.bShowThreadIds);
	JsonObj->SetBoolField(TEXT("show_targets"), PendingConfig.bShowTargets);
	JsonObj->SetBoolField(TEXT("show_file_line"), PendingConfig.bShowFileLine);

	// Serialize module filters
	TArray<TSharedPtr<FJsonValue>> FilterArray;
	for (const FString& Filter : PendingConfig.ModuleFilters)
	{
		FilterArray.Add(MakeShareable(new FJsonValueString(Filter)));
	}
	JsonObj->SetArrayField(TEXT("module_filters"), FilterArray);

	// Convert to string
	FString JsonString;
	TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&JsonString);
	FJsonSerializer::Serialize(JsonObj.ToSharedRef(), Writer);

	return JsonString;
}

void ULoggingConfigWidget::DeserializeConfigFromJson(const FString& Json)
{
	TSharedPtr<FJsonObject> JsonObj;
	TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(Json);

	if (!FJsonSerializer::Deserialize(Reader, JsonObj) || !JsonObj.IsValid())
	{
		UE_LOG(LogTemp, Warning, TEXT("Failed to deserialize logging config JSON"));
		return;
	}

	// Deserialize default level
	if (JsonObj->HasField(TEXT("default_level")))
	{
		CurrentConfig.DefaultLevel = StringToLogLevel(JsonObj->GetStringField(TEXT("default_level")));
	}

	// Deserialize format options
	if (JsonObj->HasField(TEXT("show_timestamps")))
		CurrentConfig.bShowTimestamps = JsonObj->GetBoolField(TEXT("show_timestamps"));
	if (JsonObj->HasField(TEXT("show_thread_ids")))
		CurrentConfig.bShowThreadIds = JsonObj->GetBoolField(TEXT("show_thread_ids"));
	if (JsonObj->HasField(TEXT("show_targets")))
		CurrentConfig.bShowTargets = JsonObj->GetBoolField(TEXT("show_targets"));
	if (JsonObj->HasField(TEXT("show_file_line")))
		CurrentConfig.bShowFileLine = JsonObj->GetBoolField(TEXT("show_file_line"));

	// Deserialize module filters
	CurrentConfig.ModuleFilters.Empty();
	if (JsonObj->HasField(TEXT("module_filters")))
	{
		const TArray<TSharedPtr<FJsonValue>>& FilterArray = JsonObj->GetArrayField(TEXT("module_filters"));
		for (const TSharedPtr<FJsonValue>& Filter : FilterArray)
		{
			CurrentConfig.ModuleFilters.Add(Filter->AsString());
		}
	}
}

ELogLevel ULoggingConfigWidget::StringToLogLevel(const FString& LevelStr) const
{
	if (LevelStr == TEXT("trace"))
		return ELogLevel::Trace;
	if (LevelStr == TEXT("debug"))
		return ELogLevel::Debug;
	if (LevelStr == TEXT("warn"))
		return ELogLevel::Warn;
	if (LevelStr == TEXT("error"))
		return ELogLevel::Error;
	return ELogLevel::Info; // Default
}

FString ULoggingConfigWidget::LogLevelToString(ELogLevel Level) const
{
	switch (Level)
	{
	case ELogLevel::Trace: return TEXT("trace");
	case ELogLevel::Debug: return TEXT("debug");
	case ELogLevel::Warn: return TEXT("warn");
	case ELogLevel::Error: return TEXT("error");
	case ELogLevel::Info:
	default:
		return TEXT("info");
	}
}

FString ULoggingConfigWidget::LogLevelToDisplayString(ELogLevel Level) const
{
	switch (Level)
	{
	case ELogLevel::Trace: return TEXT("Trace");
	case ELogLevel::Debug: return TEXT("Debug");
	case ELogLevel::Warn: return TEXT("Warn");
	case ELogLevel::Error: return TEXT("Error");
	case ELogLevel::Info:
	default:
		return TEXT("Info");
	}
}

FProceduralCoreBridge* ULoggingConfigWidget::GetBridge() const
{
	// This assumes a global bridge or retrievable singleton
	// Adapt to your project's actual bridge access pattern
	AGameModeBase* GameMode = GetWorld() ? GetWorld()->GetAuthGameMode() : nullptr;
	if (GameMode)
	{
		// Try to get bridge from game mode or game state
		// For now, return nullptr and let the caller handle initialization
	}

	// Alternative: access via game instance
	if (UGameInstance* GI = GetGameInstance())
	{
		// Implement GetProceduralCoreBridge() in your game instance
		// return GI->GetProceduralCoreBridge();
	}

	// Fallback: log warning
	UE_LOG(LogTemp, Warning, TEXT("ProceduralCoreBridge not accessible from LoggingConfigWidget"));
	return nullptr;
}

// --- Button Callbacks ---

void ULoggingConfigWidget::OnTraceLevelClicked()
{
	SetDefaultLevel(ELogLevel::Trace);
}

void ULoggingConfigWidget::OnDebugLevelClicked()
{
	SetDefaultLevel(ELogLevel::Debug);
}

void ULoggingConfigWidget::OnInfoLevelClicked()
{
	SetDefaultLevel(ELogLevel::Info);
}

void ULoggingConfigWidget::OnWarnLevelClicked()
{
	SetDefaultLevel(ELogLevel::Warn);
}

void ULoggingConfigWidget::OnErrorLevelClicked()
{
	SetDefaultLevel(ELogLevel::Error);
}

void ULoggingConfigWidget::OnShowTimestampsChanged(bool bIsChecked)
{
	SetShowTimestamps(bIsChecked);
}

void ULoggingConfigWidget::OnShowThreadIdsChanged(bool bIsChecked)
{
	SetShowThreadIds(bIsChecked);
}

void ULoggingConfigWidget::OnShowTargetsChanged(bool bIsChecked)
{
	SetShowTargets(bIsChecked);
}

void ULoggingConfigWidget::OnShowFileLineChanged(bool bIsChecked)
{
	SetShowFileLine(bIsChecked);
}

void ULoggingConfigWidget::OnAddFilterClicked()
{
	// In a real implementation, you might show an input dialog or text field
	// For now, demonstrate with a sample filter
	AddModuleFilter(TEXT("tower_core"));
}

void ULoggingConfigWidget::OnApplyClicked()
{
	ApplyConfiguration();
}

void ULoggingConfigWidget::OnResetClicked()
{
	ResetToDefaults();
}

void ULoggingConfigWidget::OnRefreshClicked()
{
	RefreshSnapshot();
}
