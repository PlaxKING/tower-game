#include "SocketWidget.h"
#include "Components/ScrollBox.h"
#include "Components/TextBlock.h"
#include "Components/Button.h"
#include "Components/VerticalBox.h"
#include "Components/HorizontalBox.h"
#include "Components/Border.h"
#include "Components/Image.h"
#include "Dom/JsonObject.h"
#include "Dom/JsonValue.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

void USocketWidget::NativeConstruct()
{
    Super::NativeConstruct();

    CombineSlots.SetNum(3);

    if (InsertButton)
    {
        InsertButton->OnClicked.AddDynamic(this, &USocketWidget::OnInsertClicked);
        InsertButton->SetIsEnabled(false);
    }
    if (RemoveButton)
    {
        RemoveButton->OnClicked.AddDynamic(this, &USocketWidget::OnRemoveClicked);
        RemoveButton->SetIsEnabled(false);
    }
    if (CombineButton)
    {
        CombineButton->OnClicked.AddDynamic(this, &USocketWidget::OnCombineClicked);
        CombineButton->SetIsEnabled(false);
    }

    RebuildSocketDisplay();
    UpdateSocketDetail();
    RebuildGemList();
    RebuildRuneList();
    UpdateCombinePanel();
}

// =============================================================================
// Data Loading
// =============================================================================

void USocketWidget::LoadEquipmentSockets(const FString& EquipmentId, const FString& SocketsJson)
{
    CurrentEquipmentId = EquipmentId;
    Sockets.Empty();
    SelectedSocketIndex = -1;

    TSharedPtr<FJsonObject> Root;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(SocketsJson);
    if (!FJsonSerializer::Deserialize(Reader, Root) || !Root.IsValid())
    {
        UE_LOG(LogTemp, Warning, TEXT("SocketWidget: Failed to parse sockets JSON"));
        return;
    }

    // Parse equipment name
    if (EquipmentNameText)
    {
        FString Name = Root->HasField(TEXT("name"))
            ? Root->GetStringField(TEXT("name"))
            : EquipmentId;
        EquipmentNameText->SetText(FText::FromString(Name));
    }

    // Parse sockets array
    const TArray<TSharedPtr<FJsonValue>>* SocketArray;
    if (Root->TryGetArrayField(TEXT("sockets"), SocketArray))
    {
        for (int32 i = 0; i < SocketArray->Num(); i++)
        {
            const TSharedPtr<FJsonObject>& SObj = (*SocketArray)[i]->AsObject();
            if (!SObj) continue;

            FSocketDisplay Socket;
            Socket.Index = i;
            Socket.Color = ParseSocketColor(SObj->GetStringField(TEXT("color")));
            Socket.bIsEmpty = true;

            // Check for content
            const TSharedPtr<FJsonObject>* ContentObj;
            if (SObj->TryGetObjectField(TEXT("content"), ContentObj))
            {
                Socket.bIsEmpty = false;
                Socket.ContentName = (*ContentObj)->GetStringField(TEXT("name"));
                Socket.ContentDescription = (*ContentObj)->HasField(TEXT("description"))
                    ? (*ContentObj)->GetStringField(TEXT("description"))
                    : TEXT("");
                Socket.bIsGem = (*ContentObj)->HasField(TEXT("tier")); // gems have tier, runes don't
            }

            Sockets.Add(Socket);
        }
    }

    UE_LOG(LogTemp, Log, TEXT("SocketWidget: Loaded %d sockets for %s"), Sockets.Num(), *EquipmentId);

    RebuildSocketDisplay();
    UpdateSocketDetail();
}

