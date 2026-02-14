// Copyright Epic Games, Inc. All Rights Reserved.

#include "UI/MutatorWidget.h"
#include "Components/TextBlock.h"
#include "Components/VerticalBox.h"
#include "Components/HorizontalBox.h"
#include "Components/Button.h"
#include "Components/Image.h"
#include "Components/Border.h"
#include "Components/VerticalBoxSlot.h"
#include "Components/HorizontalBoxSlot.h"
#include "Components/Spacer.h"
#include "Dom/JsonObject.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"
#include "Engine/Texture2D.h"
#include "UObject/ConstructorHelpers.h"

// Forward declare Rust FFI bridge
// In production, this would be in a separate header
extern "C" {
	const char* generate_floor_mutators(int32_t seed, int32_t floor_id);
	void free_rust_string(const char* ptr);
}

UMutatorWidget::UMutatorWidget(const FObjectInitializer& ObjectInitializer)
	: Super(ObjectInitializer)
	, CurrentFloor(0)
	, AnimationTime(0.0f)
{
}

void UMutatorWidget::NativeConstruct()
{
	Super::NativeConstruct();

	// Bind button events
	if (BeginButton)
	{
		BeginButton->OnClicked.AddDynamic(this, &UMutatorWidget::OnBeginButtonClicked);
	}

	// Initialize animation state
	AnimationTime = 0.0f;
}

void UMutatorWidget::NativeTick(const FGeometry& MyGeometry, float InDeltaTime)
{
	Super::NativeTick(MyGeometry, InDeltaTime);

	UpdateAnimations(InDeltaTime);
}

void UMutatorWidget::InitializeForFloor(int32 FloorNumber, int32 Seed)
{
	CurrentFloor = FloorNumber;

	// Call Rust FFI to generate mutators
	const char* JsonCStr = generate_floor_mutators(Seed, FloorNumber);

	if (JsonCStr)
	{
		FString JsonString(UTF8_TO_TCHAR(JsonCStr));

		// Parse the JSON response
		bool bSuccess = ParseMutatorJSON(JsonString);

		// Free the Rust-allocated string
		free_rust_string(JsonCStr);

		if (bSuccess)
		{
			// Update UI with parsed data
			if (FloorTitleText)
			{
				FloorTitleText->SetText(FText::FromString(FString::Printf(TEXT("Floor %d Mutators"), FloorNumber)));
			}

			BuildMutatorList();
			BuildEffectsSummary();
			UpdateTotalStats();
		}
		else
		{
			UE_LOG(LogTemp, Error, TEXT("Failed to parse mutator JSON for floor %d"), FloorNumber);
		}
	}
	else
	{
		UE_LOG(LogTemp, Error, TEXT("Rust FFI returned null for floor %d"), FloorNumber);
	}
}

void UMutatorWidget::SetMutatorData(const TArray<FMutatorData>& Mutators, const FMutatorEffects& Effects)
{
	ActiveMutators = Mutators;
	AggregateEffects = Effects;

	BuildMutatorList();
	BuildEffectsSummary();
	UpdateTotalStats();
}

void UMutatorWidget::OnBeginButtonClicked()
{
	// Broadcast event to notify game that floor should start
	OnFloorBegin.Broadcast();

	// Optional: Play click sound, add visual feedback
	UE_LOG(LogTemp, Log, TEXT("Floor %d begin clicked with %d mutators"), CurrentFloor, ActiveMutators.Num());
}

void UMutatorWidget::BuildMutatorList()
{
	if (!MutatorListBox)
	{
		return;
	}

	// Clear existing entries
	MutatorListBox->ClearChildren();

	// Create entry for each mutator
	for (const FMutatorData& Mutator : ActiveMutators)
	{
		UWidget* Entry = CreateMutatorEntry(Mutator);
		if (Entry)
		{
			MutatorListBox->AddChild(Entry);
		}
	}
}

void UMutatorWidget::BuildEffectsSummary()
{
	if (!EffectsSummaryBox)
	{
		return;
	}

	// Clear existing effects
	EffectsSummaryBox->ClearChildren();

	// Add effect rows for non-default values
	if (!FMath::IsNearlyEqual(AggregateEffects.DamageMultiplier, 1.0f))
	{
		EffectsSummaryBox->AddChild(CreateEffectRow(TEXT("Damage"), AggregateEffects.DamageMultiplier));
	}

	if (!FMath::IsNearlyEqual(AggregateEffects.HealthMultiplier, 1.0f))
	{
		EffectsSummaryBox->AddChild(CreateEffectRow(TEXT("Health"), AggregateEffects.HealthMultiplier));
	}

	if (!FMath::IsNearlyEqual(AggregateEffects.LootMultiplier, 1.0f))
	{
		EffectsSummaryBox->AddChild(CreateEffectRow(TEXT("Loot"), AggregateEffects.LootMultiplier));
	}

	if (!FMath::IsNearlyEqual(AggregateEffects.ExperienceMultiplier, 1.0f))
	{
		EffectsSummaryBox->AddChild(CreateEffectRow(TEXT("Experience"), AggregateEffects.ExperienceMultiplier));
	}

	if (!FMath::IsNearlyEqual(AggregateEffects.MovementSpeedMultiplier, 1.0f))
	{
		EffectsSummaryBox->AddChild(CreateEffectRow(TEXT("Movement Speed"), AggregateEffects.MovementSpeedMultiplier));
	}
}

