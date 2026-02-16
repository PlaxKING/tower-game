#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "TransmogWidget.generated.h"

class UScrollBox;
class UTextBlock;
class UButton;
class UImage;
class UBorder;
class UVerticalBox;
class UHorizontalBox;
class UGridPanel;
class UUniformGridPanel;

/// Cosmetic equipment slot — mirrors Rust CosmeticSlot
UENUM(BlueprintType)
enum class ECosmeticSlot : uint8
{
    HeadOverride,
    ChestOverride,
    LegsOverride,
    BootsOverride,
    GlovesOverride,
    WeaponSkin,
    BackAccessory,
    Aura,
    Emote,
    Title,
    ProfileBorder,
    NameplateStyle,
};

/// Dye channel on an equipment piece — mirrors Rust DyeChannel
UENUM(BlueprintType)
enum class EDyeChannel : uint8
{
    Primary,
    Secondary,
    Accent,
};

/// Cosmetic item display data — parsed from Rust CosmeticItem JSON
USTRUCT(BlueprintType)
struct FCosmeticItemDisplay
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString Id;
    UPROPERTY(BlueprintReadWrite) FString Name;
    UPROPERTY(BlueprintReadWrite) FString Description;
    UPROPERTY(BlueprintReadWrite) ECosmeticSlot Slot = ECosmeticSlot::HeadOverride;
    UPROPERTY(BlueprintReadWrite) FString Rarity; // Common, Uncommon, Rare, Epic, Legendary, Mythic
    UPROPERTY(BlueprintReadWrite) FString AssetRef;
    UPROPERTY(BlueprintReadWrite) bool bDyeable = false;
    UPROPERTY(BlueprintReadWrite) bool bUnlocked = false;
    UPROPERTY(BlueprintReadWrite) FString SourceDescription;
};

/// Dye display data — parsed from Rust Dye JSON
USTRUCT(BlueprintType)
struct FDyeDisplay
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString Id;
    UPROPERTY(BlueprintReadWrite) FString Name;
    UPROPERTY(BlueprintReadWrite) float R = 1.0f;
    UPROPERTY(BlueprintReadWrite) float G = 1.0f;
    UPROPERTY(BlueprintReadWrite) float B = 1.0f;
    UPROPERTY(BlueprintReadWrite) float Metallic = 0.0f;
    UPROPERTY(BlueprintReadWrite) float Glossiness = 0.5f;
    UPROPERTY(BlueprintReadWrite) bool bUnlocked = false;

    FLinearColor ToLinearColor() const { return FLinearColor(R, G, B, 1.0f); }
};

/// Slot + cosmetic ID pair used within outfit presets
USTRUCT(BlueprintType)
struct FTransmogOverrideEntry
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) ECosmeticSlot Slot = ECosmeticSlot::HeadOverride;
    UPROPERTY(BlueprintReadWrite) FString CosmeticId;
};

/// Outfit preset — named set of transmog overrides
USTRUCT(BlueprintType)
struct FOutfitPreset
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadWrite) FString Name;
    UPROPERTY(BlueprintReadWrite) TArray<FTransmogOverrideEntry> Overrides;
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnTransmogApplied, ECosmeticSlot, Slot, const FString&, CosmeticId);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_ThreeParams(FOnDyeApplied, ECosmeticSlot, Slot, EDyeChannel, Channel, const FString&, DyeId);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnPresetSaved, const FString&, PresetName);

/**
 * Transmog / Cosmetics widget.
 *
 * Layout:
 *   [Left]   Character preview area (3D viewport or image)
 *   [Center] Cosmetic slot buttons (grid), filtered cosmetics scroll list
 *   [Right]  Dye channel selectors with color swatches, active title display
 *   [Bottom] Outfit preset save/load controls
 *
 * Mirrors Rust cosmetics module: CosmeticSlot, DyeChannel, CosmeticProfile.
 * Transmog separates appearance from stats — equip strong gear but look how you want.
 * Locked cosmetics are grayed out and show their unlock source.
 */