void USocketWidget::LoadAvailableGems(const FString& GemsJson)
{
    AvailableGems.Empty();
    SelectedGemIndex = -1;

    TSharedPtr<FJsonValue> Parsed;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(GemsJson);
    if (!FJsonSerializer::Deserialize(Reader, Parsed)) return;

    const TArray<TSharedPtr<FJsonValue>>* GemArray = nullptr;
    if (Parsed->AsObject() && Parsed->AsObject()->TryGetArrayField(TEXT("gems"), GemArray))
    {
        // Object with "gems" array
    }
    else if (Parsed->Type == EJson::Array)
    {
        GemArray = &Parsed->AsArray();
    }
    else
    {
        return;
    }

    for (const TSharedPtr<FJsonValue>& Val : *GemArray)
    {
        const TSharedPtr<FJsonObject>& GObj = Val->AsObject();
        if (!GObj) continue;

        FGemDisplay Gem;
        Gem.Id = GObj->GetStringField(TEXT("id"));
        Gem.Name = GObj->GetStringField(TEXT("name"));
        Gem.Color = ParseSocketColor(GObj->GetStringField(TEXT("color")));
        Gem.Tier = ParseGemTier(GObj->GetStringField(TEXT("tier")));
        Gem.BonusDescription = GObj->HasField(TEXT("bonus_description"))
            ? GObj->GetStringField(TEXT("bonus_description"))
            : TEXT("");

        AvailableGems.Add(Gem);
    }

    UE_LOG(LogTemp, Log, TEXT("SocketWidget: Loaded %d available gems"), AvailableGems.Num());
    RebuildGemList();
}

void USocketWidget::LoadAvailableRunes(const FString& RunesJson)
{
    AvailableRunes.Empty();
    SelectedRuneIndex = -1;

    TSharedPtr<FJsonValue> Parsed;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(RunesJson);
    if (!FJsonSerializer::Deserialize(Reader, Parsed)) return;

    const TArray<TSharedPtr<FJsonValue>>* RuneArray = nullptr;
    if (Parsed->AsObject() && Parsed->AsObject()->TryGetArrayField(TEXT("runes"), RuneArray))
    {
        // Object with "runes" array
    }
    else if (Parsed->Type == EJson::Array)
    {
        RuneArray = &Parsed->AsArray();
    }
    else
    {
        return;
    }

    for (const TSharedPtr<FJsonValue>& Val : *RuneArray)
    {
        const TSharedPtr<FJsonObject>& RObj = Val->AsObject();
        if (!RObj) continue;

        FRuneDisplay Rune;
        Rune.Id = RObj->GetStringField(TEXT("id"));
        Rune.Name = RObj->GetStringField(TEXT("name"));
        Rune.Color = ParseSocketColor(RObj->GetStringField(TEXT("color")));
        Rune.Description = RObj->HasField(TEXT("description"))
            ? RObj->GetStringField(TEXT("description"))
            : TEXT("");
        Rune.EffectDescription = RObj->HasField(TEXT("effect_description"))
            ? RObj->GetStringField(TEXT("effect_description"))
            : TEXT("");

        AvailableRunes.Add(Rune);
    }

    UE_LOG(LogTemp, Log, TEXT("SocketWidget: Loaded %d available runes"), AvailableRunes.Num());
    RebuildRuneList();
}

// =============================================================================
// Socket Interaction
// =============================================================================

void USocketWidget::SelectSocket(int32 Index)
{
    if (Index >= 0 && Index < Sockets.Num())
    {
        SelectedSocketIndex = Index;
        UpdateSocketDetail();
        RebuildSocketDisplay(); // Refresh highlight
        UpdateInsertRemoveButtons();
    }
}

void USocketWidget::InsertGem(int32 SocketIndex, const FString& GemId)
{
    if (SocketIndex < 0 || SocketIndex >= Sockets.Num()) return;

    // Find gem in available list
    int32 GemIdx = -1;
    for (int32 i = 0; i < AvailableGems.Num(); i++)
    {
        if (AvailableGems[i].Id == GemId)
        {
            GemIdx = i;
            break;
        }
    }
    if (GemIdx < 0) return;

    const FGemDisplay& Gem = AvailableGems[GemIdx];

    // Check color compatibility
    if (!IsGemCompatible(SocketIndex, Gem.Color))
    {
        UE_LOG(LogTemp, Warning, TEXT("SocketWidget: Color mismatch — %s gem cannot fit %s socket"),
            *GetSocketColorName(Gem.Color), *GetSocketColorName(Sockets[SocketIndex].Color));
        return;
    }

    // Insert
    FSocketDisplay& Socket = Sockets[SocketIndex];
    Socket.bIsEmpty = false;
    Socket.ContentName = Gem.Name;
    Socket.ContentDescription = Gem.BonusDescription;
    Socket.bIsGem = true;

    // Remove gem from available list
    AvailableGems.RemoveAt(GemIdx);
    if (SelectedGemIndex >= GemIdx)
    {
        SelectedGemIndex = FMath::Max(-1, SelectedGemIndex - 1);
    }

    OnSocketModified.Broadcast(CurrentEquipmentId, SocketIndex);

    RebuildSocketDisplay();
    UpdateSocketDetail();
    RebuildGemList();
    UpdateInsertRemoveButtons();
}

