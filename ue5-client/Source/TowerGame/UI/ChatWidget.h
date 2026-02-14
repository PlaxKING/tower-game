#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "ChatWidget.generated.h"

class UScrollBox;
class UTextBlock;
class UEditableTextBox;
class UButton;

/**
 * Chat message entry.
 */
USTRUCT(BlueprintType)
struct FChatMessage
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadOnly, Category = "Chat")
    FString SenderName;

    UPROPERTY(BlueprintReadOnly, Category = "Chat")
    FString Message;

    UPROPERTY(BlueprintReadOnly, Category = "Chat")
    FLinearColor Color = FLinearColor::White;

    UPROPERTY(BlueprintReadOnly, Category = "Chat")
    float Timestamp = 0.0f;

    UPROPERTY(BlueprintReadOnly, Category = "Chat")
    bool bIsSystem = false;
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnChatMessageSent, const FString&, Message);

/**
 * In-game chat widget.
 *
 * Layout:
 *   [Message scroll area - last N messages]
 *   [Input box] [Send button]
 *
 * Features:
 * - Player messages (white), system messages (yellow), combat log (gray)
 * - Auto-fade after inactivity (semi-transparent when not focused)
 * - Enter key to focus input, Enter to send
 * - Max history (50 messages)
 * - Timestamp display option
 */
UCLASS()
class TOWERGAME_API UChatWidget : public UUserWidget
{
    GENERATED_BODY()

public:
    virtual void NativeConstruct() override;
    virtual void NativeTick(const FGeometry& MyGeometry, float InDeltaTime) override;

    // ============ API ============

    /** Add a player message */
    UFUNCTION(BlueprintCallable, Category = "Chat")
    void AddPlayerMessage(const FString& SenderName, const FString& Message);

    /** Add a system message (yellow) */
    UFUNCTION(BlueprintCallable, Category = "Chat")
    void AddSystemMessage(const FString& Message);

    /** Add a combat log entry (gray) */
    UFUNCTION(BlueprintCallable, Category = "Chat")
    void AddCombatLog(const FString& Message);

    /** Add a colored message */
    UFUNCTION(BlueprintCallable, Category = "Chat")
    void AddMessage(const FString& SenderName, const FString& Message, FLinearColor Color, bool bSystem);

    /** Focus the input box */
    UFUNCTION(BlueprintCallable, Category = "Chat")
    void FocusInput();

    /** Is the input focused? */
    UFUNCTION(BlueprintPure, Category = "Chat")
    bool IsInputFocused() const;

    // ============ Config ============

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Chat")
    int32 MaxMessages = 50;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Chat")
    float FadeDelay = 5.0f;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Chat")
    float FadeOpacity = 0.3f;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Chat")
    bool bShowTimestamps = false;

    // ============ Events ============

    UPROPERTY(BlueprintAssignable, Category = "Chat")
    FOnChatMessageSent OnMessageSent;

    // ============ Bound Widgets ============

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Chat")
    UScrollBox* MessageScrollBox;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Chat")
    UEditableTextBox* InputBox;

    UPROPERTY(meta = (BindWidgetOptional), BlueprintReadOnly, Category = "Chat")
    UButton* SendButton;

protected:
    UFUNCTION()
    void OnSendClicked();

    UFUNCTION()
    void OnInputCommitted(const FText& Text, ETextCommit::Type CommitMethod);

    void SendCurrentInput();
    void RebuildMessages();
    void ScrollToBottom();

private:
    UPROPERTY()
    TArray<FChatMessage> Messages;

    float TimeSinceLastMessage = 0.0f;
    bool bFaded = false;
};
