#include "NotificationWidget.h"
#include "Components/VerticalBox.h"
#include "Components/TextBlock.h"

void UNotificationWidget::NativeConstruct()
{
    Super::NativeConstruct();
}

void UNotificationWidget::NativeTick(const FGeometry& MyGeometry, float InDeltaTime)
{
    Super::NativeTick(MyGeometry, InDeltaTime);

    bool bChanged = false;

    for (FNotificationEntry& Entry : ActiveNotifications)
    {
        Entry.ElapsedTime += InDeltaTime;
        if (Entry.ElapsedTime >= Entry.Lifetime)
        {
            bChanged = true;
        }
    }

    // Remove expired
    int32 Before = ActiveNotifications.Num();
    ActiveNotifications.RemoveAll([](const FNotificationEntry& E) {
        return E.ElapsedTime >= E.Lifetime;
    });

    if (ActiveNotifications.Num() != Before || bChanged)
    {
        RebuildDisplay();
    }
}

void UNotificationWidget::ShowNotification(const FString& Title, const FString& Message,
    ENotificationType Type, float Duration)
{
    FNotificationEntry Entry;
    Entry.Title = Title;
    Entry.Message = Message;
    Entry.Type = Type;
    Entry.Lifetime = Duration;
    Entry.ElapsedTime = 0.0f;

    ActiveNotifications.Add(Entry);

    // Limit visible count
    while (ActiveNotifications.Num() > MaxVisibleNotifications)
    {
        ActiveNotifications.RemoveAt(0);
    }

    RebuildDisplay();
}

void UNotificationWidget::ShowLootNotification(const FString& ItemName,
    const FString& Rarity, int32 Quantity)
{
    FString Title = TEXT("Item Acquired");
    FString Message = Quantity > 1 ?
        FString::Printf(TEXT("%s x%d (%s)"), *ItemName, Quantity, *Rarity) :
        FString::Printf(TEXT("%s (%s)"), *ItemName, *Rarity);

    FNotificationEntry Entry;
    Entry.Title = Title;
    Entry.Message = Message;
    Entry.Type = ENotificationType::LootDrop;
    Entry.Lifetime = 4.0f;
    Entry.ExtraData = Rarity;

    ActiveNotifications.Add(Entry);
    while (ActiveNotifications.Num() > MaxVisibleNotifications)
    {
        ActiveNotifications.RemoveAt(0);
    }

    RebuildDisplay();
}

void UNotificationWidget::ShowWorldEventNotification(const FString& EventName,
    const FString& Description)
{
    FNotificationEntry Entry;
    Entry.Title = EventName;
    Entry.Message = Description;
    Entry.Type = ENotificationType::WorldEvent;
    Entry.Lifetime = 8.0f; // Longer for important events
    Entry.FadeInDuration = 0.5f;

    ActiveNotifications.Add(Entry);
    while (ActiveNotifications.Num() > MaxVisibleNotifications)
    {
        ActiveNotifications.RemoveAt(0);
    }

    RebuildDisplay();
}

void UNotificationWidget::ShowFactionNotification(const FString& Faction, int32 RepChange)
{
    FString Title = FString::Printf(TEXT("%s Reputation"), *Faction);
    FString Message = RepChange > 0 ?
        FString::Printf(TEXT("+%d reputation"), RepChange) :
        FString::Printf(TEXT("%d reputation"), RepChange);

    FNotificationEntry Entry;
    Entry.Title = Title;
    Entry.Message = Message;
    Entry.Type = ENotificationType::FactionRep;
    Entry.Lifetime = 4.0f;
    Entry.ExtraData = Faction;

    ActiveNotifications.Add(Entry);
    while (ActiveNotifications.Num() > MaxVisibleNotifications)
    {
        ActiveNotifications.RemoveAt(0);
    }

    RebuildDisplay();
}

void UNotificationWidget::ShowAchievement(const FString& Title, const FString& Description)
{
    FNotificationEntry Entry;
    Entry.Title = Title;
    Entry.Message = Description;
    Entry.Type = ENotificationType::Achievement;
    Entry.Lifetime = 8.0f;
    Entry.FadeInDuration = 0.5f;

    ActiveNotifications.Add(Entry);
    while (ActiveNotifications.Num() > MaxVisibleNotifications)
    {
        ActiveNotifications.RemoveAt(0);
    }

    RebuildDisplay();
}

void UNotificationWidget::ClearAll()
{
    ActiveNotifications.Empty();
    RebuildDisplay();
}