void USocketWidget::InsertRune(int32 SocketIndex, const FString& RuneId)
{
    if (SocketIndex < 0 || SocketIndex >= Sockets.Num()) return;

    // Find rune in available list
    int32 RuneIdx = -1;
    for (int32 i = 0; i < AvailableRunes.Num(); i++)
    {
        if (AvailableRunes[i].Id == RuneId)
        {
            RuneIdx = i;
            break;
        }
    }
    if (RuneIdx < 0) return;

    const FRuneDisplay& Rune = AvailableRunes[RuneIdx];

    // Check color compatibility
    if (!IsRuneCompatible(SocketIndex, Rune.Color))
    {
        UE_LOG(LogTemp, Warning, TEXT("SocketWidget: Color mismatch — %s rune cannot fit %s socket"),
            *GetSocketColorName(Rune.Color), *GetSocketColorName(Sockets[SocketIndex].Color));
        return;
    }

    // Insert
    FSocketDisplay& Socket = Sockets[SocketIndex];
    Socket.bIsEmpty = false;
    Socket.ContentName = Rune.Name;
    Socket.ContentDescription = Rune.EffectDescription;
    Socket.bIsGem = false;

    // Remove rune from available list
    AvailableRunes.RemoveAt(RuneIdx);
    if (SelectedRuneIndex >= RuneIdx)
    {
        SelectedRuneIndex = FMath::Max(-1, SelectedRuneIndex - 1);
    }

    OnSocketModified.Broadcast(CurrentEquipmentId, SocketIndex);

    RebuildSocketDisplay();
    UpdateSocketDetail();
    RebuildRuneList();
    UpdateInsertRemoveButtons();
}

void USocketWidget::RemoveContent(int32 SocketIndex)
{
    if (SocketIndex < 0 || SocketIndex >= Sockets.Num()) return;

    FSocketDisplay& Socket = Sockets[SocketIndex];
    if (Socket.bIsEmpty) return;

    Socket.bIsEmpty = true;
    Socket.ContentName = TEXT("");
    Socket.ContentDescription = TEXT("");

    OnSocketModified.Broadcast(CurrentEquipmentId, SocketIndex);

    RebuildSocketDisplay();
    UpdateSocketDetail();
    UpdateInsertRemoveButtons();
}

// =============================================================================
// Gem Combining
// =============================================================================

