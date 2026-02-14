#include "CharacterSelectWidget.h"
#include "Components/TextBlock.h"
#include "Components/Button.h"
#include "Components/ProgressBar.h"

void UCharacterSelectWidget::NativeConstruct()
{
    Super::NativeConstruct();

    InitWeaponInfos();

    // Bind button clicks
    if (ConfirmButton)
        ConfirmButton->OnClicked.AddDynamic(this, &UCharacterSelectWidget::OnConfirmClicked);
    if (WeaponLeftBtn)
        WeaponLeftBtn->OnClicked.AddDynamic(this, &UCharacterSelectWidget::OnWeaponLeftClicked);
    if (WeaponRightBtn)
        WeaponRightBtn->OnClicked.AddDynamic(this, &UCharacterSelectWidget::OnWeaponRightClicked);
    if (ElementLeftBtn)
        ElementLeftBtn->OnClicked.AddDynamic(this, &UCharacterSelectWidget::OnElementLeftClicked);
    if (ElementRightBtn)
        ElementRightBtn->OnClicked.AddDynamic(this, &UCharacterSelectWidget::OnElementRightClicked);

    // Default build
    CurrentBuild.CharacterName = TEXT("Climber");
    CurrentBuild.Weapon = EStartingWeapon::Sword;
    CurrentBuild.Element = EStartingElement::Neutral;
    CurrentBuild.Strength = 4;
    CurrentBuild.Agility = 4;
    CurrentBuild.Vitality = 4;
    CurrentBuild.Mind = 4;
    CurrentBuild.Spirit = 4;
    CurrentBuild.RemainingPoints = 0;

    UpdateDisplay();
}

void UCharacterSelectWidget::InitWeaponInfos()
{
    WeaponInfos.Empty();

    // Matches Rust combat::weapons — combo lengths, relative speeds/damage
    WeaponInfos.Add({ EStartingWeapon::Sword, TEXT("Sword"), TEXT("Balanced blade. 3-hit combo, parry-friendly."), 3, 0.7f, 0.6f, 0.5f });
    WeaponInfos.Add({ EStartingWeapon::Greatsword, TEXT("Greatsword"), TEXT("Slow but devastating. 2-hit combo, high stagger."), 2, 0.3f, 1.0f, 0.6f });
    WeaponInfos.Add({ EStartingWeapon::DualDaggers, TEXT("Dual Daggers"), TEXT("Lightning fast. 5-hit combo, low range."), 5, 1.0f, 0.3f, 0.2f });
    WeaponInfos.Add({ EStartingWeapon::Spear, TEXT("Spear"), TEXT("Long reach. 3-hit combo, thrust-focused."), 3, 0.6f, 0.5f, 1.0f });
    WeaponInfos.Add({ EStartingWeapon::Gauntlets, TEXT("Gauntlets"), TEXT("Rapid strikes. 5-hit combo, aerial mastery."), 5, 0.9f, 0.4f, 0.1f });
    WeaponInfos.Add({ EStartingWeapon::Staff, TEXT("Staff"), TEXT("Semantic channeler. 2-hit combo, tag-powered."), 2, 0.5f, 0.7f, 0.8f });
}

void UCharacterSelectWidget::SelectWeapon(EStartingWeapon Weapon)
{
    CurrentBuild.Weapon = Weapon;
    UpdateDisplay();
}

void UCharacterSelectWidget::CycleWeapon(bool bForward)
{
    int32 Current = static_cast<int32>(CurrentBuild.Weapon);
    int32 Count = 6; // 6 weapon types
    Current = bForward ? (Current + 1) % Count : (Current - 1 + Count) % Count;
    CurrentBuild.Weapon = static_cast<EStartingWeapon>(Current);
    UpdateDisplay();
}

void UCharacterSelectWidget::SelectElement(EStartingElement Element)
{
    CurrentBuild.Element = Element;
    UpdateDisplay();
}

void UCharacterSelectWidget::CycleElement(bool bForward)
{
    int32 Current = static_cast<int32>(CurrentBuild.Element);
    int32 Count = 6; // 6 elements
    Current = bForward ? (Current + 1) % Count : (Current - 1 + Count) % Count;
    CurrentBuild.Element = static_cast<EStartingElement>(Current);
    UpdateDisplay();
}

bool UCharacterSelectWidget::IncreaseStat(const FString& StatName)
{
    if (CurrentBuild.RemainingPoints <= 0) return false;

    int32* Stat = GetStatPtr(StatName);
    if (!Stat || *Stat >= MaxSingleStat) return false;

    (*Stat)++;
    CurrentBuild.RemainingPoints--;
    UpdateDisplay();
    return true;
}

bool UCharacterSelectWidget::DecreaseStat(const FString& StatName)
{
    int32* Stat = GetStatPtr(StatName);
    if (!Stat || *Stat <= MinSingleStat) return false;

    (*Stat)--;
    CurrentBuild.RemainingPoints++;
    UpdateDisplay();
    return true;
}

void UCharacterSelectWidget::ResetStats()
{
    CurrentBuild.Strength = 4;
    CurrentBuild.Agility = 4;
    CurrentBuild.Vitality = 4;
    CurrentBuild.Mind = 4;
    CurrentBuild.Spirit = 4;
    CurrentBuild.RemainingPoints = 0;
    UpdateDisplay();
}

