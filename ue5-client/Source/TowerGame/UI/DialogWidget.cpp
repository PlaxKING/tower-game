#include "DialogWidget.h"
#include "Components/TextBlock.h"
#include "Components/VerticalBox.h"
#include "Components/Button.h"
#include "Dom/JsonObject.h"
#include "Dom/JsonValue.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

void UDialogWidget::NativeConstruct()
{
    Super::NativeConstruct();
    SetVisibility(ESlateVisibility::Collapsed);
}

void UDialogWidget::NativeTick(const FGeometry& MyGeometry, float InDeltaTime)
{
    Super::NativeTick(MyGeometry, InDeltaTime);

    if (!bShowing || !bTypewriting) return;

    TypewriterTimer += InDeltaTime;

    int32 CharsToShow = FMath::FloorToInt(TypewriterTimer * TypewriterSpeed);
    if (CharsToShow > RevealedChars)
    {
        RevealedChars = FMath::Min(CharsToShow, FullText.Len());

        if (DialogText)
        {
            DialogText->SetText(FText::FromString(FullText.Left(RevealedChars)));
        }

        if (RevealedChars >= FullText.Len())
        {
            bTypewriting = false;

            // Show continue hint
            if (ContinueHintText)
            {
                ContinueHintText->SetVisibility(ESlateVisibility::Visible);
            }
        }
    }
}

void UDialogWidget::ShowNode(const FDialogNodeData& Node)
{
    CurrentNode = Node;
    bShowing = true;
    bTypewriting = true;
    FullText = Node.Text;
    RevealedChars = 0;
    TypewriterTimer = 0.0f;

    SetVisibility(ESlateVisibility::Visible);

    // Speaker name
    if (SpeakerNameText)
    {
        SpeakerNameText->SetText(FText::FromString(Node.Speaker));
        SpeakerNameText->SetColorAndOpacity(FSlateColor(GetFactionColor(Node.Faction)));
    }

    // Start empty for typewriter
    if (DialogText)
    {
        DialogText->SetText(FText::GetEmpty());
    }

    // Hide continue hint during typewriting
    if (ContinueHintText)
    {
        ContinueHintText->SetVisibility(ESlateVisibility::Collapsed);
        ContinueHintText->SetText(FText::FromString(TEXT("Click to continue...")));
    }

    // Build choices (hidden until text finishes)
    BuildChoices(Node.Choices);

    // Pause game and show cursor
    APlayerController* PC = GetOwningPlayer();
    if (PC)
    {
        PC->SetShowMouseCursor(true);
        PC->SetInputMode(FInputModeGameAndUI());
    }
}

void UDialogWidget::ShowNodeFromJson(const FString& NodeJson)
{
    TSharedPtr<FJsonObject> Json;
    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(NodeJson);
    if (!FJsonSerializer::Deserialize(Reader, Json) || !Json.IsValid()) return;

    FDialogNodeData Node;
    Node.NodeId = Json->GetIntegerField(TEXT("id"));
    Node.Speaker = Json->GetStringField(TEXT("speaker"));
    Node.Text = Json->GetStringField(TEXT("text"));
    Node.Faction = Json->HasField(TEXT("faction")) ? Json->GetStringField(TEXT("faction")) : TEXT("neutral");

    const TArray<TSharedPtr<FJsonValue>>* Choices;
    if (Json->TryGetArrayField(TEXT("choices"), Choices))
    {
        for (const TSharedPtr<FJsonValue>& ChoiceVal : *Choices)
        {
            const TSharedPtr<FJsonObject>& ChoiceObj = ChoiceVal->AsObject();
            if (!ChoiceObj.IsValid()) continue;

            FDialogChoiceData Choice;
            Choice.Text = ChoiceObj->GetStringField(TEXT("text"));
            Choice.NextNodeId = ChoiceObj->GetIntegerField(TEXT("next_node"));
            Choice.bAvailable = ChoiceObj->HasField(TEXT("available")) ?
                ChoiceObj->GetBoolField(TEXT("available")) : true;
            Choice.RequirementHint = ChoiceObj->HasField(TEXT("requirement")) ?
                ChoiceObj->GetStringField(TEXT("requirement")) : TEXT("");

            Node.Choices.Add(Choice);
        }
    }

    ShowNode(Node);
}

void UDialogWidget::CloseDialog()
{
    bShowing = false;
    bTypewriting = false;
    SetVisibility(ESlateVisibility::Collapsed);

    APlayerController* PC = GetOwningPlayer();
    if (PC)
    {
        PC->SetShowMouseCursor(false);
        PC->SetInputMode(FInputModeGameOnly());
    }

    OnClosed.Broadcast();
}

void UDialogWidget::SkipTypewriter()
{
    if (!bTypewriting) return;

    bTypewriting = false;
    RevealedChars = FullText.Len();

    if (DialogText)
    {
        DialogText->SetText(FText::FromString(FullText));
    }

    if (ContinueHintText)
    {
        ContinueHintText->SetVisibility(ESlateVisibility::Visible);
    }
}

void UDialogWidget::BuildChoices(const TArray<FDialogChoiceData>& Choices)
{
    if (!ChoicesBox) return;

    ChoicesBox->ClearChildren();

    for (int32 i = 0; i < Choices.Num(); i++)
    {
        const FDialogChoiceData& Choice = Choices[i];

        UTextBlock* ChoiceText = NewObject<UTextBlock>(this);

        FString DisplayText = FString::Printf(TEXT("[%d] %s"), i + 1, *Choice.Text);

        if (!Choice.bAvailable && !Choice.RequirementHint.IsEmpty())
        {
            DisplayText += FString::Printf(TEXT(" (%s)"), *Choice.RequirementHint);
        }

        ChoiceText->SetText(FText::FromString(DisplayText));

        if (Choice.bAvailable)
        {
            ChoiceText->SetColorAndOpacity(FSlateColor(FLinearColor(0.9f, 0.9f, 0.9f)));
        }
        else
        {
            ChoiceText->SetColorAndOpacity(FSlateColor(FLinearColor(0.4f, 0.4f, 0.4f)));
        }

        FSlateFontInfo Font = ChoiceText->GetFont();
        Font.Size = 13;
        ChoiceText->SetFont(Font);

        ChoicesBox->AddChild(ChoiceText);
    }
}

FLinearColor UDialogWidget::GetFactionColor(const FString& Faction) const
{
    if (Faction == TEXT("seekers"))   return FLinearColor(0.2f, 0.6f, 1.0f);
    if (Faction == TEXT("wardens"))   return FLinearColor(0.2f, 0.8f, 0.3f);
    if (Faction == TEXT("breakers"))  return FLinearColor(1.0f, 0.3f, 0.2f);
    if (Faction == TEXT("weavers"))   return FLinearColor(0.7f, 0.3f, 1.0f);
    return FLinearColor(0.8f, 0.8f, 0.8f);
}