void USocketWidget::CombineGems(const FString& GemId1, const FString& GemId2, const FString& GemId3)
{
    // Find all three gems
    const FGemDisplay* Gem1 = nullptr;
    const FGemDisplay* Gem2 = nullptr;
    const FGemDisplay* Gem3 = nullptr;

    for (const FGemDisplay& G : AvailableGems)
    {
        if (G.Id == GemId1 && !Gem1) { Gem1 = &G; continue; }
        if (G.Id == GemId2 && !Gem2) { Gem2 = &G; continue; }
        if (G.Id == GemId3 && !Gem3) { Gem3 = &G; continue; }
    }

    if (!Gem1 || !Gem2 || !Gem3)
    {
        UE_LOG(LogTemp, Warning, TEXT("SocketWidget: Cannot find all 3 gems for combining"));
        if (CombineResultText)
        {
            CombineResultText->SetText(FText::FromString(TEXT("Missing gems!")));
            CombineResultText->SetColorAndOpacity(FSlateColor(FLinearColor(1.0f, 0.3f, 0.3f)));
        }
        return;
    }

    // Validate: same color and tier
    if (Gem1->Color != Gem2->Color || Gem2->Color != Gem3->Color)
    {
        UE_LOG(LogTemp, Warning, TEXT("SocketWidget: Gems must be same color to combine"));
        if (CombineResultText)
        {
            CombineResultText->SetText(FText::FromString(TEXT("Color mismatch!")));
            CombineResultText->SetColorAndOpacity(FSlateColor(FLinearColor(1.0f, 0.3f, 0.3f)));
        }
        return;
    }

    if (Gem1->Tier != Gem2->Tier || Gem2->Tier != Gem3->Tier)
    {
        UE_LOG(LogTemp, Warning, TEXT("SocketWidget: Gems must be same tier to combine"));
        if (CombineResultText)
        {
            CombineResultText->SetText(FText::FromString(TEXT("Tier mismatch!")));
            CombineResultText->SetColorAndOpacity(FSlateColor(FLinearColor(1.0f, 0.3f, 0.3f)));
        }
        return;
    }

    // Check not already max tier (Radiant)
    if (Gem1->Tier == EGemTier::Radiant)
    {
        UE_LOG(LogTemp, Warning, TEXT("SocketWidget: Radiant gems cannot be combined further"));
        if (CombineResultText)
        {
            CombineResultText->SetText(FText::FromString(TEXT("Already max tier!")));
            CombineResultText->SetColorAndOpacity(FSlateColor(FLinearColor(1.0f, 0.8f, 0.1f)));
        }
        return;
    }

    // Determine next tier name for display
    EGemTier NextTier;
    switch (Gem1->Tier)
    {
    case EGemTier::Chipped:  NextTier = EGemTier::Flawed;   break;
    case EGemTier::Flawed:   NextTier = EGemTier::Regular;  break;
    case EGemTier::Regular:  NextTier = EGemTier::Flawless; break;
    case EGemTier::Flawless: NextTier = EGemTier::Perfect;  break;
    case EGemTier::Perfect:  NextTier = EGemTier::Radiant;  break;
    default:                 NextTier = EGemTier::Chipped;   break;
    }

    // Build result gem ID — server will assign final ID, but we create a preview
    FString NewGemId = FString::Printf(TEXT("combined_%s_%s"),
        *GetSocketColorName(Gem1->Color).ToLower(),
        *GetGemTierName(NextTier).ToLower());

    // Remove the 3 consumed gems (remove in reverse order to keep indices valid)
    TArray<int32> RemoveIndices;
    for (int32 i = 0; i < AvailableGems.Num(); i++)
    {
        if (AvailableGems[i].Id == GemId1 || AvailableGems[i].Id == GemId2 || AvailableGems[i].Id == GemId3)
        {
            RemoveIndices.Add(i);
        }
        if (RemoveIndices.Num() >= 3) break;
    }
    for (int32 i = RemoveIndices.Num() - 1; i >= 0; i--)
    {
        AvailableGems.RemoveAt(RemoveIndices[i]);
    }

    SelectedGemIndex = -1;

    // Show result
    if (CombineResultText)
    {
        FString ResultMsg = FString::Printf(TEXT("Combined! New: %s gem"),
            *GetGemTierName(NextTier));
        CombineResultText->SetText(FText::FromString(ResultMsg));
        CombineResultText->SetColorAndOpacity(FSlateColor(GetGemTierColor(NextTier)));
    }

    OnGemsCombined.Broadcast(NewGemId);

    ClearCombineSlots();
    RebuildGemList();
    UpdateCombinePanel();
}

// =============================================================================
// Queries
// =============================================================================

bool USocketWidget::IsGemCompatible(int32 SocketIndex, ESocketColor GemColor) const
{
    if (SocketIndex < 0 || SocketIndex >= Sockets.Num()) return false;
    ESocketColor SocketColor = Sockets[SocketIndex].Color;

    // Prismatic accepts any color
    if (SocketColor == ESocketColor::Prismatic) return true;

    return SocketColor == GemColor;
}

bool USocketWidget::IsRuneCompatible(int32 SocketIndex, ESocketColor RuneColor) const
{
    // Same rules as gems
    return IsGemCompatible(SocketIndex, RuneColor);
}

FLinearColor USocketWidget::GetSocketColorValue(ESocketColor Color)
{
    switch (Color)
    {
    case ESocketColor::Red:       return FLinearColor(1.0f, 0.267f, 0.267f);   // #FF4444
    case ESocketColor::Blue:      return FLinearColor(0.267f, 0.533f, 1.0f);   // #4488FF
    case ESocketColor::Yellow:    return FLinearColor(1.0f, 0.8f, 0.133f);     // #FFCC22
    case ESocketColor::Prismatic: return FLinearColor(1.0f, 1.0f, 1.0f);       // white/rainbow
    default:                      return FLinearColor::White;
    }
}

