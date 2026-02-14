// Copyright Epic Games, Inc. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "Components/TextBlock.h"
#include "Components/VerticalBox.h"
#include "Components/Button.h"
#include "Components/Image.h"
#include "Components/Border.h"
#include "Components/HorizontalBox.h"
#include "MutatorWidget.generated.h"

/**
 * Represents a single floor mutator with all its properties
 */
USTRUCT(BlueprintType)
struct FMutatorData
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadWrite)
	FString Id;

	UPROPERTY(BlueprintReadWrite)
	FString Name;

	UPROPERTY(BlueprintReadWrite)
	FString Description;

	UPROPERTY(BlueprintReadWrite)
	int32 Difficulty;

	UPROPERTY(BlueprintReadWrite)
	FString Category;

	UPROPERTY(BlueprintReadWrite)
	FString IconPath;

	FMutatorData()
		: Difficulty(1)
	{}
};

/**
 * Aggregate effects from all active mutators
 */
USTRUCT(BlueprintType)
struct FMutatorEffects
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadWrite)
	float DamageMultiplier;

	UPROPERTY(BlueprintReadWrite)
	float HealthMultiplier;

	UPROPERTY(BlueprintReadWrite)
	float LootMultiplier;

	UPROPERTY(BlueprintReadWrite)
	float ExperienceMultiplier;

	UPROPERTY(BlueprintReadWrite)
	float MovementSpeedMultiplier;

	UPROPERTY(BlueprintReadWrite)
	float RewardMultiplier;

	FMutatorEffects()
		: DamageMultiplier(1.0f)
		, HealthMultiplier(1.0f)
		, LootMultiplier(1.0f)
		, ExperienceMultiplier(1.0f)
		, MovementSpeedMultiplier(1.0f)
		, RewardMultiplier(1.0f)
	{}
};

/**
 * Widget for displaying floor mutators before starting a floor
 * Fetches mutator data from Rust FFI bridge and displays:
 * - Floor title
 * - List of active mutators with icons, descriptions, difficulty
 * - Total difficulty and reward multiplier
 * - Aggregate effects summary
 * - BEGIN button to confirm and start floor
 */
UCLASS(BlueprintType)
class TOWERGAME_API UMutatorWidget : public UUserWidget
{
	GENERATED_BODY()

public:
	UMutatorWidget(const FObjectInitializer& ObjectInitializer);

	// Widget lifecycle
	virtual void NativeConstruct() override;
	virtual void NativeTick(const FGeometry& MyGeometry, float InDeltaTime) override;

	/**
	 * Initialize the widget with floor data
	 * Calls Rust FFI to generate mutators for the given floor
	 */
	UFUNCTION(BlueprintCallable, Category = "Mutator")
	void InitializeForFloor(int32 FloorNumber, int32 Seed);

	/**
	 * Manually set mutator data (for testing or external sources)
	 */
	UFUNCTION(BlueprintCallable, Category = "Mutator")
	void SetMutatorData(const TArray<FMutatorData>& Mutators, const FMutatorEffects& Effects);

protected:
	// BindWidget properties - must match widget blueprint hierarchy
	UPROPERTY(meta = (BindWidget))
	UTextBlock* FloorTitleText;

	UPROPERTY(meta = (BindWidget))
	UVerticalBox* MutatorListBox;

	UPROPERTY(meta = (BindWidget))
	UTextBlock* TotalDifficultyText;

	UPROPERTY(meta = (BindWidget))
	UTextBlock* RewardMultiplierText;

	UPROPERTY(meta = (BindWidget))
	UVerticalBox* EffectsSummaryBox;

	UPROPERTY(meta = (BindWidget))
	UButton* BeginButton;

	UPROPERTY(meta = (BindWidget))
	UBorder* MainBorder;

	// Button callbacks
	UFUNCTION()
	void OnBeginButtonClicked();

	// Event dispatchers
	UPROPERTY(BlueprintAssignable, Category = "Mutator")
	FOnButtonClickedEvent OnFloorBegin;

private:
	// Internal state
	int32 CurrentFloor;
	TArray<FMutatorData> ActiveMutators;
	FMutatorEffects AggregateEffects;
	float AnimationTime;

	// UI building methods
	void BuildMutatorList();
	void BuildEffectsSummary();
	void UpdateTotalStats();

	// Helper methods
	UWidget* CreateMutatorEntry(const FMutatorData& Mutator);
	UWidget* CreateDifficultyStars(int32 Difficulty);
	UWidget* CreateCategoryBadge(const FString& Category);
	UWidget* CreateEffectRow(const FString& EffectName, float Multiplier);

	FLinearColor GetDifficultyColor(int32 Difficulty) const;
	FLinearColor GetCategoryColor(const FString& Category) const;
	FString GetEffectDisplayText(float Multiplier) const;

	// JSON parsing from Rust FFI response
	bool ParseMutatorJSON(const FString& JSONString);

	// Animation helpers
	void UpdateAnimations(float DeltaTime);
};
