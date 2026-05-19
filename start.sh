#!/bin/bash
set -e

echo "Building jane-mcp..."
cargo build --release

LOG_DIR="./logs"
mkdir -p "$LOG_DIR"

echo "Starting jane-mcp on port 8081..."
nohup cargo run --release --bin jane-mcp > "$LOG_DIR/jane-mcp.log" 2>&1 &
PID=$!
echo $PID > "$LOG_DIR/jane-mcp.pid"

echo ""
echo "Server is running in background:"
echo "  URL: http://localhost:8081/mcp  (PID: $PID)"
echo ""
echo "Logs: $LOG_DIR/"
echo "Use ./stop.sh to stop the server."