FLinearColor USocketWidget::GetGemTierColor(EGemTier Tier)
{
    switch (Tier)
    {
    case EGemTier::Chipped:  return FLinearColor(0.5f, 0.5f, 0.5f);   // gray
    case EGemTier::Flawed:   return FLinearColor(0.9f, 0.9f, 0.9f);   // white
    case EGemTier::Regular:  return FLinearColor(0.2f, 0.9f, 0.3f);   // green
    case EGemTier::Flawless: return FLinearColor(0.3f, 0.5f, 1.0f);   // blue
    case EGemTier::Perfect:  return FLinearColor(0.7f, 0.3f, 1.0f);   // purple
    case EGemTier::Radiant:  return FLinearColor(1.0f, 0.84f, 0.0f);  // gold
    default:                 return FLinearColor::White;
    }
}

// =============================================================================
// Display Rebuild
// =============================================================================

void USocketWidget::RebuildSocketDisplay()
{
    if (!SocketSlotsBox) return;
    SocketSlotsBox->ClearChildren();

    for (int32 i = 0; i < Sockets.Num(); i++)
    {
        const FSocketDisplay& Socket = Sockets[i];

        UTextBlock* SlotText = NewObject<UTextBlock>(this);

        // Build display: [Color] Content or [Color] Empty
        FString ContentStr = Socket.bIsEmpty
            ? TEXT("Empty")
            : FString::Printf(TEXT("%s %s"), Socket.bIsGem ? TEXT("[Gem]") : TEXT("[Rune]"), *Socket.ContentName);

        FString Display = FString::Printf(TEXT("[%d] %s: %s"),
            i + 1, *GetSocketColorName(Socket.Color), *ContentStr);

        SlotText->SetText(FText::FromString(Display));

        // Color by socket color, highlight selected
        FLinearColor Color = GetSocketColorValue(Socket.Color);
        if (i == SelectedSocketIndex)
        {
            // Brighten selected socket
            Color = FLinearColor(
                FMath::Min(Color.R + 0.3f, 1.0f),
                FMath::Min(Color.G + 0.3f, 1.0f),
                FMath::Min(Color.B + 0.3f, 1.0f));
        }
        else if (!Socket.bIsEmpty)
        {
            // Slightly dim filled sockets
            Color *= 0.8f;
            Color.A = 1.0f;
        }

        SlotText->SetColorAndOpacity(FSlateColor(Color));

        FSlateFontInfo Font = SlotText->GetFont();
        Font.Size = (i == SelectedSocketIndex) ? 14 : 12;
        SlotText->SetFont(Font);

        SocketSlotsBox->AddChild(SlotText);
    }
}

