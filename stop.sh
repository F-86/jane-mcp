#!/bin/bash

LOG_DIR="./logs"
PID_FILE="$LOG_DIR/jane-mcp.pid"

if [ -f "$PID_FILE" ]; then
    PID=$(cat "$PID_FILE")
    if kill -0 "$PID" 2>/dev/null; then
        echo "Stopping jane-mcp (PID: $PID)..."
        kill "$PID"
        rm -f "$PID_FILE"
    else
        echo "jane-mcp is not running."
        rm -f "$PID_FILE"
    fi
else
    echo "jane-mcp PID file not found."
fi

echo "Done."
