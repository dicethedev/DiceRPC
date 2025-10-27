#!/bin/bash

# Monitor DiceRPC metrics
while true; do
    clear
    echo "╔══════════════════════════════════════╗"
    echo "║     DiceRPC Metrics Monitor          ║"
    echo "╚══════════════════════════════════════╝"
    echo ""
    curl -s http://127.0.0.1:3000/metrics | jq
    echo ""
    echo "Refreshing in 5 seconds... (Ctrl+C to stop)"
    sleep 5
done

chmod +x scripts/monitor_metrics.sh