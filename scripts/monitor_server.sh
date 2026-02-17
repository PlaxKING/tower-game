#!/bin/bash
# Real-time server performance monitor
#
# Usage: ./monitor_server.sh [log_file]

LOG_FILE=${1:-"../logs/bevy-server.log"}
INTERVAL=2  # Update every 2 seconds

echo "📊 Tower Game Server Monitor"
echo "============================="
echo "Log file: $LOG_FILE"
echo "Press Ctrl+C to stop"
echo ""

# Function to extract metric from logs
get_metric() {
    local pattern=$1
    local default=${2:-0}
    tail -100 "$LOG_FILE" 2>/dev/null | grep -oP "$pattern" | tail -1 || echo $default
}

# Main monitoring loop
while true; do
    # Clear screen
    clear

    echo "📊 Tower Game Server Monitor - $(date '+%H:%M:%S')"
    echo "============================================="
    echo ""

    # Check if server is running
    if pgrep -x "tower-bevy-server" > /dev/null; then
        PID=$(pgrep -x "tower-bevy-server")
        echo "🟢 Server Status: RUNNING (PID: $PID)"

        # Get CPU and memory usage
        if command -v ps &> /dev/null; then
            CPU=$(ps -p $PID -o %cpu= 2>/dev/null | tr -d ' ')
            MEM=$(ps -p $PID -o %mem= 2>/dev/null | tr -d ' ')
            echo "   CPU: ${CPU}%"
            echo "   Memory: ${MEM}%"
        fi
    else
        echo "🔴 Server Status: NOT RUNNING"
    fi

    echo ""
    echo "Network:"
    echo "─────────"

    # Extract metrics from logs
    if [ -f "$LOG_FILE" ]; then
        # Player count
        PLAYERS=$(tail -50 "$LOG_FILE" | grep -o "ReplicationManager Stats.*players" | grep -oP '\d+ players' | head -1 | grep -oP '\d+' || echo "0")
        echo "   Connected players: $PLAYERS"

        # Packets
        PACKETS=$(tail -50 "$LOG_FILE" | grep "ReplicationManager Stats" | tail -1 | grep -oP '\d+ packets' | grep -oP '\d+' || echo "0")
        echo "   Packets received: $PACKETS"

        # Capacity
        CAPACITY=$(tail -50 "$LOG_FILE" | grep -oP 'capacity to \d+' | tail -1 | grep -oP '\d+' || echo "100")
        echo "   Current capacity: $CAPACITY"

        echo ""
        echo "Performance:"
        echo "────────────"

        # Recent performance messages
        tail -20 "$LOG_FILE" | grep -E "(Performance|capacity)" | tail -3

        echo ""
        echo "Recent Activity:"
        echo "────────────────"

        # Last few events
        tail -10 "$LOG_FILE" | grep -vE "(VeryVerbose|Verbose)" | tail -5

    else
        echo "   ⚠️  Log file not found: $LOG_FILE"
    fi

    echo ""
    echo "───────────────────────────────────────────"
    echo "Updating every ${INTERVAL}s... (Ctrl+C to stop)"

    sleep $INTERVAL
done
