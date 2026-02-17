# Test script for 3-tier caching and performance metrics

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Testing 3-Tier Caching & Metrics" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "Step 1: Compile bevy-server library..." -ForegroundColor Yellow
$compileResult = cargo check --lib 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Compilation failed" -ForegroundColor Red
    Write-Host $compileResult
    exit 1
}
Write-Host "✅ Compilation successful" -ForegroundColor Green
Write-Host ""

Write-Host "Step 2: Run 3-tier caching test..." -ForegroundColor Yellow
$test1Result = cargo test --lib test_3tier_caching -- --nocapture 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ 3-tier test failed" -ForegroundColor Red
    Write-Host $test1Result
    exit 1
}
Write-Host "✅ 3-tier caching test passed" -ForegroundColor Green
Write-Host $test1Result | Select-String -Pattern "Cache Stats:"
Write-Host ""

Write-Host "Step 3: Run performance metrics test..." -ForegroundColor Yellow
$test2Result = cargo test --lib test_performance_metrics -- --nocapture 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Performance metrics test failed" -ForegroundColor Red
    Write-Host $test2Result
    exit 1
}
Write-Host "✅ Performance metrics test passed" -ForegroundColor Green
Write-Host $test2Result | Select-String -Pattern "Cache Stats:"
Write-Host ""

Write-Host "Step 4: Run all async_generation tests..." -ForegroundColor Yellow
$testAllResult = cargo test --lib async_generation 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Some tests failed" -ForegroundColor Red
    Write-Host $testAllResult
    exit 1
}
Write-Host "✅ All tests passed" -ForegroundColor Green
Write-Host ""

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "✅ All Tests Completed Successfully!" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