void UCharacterSelectWidget::SetCharacterName(const FString& Name)
{
    CurrentBuild.CharacterName = Name.Left(20); // 20 char limit
    UpdateDisplay();
}

void UCharacterSelectWidget::ConfirmCharacter()
{
    if (!IsValid()) return;
    OnConfirmed.Broadcast(CurrentBuild);
}

bool UCharacterSelectWidget::IsValid() const
{
    if (CurrentBuild.CharacterName.IsEmpty()) return false;
    if (CurrentBuild.RemainingPoints != 0) return false;
    int32 Total = CurrentBuild.Strength + CurrentBuild.Agility + CurrentBuild.Vitality
                + CurrentBuild.Mind + CurrentBuild.Spirit;
    return Total == TotalStartingPoints;
}

void UCharacterSelectWidget::UpdateDisplay()
{
    // Weapon info
    FWeaponPreviewInfo WInfo = GetWeaponInfo(CurrentBuild.Weapon);
    if (WeaponNameText)
        WeaponNameText->SetText(FText::FromString(WInfo.Name));
    if (WeaponDescText)
        WeaponDescText->SetText(FText::FromString(WInfo.Description));
    if (SpeedBar)
        SpeedBar->SetPercent(WInfo.AttackSpeed);
    if (DamageBar)
        DamageBar->SetPercent(WInfo.DamageRating);
    if (RangeBar)
        RangeBar->SetPercent(WInfo.RangeRating);

    // Element
    if (ElementNameText)
        ElementNameText->SetText(FText::FromString(GetElementName(CurrentBuild.Element)));

    // Stats
    if (StrengthText)
        StrengthText->SetText(FText::AsNumber(CurrentBuild.Strength));
    if (AgilityText)
        AgilityText->SetText(FText::AsNumber(CurrentBuild.Agility));
    if (VitalityText)
        VitalityText->SetText(FText::AsNumber(CurrentBuild.Vitality));
    if (MindText)
        MindText->SetText(FText::AsNumber(CurrentBuild.Mind));
    if (SpiritText)
        SpiritText->SetText(FText::AsNumber(CurrentBuild.Spirit));
    if (RemainingPointsText)
        RemainingPointsText->SetText(FText::AsNumber(CurrentBuild.RemainingPoints));

    // Name
    if (CharacterNameText)
        CharacterNameText->SetText(FText::FromString(CurrentBuild.CharacterName));

    // Confirm button state
    if (ConfirmButton)
        ConfirmButton->SetIsEnabled(IsValid());
}

FWeaponPreviewInfo UCharacterSelectWidget::GetWeaponInfo(EStartingWeapon Weapon) const
{
    for (const auto& Info : WeaponInfos)
    {
        if (Info.Type == Weapon) return Info;
    }
    return FWeaponPreviewInfo();
}

FString UCharacterSelectWidget::GetElementName(EStartingElement Element) const
{
    switch (Element)
    {
    case EStartingElement::Fire:    return TEXT("Fire");
    case EStartingElement::Water:   return TEXT("Water");
    case EStartingElement::Earth:   return TEXT("Earth");
    case EStartingElement::Wind:    return TEXT("Wind");
    case EStartingElement::Void:    return TEXT("Void");
    case EStartingElement::Neutral: return TEXT("Neutral");
    default:                        return TEXT("Unknown");
    }
}

FLinearColor UCharacterSelectWidget::GetElementColor(EStartingElement Element) const
{
    switch (Element)
    {
    case EStartingElement::Fire:    return FLinearColor(1.0f, 0.3f, 0.1f);
    case EStartingElement::Water:   return FLinearColor(0.2f, 0.5f, 1.0f);
    case EStartingElement::Earth:   return FLinearColor(0.6f, 0.4f, 0.2f);
    case EStartingElement::Wind:    return FLinearColor(0.7f, 1.0f, 0.8f);
    case EStartingElement::Void:    return FLinearColor(0.4f, 0.0f, 0.6f);
    case EStartingElement::Neutral: return FLinearColor(0.7f, 0.7f, 0.7f);
    default:                        return FLinearColor::White;
    }
}

int32* UCharacterSelectWidget::GetStatPtr(const FString& StatName)
{
    if (StatName == TEXT("Strength"))  return &CurrentBuild.Strength;
    if (StatName == TEXT("Agility"))   return &CurrentBuild.Agility;
    if (StatName == TEXT("Vitality"))  return &CurrentBuild.Vitality;
    if (StatName == TEXT("Mind"))      return &CurrentBuild.Mind;
    if (StatName == TEXT("Spirit"))    return &CurrentBuild.Spirit;
    return nullptr;
}

void UCharacterSelectWidget::OnConfirmClicked()     { ConfirmCharacter(); }
void UCharacterSelectWidget::OnWeaponLeftClicked()   { CycleWeapon(false); }
void UCharacterSelectWidget::OnWeaponRightClicked()  { CycleWeapon(true); }
void UCharacterSelectWidget::OnElementLeftClicked()  { CycleElement(false); }
void UCharacterSelectWidget::OnElementRightClicked() { CycleElement(true); }
