#!/bin/bash
# Stress test runner for Tower Game networking
#
# Usage:
#   ./run_stress_test.sh              # Default: 10 clients, 60s
#   ./run_stress_test.sh 20 120       # 20 clients, 120s

set -e

# Configuration
CLIENTS=${1:-10}
DURATION=${2:-60}
SERVER_LOG="../logs/bevy-server/stress_test.log"

echo "🧪 Tower Game Stress Test"
echo "=========================="
echo "Clients: $CLIENTS"
echo "Duration: ${DURATION}s"
echo ""

# Check if server is already running
if pgrep -x "tower-bevy-server" > /dev/null; then
    echo "⚠️  Server already running. Using existing instance."
else
    echo "🚀 Starting Bevy server..."
    cd ../bevy-server
    cargo build --release --quiet 2>&1 | grep -E "(Compiling|Finished)" || true
    ./target/release/tower-bevy-server > "$SERVER_LOG" 2>&1 &
    SERVER_PID=$!
    echo "   Server PID: $SERVER_PID"

    # Wait for server to start
    echo "   Waiting for server to initialize..."
    sleep 3
fi

echo ""
echo "🧪 Building stress test client..."
cd ../bevy-test-client
cargo build --release --bin stress_test --quiet 2>&1 | grep -E "(Compiling|Finished)" || true

echo ""
echo "🚀 Launching $CLIENTS clients..."
START_TIME=$(date +%s)

# Run stress test
./target/release/stress_test --clients $CLIENTS --duration $DURATION

END_TIME=$(date +%s)
ELAPSED=$((END_TIME - START_TIME))

echo ""
echo "✅ Stress test completed in ${ELAPSED}s"
echo ""
echo "📊 Results:"
echo "─────────────────────────────"

# Analyze server logs
cd ..
if [ -f "$SERVER_LOG" ]; then
    echo "Server logs:"
    grep -E "(Performance|capacity|Player|connected)" "$SERVER_LOG" | tail -20
    echo ""

    # Count unique players
    PLAYER_COUNT=$(grep "Player.*connected" "$SERVER_LOG" | wc -l)
    echo "Total player connections: $PLAYER_COUNT"

    # Check for errors
    ERROR_COUNT=$(grep -i "error" "$SERVER_LOG" | wc -l)
    if [ $ERROR_COUNT -gt 0 ]; then
        echo "⚠️  Errors found: $ERROR_COUNT"
        grep -i "error" "$SERVER_LOG" | tail -5
    else
        echo "✅ No errors detected"
    fi
fi

echo ""
echo "📁 Server log saved to: $SERVER_LOG"
echo ""

# Cleanup
if [ ! -z "$SERVER_PID" ]; then
    echo "🛑 Stopping server (PID: $SERVER_PID)..."
    kill $SERVER_PID 2>/dev/null || true
fi

echo "Done!"
