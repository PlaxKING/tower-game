// Copyright Epic Games, Inc. All Rights Reserved.

#include "SaveMigrationWidget.h"
#include "Components/TextBlock.h"
#include "Components/Button.h"
#include "Components/ProgressBar.h"
#include "Components/VerticalBox.h"
#include "Components/Border.h"
#include "Json.h"
#include "JsonUtilities.h"

// Forward declaration for Rust FFI bridge
extern "C" {
	const char* migrate_save(const char* save_json);
	uint32_t get_save_version(const char* save_json);
	uint32_t get_current_save_version();
	uint32_t validate_save(const char* save_json);
	void free_rust_string(char* ptr);
}

USaveMigrationWidget::USaveMigrationWidget(const FObjectInitializer& ObjectInitializer)
	: Super(ObjectInitializer)
{
	// Initialize color scheme
	SuccessColor = FLinearColor(0.0f, 0.8f, 0.2f, 1.0f); // Green
	ErrorColor = FLinearColor(0.9f, 0.1f, 0.1f, 1.0f);   // Red
	ProgressColor = FLinearColor(1.0f, 0.8f, 0.0f, 1.0f); // Yellow
	NeutralColor = FLinearColor(0.7f, 0.7f, 0.7f, 1.0f);  // Gray
}

void USaveMigrationWidget::NativeConstruct()
{
	Super::NativeConstruct();

	// Bind button events
	if (ContinueButton)
	{
		ContinueButton->OnClicked.AddDynamic(this, &USaveMigrationWidget::OnContinueClicked);
		ContinueButton->SetIsEnabled(false);
	}

	// Initialize progress bar
	if (MigrationProgressBar)
	{
		MigrationProgressBar->SetVisibility(ESlateVisibility::Collapsed);
	}

	// Initialize error text
	if (ErrorText)
	{
		ErrorText->SetVisibility(ESlateVisibility::Collapsed);
	}
}

void USaveMigrationWidget::InitializeMigration(const FString& SaveJson)
{
	SaveDataJson = SaveJson;

	// Validate save data
	if (!CallValidateSave(SaveJson))
	{
		DisplayMigrationResult(false, 0, 0, TArray<FString>(),
			TEXT("Invalid save data format"));
		return;
	}

	// Get versions
	int32 CurrentVersion = CallGetSaveVersion(SaveJson);
	int32 TargetVersion = CallGetCurrentSaveVersion();

	// Update UI
	if (TitleText)
	{
		TitleText->SetText(FText::FromString(TEXT("Save File Update Required")));
	}

	UpdateVersionDisplay(CurrentVersion, TargetVersion);

	if (StatusText)
	{
		StatusText->SetText(FText::FromString(
			TEXT("Your save file needs to be updated to the latest version. Click Continue to proceed.")));
	}

	// Enable continue button
	if (ContinueButton)
	{
		ContinueButton->SetIsEnabled(true);
	}

	SetProgressState();
}

void USaveMigrationWidget::StartMigration()
{
	if (SaveDataJson.IsEmpty())
	{
		DisplayMigrationResult(false, 0, 0, TArray<FString>(),
			TEXT("No save data to migrate"));
		return;
	}

	// Show progress
	SetProgressState();
	if (MigrationProgressBar)
	{
		MigrationProgressBar->SetVisibility(ESlateVisibility::Visible);
		MigrationProgressBar->SetPercent(0.5f);
	}

	if (StatusText)
	{
		StatusText->SetText(FText::FromString(TEXT("Migrating save file...")));
	}

	if (ContinueButton)
	{
		ContinueButton->SetIsEnabled(false);
	}

	// Call Rust migration
	FString MigrationResultJson = CallMigrateSave(SaveDataJson);

	// Parse and display results
	ParseAndDisplayMigrationResult(MigrationResultJson);

	if (MigrationProgressBar)
	{
		MigrationProgressBar->SetPercent(1.0f);
	}
}

void USaveMigrationWidget::DisplayMigrationResult(bool bSuccess, int32 OriginalVersion,
	int32 FinalVersion, const TArray<FString>& StepsApplied, const FString& ErrorMessage)
{
	// Update title
	if (TitleText)
	{
		if (bSuccess)
		{
			TitleText->SetText(FText::FromString(TEXT("Save Updated Successfully")));
			SetSuccessState();
		}
		else
		{
			TitleText->SetText(FText::FromString(TEXT("Save Migration Failed")));
			SetErrorState();
		}
	}

	// Update version display
	UpdateVersionDisplay(OriginalVersion, FinalVersion);

	// Display migration steps
	ClearStepsList();
	for (const FString& Step : StepsApplied)
	{
		AddMigrationStep(Step);
	}

	// Display error if present
	if (!ErrorMessage.IsEmpty() && ErrorText)
	{
		ErrorText->SetText(FText::FromString(ErrorMessage));
		ErrorText->SetVisibility(ESlateVisibility::Visible);
	}

	// Update status
	if (StatusText)
	{
		if (bSuccess)
		{
			StatusText->SetText(FText::FromString(
				TEXT("Your save file has been successfully updated. Click Continue to proceed.")));
		}
		else
		{
			StatusText->SetText(FText::FromString(
				TEXT("Failed to update save file. Please check the error message below.")));
		}
	}

	// Hide progress bar
	if (MigrationProgressBar)
	{
		MigrationProgressBar->SetVisibility(ESlateVisibility::Collapsed);
	}

	// Enable continue button
	if (ContinueButton)
	{
		ContinueButton->SetIsEnabled(true);
	}

	// Broadcast completion
	OnMigrationComplete.Broadcast(bSuccess);
}