void USocketWidget::UpdateSocketDetail()
{
    if (!Sockets.IsValidIndex(SelectedSocketIndex))
    {
        if (SocketDetailTitle) SocketDetailTitle->SetText(FText::FromString(TEXT("No socket selected")));
        if (SocketColorText) SocketColorText->SetText(FText::GetEmpty());
        if (SocketContentText) SocketContentText->SetText(FText::GetEmpty());
        if (SocketContentDesc) SocketContentDesc->SetText(FText::GetEmpty());
        if (CompatibilityText) CompatibilityText->SetText(FText::GetEmpty());
        return;
    }

    const FSocketDisplay& Socket = Sockets[SelectedSocketIndex];

    if (SocketDetailTitle)
    {
        SocketDetailTitle->SetText(FText::FromString(
            FString::Printf(TEXT("Socket %d"), SelectedSocketIndex + 1)));
    }

    if (SocketColorText)
    {
        FString ColorName = GetSocketColorName(Socket.Color);
        FString ColorDesc;
        switch (Socket.Color)
        {
        case ESocketColor::Red:       ColorDesc = TEXT("Offensive"); break;
        case ESocketColor::Blue:      ColorDesc = TEXT("Defensive"); break;
        case ESocketColor::Yellow:    ColorDesc = TEXT("Utility");   break;
        case ESocketColor::Prismatic: ColorDesc = TEXT("Any");       break;
        }
        SocketColorText->SetText(FText::FromString(
            FString::Printf(TEXT("%s (%s)"), *ColorName, *ColorDesc)));
        SocketColorText->SetColorAndOpacity(FSlateColor(GetSocketColorValue(Socket.Color)));
    }

    if (SocketContentText)
    {
        if (Socket.bIsEmpty)
        {
            SocketContentText->SetText(FText::FromString(TEXT("< Empty >")));
            SocketContentText->SetColorAndOpacity(FSlateColor(FLinearColor(0.5f, 0.5f, 0.5f)));
        }
        else
        {
            FString TypeStr = Socket.bIsGem ? TEXT("Gem") : TEXT("Rune");
            SocketContentText->SetText(FText::FromString(
                FString::Printf(TEXT("[%s] %s"), *TypeStr, *Socket.ContentName)));
            SocketContentText->SetColorAndOpacity(FSlateColor(GetSocketColorValue(Socket.Color)));
        }
    }

    if (SocketContentDesc)
    {
        SocketContentDesc->SetText(FText::FromString(Socket.ContentDescription));
    }

    // Show compatibility info for currently selected gem/rune
    if (CompatibilityText)
    {
        FString CompatMsg;
        if (SelectedGemIndex >= 0 && SelectedGemIndex < AvailableGems.Num())
        {
            const FGemDisplay& Gem = AvailableGems[SelectedGemIndex];
            bool bCompat = IsGemCompatible(SelectedSocketIndex, Gem.Color);
            if (bCompat)
            {
                CompatMsg = FString::Printf(TEXT("%s fits this socket"), *Gem.Name);
                CompatibilityText->SetColorAndOpacity(FSlateColor(FLinearColor(0.3f, 1.0f, 0.3f)));
            }
            else
            {
                CompatMsg = FString::Printf(TEXT("%s (%s) cannot fit %s socket"),
                    *Gem.Name, *GetSocketColorName(Gem.Color), *GetSocketColorName(Socket.Color));
                CompatibilityText->SetColorAndOpacity(FSlateColor(FLinearColor(1.0f, 0.3f, 0.3f)));
            }
        }
        else if (SelectedRuneIndex >= 0 && SelectedRuneIndex < AvailableRunes.Num())
        {
            const FRuneDisplay& Rune = AvailableRunes[SelectedRuneIndex];
            bool bCompat = IsRuneCompatible(SelectedSocketIndex, Rune.Color);
            if (bCompat)
            {
                CompatMsg = FString::Printf(TEXT("%s fits this socket"), *Rune.Name);
                CompatibilityText->SetColorAndOpacity(FSlateColor(FLinearColor(0.3f, 1.0f, 0.3f)));
            }
            else
            {
                CompatMsg = FString::Printf(TEXT("%s (%s) cannot fit %s socket"),
                    *Rune.Name, *GetSocketColorName(Rune.Color), *GetSocketColorName(Socket.Color));
                CompatibilityText->SetColorAndOpacity(FSlateColor(FLinearColor(1.0f, 0.3f, 0.3f)));
            }
        }
        else if (Socket.Color == ESocketColor::Prismatic)
        {
            CompatMsg = TEXT("Prismatic — accepts any gem or rune");
            CompatibilityText->SetColorAndOpacity(FSlateColor(FLinearColor(0.8f, 0.8f, 1.0f)));
        }

        CompatibilityText->SetText(FText::FromString(CompatMsg));
    }
}

void USocketWidget::RebuildGemList()
{
    if (!GemListScrollBox) return;
    GemListScrollBox->ClearChildren();

    for (int32 i = 0; i < AvailableGems.Num(); i++)
    {
        const FGemDisplay& Gem = AvailableGems[i];

        UTextBlock* GemText = NewObject<UTextBlock>(this);

        FString Display = FString::Printf(TEXT("[%s] %s (%s)"),
            *GetGemTierName(Gem.Tier), *Gem.Name, *GetSocketColorName(Gem.Color));
        GemText->SetText(FText::FromString(Display));

        // Color by gem tier, highlight selected
        FLinearColor Color = GetGemTierColor(Gem.Tier);
        if (i == SelectedGemIndex)
        {
            Color = FLinearColor(1.0f, 1.0f, 0.3f); // Bright yellow highlight
        }

        // Show compatibility dimming if a socket is selected
        if (SelectedSocketIndex >= 0 && SelectedSocketIndex < Sockets.Num())
        {
            if (!IsGemCompatible(SelectedSocketIndex, Gem.Color) && i != SelectedGemIndex)
            {
                Color *= 0.4f; // Dim incompatible gems
                Color.A = 1.0f;
            }
        }

        GemText->SetColorAndOpacity(FSlateColor(Color));

        FSlateFontInfo Font = GemText->GetFont();
        Font.Size = 11;
        GemText->SetFont(Font);

        GemListScrollBox->AddChild(GemText);
    }
}

