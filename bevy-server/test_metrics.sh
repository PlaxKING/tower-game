#!/bin/bash
# Test script for 3-tier caching and performance metrics

cd "$(dirname "$0")"

echo "========================================" echo "Testing 3-Tier Caching & Metrics"
echo "========================================"
echo ""

echo "Step 1: Compile bevy-server library..."
cargo check --lib
if [ $? -ne 0 ]; then
    echo "❌ Compilation failed"
    exit 1
fi
echo "✅ Compilation successful"
echo ""

echo "Step 2: Run 3-tier caching test..."
cargo test --lib test_3tier_caching -- --nocapture
if [ $? -ne 0 ]; then
    echo "❌ 3-tier test failed"
    exit 1
fi
echo "✅ 3-tier caching test passed"
echo ""

echo "Step 3: Run performance metrics test..."
cargo test --lib test_performance_metrics -- --nocapture
if [ $? -ne 0 ]; then
    echo "❌ Performance metrics test failed"
    exit 1
fi
echo "✅ Performance metrics test passed"
echo ""

echo "Step 4: Run all async_generation tests..."
cargo test --lib async_generation -- --nocapture
if [ $? -ne 0 ]; then
    echo "❌ Some tests failed"
    exit 1
fi
echo "✅ All tests passed"
echo ""

echo "========================================" echo "✅ All Tests Completed Successfully!"
echo "========================================"
