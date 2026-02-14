#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "SocketWidget.generated.h"

class UScrollBox;
class UTextBlock;
class UButton;
class UVerticalBox;
class UHorizontalBox;
class UBorder;
class UImage;

/// Socket color — mirrors Rust SocketColor
UENUM(BlueprintType)
enum class ESocketColor : uint8
{
    Red,       // Offensive — damage, crit
    Blue,      // Defensive — HP, defense, resistance
    Yellow,    // Utility — speed, resource, cooldown
    Prismatic, // Accepts any gem/rune
};

/// Gem quality tier — mirrors Rust GemTier
UENUM(BlueprintType)
enum class EGemTier : uint8
{
    Chipped,   // +1x
    Flawed,    // +2x
    Regular,   // +3x
    Flawless,  // +5x
    Perfect,   // +8x
    Radiant,   // +12x
};

/// Display data for a single socket on equipment
USTRUCT(BlueprintType)
struct FSocketDisplay
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) int32 Index = 0;
    UPROPERTY(BlueprintReadWrite) ESocketColor Color = ESocketColor::Red;
    UPROPERTY(BlueprintReadWrite) bool bIsEmpty = true;
    UPROPERTY(BlueprintReadWrite) FString ContentName;
    UPROPERTY(BlueprintReadWrite) FString ContentDescription;
    UPROPERTY(BlueprintReadWrite) bool bIsGem = true; // true = gem, false = rune
};

/// Display data for a gem in inventory
USTRUCT(BlueprintType)
struct FGemDisplay
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString Id;
    UPROPERTY(BlueprintReadWrite) FString Name;
    UPROPERTY(BlueprintReadWrite) ESocketColor Color = ESocketColor::Red;
    UPROPERTY(BlueprintReadWrite) EGemTier Tier = EGemTier::Chipped;
    UPROPERTY(BlueprintReadWrite) FString BonusDescription;
};

/// Display data for a rune in inventory
USTRUCT(BlueprintType)
struct FRuneDisplay
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString Id;
    UPROPERTY(BlueprintReadWrite) FString Name;
    UPROPERTY(BlueprintReadWrite) ESocketColor Color = ESocketColor::Red;
    UPROPERTY(BlueprintReadWrite) FString Description;
    UPROPERTY(BlueprintReadWrite) FString EffectDescription;
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnSocketModified, const FString&, EquipmentId, int32, SocketIndex);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnGemsCombined, const FString&, NewGemId);

/**
 * Equipment socket / gem insertion widget.
 *
 * Layout:
 *   [Left]   Equipment preview with color-coded socket circles
 *   [Center] Selected socket detail panel (color, content, compatibility)
 *   [Right]  Gem/Rune inventory list (scrollbox) with insert/remove buttons
 *   [Bottom] Gem combine panel (select 3 gems of same tier+color to upgrade)
 *
 * Mirrors Rust sockets module: SocketColor, GemTier, Socket, Gem, Rune,
 * SocketedEquipment, combine_gems.
 *
 * Color rules:
 *   - Gem color must match socket color, unless socket is Prismatic (accepts all)
 *   - Rune color must match socket color, unless socket is Prismatic
 *   - Combine requires 3 gems of same color and tier
 */