void USocketWidget::RebuildRuneList()
{
    if (!RuneListScrollBox) return;
    RuneListScrollBox->ClearChildren();

    for (int32 i = 0; i < AvailableRunes.Num(); i++)
    {
        const FRuneDisplay& Rune = AvailableRunes[i];

        UTextBlock* RuneText = NewObject<UTextBlock>(this);

        FString Display = FString::Printf(TEXT("[%s] %s"),
            *GetSocketColorName(Rune.Color), *Rune.Name);
        RuneText->SetText(FText::FromString(Display));

        // Color by socket color, highlight selected
        FLinearColor Color = GetSocketColorValue(Rune.Color);
        if (i == SelectedRuneIndex)
        {
            Color = FLinearColor(1.0f, 1.0f, 0.3f);
        }

        // Show compatibility dimming if a socket is selected
        if (SelectedSocketIndex >= 0 && SelectedSocketIndex < Sockets.Num())
        {
            if (!IsRuneCompatible(SelectedSocketIndex, Rune.Color) && i != SelectedRuneIndex)
            {
                Color *= 0.4f;
                Color.A = 1.0f;
            }
        }

        RuneText->SetColorAndOpacity(FSlateColor(Color));

        FSlateFontInfo Font = RuneText->GetFont();
        Font.Size = 11;
        RuneText->SetFont(Font);

        RuneListScrollBox->AddChild(RuneText);
    }
}

void USocketWidget::UpdateInsertRemoveButtons()
{
    bool bSocketSelected = Sockets.IsValidIndex(SelectedSocketIndex);
    bool bGemSelected = AvailableGems.IsValidIndex(SelectedGemIndex);
    bool bRuneSelected = AvailableRunes.IsValidIndex(SelectedRuneIndex);

    if (InsertButton)
    {
        bool bCanInsert = false;
        if (bSocketSelected && Sockets[SelectedSocketIndex].bIsEmpty)
        {
            if (bGemSelected)
            {
                bCanInsert = IsGemCompatible(SelectedSocketIndex, AvailableGems[SelectedGemIndex].Color);
            }
            else if (bRuneSelected)
            {
                bCanInsert = IsRuneCompatible(SelectedSocketIndex, AvailableRunes[SelectedRuneIndex].Color);
            }
        }
        InsertButton->SetIsEnabled(bCanInsert);
    }

    if (RemoveButton)
    {
        bool bCanRemove = bSocketSelected && !Sockets[SelectedSocketIndex].bIsEmpty;
        RemoveButton->SetIsEnabled(bCanRemove);
    }
}

void USocketWidget::UpdateCombinePanel()
{
    // Update combine slot texts
    auto UpdateSlotText = [this](UTextBlock* Text, int32 SlotIdx)
    {
        if (!Text) return;
        if (SlotIdx < CombineSlots.Num() && !CombineSlots[SlotIdx].IsEmpty())
        {
            // Find gem name for display
            FString GemName = CombineSlots[SlotIdx];
            for (const FGemDisplay& G : AvailableGems)
            {
                if (G.Id == CombineSlots[SlotIdx])
                {
                    GemName = G.Name;
                    Text->SetColorAndOpacity(FSlateColor(GetGemTierColor(G.Tier)));
                    break;
                }
            }
            Text->SetText(FText::FromString(GemName));
        }
        else
        {
            Text->SetText(FText::FromString(TEXT("< Empty >")));
            Text->SetColorAndOpacity(FSlateColor(FLinearColor(0.4f, 0.4f, 0.4f)));
        }
    };

    UpdateSlotText(CombineSlot1Text, 0);
    UpdateSlotText(CombineSlot2Text, 1);
    UpdateSlotText(CombineSlot3Text, 2);

    // Enable combine button only if all 3 slots filled
    if (CombineButton)
    {
        bool bAllFilled = CombineSlots.Num() >= 3
            && !CombineSlots[0].IsEmpty()
            && !CombineSlots[1].IsEmpty()
            && !CombineSlots[2].IsEmpty();
        CombineButton->SetIsEnabled(bAllFilled);
    }
}

// =============================================================================
// Parsers & Helpers
// =============================================================================

FString USocketWidget::GetSocketColorName(ESocketColor Color) const
{
    switch (Color)
    {
    case ESocketColor::Red:       return TEXT("Red");
    case ESocketColor::Blue:      return TEXT("Blue");
    case ESocketColor::Yellow:    return TEXT("Yellow");
    case ESocketColor::Prismatic: return TEXT("Prismatic");
    default:                      return TEXT("Unknown");
    }
}

