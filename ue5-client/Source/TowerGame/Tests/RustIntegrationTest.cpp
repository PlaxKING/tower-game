// Copyright Tower Game 2026. All Rights Reserved.

#include "RustIntegrationTest.h"
#include "Json.h"
#include "JsonUtilities.h"
#include "TimerManager.h"

ARustIntegrationTest::ARustIntegrationTest()
{
	PrimaryActorTick.bCanEverTick = false;
}

void ARustIntegrationTest::BeginPlay()
{
	Super::BeginPlay();

	if (bAutoRunTests)
	{
		UE_LOG(LogTemp, Display, TEXT("========================================"));
		UE_LOG(LogTemp, Display, TEXT("🧪 Rust Integration Test Suite v0.6.0"));
		UE_LOG(LogTemp, Display, TEXT("========================================"));

		// Start tests after a small delay
		GetWorld()->GetTimerManager().SetTimer(
			TestDelayTimer,
			this,
			&ARustIntegrationTest::RunAllTests,
			0.5f,
			false
		);
	}
}

// ============================================================
// Test Execution
// ============================================================

void ARustIntegrationTest::RunAllTests()
{
	TestsPassed = 0;
	TestsFailed = 0;
	FailureMessages.Empty();
	CurrentTestIndex = 0;

	RunNextTest();
}

void ARustIntegrationTest::RunNextTest()
{
	switch (CurrentTestIndex)
	{
	case 0:
		Test1_VersionCheck();
		break;
	case 1:
		Test2_FloorGeneration();
		break;
	case 2:
		Test3_CombatCalculation();
		break;
	case 3:
		Test4_HotReload();
		break;
	case 4:
		Test5_Analytics();
		break;
	case 5:
		Test6_MonsterGeneration();
		break;
	default:
		OnAllTestsComplete();
		return;
	}

	CurrentTestIndex++;

	// Schedule next test
	if (CurrentTestIndex < TestsTotal && !(bStopOnFailure && TestsFailed > 0))
	{
		GetWorld()->GetTimerManager().SetTimer(
			TestDelayTimer,
			this,
			&ARustIntegrationTest::RunNextTest,
			DelayBetweenTests,
			false
		);
	}
	else
	{
		OnAllTestsComplete();
	}
}

void ARustIntegrationTest::OnAllTestsComplete()
{
	LogTestResult();
}

// ============================================================
// Individual Tests
// ============================================================

void ARustIntegrationTest::Test1_VersionCheck()
{
	LogTestStart(TEXT("Version Check"));

	UTowerGameSubsystem* TowerSys = GetTowerSubsystem();
	if (!TowerSys)
	{
		LogTestFail(TEXT("Version Check"), TEXT("TowerGameSubsystem not found"));
		return;
	}

	if (!TowerSys->IsRustCoreReady())
	{
		LogTestFail(TEXT("Version Check"), TEXT("Rust Core not initialized"));
		return;
	}

	FString Version = TowerSys->GetCoreVersion();
	if (Version.IsEmpty())
	{
		LogTestFail(TEXT("Version Check"), TEXT("Version string is empty"));
		return;
	}

	if (Version != TEXT("0.6.0"))
	{
		LogTestFail(TEXT("Version Check"), FString::Printf(TEXT("Expected '0.6.0', got '%s'"), *Version));
		return;
	}

	LogTestPass(TEXT("Version Check"));
}

void ARustIntegrationTest::Test2_FloorGeneration()
{
	LogTestStart(TEXT("Floor Generation"));

	UTowerGameSubsystem* TowerSys = GetTowerSubsystem();
	if (!TowerSys || !TowerSys->IsRustCoreReady())
	{
		LogTestFail(TEXT("Floor Generation"), TEXT("Subsystem not ready"));
		return;
	}

	FString FloorJson = TowerSys->RequestFloorLayout(42, 5);
	if (FloorJson.IsEmpty())
	{
		LogTestFail(TEXT("Floor Generation"), TEXT("Returned empty JSON"));
		return;
	}

	if (!ValidateJSON(FloorJson, TEXT("floor_id")))
	{
		LogTestFail(TEXT("Floor Generation"), TEXT("Invalid JSON or missing 'floor_id'"));
		return;
	}

	// Parse and validate floor_id
	TSharedPtr<FJsonObject> JsonObject;
	TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(FloorJson);
	if (FJsonSerializer::Deserialize(Reader, JsonObject) && JsonObject.IsValid())
	{
		int32 FloorId = JsonObject->GetIntegerField(TEXT("floor_id"));
		if (FloorId != 5)
		{
			LogTestFail(TEXT("Floor Generation"), FString::Printf(TEXT("Expected floor_id=5, got %d"), FloorId));
			return;
		}
	}

	LogTestPass(TEXT("Floor Generation"));
}

