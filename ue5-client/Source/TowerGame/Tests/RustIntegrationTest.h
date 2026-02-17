// Copyright Tower Game 2026. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "TowerGame/Core/TowerGameSubsystem.h"
#include "RustIntegrationTest.generated.h"

/**
 * Test actor for verifying Rust DLL integration
 *
 * Usage:
 * 1. Place this actor on a level
 * 2. Press Play
 * 3. Check Output Log for test results
 *
 * All tests run automatically on BeginPlay
 */
UCLASS()
class TOWERGAME_API ARustIntegrationTest : public AActor
{
	GENERATED_BODY()

public:
	ARustIntegrationTest();

protected:
	virtual void BeginPlay() override;

public:
	// ============================================================
	// Test Configuration
	// ============================================================

	/** If true, tests will run automatically on BeginPlay */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Testing")
	bool bAutoRunTests = true;

	/** If true, stops on first test failure */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Testing")
	bool bStopOnFailure = false;

	/** Delay between tests (seconds) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Testing")
	float DelayBetweenTests = 0.5f;

	// ============================================================
	// Manual Test Triggers (Blueprint Callable)
	// ============================================================

	UFUNCTION(BlueprintCallable, Category = "Testing|Manual")
	void RunAllTests();

	UFUNCTION(BlueprintCallable, Category = "Testing|Manual")
	void Test1_VersionCheck();

	UFUNCTION(BlueprintCallable, Category = "Testing|Manual")
	void Test2_FloorGeneration();

	UFUNCTION(BlueprintCallable, Category = "Testing|Manual")
	void Test3_CombatCalculation();

	UFUNCTION(BlueprintCallable, Category = "Testing|Manual")
	void Test4_HotReload();

	UFUNCTION(BlueprintCallable, Category = "Testing|Manual")
	void Test5_Analytics();

	UFUNCTION(BlueprintCallable, Category = "Testing|Manual")
	void Test6_MonsterGeneration();

	// ============================================================
	// Test Results
	// ============================================================

	UPROPERTY(BlueprintReadOnly, Category = "Testing|Results")
	int32 TestsPassed = 0;

	UPROPERTY(BlueprintReadOnly, Category = "Testing|Results")
	int32 TestsFailed = 0;

	UPROPERTY(BlueprintReadOnly, Category = "Testing|Results")
	int32 TestsTotal = 6;

	UPROPERTY(BlueprintReadOnly, Category = "Testing|Results")
	TArray<FString> FailureMessages;

private:
	// Internal helpers
	UTowerGameSubsystem* GetTowerSubsystem();
	void LogTestStart(const FString& TestName);
	void LogTestPass(const FString& TestName);
	void LogTestFail(const FString& TestName, const FString& Reason);
	void LogTestResult();

	bool ValidateJSON(const FString& JsonString, const FString& RequiredField);

	int32 CurrentTestIndex = 0;
	FTimerHandle TestDelayTimer;

	void RunNextTest();
	void OnAllTestsComplete();
};