FString USocketWidget::GetGemTierName(EGemTier Tier) const
{
    switch (Tier)
    {
    case EGemTier::Chipped:  return TEXT("Chipped");
    case EGemTier::Flawed:   return TEXT("Flawed");
    case EGemTier::Regular:  return TEXT("Regular");
    case EGemTier::Flawless: return TEXT("Flawless");
    case EGemTier::Perfect:  return TEXT("Perfect");
    case EGemTier::Radiant:  return TEXT("Radiant");
    default:                 return TEXT("Unknown");
    }
}

ESocketColor USocketWidget::ParseSocketColor(const FString& Str) const
{
    if (Str == TEXT("Red"))       return ESocketColor::Red;
    if (Str == TEXT("Blue"))      return ESocketColor::Blue;
    if (Str == TEXT("Yellow"))    return ESocketColor::Yellow;
    if (Str == TEXT("Prismatic")) return ESocketColor::Prismatic;
    return ESocketColor::Red;
}

EGemTier USocketWidget::ParseGemTier(const FString& Str) const
{
    if (Str == TEXT("Chipped"))  return EGemTier::Chipped;
    if (Str == TEXT("Flawed"))   return EGemTier::Flawed;
    if (Str == TEXT("Regular"))  return EGemTier::Regular;
    if (Str == TEXT("Flawless")) return EGemTier::Flawless;
    if (Str == TEXT("Perfect"))  return EGemTier::Perfect;
    if (Str == TEXT("Radiant"))  return EGemTier::Radiant;
    return EGemTier::Chipped;
}

void USocketWidget::SelectGem(int32 Index)
{
    if (Index >= 0 && Index < AvailableGems.Num())
    {
        SelectedGemIndex = Index;
        SelectedRuneIndex = -1; // Deselect rune when gem is selected
        RebuildGemList();
        RebuildRuneList();
        UpdateSocketDetail(); // Refresh compatibility display
        UpdateInsertRemoveButtons();
    }
}

void USocketWidget::SelectRune(int32 Index)
{
    if (Index >= 0 && Index < AvailableRunes.Num())
    {
        SelectedRuneIndex = Index;
        SelectedGemIndex = -1; // Deselect gem when rune is selected
        RebuildRuneList();
        RebuildGemList();
        UpdateSocketDetail(); // Refresh compatibility display
        UpdateInsertRemoveButtons();
    }
}

void USocketWidget::AddGemToCombine(const FString& GemId)
{
    // Fill next empty combine slot
    for (int32 i = 0; i < CombineSlots.Num(); i++)
    {
        if (CombineSlots[i].IsEmpty())
        {
            CombineSlots[i] = GemId;
            UpdateCombinePanel();
            return;
        }
    }

    UE_LOG(LogTemp, Warning, TEXT("SocketWidget: All 3 combine slots are full"));
}

void USocketWidget::ClearCombineSlots()
{
    for (FString& SlotStr : CombineSlots)
    {
        SlotStr.Empty();
    }
    UpdateCombinePanel();
}

// =============================================================================
// Button Callbacks
// =============================================================================

void USocketWidget::OnInsertClicked()
{
    if (!Sockets.IsValidIndex(SelectedSocketIndex)) return;

    if (AvailableGems.IsValidIndex(SelectedGemIndex))
    {
        InsertGem(SelectedSocketIndex, AvailableGems[SelectedGemIndex].Id);
    }
    else if (AvailableRunes.IsValidIndex(SelectedRuneIndex))
    {
        InsertRune(SelectedSocketIndex, AvailableRunes[SelectedRuneIndex].Id);
    }
}

void USocketWidget::OnRemoveClicked()
{
    if (Sockets.IsValidIndex(SelectedSocketIndex))
    {
        RemoveContent(SelectedSocketIndex);
    }
}

void USocketWidget::OnCombineClicked()
{
    if (CombineSlots.Num() >= 3
        && !CombineSlots[0].IsEmpty()
        && !CombineSlots[1].IsEmpty()
        && !CombineSlots[2].IsEmpty())
    {
        CombineGems(CombineSlots[0], CombineSlots[1], CombineSlots[2]);
    }
}