void ARustIntegrationTest::Test3_CombatCalculation()
{
	LogTestStart(TEXT("Combat Calculation"));

	UTowerGameSubsystem* TowerSys = GetTowerSubsystem();
	if (!TowerSys || !TowerSys->IsRustCoreReady())
	{
		LogTestFail(TEXT("Combat Calculation"), TEXT("Subsystem not ready"));
		return;
	}

	// CalculateDamage signature: (float BaseDamage, int32 AngleId, int32 ComboStep)
	// Returns float damage value directly, not JSON
	float DamageResult = TowerSys->CalculateDamage(
		100.0f,  // Base damage
		2,       // Angle ID (quantized angle, e.g., 45° = ID 2)
		0        // Combo step
	);

	// Validate damage result (should be modified by angle multiplier)
	if (DamageResult < 80.0f || DamageResult > 200.0f)
	{
		LogTestFail(TEXT("Combat Calculation"), FString::Printf(TEXT("Unexpected damage result: %.2f (expected 80-200)"), DamageResult));
		return;
	}

	LogTestPass(TEXT("Combat Calculation"));
}

void ARustIntegrationTest::Test4_HotReload()
{
	LogTestStart(TEXT("Hot-Reload System"));

	UTowerGameSubsystem* TowerSys = GetTowerSubsystem();
	if (!TowerSys || !TowerSys->IsRustCoreReady())
	{
		LogTestFail(TEXT("Hot-Reload System"), TEXT("Subsystem not ready"));
		return;
	}

	FString Status = TowerSys->GetHotReloadStatus();
	if (Status.IsEmpty())
	{
		LogTestFail(TEXT("Hot-Reload System"), TEXT("Status is empty"));
		return;
	}

	if (!ValidateJSON(Status, TEXT("enabled")))
	{
		LogTestFail(TEXT("Hot-Reload System"), TEXT("Invalid status JSON"));
		return;
	}

	int32 ReloadCount = TowerSys->TriggerConfigReload();
	if (ReloadCount < 0)
	{
		LogTestFail(TEXT("Hot-Reload System"), TEXT("Reload returned negative count"));
		return;
	}

	LogTestPass(TEXT("Hot-Reload System"));
	UE_LOG(LogTemp, Display, TEXT("  → Reloaded %d configs"), ReloadCount);
}

void ARustIntegrationTest::Test5_Analytics()
{
	LogTestStart(TEXT("Analytics System"));

	UTowerGameSubsystem* TowerSys = GetTowerSubsystem();
	if (!TowerSys || !TowerSys->IsRustCoreReady())
	{
		LogTestFail(TEXT("Analytics System"), TEXT("Subsystem not ready"));
		return;
	}

	// Reset analytics first
	TowerSys->ResetAnalytics();

	// Record test events
	TowerSys->RecordDamageDealt(TEXT("TestSword"), 100);
	TowerSys->RecordFloorCleared(1, 1, 60.0f);
	TowerSys->RecordGoldEarned(500);

	// Get snapshot
	FString Snapshot = TowerSys->GetAnalyticsSnapshot();
	if (Snapshot.IsEmpty())
	{
		LogTestFail(TEXT("Analytics System"), TEXT("Snapshot is empty"));
		return;
	}

	if (!ValidateJSON(Snapshot, TEXT("total_events")))
	{
		LogTestFail(TEXT("Analytics System"), TEXT("Invalid snapshot JSON"));
		return;
	}

	// Verify event count
	TSharedPtr<FJsonObject> JsonObject;
	TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(Snapshot);
	if (FJsonSerializer::Deserialize(Reader, JsonObject) && JsonObject.IsValid())
	{
		int32 TotalEvents = JsonObject->GetIntegerField(TEXT("total_events"));
		if (TotalEvents != 3)
		{
			LogTestFail(TEXT("Analytics System"), FString::Printf(TEXT("Expected 3 events, got %d"), TotalEvents));
			return;
		}
	}

	LogTestPass(TEXT("Analytics System"));
}