UCLASS()
class TOWERGAME_API UTransmogWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;

    // ============ Data Loading ============

    /** Load available cosmetics from JSON (includes unlock state) */
    UFUNCTION(BlueprintCallable, Category = "Transmog")
    void LoadUnlockedCosmetics(const FString& CosmeticsJson);

    /** Load available dyes from JSON (includes unlock state) */
    UFUNCTION(BlueprintCallable, Category = "Transmog")
    void LoadUnlockedDyes(const FString& DyesJson);

    // ============ Slot Selection ============

    /** Select a cosmetic slot to browse available cosmetics for it */
    UFUNCTION(BlueprintCallable, Category = "Transmog")
    void SelectSlot(ECosmeticSlot SlotType);

    /** Get the currently selected slot */
    UFUNCTION(BlueprintPure, Category = "Transmog")
    ECosmeticSlot GetSelectedSlot() const { return CurrentSlot; }

    // ============ Transmog Operations ============

    /** Apply a cosmetic as a transmog override on a slot */
    UFUNCTION(BlueprintCallable, Category = "Transmog")
    void ApplyTransmog(ECosmeticSlot SlotType, const FString& CosmeticId);

    /** Remove a transmog override (show actual gear) */
    UFUNCTION(BlueprintCallable, Category = "Transmog")
    void RemoveTransmog(ECosmeticSlot SlotType);

    /** Preview a cosmetic without applying (temporary visual) */
    UFUNCTION(BlueprintCallable, Category = "Transmog")
    void PreviewCosmetic(const FString& CosmeticId);

    /** Cancel preview and restore previous state */
    UFUNCTION(BlueprintCallable, Category = "Transmog")
    void CancelPreview();

    // ============ Dye Operations ============

    /** Apply a dye to a cosmetic slot on a specific channel */
    UFUNCTION(BlueprintCallable, Category = "Transmog")
    void ApplyDye(ECosmeticSlot SlotType, EDyeChannel Channel, const FString& DyeId);

    /** Select a dye channel to browse available dyes */
    UFUNCTION(BlueprintCallable, Category = "Transmog")
    void SelectDyeChannel(EDyeChannel Channel);

    // ============ Preset Management ============

    /** Save current transmog setup as a named outfit preset */
    UFUNCTION(BlueprintCallable, Category = "Transmog")
    void SavePreset(const FString& PresetName);

    /** Load an outfit preset by name */
    UFUNCTION(BlueprintCallable, Category = "Transmog")
    void LoadPreset(const FString& PresetName);

    /** Get all saved preset names */
    UFUNCTION(BlueprintPure, Category = "Transmog")
    TArray<FString> GetPresetNames() const;

    // ============ Title & Aura ============

    /** Set active title cosmetic */
    UFUNCTION(BlueprintCallable, Category = "Transmog")
    void SetTitle(const FString& TitleId);

    /** Set active aura cosmetic */
    UFUNCTION(BlueprintCallable, Category = "Transmog")
    void SetAura(const FString& AuraId);

    // ============ Queries ============

    /** Get cosmetics filtered by slot */
    UFUNCTION(BlueprintPure, Category = "Transmog")
    TArray<FCosmeticItemDisplay> GetCosmeticsForSlot(ECosmeticSlot SlotType) const;

    /** Get all unlocked dyes */
    UFUNCTION(BlueprintPure, Category = "Transmog")
    TArray<FDyeDisplay> GetUnlockedDyes() const;

    /** Get the currently active transmog on a slot, empty string if none */
    UFUNCTION(BlueprintPure, Category = "Transmog")
    FString GetActiveTransmog(ECosmeticSlot SlotType) const;

    /** Get active title ID */
    UFUNCTION(BlueprintPure, Category = "Transmog")
    FString GetActiveTitle() const { return ActiveTitleId; }

    /** Get active aura ID */
    UFUNCTION(BlueprintPure, Category = "Transmog")
    FString GetActiveAura() const { return ActiveAuraId; }

    // ============ Events ============

    UPROPERTY(BlueprintAssignable, Category = "Transmog")
    FOnTransmogApplied OnTransmogApplied;

    UPROPERTY(BlueprintAssignable, Category = "Transmog")
    FOnDyeApplied OnDyeApplied;

    UPROPERTY(BlueprintAssignable, Category = "Transmog")
    FOnPresetSaved OnPresetSaved;