void UMutatorWidget::UpdateTotalStats()
{
	// Calculate total difficulty
	int32 TotalDifficulty = 0;
	for (const FMutatorData& Mutator : ActiveMutators)
	{
		TotalDifficulty += Mutator.Difficulty;
	}

	// Update difficulty text with color coding
	if (TotalDifficultyText)
	{
		FLinearColor DifficultyColor = GetDifficultyColor(TotalDifficulty);
		TotalDifficultyText->SetText(FText::FromString(FString::Printf(TEXT("Total Difficulty: %d"), TotalDifficulty)));
		TotalDifficultyText->SetColorAndOpacity(FSlateColor(DifficultyColor));
	}

	// Update reward multiplier
	if (RewardMultiplierText)
	{
		FString MultiplierText = FString::Printf(TEXT("Reward Multiplier: x%.2f"), AggregateEffects.RewardMultiplier);
		RewardMultiplierText->SetText(FText::FromString(MultiplierText));

		// Green for positive multipliers
		FLinearColor MultiplierColor = AggregateEffects.RewardMultiplier > 1.0f
			? FLinearColor(0.2f, 1.0f, 0.2f)
			: FLinearColor::White;
		RewardMultiplierText->SetColorAndOpacity(FSlateColor(MultiplierColor));
	}
}

UWidget* UMutatorWidget::CreateMutatorEntry(const FMutatorData& Mutator)
{
	// Create horizontal box for mutator entry
	UHorizontalBox* EntryBox = NewObject<UHorizontalBox>(this);
	if (!EntryBox)
	{
		return nullptr;
	}

	// Create border for the entire entry
	UBorder* EntryBorder = NewObject<UBorder>(this);
	if (!EntryBorder)
	{
		return nullptr;
	}

	EntryBorder->SetPadding(FMargin(10.0f, 5.0f));
	EntryBorder->SetBrushColor(FLinearColor(0.1f, 0.1f, 0.1f, 0.8f));

	// Icon (placeholder for now)
	UImage* Icon = NewObject<UImage>(this);
	if (Icon)
	{
		Icon->SetColorAndOpacity(GetCategoryColor(Mutator.Category));
		UHorizontalBoxSlot* IconSlot = EntryBox->AddChildToHorizontalBox(Icon);
		IconSlot->SetSize(FSlateChildSize(ESlateSizeRule::Automatic));
		IconSlot->SetPadding(FMargin(0.0f, 0.0f, 10.0f, 0.0f));
	}

	// Create vertical box for text content
	UVerticalBox* ContentBox = NewObject<UVerticalBox>(this);
	if (ContentBox)
	{
		// Mutator name with category badge
		UHorizontalBox* HeaderBox = NewObject<UHorizontalBox>(this);
		if (HeaderBox)
		{
			UTextBlock* NameText = NewObject<UTextBlock>(this);
			if (NameText)
			{
				NameText->SetText(FText::FromString(Mutator.Name));
				NameText->Font.Size = 18;
				NameText->SetColorAndOpacity(FSlateColor(FLinearColor::White));
				HeaderBox->AddChildToHorizontalBox(NameText);
			}

			// Category badge
			UWidget* CategoryBadge = CreateCategoryBadge(Mutator.Category);
			if (CategoryBadge)
			{
				UHorizontalBoxSlot* BadgeSlot = HeaderBox->AddChildToHorizontalBox(CategoryBadge);
				BadgeSlot->SetPadding(FMargin(10.0f, 0.0f, 0.0f, 0.0f));
			}

			ContentBox->AddChildToVerticalBox(HeaderBox);
		}

		// Description
		UTextBlock* DescText = NewObject<UTextBlock>(this);
		if (DescText)
		{
			DescText->SetText(FText::FromString(Mutator.Description));
			DescText->Font.Size = 12;
			DescText->SetColorAndOpacity(FSlateColor(FLinearColor(0.7f, 0.7f, 0.7f)));
			DescText->SetAutoWrapText(true);
			ContentBox->AddChildToVerticalBox(DescText);
		}

		UHorizontalBoxSlot* ContentSlot = EntryBox->AddChildToHorizontalBox(ContentBox);
		ContentSlot->SetSize(FSlateChildSize(ESlateSizeRule::Fill));
	}

	// Difficulty stars
	UWidget* Stars = CreateDifficultyStars(Mutator.Difficulty);
	if (Stars)
	{
		UHorizontalBoxSlot* StarsSlot = EntryBox->AddChildToHorizontalBox(Stars);
		StarsSlot->SetSize(FSlateChildSize(ESlateSizeRule::Automatic));
		StarsSlot->SetPadding(FMargin(10.0f, 0.0f, 0.0f, 0.0f));
		StarsSlot->SetVerticalAlignment(VAlign_Center);
	}

	EntryBorder->AddChild(EntryBox);
	return EntryBorder;
}

