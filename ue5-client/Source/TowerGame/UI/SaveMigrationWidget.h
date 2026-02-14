// Copyright Epic Games, Inc. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "SaveMigrationWidget.generated.h"

class UTextBlock;
class UButton;
class UProgressBar;
class UVerticalBox;
class UBorder;

/**
 * Widget for displaying save file migration status and results
 * Integrates with Rust FFI bridge via ProceduralCoreBridge
 */
UCLASS(BlueprintType)
class TOWERGAME_API USaveMigrationWidget : public UUserWidget
{
	GENERATED_BODY()

public:
	USaveMigrationWidget(const FObjectInitializer& ObjectInitializer);

	virtual void NativeConstruct() override;

	/**
	 * Initialize the widget with save data to migrate
	 * @param SaveJson JSON string of save data to migrate
	 */
	UFUNCTION(BlueprintCallable, Category = "Save Migration")
	void InitializeMigration(const FString& SaveJson);

	/**
	 * Start the migration process
	 */
	UFUNCTION(BlueprintCallable, Category = "Save Migration")
	void StartMigration();

	/**
	 * Display migration results
	 */
	UFUNCTION(BlueprintCallable, Category = "Save Migration")
	void DisplayMigrationResult(bool bSuccess, int32 OriginalVersion, int32 FinalVersion,
		const TArray<FString>& StepsApplied, const FString& ErrorMessage);

protected:
	// Widget Components
	UPROPERTY(meta = (BindWidget))
	UTextBlock* TitleText;

	UPROPERTY(meta = (BindWidget))
	UTextBlock* VersionText;

	UPROPERTY(meta = (BindWidget))
	UTextBlock* StatusText;

	UPROPERTY(meta = (BindWidget))
	UVerticalBox* StepsListBox;

	UPROPERTY(meta = (BindWidget))
	UTextBlock* ErrorText;

	UPROPERTY(meta = (BindWidget))
	UButton* ContinueButton;

	UPROPERTY(meta = (BindWidget))
	UProgressBar* MigrationProgressBar;

	UPROPERTY(meta = (BindWidget))
	UBorder* MainBorder;

	// Button callbacks
	UFUNCTION()
	void OnContinueClicked();

	// Helper functions
	void UpdateVersionDisplay(int32 OriginalVersion, int32 NewVersion);
	void AddMigrationStep(const FString& StepDescription);
	void ClearStepsList();
	void SetSuccessState();
	void SetErrorState();
	void SetProgressState();
	void ParseAndDisplayMigrationResult(const FString& MigrationResultJson);

	// Rust FFI bridge functions
	FString CallMigrateSave(const FString& SaveJson);
	int32 CallGetSaveVersion(const FString& SaveJson);
	int32 CallGetCurrentSaveVersion();
	bool CallValidateSave(const FString& SaveJson);

private:
	// Stored save data
	FString SaveDataJson;

	// Color scheme
	FLinearColor SuccessColor;
	FLinearColor ErrorColor;
	FLinearColor ProgressColor;
	FLinearColor NeutralColor;

	// Delegate for migration completion
	DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnMigrationComplete, bool, bSuccess);

public:
	UPROPERTY(BlueprintAssignable, Category = "Save Migration")
	FOnMigrationComplete OnMigrationComplete;
};
