#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "NotificationWidget.generated.h"

class UVerticalBox;
class UTextBlock;

/// Notification priority/type
UENUM(BlueprintType)
enum class ENotificationType : uint8
{
    Info,           // Blue - general info
    Success,        // Green - quest complete, craft success
    Warning,        // Orange - low health, corruption rising
    Error,          // Red - failed action
    LootDrop,       // Rarity-colored - item pickup
    LevelUp,        // Gold - level/floor milestone
    Achievement,    // Purple - achievement unlocked
    WorldEvent,     // Cyan - procedural event triggered
    FactionRep,     // Faction-colored - reputation change
    EchoAppear,     // Ghost blue - echo spotted
};

/// A single notification entry
USTRUCT(BlueprintType)
struct FNotificationEntry
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString Title;
    UPROPERTY(BlueprintReadWrite) FString Message;
    UPROPERTY(BlueprintReadWrite) ENotificationType Type = ENotificationType::Info;
    UPROPERTY(BlueprintReadWrite) float Lifetime = 5.0f;
    UPROPERTY(BlueprintReadWrite) float ElapsedTime = 0.0f;
    UPROPERTY(BlueprintReadWrite) float FadeInDuration = 0.3f;
    UPROPERTY(BlueprintReadWrite) float FadeOutDuration = 0.5f;
    UPROPERTY(BlueprintReadWrite) FString ExtraData; // Rarity, faction, etc.
};

/**
 * Toast-style notification system.
 * Queued notifications stack vertically, auto-fade after duration.
 */
UCLASS()
class TOWERGAME_API UNotificationWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;
    virtual void NativeTick(const FGeometry& MyGeometry, float InDeltaTime) override;

    // --- API ---
    UFUNCTION(BlueprintCallable) void ShowNotification(const FString& Title,
        const FString& Message, ENotificationType Type, float Duration = 5.0f);
    UFUNCTION(BlueprintCallable) void ShowLootNotification(const FString& ItemName,
        const FString& Rarity, int32 Quantity);
    UFUNCTION(BlueprintCallable) void ShowWorldEventNotification(const FString& EventName,
        const FString& Description);
    UFUNCTION(BlueprintCallable) void ShowFactionNotification(const FString& Faction,
        int32 RepChange);
    UFUNCTION(BlueprintCallable) void ShowAchievement(const FString& Title,
        const FString& Description);
    UFUNCTION(BlueprintCallable) void ClearAll();

protected:
    UPROPERTY(meta = (BindWidgetOptional)) UVerticalBox* NotificationBox = nullptr;

    UPROPERTY(EditDefaultsOnly) int32 MaxVisibleNotifications = 5;
    UPROPERTY(EditDefaultsOnly) float DefaultLifetime = 5.0f;

    TArray<FNotificationEntry> ActiveNotifications;

    void RebuildDisplay();
    FLinearColor GetTypeColor(ENotificationType Type, const FString& ExtraData) const;
    FString GetTypePrefix(ENotificationType Type) const;
};