void ARustIntegrationTest::Test6_MonsterGeneration()
{
	LogTestStart(TEXT("Monster Generation"));

	UTowerGameSubsystem* TowerSys = GetTowerSubsystem();
	if (!TowerSys || !TowerSys->IsRustCoreReady())
	{
		LogTestFail(TEXT("Monster Generation"), TEXT("Subsystem not ready"));
		return;
	}

	// RequestFloorMonsters signature: (int64 Seed, int32 FloorId, int32 Count)
	FString MonstersJson = TowerSys->RequestFloorMonsters(42, 5, 10);
	if (MonstersJson.IsEmpty())
	{
		LogTestFail(TEXT("Monster Generation"), TEXT("Returned empty JSON"));
		return;
	}

	if (!ValidateJSON(MonstersJson, TEXT("monsters")))
	{
		LogTestFail(TEXT("Monster Generation"), TEXT("Invalid JSON or missing 'monsters'"));
		return;
	}

	LogTestPass(TEXT("Monster Generation"));
}

// ============================================================
// Helper Functions
// ============================================================

UTowerGameSubsystem* ARustIntegrationTest::GetTowerSubsystem()
{
	if (!GetGameInstance())
	{
		return nullptr;
	}

	return GetGameInstance()->GetSubsystem<UTowerGameSubsystem>();
}

void ARustIntegrationTest::LogTestStart(const FString& TestName)
{
	UE_LOG(LogTemp, Display, TEXT(""));
	UE_LOG(LogTemp, Display, TEXT("🔹 Running: %s"), *TestName);
}

void ARustIntegrationTest::LogTestPass(const FString& TestName)
{
	TestsPassed++;
	UE_LOG(LogTemp, Display, TEXT("✅ PASS: %s"), *TestName);
}

void ARustIntegrationTest::LogTestFail(const FString& TestName, const FString& Reason)
{
	TestsFailed++;
	FailureMessages.Add(FString::Printf(TEXT("%s: %s"), *TestName, *Reason));
	UE_LOG(LogTemp, Error, TEXT("❌ FAIL: %s"), *TestName);
	UE_LOG(LogTemp, Error, TEXT("  Reason: %s"), *Reason);
}

void ARustIntegrationTest::LogTestResult()
{
	UE_LOG(LogTemp, Display, TEXT(""));
	UE_LOG(LogTemp, Display, TEXT("========================================"));
	UE_LOG(LogTemp, Display, TEXT("📊 Test Results"));
	UE_LOG(LogTemp, Display, TEXT("========================================"));
	UE_LOG(LogTemp, Display, TEXT("Total:  %d"), TestsTotal);
	UE_LOG(LogTemp, Display, TEXT("Passed: %d ✅"), TestsPassed);
	UE_LOG(LogTemp, Display, TEXT("Failed: %d ❌"), TestsFailed);

	if (TestsFailed > 0)
	{
		UE_LOG(LogTemp, Display, TEXT(""));
		UE_LOG(LogTemp, Display, TEXT("Failed tests:"));
		for (const FString& Msg : FailureMessages)
		{
			UE_LOG(LogTemp, Display, TEXT("  - %s"), *Msg);
		}
	}

	UE_LOG(LogTemp, Display, TEXT("========================================"));

	if (TestsFailed == 0)
	{
		UE_LOG(LogTemp, Display, TEXT("🎉 All tests passed! Integration working perfectly."));
	}
	else
	{
		UE_LOG(LogTemp, Warning, TEXT("⚠️  Some tests failed. Check errors above."));
	}
}

bool ARustIntegrationTest::ValidateJSON(const FString& JsonString, const FString& RequiredField)
{
	TSharedPtr<FJsonObject> JsonObject;
	TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(JsonString);

	if (!FJsonSerializer::Deserialize(Reader, JsonObject) || !JsonObject.IsValid())
	{
		return false;
	}

	if (!RequiredField.IsEmpty() && !JsonObject->HasField(RequiredField))
	{
		return false;
	}

	return true;
}
