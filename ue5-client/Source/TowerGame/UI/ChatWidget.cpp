#include "ChatWidget.h"
#include "Components/ScrollBox.h"
#include "Components/TextBlock.h"
#include "Components/EditableTextBox.h"
#include "Components/Button.h"
#include "Kismet/GameplayStatics.h"

void UChatWidget::NativeConstruct()
{
    Super::NativeConstruct();

    if (SendButton)
    {
        SendButton->OnClicked.AddDynamic(this, &UChatWidget::OnSendClicked);
    }
    if (InputBox)
    {
        InputBox->OnTextCommitted.AddDynamic(this, &UChatWidget::OnInputCommitted);
    }
}

void UChatWidget::NativeTick(const FGeometry& MyGeometry, float InDeltaTime)
{
    Super::NativeTick(MyGeometry, InDeltaTime);

    // Auto-fade when not interacting
    if (!IsInputFocused())
    {
        TimeSinceLastMessage += InDeltaTime;
        if (TimeSinceLastMessage > FadeDelay && !bFaded)
        {
            bFaded = true;
            SetRenderOpacity(FadeOpacity);
        }
    }
}

void UChatWidget::AddPlayerMessage(const FString& SenderName, const FString& Message)
{
    AddMessage(SenderName, Message, FLinearColor::White, false);
}

void UChatWidget::AddSystemMessage(const FString& Message)
{
    AddMessage(TEXT("System"), Message, FLinearColor(1.0f, 0.9f, 0.3f), true);
}

void UChatWidget::AddCombatLog(const FString& Message)
{
    AddMessage(TEXT(""), Message, FLinearColor(0.5f, 0.5f, 0.5f), true);
}

void UChatWidget::AddMessage(const FString& SenderName, const FString& Message,
    FLinearColor Color, bool bSystem)
{
    FChatMessage Msg;
    Msg.SenderName = SenderName;
    Msg.Message = Message;
    Msg.Color = Color;
    Msg.bIsSystem = bSystem;
    Msg.Timestamp = GetWorld() ? GetWorld()->GetTimeSeconds() : 0.0f;

    Messages.Add(Msg);

    // Trim old messages
    while (Messages.Num() > MaxMessages)
    {
        Messages.RemoveAt(0);
    }

    // Reset fade
    TimeSinceLastMessage = 0.0f;
    if (bFaded)
    {
        bFaded = false;
        SetRenderOpacity(1.0f);
    }

    RebuildMessages();
    ScrollToBottom();
}

void UChatWidget::FocusInput()
{
    if (InputBox)
    {
        InputBox->SetKeyboardFocus();

        // Unfade when focusing
        TimeSinceLastMessage = 0.0f;
        if (bFaded)
        {
            bFaded = false;
            SetRenderOpacity(1.0f);
        }
    }
}

bool UChatWidget::IsInputFocused() const
{
    return InputBox && InputBox->HasKeyboardFocus();
}

void UChatWidget::OnSendClicked()
{
    SendCurrentInput();
}

void UChatWidget::OnInputCommitted(const FText& Text, ETextCommit::Type CommitMethod)
{
    if (CommitMethod == ETextCommit::OnEnter)
    {
        SendCurrentInput();
    }
}

void UChatWidget::SendCurrentInput()
{
    if (!InputBox) return;

    FString Text = InputBox->GetText().ToString().TrimStartAndEnd();
    if (Text.IsEmpty()) return;

    // Clear input
    InputBox->SetText(FText::GetEmpty());

    // Broadcast for match connection to send
    OnMessageSent.Broadcast(Text);

    // Add to local chat immediately
    AddPlayerMessage(TEXT("You"), Text);
}

void UChatWidget::RebuildMessages()
{
    if (!MessageScrollBox) return;

    MessageScrollBox->ClearChildren();

    for (const FChatMessage& Msg : Messages)
    {
        UTextBlock* MsgText = NewObject<UTextBlock>(this);

        FString DisplayText;
        if (bShowTimestamps)
        {
            int32 Mins = FMath::FloorToInt(Msg.Timestamp / 60.0f);
            int32 Secs = FMath::FloorToInt(FMath::Fmod(Msg.Timestamp, 60.0f));
            DisplayText = FString::Printf(TEXT("[%02d:%02d] "), Mins, Secs);
        }

        if (!Msg.SenderName.IsEmpty())
        {
            DisplayText += FString::Printf(TEXT("%s: "), *Msg.SenderName);
        }
        DisplayText += Msg.Message;

        MsgText->SetText(FText::FromString(DisplayText));
        MsgText->SetColorAndOpacity(FSlateColor(Msg.Color));
        MsgText->SetAutoWrapText(true);

        FSlateFontInfo Font = MsgText->GetFont();
        Font.Size = 11;
        MsgText->SetFont(Font);

        MessageScrollBox->AddChild(MsgText);
    }
}

void UChatWidget::ScrollToBottom()
{
    if (MessageScrollBox)
    {
        MessageScrollBox->ScrollToEnd();
    }
}