protected:
    // ============ Bound Widgets ============

    // -- Character Preview (left panel) --
    UPROPERTY(meta = (BindWidgetOptional)) UImage* CharacterPreviewImage = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* PreviewStatusText = nullptr;

    // -- Slot Buttons (center-top grid) --
    UPROPERTY(meta = (BindWidgetOptional)) UUniformGridPanel* SlotButtonGrid = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* SelectedSlotNameText = nullptr;

    // -- Cosmetic List (center scroll area, filtered by selected slot) --
    UPROPERTY(meta = (BindWidgetOptional)) UScrollBox* CosmeticListBox = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* CosmeticCountText = nullptr;

    // -- Selected Cosmetic Detail --
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* CosmeticNameText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* CosmeticDescText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* CosmeticRarityText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* CosmeticSourceText = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* ApplyButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* RemoveButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* PreviewButton = nullptr;

    // -- Dye Panel (right side) --
    UPROPERTY(meta = (BindWidgetOptional)) UButton* DyePrimaryButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* DyeSecondaryButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* DyeAccentButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UImage* DyePrimarySwatch = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UImage* DyeSecondarySwatch = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UImage* DyeAccentSwatch = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UScrollBox* DyeListBox = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* DyeChannelNameText = nullptr;

    // -- Title Display --
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* ActiveTitleText = nullptr;

    // -- Preset Controls (bottom) --
    UPROPERTY(meta = (BindWidgetOptional)) UScrollBox* PresetListBox = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* SavePresetButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UButton* LoadPresetButton = nullptr;
    UPROPERTY(meta = (BindWidgetOptional)) UTextBlock* PresetNameInputText = nullptr;

    // -- Close --
    UPROPERTY(meta = (BindWidgetOptional)) UButton* CloseButton = nullptr;

    // ============ State ============

    TArray<FCosmeticItemDisplay> AllCosmetics;
    TArray<FDyeDisplay> AllDyes;
    TMap<ECosmeticSlot, FString> ActiveTransmogs;     // slot → cosmetic_id
    TMap<FString, FOutfitPreset> SavedPresets;         // name → preset
    ECosmeticSlot CurrentSlot = ECosmeticSlot::HeadOverride;
    EDyeChannel CurrentDyeChannel = EDyeChannel::Primary;
    int32 SelectedCosmeticIndex = -1;
    FString ActiveTitleId;
    FString ActiveAuraId;
    FString PreviewingCosmeticId;
    bool bIsPreviewing = false;

    // ============ Internal ============

    void RebuildSlotGrid();
    void RebuildCosmeticList();
    void UpdateCosmeticDetail();
    void RebuildDyeList();
    void UpdateDyeSwatches();
    void RebuildPresetList();
    void UpdateTitleDisplay();

    void SelectCosmeticAtIndex(int32 Index);

    /** Get rarity color matching the existing system */
    static FLinearColor GetRarityColor(const FString& Rarity);
    /** Get display name for a cosmetic slot */
    static FString GetSlotDisplayName(ECosmeticSlot SlotType);
    /** Get display name for a dye channel */
    static FString GetDyeChannelName(EDyeChannel Channel);
    /** Parse ECosmeticSlot from JSON string */
    static ECosmeticSlot ParseSlot(const FString& Str);
    /** Parse EDyeChannel from JSON string */
    static EDyeChannel ParseDyeChannel(const FString& Str);
    /** Build cosmetic source description from JSON source object */
    static FString BuildSourceDescription(const TSharedPtr<class FJsonObject>& SourceObj);

    UFUNCTION() void OnApplyClicked();
    UFUNCTION() void OnRemoveClicked();
    UFUNCTION() void OnPreviewClicked();
    UFUNCTION() void OnDyePrimaryClicked();
    UFUNCTION() void OnDyeSecondaryClicked();
    UFUNCTION() void OnDyeAccentClicked();
    UFUNCTION() void OnSavePresetClicked();
    UFUNCTION() void OnLoadPresetClicked();
    UFUNCTION() void OnCloseClicked();
};
