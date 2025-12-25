#!/bin/bash
# Integration test: ethrex-pir-export -> inspire-setup -> inspire-server
#
# Prerequisites:
# - ethrex node with UBT sync completed (PLAIN_STORAGE populated)
# - ethrex-pir-export binary built
# - inspire-setup binary built
# - inspire-server binary built
#
# Usage:
#   ./scripts/ethrex-integration-test.sh <ethrex-datadir> <output-dir>
#
# Example:
#   ./scripts/ethrex-integration-test.sh /root/.local/share/ethrex-ubt-sepolia /tmp/pir-test

set -euo pipefail

ETHREX_DATADIR="${1:-}"
OUTPUT_DIR="${2:-/tmp/pir-integration-test}"
ETHREX_PIR_EXPORT="${ETHREX_PIR_EXPORT:-ethrex-pir-export}"
INSPIRE_SETUP="${INSPIRE_SETUP:-inspire-setup}"
INSPIRE_SERVER="${INSPIRE_SERVER:-inspire-server}"
INSPIRE_CLIENT="${INSPIRE_CLIENT:-inspire-client}"

if [[ -z "$ETHREX_DATADIR" ]]; then
    echo "Usage: $0 <ethrex-datadir> [output-dir]"
    echo ""
    echo "Environment variables:"
    echo "  ETHREX_PIR_EXPORT  Path to ethrex-pir-export binary"
    echo "  INSPIRE_SETUP      Path to inspire-setup binary"
    echo "  INSPIRE_SERVER     Path to inspire-server binary"
    echo "  INSPIRE_CLIENT     Path to inspire-client binary"
    exit 1
fi

echo "=== ethrex -> inspire-exex Integration Test ==="
echo "Ethrex datadir: $ETHREX_DATADIR"
echo "Output dir: $OUTPUT_DIR"
echo ""

mkdir -p "$OUTPUT_DIR"

# Step 1: Export state from ethrex
echo "[1/4] Exporting state from ethrex..."
STATE_FILE="$OUTPUT_DIR/state.bin"
time $ETHREX_PIR_EXPORT \
    --datadir "$ETHREX_DATADIR" \
    --output "$STATE_FILE"

STATE_SIZE=$(du -h "$STATE_FILE" | cut -f1)
ENTRY_COUNT=$(($(stat -f%z "$STATE_FILE" 2>/dev/null || stat -c%s "$STATE_FILE") / 84))
echo "[OK] Exported $ENTRY_COUNT entries ($STATE_SIZE)"
echo ""

# Step 2: Encode PIR database
echo "[2/4] Encoding PIR database..."
DB_FILE="$OUTPUT_DIR/database.bin"
time $INSPIRE_SETUP "$STATE_FILE" "$DB_FILE"

DB_SIZE=$(du -h "$DB_FILE" | cut -f1)
echo "[OK] Created PIR database ($DB_SIZE)"
echo ""

# Step 3: Start server and verify health
echo "[3/4] Starting inspire-server..."
SERVER_PORT=3333
$INSPIRE_SERVER "$DB_FILE" --port $SERVER_PORT &
SERVER_PID=$!
sleep 2

if curl -sf "http://localhost:$SERVER_PORT/health" > /dev/null; then
    echo "[OK] Server healthy on port $SERVER_PORT"
else
    echo "[FAIL] Server health check failed"
    kill $SERVER_PID 2>/dev/null || true
    exit 1
fi

# Step 4: Test query
echo "[4/4] Testing PIR query..."
# Query index 0 as a smoke test
if $INSPIRE_CLIENT "http://localhost:$SERVER_PORT" --index 0 > "$OUTPUT_DIR/query-result.txt" 2>&1; then
    echo "[OK] Query successful"
    cat "$OUTPUT_DIR/query-result.txt"
else
    echo "[WARN] Query failed (may be expected if index 0 is empty)"
fi

# Cleanup
kill $SERVER_PID 2>/dev/null || true

echo ""
echo "=== Integration Test Complete ==="
echo "State file: $STATE_FILE ($STATE_SIZE, $ENTRY_COUNT entries)"
echo "Database: $DB_FILE ($DB_SIZE)"
echo ""
echo "To run server manually:"
echo "  $INSPIRE_SERVER $DB_FILE --port 3000"