void UNotificationWidget::RebuildDisplay()
{
    if (!NotificationBox) return;
    NotificationBox->ClearChildren();

    for (int32 i = 0; i < ActiveNotifications.Num(); i++)
    {
        const FNotificationEntry& Entry = ActiveNotifications[i];

        UTextBlock* NotifText = NewObject<UTextBlock>(this);

        FString Prefix = GetTypePrefix(Entry.Type);
        FString Display = FString::Printf(TEXT("%s %s: %s"), *Prefix, *Entry.Title, *Entry.Message);
        NotifText->SetText(FText::FromString(Display));

        FLinearColor Color = GetTypeColor(Entry.Type, Entry.ExtraData);

        // Fade in/out
        float Alpha = 1.0f;
        if (Entry.ElapsedTime < Entry.FadeInDuration)
        {
            Alpha = Entry.ElapsedTime / Entry.FadeInDuration;
        }
        else if (Entry.ElapsedTime > Entry.Lifetime - Entry.FadeOutDuration)
        {
            float FadeProgress = (Entry.ElapsedTime - (Entry.Lifetime - Entry.FadeOutDuration)) / Entry.FadeOutDuration;
            Alpha = 1.0f - FMath::Clamp(FadeProgress, 0.0f, 1.0f);
        }

        Color.A = Alpha;
        NotifText->SetColorAndOpacity(FSlateColor(Color));

        FSlateFontInfo Font = NotifText->GetFont();
        Font.Size = (Entry.Type == ENotificationType::Achievement ||
                     Entry.Type == ENotificationType::LevelUp) ? 14 : 11;
        NotifText->SetFont(Font);

        NotificationBox->AddChild(NotifText);
    }
}

FLinearColor UNotificationWidget::GetTypeColor(ENotificationType Type, const FString& ExtraData) const
{
    switch (Type)
    {
    case ENotificationType::Info:        return FLinearColor(0.4f, 0.7f, 1.0f);
    case ENotificationType::Success:     return FLinearColor(0.3f, 1.0f, 0.4f);
    case ENotificationType::Warning:     return FLinearColor(1.0f, 0.7f, 0.2f);
    case ENotificationType::Error:       return FLinearColor(1.0f, 0.3f, 0.3f);
    case ENotificationType::LevelUp:     return FLinearColor(1.0f, 0.84f, 0.0f);
    case ENotificationType::Achievement: return FLinearColor(0.7f, 0.3f, 1.0f);
    case ENotificationType::WorldEvent:  return FLinearColor(0.2f, 0.9f, 0.9f);
    case ENotificationType::EchoAppear:  return FLinearColor(0.4f, 0.6f, 1.0f);

    case ENotificationType::LootDrop:
    {
        if (ExtraData == TEXT("Common"))    return FLinearColor(0.7f, 0.7f, 0.7f);
        if (ExtraData == TEXT("Uncommon"))  return FLinearColor(0.3f, 0.9f, 0.3f);
        if (ExtraData == TEXT("Rare"))      return FLinearColor(0.3f, 0.5f, 1.0f);
        if (ExtraData == TEXT("Epic"))      return FLinearColor(0.7f, 0.3f, 1.0f);
        if (ExtraData == TEXT("Legendary")) return FLinearColor(1.0f, 0.6f, 0.1f);
        if (ExtraData == TEXT("Mythic"))    return FLinearColor(1.0f, 0.2f, 0.2f);
        return FLinearColor(0.7f, 0.7f, 0.7f);
    }

    case ENotificationType::FactionRep:
    {
        if (ExtraData == TEXT("Seekers"))  return FLinearColor(0.2f, 0.6f, 1.0f);
        if (ExtraData == TEXT("Wardens"))  return FLinearColor(0.2f, 0.8f, 0.3f);
        if (ExtraData == TEXT("Breakers")) return FLinearColor(1.0f, 0.3f, 0.2f);
        if (ExtraData == TEXT("Weavers"))  return FLinearColor(0.7f, 0.3f, 1.0f);
        return FLinearColor(0.8f, 0.8f, 0.8f);
    }

    default: return FLinearColor(0.8f, 0.8f, 0.8f);
    }
}

FString UNotificationWidget::GetTypePrefix(ENotificationType Type) const
{
    switch (Type)
    {
    case ENotificationType::Info:        return TEXT("[i]");
    case ENotificationType::Success:     return TEXT("[+]");
    case ENotificationType::Warning:     return TEXT("[!]");
    case ENotificationType::Error:       return TEXT("[X]");
    case ENotificationType::LootDrop:    return TEXT("[>]");
    case ENotificationType::LevelUp:     return TEXT("[^]");
    case ENotificationType::Achievement: return TEXT("[*]");
    case ENotificationType::WorldEvent:  return TEXT("[~]");
    case ENotificationType::FactionRep:  return TEXT("[F]");
    case ENotificationType::EchoAppear:  return TEXT("[E]");
    default: return TEXT("[-]");
    }
}