UWidget* UMutatorWidget::CreateDifficultyStars(int32 Difficulty)
{
	UHorizontalBox* StarsBox = NewObject<UHorizontalBox>(this);
	if (!StarsBox)
	{
		return nullptr;
	}

	FLinearColor StarColor = GetDifficultyColor(Difficulty);

	for (int32 i = 0; i < Difficulty && i < 5; ++i)
	{
		UTextBlock* Star = NewObject<UTextBlock>(this);
		if (Star)
		{
			Star->SetText(FText::FromString(TEXT("★")));
			Star->Font.Size = 16;
			Star->SetColorAndOpacity(FSlateColor(StarColor));
			StarsBox->AddChildToHorizontalBox(Star);
		}
	}

	return StarsBox;
}

UWidget* UMutatorWidget::CreateCategoryBadge(const FString& Category)
{
	UBorder* Badge = NewObject<UBorder>(this);
	if (!Badge)
	{
		return nullptr;
	}

	FLinearColor CategoryColor = GetCategoryColor(Category);
	Badge->SetBrushColor(CategoryColor);
	Badge->SetPadding(FMargin(8.0f, 2.0f));

	UTextBlock* CategoryText = NewObject<UTextBlock>(this);
	if (CategoryText)
	{
		CategoryText->SetText(FText::FromString(Category.ToUpper()));
		CategoryText->Font.Size = 10;
		CategoryText->SetColorAndOpacity(FSlateColor(FLinearColor::Black));
		Badge->AddChild(CategoryText);
	}

	return Badge;
}

UWidget* UMutatorWidget::CreateEffectRow(const FString& EffectName, float Multiplier)
{
	UHorizontalBox* RowBox = NewObject<UHorizontalBox>(this);
	if (!RowBox)
	{
		return nullptr;
	}

	// Effect name
	UTextBlock* NameText = NewObject<UTextBlock>(this);
	if (NameText)
	{
		NameText->SetText(FText::FromString(EffectName + TEXT(":")));
		NameText->Font.Size = 14;
		NameText->SetColorAndOpacity(FSlateColor(FLinearColor(0.8f, 0.8f, 0.8f)));
		UHorizontalBoxSlot* NameSlot = RowBox->AddChildToHorizontalBox(NameText);
		NameSlot->SetSize(FSlateChildSize(ESlateSizeRule::Fill));
	}

	// Effect value with color coding
	UTextBlock* ValueText = NewObject<UTextBlock>(this);
	if (ValueText)
	{
		ValueText->SetText(FText::FromString(GetEffectDisplayText(Multiplier)));
		ValueText->Font.Size = 14;

		// Green for buffs, red for debuffs
		FLinearColor ValueColor = Multiplier > 1.0f
			? FLinearColor(0.2f, 1.0f, 0.2f)
			: FLinearColor(1.0f, 0.3f, 0.3f);
		ValueText->SetColorAndOpacity(FSlateColor(ValueColor));

		UHorizontalBoxSlot* ValueSlot = RowBox->AddChildToHorizontalBox(ValueText);
		ValueSlot->SetSize(FSlateChildSize(ESlateSizeRule::Automatic));
	}

	return RowBox;
}

FLinearColor UMutatorWidget::GetDifficultyColor(int32 Difficulty) const
{
	if (Difficulty <= 1)
	{
		return FLinearColor(0.2f, 1.0f, 0.2f); // Green
	}
	else if (Difficulty <= 3)
	{
		return FLinearColor(1.0f, 1.0f, 0.2f); // Yellow
	}
	else if (Difficulty == 4)
	{
		return FLinearColor(1.0f, 0.6f, 0.2f); // Orange
	}
	else
	{
		return FLinearColor(1.0f, 0.2f, 0.2f); // Red
	}
}