UCLASS()
class TOWERGAME_API USocketWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;

    // ============ Data Loading ============

    /** Load equipment sockets from Rust JSON payload */
    UFUNCTION(BlueprintCallable, Category = "Sockets")
    void LoadEquipmentSockets(const FString& EquipmentId, const FString& SocketsJson);

    /** Load available gems into inventory list */
    UFUNCTION(BlueprintCallable, Category = "Sockets")
    void LoadAvailableGems(const FString& GemsJson);

    /** Load available runes into inventory list */
    UFUNCTION(BlueprintCallable, Category = "Sockets")
    void LoadAvailableRunes(const FString& RunesJson);

    // ============ Socket Interaction ============

    /** Select a socket by index to view details */
    UFUNCTION(BlueprintCallable, Category = "Sockets")
    void SelectSocket(int32 Index);

    /** Insert a gem into the selected socket */
    UFUNCTION(BlueprintCallable, Category = "Sockets")
    void InsertGem(int32 SocketIndex, const FString& GemId);

    /** Insert a rune into the selected socket */
    UFUNCTION(BlueprintCallable, Category = "Sockets")
    void InsertRune(int32 SocketIndex, const FString& RuneId);

    /** Remove content from a socket */
    UFUNCTION(BlueprintCallable, Category = "Sockets")
    void RemoveContent(int32 SocketIndex);

    // ============ Gem Combining ============

    /** Combine 3 gems of same color+tier into next tier */
    UFUNCTION(BlueprintCallable, Category = "Sockets")
    void CombineGems(const FString& GemId1, const FString& GemId2, const FString& GemId3);

    // ============ Queries ============

    /** Check if a gem is compatible with a specific socket */
    UFUNCTION(BlueprintPure, Category = "Sockets")
    bool IsGemCompatible(int32 SocketIndex, ESocketColor GemColor) const;

    /** Check if a rune is compatible with a specific socket */
    UFUNCTION(BlueprintPure, Category = "Sockets")
    bool IsRuneCompatible(int32 SocketIndex, ESocketColor RuneColor) const;

    /** Get color for a socket color enum */
    UFUNCTION(BlueprintPure, Category = "Sockets")
    static FLinearColor GetSocketColorValue(ESocketColor Color);

    /** Get color for a gem tier enum */
    UFUNCTION(BlueprintPure, Category = "Sockets")
    static FLinearColor GetGemTierColor(EGemTier Tier);

    // ============ Events ============

    UPROPERTY(BlueprintAssignable, Category = "Sockets")
    FOnSocketModified OnSocketModified;

    UPROPERTY(BlueprintAssignable, Category = "Sockets")
    FOnGemsCombined OnGemsCombined;

protected:
    // ============ Bound Widgets — Equipment Socket Display ============

    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* EquipmentNameText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UHorizontalBox* SocketSlotsBox = nullptr;

    // ============ Bound Widgets — Selected Socket Detail ============

    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* SocketDetailTitle = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* SocketColorText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* SocketContentText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* SocketContentDesc = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* CompatibilityText = nullptr;

    // ============ Bound Widgets — Gem/Rune Inventory ============

    UPROPERTY(meta = (BindWidgetOptional)) UScrollBox* GemListScrollBox = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UScrollBox* RuneListScrollBox = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* InsertButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* RemoveButton = nullptr;

    // ============ Bound Widgets — Combine Panel ============

    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* CombineSlot1Text = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* CombineSlot2Text = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* CombineSlot3Text = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* CombineButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* CombineResultText = nullptr;

    // ============ State ============

    FString CurrentEquipmentId;
    TArray<FSocketDisplay> Sockets;
    TArray<FGemDisplay> AvailableGems;
    TArray<FRuneDisplay> AvailableRunes;

    int32 SelectedSocketIndex = -1;
    int32 SelectedGemIndex = -1;
    int32 SelectedRuneIndex = -1;

    /** Gem IDs staged for combining (up to 3) */
    TArray<FString> CombineSlots;

    // ============ Internal ============

    void RebuildSocketDisplay();
    void UpdateSocketDetail();
    void RebuildGemList();
    void RebuildRuneList();
    void UpdateInsertRemoveButtons();
    void UpdateCombinePanel();

    FString GetSocketColorName(ESocketColor Color) const;
    FString GetGemTierName(EGemTier Tier) const;
    ESocketColor ParseSocketColor(const FString& Str) const;
    EGemTier ParseGemTier(const FString& Str) const;

    void SelectGem(int32 Index);
    void SelectRune(int32 Index);
    void AddGemToCombine(const FString& GemId);
    void ClearCombineSlots();

    UFUNCTION() void OnInsertClicked();
    UFUNCTION() void OnRemoveClicked();
    UFUNCTION() void OnCombineClicked();
};