void USaveMigrationWidget::OnContinueClicked()
{
	// Remove from parent or hide
	RemoveFromParent();
}

void USaveMigrationWidget::UpdateVersionDisplay(int32 OriginalVersion, int32 NewVersion)
{
	if (VersionText)
	{
		FString VersionString = FString::Printf(TEXT("Version %d → %d"),
			OriginalVersion, NewVersion);
		VersionText->SetText(FText::FromString(VersionString));
	}
}

void USaveMigrationWidget::AddMigrationStep(const FString& StepDescription)
{
	if (!StepsListBox)
	{
		return;
	}

	// Create a text block for the step
	UTextBlock* StepText = NewObject<UTextBlock>(this);
	if (StepText)
	{
		StepText->SetText(FText::FromString(FString::Printf(TEXT("✓ %s"), *StepDescription)));
		StepText->SetColorAndOpacity(FSlateColor(SuccessColor));

		// Add to vertical box
		StepsListBox->AddChildToVerticalBox(StepText);
	}
}

void USaveMigrationWidget::ClearStepsList()
{
	if (StepsListBox)
	{
		StepsListBox->ClearChildren();
	}
}

void USaveMigrationWidget::SetSuccessState()
{
	if (MainBorder)
	{
		MainBorder->SetBrushColor(SuccessColor);
	}
}

void USaveMigrationWidget::SetErrorState()
{
	if (MainBorder)
	{
		MainBorder->SetBrushColor(ErrorColor);
	}
}

void USaveMigrationWidget::SetProgressState()
{
	if (MainBorder)
	{
		MainBorder->SetBrushColor(ProgressColor);
	}
}

void USaveMigrationWidget::ParseAndDisplayMigrationResult(const FString& MigrationResultJson)
{
	// Parse JSON result
	TSharedPtr<FJsonObject> JsonObject;
	TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(MigrationResultJson);

	if (!FJsonSerializer::Deserialize(Reader, JsonObject) || !JsonObject.IsValid())
	{
		DisplayMigrationResult(false, 0, 0, TArray<FString>(),
			TEXT("Failed to parse migration result"));
		return;
	}

	// Extract fields
	bool bSuccess = JsonObject->GetBoolField(TEXT("success"));
	int32 OriginalVersion = JsonObject->GetIntegerField(TEXT("original_version"));
	int32 FinalVersion = JsonObject->GetIntegerField(TEXT("final_version"));

	// Extract steps
	TArray<FString> Steps;
	const TArray<TSharedPtr<FJsonValue>>* StepsArray;
	if (JsonObject->TryGetArrayField(TEXT("steps_applied"), StepsArray))
	{
		for (const TSharedPtr<FJsonValue>& StepValue : *StepsArray)
		{
			Steps.Add(StepValue->AsString());
		}
	}

	// Extract error message
	FString ErrorMessage;
	if (JsonObject->HasField(TEXT("error")))
	{
		ErrorMessage = JsonObject->GetStringField(TEXT("error"));
	}

	// Display results
	DisplayMigrationResult(bSuccess, OriginalVersion, FinalVersion, Steps, ErrorMessage);
}

// Rust FFI bridge function implementations
FString USaveMigrationWidget::CallMigrateSave(const FString& SaveJson)
{
	const char* SaveJsonCStr = TCHAR_TO_UTF8(*SaveJson);
	const char* ResultCStr = migrate_save(SaveJsonCStr);

	FString Result = FString(UTF8_TO_TCHAR(ResultCStr));

	// Free Rust-allocated string
	free_rust_string(const_cast<char*>(ResultCStr));

	return Result;
}

int32 USaveMigrationWidget::CallGetSaveVersion(const FString& SaveJson)
{
	const char* SaveJsonCStr = TCHAR_TO_UTF8(*SaveJson);
	return static_cast<int32>(get_save_version(SaveJsonCStr));
}

int32 USaveMigrationWidget::CallGetCurrentSaveVersion()
{
	return static_cast<int32>(get_current_save_version());
}

bool USaveMigrationWidget::CallValidateSave(const FString& SaveJson)
{
	const char* SaveJsonCStr = TCHAR_TO_UTF8(*SaveJson);
	return validate_save(SaveJsonCStr) == 1;
}