FLinearColor UMutatorWidget::GetCategoryColor(const FString& Category) const
{
	if (Category.Equals(TEXT("Combat"), ESearchCase::IgnoreCase))
	{
		return FLinearColor(1.0f, 0.2f, 0.2f); // Red
	}
	else if (Category.Equals(TEXT("Environment"), ESearchCase::IgnoreCase))
	{
		return FLinearColor(0.2f, 1.0f, 0.2f); // Green
	}
	else if (Category.Equals(TEXT("Economy"), ESearchCase::IgnoreCase))
	{
		return FLinearColor(1.0f, 0.84f, 0.0f); // Gold
	}
	else if (Category.Equals(TEXT("Semantic"), ESearchCase::IgnoreCase))
	{
		return FLinearColor(0.6f, 0.2f, 1.0f); // Purple
	}
	else if (Category.Equals(TEXT("Challenge"), ESearchCase::IgnoreCase))
	{
		return FLinearColor(1.0f, 1.0f, 1.0f); // White
	}
	else
	{
		return FLinearColor(0.5f, 0.5f, 0.5f); // Gray (default)
	}
}

FString UMutatorWidget::GetEffectDisplayText(float Multiplier) const
{
	if (Multiplier > 1.0f)
	{
		return FString::Printf(TEXT("+%.0f%%"), (Multiplier - 1.0f) * 100.0f);
	}
	else if (Multiplier < 1.0f)
	{
		return FString::Printf(TEXT("%.0f%%"), (Multiplier - 1.0f) * 100.0f);
	}
	else
	{
		return TEXT("No Change");
	}
}

bool UMutatorWidget::ParseMutatorJSON(const FString& JSONString)
{
	TSharedPtr<FJsonObject> JsonObject;
	TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(JSONString);

	if (!FJsonSerializer::Deserialize(Reader, JsonObject) || !JsonObject.IsValid())
	{
		UE_LOG(LogTemp, Error, TEXT("Failed to parse mutator JSON"));
		return false;
	}

	// Parse mutators array
	const TArray<TSharedPtr<FJsonValue>>* MutatorsArray;
	if (JsonObject->TryGetArrayField(TEXT("mutators"), MutatorsArray))
	{
		ActiveMutators.Empty();

		for (const TSharedPtr<FJsonValue>& MutatorValue : *MutatorsArray)
		{
			TSharedPtr<FJsonObject> MutatorObj = MutatorValue->AsObject();
			if (MutatorObj.IsValid())
			{
				FMutatorData Mutator;
				Mutator.Id = MutatorObj->GetStringField(TEXT("id"));
				Mutator.Name = MutatorObj->GetStringField(TEXT("name"));
				Mutator.Description = MutatorObj->GetStringField(TEXT("description"));
				Mutator.Difficulty = MutatorObj->GetIntegerField(TEXT("difficulty"));
				Mutator.Category = MutatorObj->GetStringField(TEXT("category"));

				if (MutatorObj->HasField(TEXT("icon_path")))
				{
					Mutator.IconPath = MutatorObj->GetStringField(TEXT("icon_path"));
				}

				ActiveMutators.Add(Mutator);
			}
		}
	}

	// Parse aggregate effects
	const TSharedPtr<FJsonObject>* EffectsObj;
	if (JsonObject->TryGetObjectField(TEXT("effects"), EffectsObj))
	{
		AggregateEffects.DamageMultiplier = (*EffectsObj)->GetNumberField(TEXT("damage_multiplier"));
		AggregateEffects.HealthMultiplier = (*EffectsObj)->GetNumberField(TEXT("health_multiplier"));
		AggregateEffects.LootMultiplier = (*EffectsObj)->GetNumberField(TEXT("loot_multiplier"));
		AggregateEffects.ExperienceMultiplier = (*EffectsObj)->GetNumberField(TEXT("experience_multiplier"));
		AggregateEffects.MovementSpeedMultiplier = (*EffectsObj)->GetNumberField(TEXT("movement_speed_multiplier"));
		AggregateEffects.RewardMultiplier = (*EffectsObj)->GetNumberField(TEXT("reward_multiplier"));
	}

	return true;
}

void UMutatorWidget::UpdateAnimations(float DeltaTime)
{
	AnimationTime += DeltaTime;

	// Pulse effect on main border
	if (MainBorder)
	{
		float PulseValue = 0.8f + 0.2f * FMath::Sin(AnimationTime * 2.0f);
		MainBorder->SetRenderOpacity(PulseValue);
	}

	// Additional animations can be added here:
	// - Fade in mutator entries sequentially
	// - Animate difficulty stars
	// - Particle effects for high difficulty
}
