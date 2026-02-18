#!/usr/bin/env bash
# test_cloud_e2e.sh — End-to-end integration tests for vehicle-to-cloud connectivity.
#
# Exercises the full command flow:
#   COMPANION_APP CLI → CLOUD_GATEWAY → MQTT → CLOUD_GATEWAY_CLIENT → DATA_BROKER → LOCKING_SERVICE
#   and the reverse path for responses and telemetry.
#
# Tests:
#   1. Pairing flow (03-REQ-7.1)
#   2. Lock command end-to-end (03-REQ-7.2, 03-REQ-7.3, 03-REQ-7.4)
#   3. Telemetry flow (03-REQ-7.5)
#   4. Rejection propagation (03-REQ-7.2, 03-REQ-7.3)
#   5. Skip behavior when infrastructure unavailable (03-REQ-7.E1)
#
# Prerequisites:
#   - `make infra-up` running (Kuksa Databroker + Mosquitto)
#   - Go and Rust toolchains available
#
# Usage:
#   ./tests/test_cloud_e2e.sh
#
# Environment variables:
#   MQTT_ADDR         MQTT broker address (default: localhost:1883)
#   DATABROKER_ADDR   Kuksa Databroker address (default: http://localhost:55555)
#   GATEWAY_PORT      Port for CLOUD_GATEWAY REST API (default: 18081)

set -euo pipefail

# ─── Resolve paths ────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ─── Terminal colours ─────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m'

passed=0
failed=0
skipped=0

pass() {
    printf "  ${GREEN}PASS${NC} %s\n" "$1"
    passed=$((passed + 1))
}

fail() {
    printf "  ${RED}FAIL${NC} %s — %s\n" "$1" "${2:-}"
    failed=$((failed + 1))
}

skip() {
    printf "  ${YELLOW}SKIP${NC} %s — %s\n" "$1" "${2:-}"
    skipped=$((skipped + 1))
}

info() {
    printf "  ${CYAN}INFO${NC} %s\n" "$1"
}

# http_status: make an HTTP request and return just the status code.
# Usage: http_status METHOD URL [HEADERS...] [--data BODY]
http_status() {
    curl -s -o /dev/null -w "%{http_code}" "$@" 2>/dev/null || echo "000"
}

# http_body: make an HTTP request and return the response body.
# Usage: http_body METHOD URL [HEADERS...] [--data BODY]
http_body() {
    curl -s "$@" 2>/dev/null || echo ""
}

# json_get: extract a top-level field from JSON using python3.
# Returns the raw JSON value (true/false for bools, not Python True/False).
# Usage: echo '{"key":"val"}' | json_get key
json_get() {
    python3 -c "
import sys, json
d = json.load(sys.stdin)
v = d.get('$1')
if v is None:
    print('')
elif isinstance(v, bool):
    print('true' if v else 'false')
else:
    print(v)
" 2>/dev/null || echo ""
}

# json_nested: extract a nested field from JSON using python3.
# Returns the raw JSON value (true/false for bools).
# Usage: echo '{"a":{"b":"val"}}' | json_nested a b
json_nested() {
    python3 -c "
import sys, json
d = json.load(sys.stdin)
v = d.get('$1', {}).get('$2')
if v is None:
    print('')
elif isinstance(v, bool):
    print('true' if v else 'false')
else:
    print(v)
" 2>/dev/null || echo ""
}

# ─── Configuration ────────────────────────────────────────────────────────────
MQTT_ADDR="${MQTT_ADDR:-localhost:1883}"
DATABROKER_ADDR="${DATABROKER_ADDR:-http://localhost:55555}"
GATEWAY_PORT="${GATEWAY_PORT:-18081}"
GATEWAY_URL="http://localhost:${GATEWAY_PORT}"
TELEMETRY_INTERVAL=2  # Seconds
CGC_DATA_DIR=""
TOKEN=""
PIDS_TO_KILL=()

# ─── Cleanup ──────────────────────────────────────────────────────────────────
cleanup() {
    info "Cleaning up..."
    if [ ${#PIDS_TO_KILL[@]} -gt 0 ]; then
        for pid in "${PIDS_TO_KILL[@]}"; do
            if kill -0 "$pid" 2>/dev/null; then
                kill "$pid" 2>/dev/null || true
                wait "$pid" 2>/dev/null || true
            fi
        done
    fi
    if [ -n "$CGC_DATA_DIR" ] && [ -d "$CGC_DATA_DIR" ]; then
        rm -rf "$CGC_DATA_DIR"
    fi
    # Clean up log files
    rm -f /tmp/cloud-gateway-e2e.log /tmp/cloud-gateway-client-e2e.log /tmp/locking-service-e2e.log
}
trap cleanup EXIT

# ─── Infrastructure check (03-REQ-7.E1) ──────────────────────────────────────
printf "\n=== Cloud Connectivity E2E Integration Tests ===\n\n"
printf "Configuration:\n"
printf "  MQTT broker:   %s\n" "$MQTT_ADDR"
printf "  Databroker:    %s\n" "$DATABROKER_ADDR"
printf "  Gateway:       %s\n\n" "$GATEWAY_URL"

# Check MQTT broker availability
check_mqtt() {
    local host port
    host="${MQTT_ADDR%%:*}"
    port="${MQTT_ADDR##*:}"
    if command -v nc >/dev/null 2>&1; then
        nc -z -w2 "$host" "$port" 2>/dev/null
    else
        (echo > /dev/tcp/"$host"/"$port") 2>/dev/null
    fi
}

# Check Kuksa Databroker availability via gRPC
check_kuksa() {
    local addr="${DATABROKER_ADDR#http://}"
    local host="${addr%%:*}"
    local port="${addr##*:}"
    if command -v nc >/dev/null 2>&1; then
        nc -z -w2 "$host" "$port" 2>/dev/null
    else
        (echo > /dev/tcp/"$host"/"$port") 2>/dev/null
    fi
}

if ! check_mqtt; then
    printf "${YELLOW}MQTT broker not available at %s${NC}\n" "$MQTT_ADDR"
    printf "Run 'make infra-up' to start required infrastructure.\n"
    skip "All tests" "MQTT broker unavailable (03-REQ-7.E1)"
    printf "\n=== Results: %d passed, %d failed, %d skipped ===\n" "$passed" "$failed" "$skipped"
    exit 0
fi

if ! check_kuksa; then
    printf "${YELLOW}Kuksa Databroker not available at %s${NC}\n" "$DATABROKER_ADDR"
    printf "Run 'make infra-up' to start required infrastructure.\n"
    skip "All tests" "Kuksa Databroker unavailable (03-REQ-7.E1)"
    printf "\n=== Results: %d passed, %d failed, %d skipped ===\n" "$passed" "$failed" "$skipped"
    exit 0
fi

pass "Infrastructure check — MQTT and Kuksa available (03-REQ-7.E1)"

# ─── Build services ──────────────────────────────────────────────────────────
printf "\n--- Building services ---\n"

info "Building Go services..."
if ! (cd "$PROJECT_ROOT/backend/cloud-gateway" && go build -o /tmp/cloud-gateway-e2e .) 2>/dev/null; then
    fail "Build cloud-gateway" "go build failed"
    printf "\n=== Results: %d passed, %d failed, %d skipped ===\n" "$passed" "$failed" "$skipped"
    exit 1
fi

info "Building companion-app-cli..."
if ! (cd "$PROJECT_ROOT/mock/companion-app-cli" && go build -o /tmp/companion-app-cli-e2e .) 2>/dev/null; then
    fail "Build companion-app-cli" "go build failed"
    printf "\n=== Results: %d passed, %d failed, %d skipped ===\n" "$passed" "$failed" "$skipped"
    exit 1
fi

info "Building Rust services..."
if ! (cd "$PROJECT_ROOT/rhivos" && cargo build --release -p cloud-gateway-client -p locking-service -p mock-sensors 2>/dev/null); then
    fail "Build Rust services" "cargo build failed"
    printf "\n=== Results: %d passed, %d failed, %d skipped ===\n" "$passed" "$failed" "$skipped"
    exit 1
fi

CLOUD_GATEWAY_BIN="/tmp/cloud-gateway-e2e"
COMPANION_CLI_BIN="/tmp/companion-app-cli-e2e"
CGC_BIN="$PROJECT_ROOT/rhivos/target/release/cloud-gateway-client"
LOCKING_BIN="$PROJECT_ROOT/rhivos/target/release/locking-service"
MOCK_SENSORS_BIN="$PROJECT_ROOT/rhivos/target/release/mock-sensors"

pass "Build all services"

# ─── Start services ──────────────────────────────────────────────────────────
printf "\n--- Starting services ---\n"

# Create temp data directory for cloud-gateway-client VIN persistence
CGC_DATA_DIR="$(mktemp -d /tmp/cgc-data-e2e.XXXXXX)"

# Start LOCKING_SERVICE
info "Starting LOCKING_SERVICE..."
"$LOCKING_BIN" \
    --databroker-addr "$DATABROKER_ADDR" \
    > /tmp/locking-service-e2e.log 2>&1 &
LOCKING_PID=$!
PIDS_TO_KILL+=("$LOCKING_PID")
sleep 1

if ! kill -0 "$LOCKING_PID" 2>/dev/null; then
    fail "Start LOCKING_SERVICE" "process died immediately"
    printf "  Logs:\n"
    cat /tmp/locking-service-e2e.log 2>/dev/null | head -10 || true
    printf "\n=== Results: %d passed, %d failed, %d skipped ===\n" "$passed" "$failed" "$skipped"
    exit 1
fi
pass "Start LOCKING_SERVICE (PID $LOCKING_PID)"

# Start CLOUD_GATEWAY
info "Starting CLOUD_GATEWAY..."
"$CLOUD_GATEWAY_BIN" \
    --listen-addr ":${GATEWAY_PORT}" \
    --mqtt-addr "$MQTT_ADDR" \
    > /tmp/cloud-gateway-e2e.log 2>&1 &
GW_PID=$!
PIDS_TO_KILL+=("$GW_PID")
sleep 1

if ! kill -0 "$GW_PID" 2>/dev/null; then
    fail "Start CLOUD_GATEWAY" "process died immediately"
    printf "  Logs:\n"
    cat /tmp/cloud-gateway-e2e.log 2>/dev/null | head -10 || true
    printf "\n=== Results: %d passed, %d failed, %d skipped ===\n" "$passed" "$failed" "$skipped"
    exit 1
fi

# Wait for health check
HEALTHZ_OK=false
for _ in $(seq 1 20); do
    if curl -s "${GATEWAY_URL}/healthz" >/dev/null 2>&1; then
        HEALTHZ_OK=true
        break
    fi
    sleep 0.5
done

if [ "$HEALTHZ_OK" = false ]; then
    fail "CLOUD_GATEWAY health check" "healthz not responding after 10s"
    printf "\n=== Results: %d passed, %d failed, %d skipped ===\n" "$passed" "$failed" "$skipped"
    exit 1
fi
pass "Start CLOUD_GATEWAY (PID $GW_PID, healthz OK)"

# Allow time for MQTT subscriptions to be acknowledged.
# The GW subscribes asynchronously after connecting; healthz may respond before
# subscriptions are confirmed by the broker. Wait to ensure the GW is fully
# subscribed before starting CGC, which publishes a non-retained registration
# message that would be lost if the GW hasn't subscribed yet.
sleep 2

# Start CLOUD_GATEWAY_CLIENT
info "Starting CLOUD_GATEWAY_CLIENT..."
"$CGC_BIN" \
    --mqtt-addr "$MQTT_ADDR" \
    --databroker-addr "$DATABROKER_ADDR" \
    --data-dir "$CGC_DATA_DIR" \
    --telemetry-interval "$TELEMETRY_INTERVAL" \
    > /tmp/cloud-gateway-client-e2e.log 2>&1 &
CGC_PID=$!
PIDS_TO_KILL+=("$CGC_PID")

# Wait for CGC to start up and create vin.json
info "Waiting for CLOUD_GATEWAY_CLIENT to start..."
sleep 3

if ! kill -0 "$CGC_PID" 2>/dev/null; then
    fail "Start CLOUD_GATEWAY_CLIENT" "process died"
    printf "  Logs:\n"
    head -20 /tmp/cloud-gateway-client-e2e.log 2>/dev/null || true
    printf "\n=== Results: %d passed, %d failed, %d skipped ===\n" "$passed" "$failed" "$skipped"
    exit 1
fi
pass "Start CLOUD_GATEWAY_CLIENT (PID $CGC_PID)"

# ─── Extract VIN and PIN from CGC data directory ─────────────────────────────
VIN_FILE="$CGC_DATA_DIR/vin.json"
if [ ! -f "$VIN_FILE" ]; then
    fail "VIN file creation" "vin.json not found in $CGC_DATA_DIR"
    printf "\n=== Results: %d passed, %d failed, %d skipped ===\n" "$passed" "$failed" "$skipped"
    exit 1
fi

VIN=$(python3 -c "import json; d=json.load(open('$VIN_FILE')); print(d['vin'])" 2>/dev/null || echo "")
PIN=$(python3 -c "import json; d=json.load(open('$VIN_FILE')); print(d['pairing_pin'])" 2>/dev/null || echo "")

if [ -z "$VIN" ] || [ -z "$PIN" ]; then
    fail "Parse VIN/PIN" "could not extract VIN and PIN from $VIN_FILE"
    printf "  File contents: %s\n" "$(cat "$VIN_FILE")"
    printf "\n=== Results: %d passed, %d failed, %d skipped ===\n" "$passed" "$failed" "$skipped"
    exit 1
fi

info "Vehicle VIN: $VIN"
info "Pairing PIN: $PIN"
pass "VIN and PIN extracted from vin.json"

# ─── Set initial safe conditions via mock-sensors ─────────────────────────────
printf "\n--- Setting initial safe conditions ---\n"

"$MOCK_SENSORS_BIN" --databroker-addr "$DATABROKER_ADDR" set-speed 0 >/dev/null 2>&1
"$MOCK_SENSORS_BIN" --databroker-addr "$DATABROKER_ADDR" set-door closed >/dev/null 2>&1
"$MOCK_SENSORS_BIN" --databroker-addr "$DATABROKER_ADDR" set-location 48.1351 11.5820 >/dev/null 2>&1
pass "Set initial safe conditions (speed=0, door=closed, location=Munich)"

# Wait for registration message to propagate via MQTT to CLOUD_GATEWAY.
# The CGC publishes a registration message on startup (QoS 2, non-retained).
# The GW subscribes asynchronously, so we need to wait until it receives the
# registration before pairing can succeed.
info "Waiting for registration to propagate to CLOUD_GATEWAY..."
REGISTRATION_OK=false
for attempt in $(seq 1 20); do
    REG_CHECK=$(http_status -X POST "${GATEWAY_URL}/api/v1/pair" \
        -H "Content-Type: application/json" \
        -d "{\"vin\":\"${VIN}\",\"pin\":\"000000\"}")
    if [ "$REG_CHECK" = "403" ]; then
        REGISTRATION_OK=true
        info "Registration received after ~${attempt}s"
        break
    fi
    sleep 1
done

if [ "$REGISTRATION_OK" = true ]; then
    pass "CLOUD_GATEWAY received vehicle registration"
else
    fail "CLOUD_GATEWAY received vehicle registration" "timed out after 20s (last status: $REG_CHECK)"
    info "CGC log tail:"
    tail -10 /tmp/cloud-gateway-client-e2e.log 2>/dev/null || true
    info "GW log tail:"
    tail -10 /tmp/cloud-gateway-e2e.log 2>/dev/null || true
fi

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 10.2: Pairing Flow
# Requirements: 03-REQ-7.1
# Property 4: Pairing Authorization
# ═══════════════════════════════════════════════════════════════════════════════
printf "\n--- Test 10.2: Pairing Flow (03-REQ-7.1) ---\n"

# Test: wrong PIN → 403 Forbidden (registration check already done above)
WRONG_PIN_STATUS=$(http_status -X POST "${GATEWAY_URL}/api/v1/pair" \
    -H "Content-Type: application/json" \
    -d "{\"vin\":\"${VIN}\",\"pin\":\"000000\"}")
if [ "$WRONG_PIN_STATUS" = "403" ]; then
    pass "Wrong PIN returns 403 Forbidden"
else
    fail "Wrong PIN returns 403 Forbidden" "got: $WRONG_PIN_STATUS"
fi

# Test: pair with correct VIN and PIN → token
PAIR_RESP=$(http_body -X POST "${GATEWAY_URL}/api/v1/pair" \
    -H "Content-Type: application/json" \
    -d "{\"vin\":\"${VIN}\",\"pin\":\"${PIN}\"}")

TOKEN=$(echo "$PAIR_RESP" | json_get token)
PAIR_VIN=$(echo "$PAIR_RESP" | json_get vin)

if [ -n "$TOKEN" ] && [ "$PAIR_VIN" = "$VIN" ]; then
    pass "Pair with correct VIN and PIN → token received (03-REQ-7.1)"
    info "Token: ${TOKEN:0:20}..."
else
    fail "Pair with correct VIN and PIN" "response: $PAIR_RESP"
    TOKEN=""
fi

# Test: unknown VIN → 404
UNKNOWN_VIN_STATUS=$(http_status -X POST "${GATEWAY_URL}/api/v1/pair" \
    -H "Content-Type: application/json" \
    -d '{"vin":"UNKNOWN_VIN_12345","pin":"000000"}')
if [ "$UNKNOWN_VIN_STATUS" = "404" ]; then
    pass "Unknown VIN returns 404 Not Found"
else
    fail "Unknown VIN returns 404 Not Found" "got: $UNKNOWN_VIN_STATUS"
fi

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 10.3: Lock Command End-to-End
# Requirements: 03-REQ-7.2, 03-REQ-7.3, 03-REQ-7.4
# Property 1: Command Delivery
# Property 2: Result Propagation
# ═══════════════════════════════════════════════════════════════════════════════
printf "\n--- Test 10.3: Lock Command E2E (03-REQ-7.2, 03-REQ-7.3, 03-REQ-7.4) ---\n"

if [ -z "$TOKEN" ]; then
    skip "Lock command E2E" "no token available (pairing failed)"
else
    # Ensure safe conditions for lock
    "$MOCK_SENSORS_BIN" --databroker-addr "$DATABROKER_ADDR" set-speed 0 >/dev/null 2>&1
    "$MOCK_SENSORS_BIN" --databroker-addr "$DATABROKER_ADDR" set-door closed >/dev/null 2>&1
    sleep 1

    # Send lock command via REST — capture both status code and body in one call
    LOCK_HTTP_CODE=""
    LOCK_RESP=""
    LOCK_RAW=$(curl -s -w "\n%{http_code}" -X POST "${GATEWAY_URL}/api/v1/vehicles/${VIN}/lock" \
        -H "Authorization: Bearer ${TOKEN}" 2>/dev/null || echo -e "\n000")
    LOCK_HTTP_CODE=$(echo "$LOCK_RAW" | tail -1)
    LOCK_RESP=$(echo "$LOCK_RAW" | sed '$d')

    if [ "$LOCK_HTTP_CODE" = "202" ]; then
        pass "Lock command returns 202 Accepted"
    else
        fail "Lock command returns 202 Accepted" "got: $LOCK_HTTP_CODE"
    fi

    # Extract command_id
    COMMAND_ID=$(echo "$LOCK_RESP" | json_get command_id)
    if [ -n "$COMMAND_ID" ]; then
        pass "Lock response contains command_id"
        info "Command ID: ${COMMAND_ID}"
    else
        fail "Lock response contains command_id" "response: $LOCK_RESP"
    fi

    # Wait for command to propagate through MQTT → CGC → Kuksa → LOCKING_SERVICE → response
    info "Waiting for command to propagate through full pipeline..."
    sleep 6

    # Verify vehicle state via GET /status
    STATUS_RESP=$(http_body -X GET "${GATEWAY_URL}/api/v1/vehicles/${VIN}/status" \
        -H "Authorization: Bearer ${TOKEN}")

    # Check is_locked = true (03-REQ-7.2: lock results in IsLocked=true in DATA_BROKER)
    IS_LOCKED=$(echo "$STATUS_RESP" | json_get is_locked)
    if [ "$IS_LOCKED" = "true" ]; then
        pass "GET /status shows is_locked=true after lock (03-REQ-7.2, 03-REQ-7.4)"
    else
        fail "GET /status shows is_locked=true after lock" "is_locked=$IS_LOCKED"
    fi

    # Check last_command.result = SUCCESS (03-REQ-7.3: command response arrives at GW)
    LAST_CMD_RESULT=$(echo "$STATUS_RESP" | json_nested last_command result)
    if [ "$LAST_CMD_RESULT" = "SUCCESS" ]; then
        pass "GET /status shows last_command.result=SUCCESS (03-REQ-7.3)"
    else
        fail "GET /status shows last_command.result=SUCCESS" "result=$LAST_CMD_RESULT"
    fi

    # Check last_command.status = success
    LAST_CMD_STATUS=$(echo "$STATUS_RESP" | json_nested last_command status)
    if [ "$LAST_CMD_STATUS" = "success" ]; then
        pass "GET /status shows last_command.status=success"
    else
        fail "GET /status shows last_command.status=success" "status=$LAST_CMD_STATUS"
    fi
fi

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 10.4: Telemetry Flow
# Requirements: 03-REQ-7.5
# Property 5: Telemetry Accuracy
# ═══════════════════════════════════════════════════════════════════════════════
printf "\n--- Test 10.4: Telemetry Flow (03-REQ-7.5) ---\n"

if [ -z "$TOKEN" ]; then
    skip "Telemetry flow" "no token available (pairing failed)"
else
    # Set known sensor values via mock-sensors
    "$MOCK_SENSORS_BIN" --databroker-addr "$DATABROKER_ADDR" set-speed 42.5 >/dev/null 2>&1
    "$MOCK_SENSORS_BIN" --databroker-addr "$DATABROKER_ADDR" set-location 52.5200 13.4050 >/dev/null 2>&1
    "$MOCK_SENSORS_BIN" --databroker-addr "$DATABROKER_ADDR" set-door closed >/dev/null 2>&1

    # Wait for at least one telemetry cycle to push values to CLOUD_GATEWAY
    info "Waiting for telemetry cycle (${TELEMETRY_INTERVAL}s interval + buffer)..."
    sleep $((TELEMETRY_INTERVAL + 4))

    # Query status — should reflect the telemetry values
    TEL_STATUS=$(http_body -X GET "${GATEWAY_URL}/api/v1/vehicles/${VIN}/status" \
        -H "Authorization: Bearer ${TOKEN}")

    # Check speed
    TEL_SPEED=$(echo "$TEL_STATUS" | json_get speed)
    if [ -n "$TEL_SPEED" ] && [ "$TEL_SPEED" != "" ]; then
        SPEED_OK=$(python3 -c "print('yes' if abs(float('$TEL_SPEED') - 42.5) < 1.0 else 'no')" 2>/dev/null || echo "no")
        if [ "$SPEED_OK" = "yes" ]; then
            pass "Telemetry: speed=42.5 reflected in GET /status"
        else
            fail "Telemetry: speed=42.5 reflected in GET /status" "got speed=$TEL_SPEED"
        fi
    else
        fail "Telemetry: speed present in GET /status" "speed=$TEL_SPEED"
    fi

    # Check latitude
    TEL_LAT=$(echo "$TEL_STATUS" | json_get latitude)
    if [ -n "$TEL_LAT" ] && [ "$TEL_LAT" != "" ]; then
        LAT_OK=$(python3 -c "print('yes' if abs(float('$TEL_LAT') - 52.52) < 0.1 else 'no')" 2>/dev/null || echo "no")
        if [ "$LAT_OK" = "yes" ]; then
            pass "Telemetry: latitude reflected in GET /status"
        else
            fail "Telemetry: latitude reflected in GET /status" "got latitude=$TEL_LAT"
        fi
    else
        fail "Telemetry: latitude present in GET /status" "latitude=$TEL_LAT"
    fi

    # Check longitude
    TEL_LON=$(echo "$TEL_STATUS" | json_get longitude)
    if [ -n "$TEL_LON" ] && [ "$TEL_LON" != "" ]; then
        LON_OK=$(python3 -c "print('yes' if abs(float('$TEL_LON') - 13.405) < 0.1 else 'no')" 2>/dev/null || echo "no")
        if [ "$LON_OK" = "yes" ]; then
            pass "Telemetry: longitude reflected in GET /status"
        else
            fail "Telemetry: longitude reflected in GET /status" "got longitude=$TEL_LON"
        fi
    else
        fail "Telemetry: longitude present in GET /status" "longitude=$TEL_LON"
    fi

    # Check updated_at is set (telemetry updates should set the timestamp)
    TEL_UPDATED=$(echo "$TEL_STATUS" | json_get updated_at)
    if [ -n "$TEL_UPDATED" ] && [ "$TEL_UPDATED" != "" ]; then
        pass "Telemetry: updated_at timestamp present in GET /status (03-REQ-7.5)"
    else
        fail "Telemetry: updated_at timestamp present in GET /status" "updated_at=$TEL_UPDATED"
    fi
fi

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 10.5: Rejection Propagation
# Requirements: 03-REQ-7.2, 03-REQ-7.3
# ═══════════════════════════════════════════════════════════════════════════════
printf "\n--- Test 10.5: Rejection Propagation (03-REQ-7.2, 03-REQ-7.3) ---\n"

if [ -z "$TOKEN" ]; then
    skip "Rejection propagation" "no token available (pairing failed)"
else
    # First unlock the vehicle so we can test lock rejection
    "$MOCK_SENSORS_BIN" --databroker-addr "$DATABROKER_ADDR" set-speed 0 >/dev/null 2>&1
    "$MOCK_SENSORS_BIN" --databroker-addr "$DATABROKER_ADDR" set-door closed >/dev/null 2>&1
    sleep 1

    # Send unlock to reset state
    http_body -X POST "${GATEWAY_URL}/api/v1/vehicles/${VIN}/unlock" \
        -H "Authorization: Bearer ${TOKEN}" >/dev/null 2>&1 || true
    sleep 4

    # Set unsafe speed (>= 1.0 km/h)
    info "Setting unsafe speed (50 km/h)..."
    "$MOCK_SENSORS_BIN" --databroker-addr "$DATABROKER_ADDR" set-speed 50 >/dev/null 2>&1
    sleep 1

    # Send lock command — should be accepted but rejected asynchronously due to speed
    REJECT_LOCK_STATUS=$(http_status -X POST "${GATEWAY_URL}/api/v1/vehicles/${VIN}/lock" \
        -H "Authorization: Bearer ${TOKEN}")

    if [ "$REJECT_LOCK_STATUS" = "202" ]; then
        pass "Lock command at unsafe speed returns 202 (accepted, will be rejected async)"
    else
        fail "Lock command at unsafe speed returns 202" "got: $REJECT_LOCK_STATUS"
    fi

    # Wait for rejection to propagate through pipeline
    info "Waiting for rejection to propagate..."
    sleep 6

    # Check status — last_command.result should be REJECTED_SPEED
    REJECT_STATUS=$(http_body -X GET "${GATEWAY_URL}/api/v1/vehicles/${VIN}/status" \
        -H "Authorization: Bearer ${TOKEN}")

    REJECT_RESULT=$(echo "$REJECT_STATUS" | json_nested last_command result)
    if [ "$REJECT_RESULT" = "REJECTED_SPEED" ]; then
        pass "Lock at unsafe speed → last_command.result=REJECTED_SPEED (03-REQ-7.2, 03-REQ-7.3)"
    else
        fail "Lock at unsafe speed → last_command.result=REJECTED_SPEED" "result=$REJECT_RESULT"
    fi

    # Check is_locked hasn't been set to true by the rejected lock
    REJECT_LOCKED=$(echo "$REJECT_STATUS" | json_get is_locked)
    info "is_locked after speed rejection: $REJECT_LOCKED"
    if [ "$REJECT_LOCKED" = "true" ]; then
        fail "Vehicle should not be locked after speed rejection" "is_locked=$REJECT_LOCKED"
    else
        pass "Vehicle is not locked after speed rejection"
    fi

    # Test door-open rejection
    "$MOCK_SENSORS_BIN" --databroker-addr "$DATABROKER_ADDR" set-speed 0 >/dev/null 2>&1
    "$MOCK_SENSORS_BIN" --databroker-addr "$DATABROKER_ADDR" set-door open >/dev/null 2>&1
    sleep 1

    # Send lock command — should be rejected due to door open
    http_body -X POST "${GATEWAY_URL}/api/v1/vehicles/${VIN}/lock" \
        -H "Authorization: Bearer ${TOKEN}" >/dev/null 2>&1

    # Wait for rejection
    sleep 6

    DOOR_REJECT_STATUS=$(http_body -X GET "${GATEWAY_URL}/api/v1/vehicles/${VIN}/status" \
        -H "Authorization: Bearer ${TOKEN}")

    DOOR_REJECT_RESULT=$(echo "$DOOR_REJECT_STATUS" | json_nested last_command result)
    if [ "$DOOR_REJECT_RESULT" = "REJECTED_DOOR_OPEN" ]; then
        pass "Lock with door open → last_command.result=REJECTED_DOOR_OPEN"
    else
        fail "Lock with door open → last_command.result=REJECTED_DOOR_OPEN" "result=$DOOR_REJECT_RESULT"
    fi
fi

# ═══════════════════════════════════════════════════════════════════════════════
# Additional: Test unlock E2E (verify full cycle)
# ═══════════════════════════════════════════════════════════════════════════════
printf "\n--- Additional: Unlock E2E ---\n"

if [ -z "$TOKEN" ]; then
    skip "Unlock E2E" "no token available (pairing failed)"
else
    # Set safe conditions and lock first
    "$MOCK_SENSORS_BIN" --databroker-addr "$DATABROKER_ADDR" set-speed 0 >/dev/null 2>&1
    "$MOCK_SENSORS_BIN" --databroker-addr "$DATABROKER_ADDR" set-door closed >/dev/null 2>&1
    sleep 1

    # Lock
    http_body -X POST "${GATEWAY_URL}/api/v1/vehicles/${VIN}/lock" \
        -H "Authorization: Bearer ${TOKEN}" >/dev/null 2>&1
    sleep 5

    # Unlock
    UNLOCK_STATUS=$(http_status -X POST "${GATEWAY_URL}/api/v1/vehicles/${VIN}/unlock" \
        -H "Authorization: Bearer ${TOKEN}")

    if [ "$UNLOCK_STATUS" = "202" ]; then
        pass "Unlock command returns 202 Accepted"
    else
        fail "Unlock command returns 202 Accepted" "got: $UNLOCK_STATUS"
    fi

    # Wait for unlock to propagate
    sleep 5

    UNLOCK_RESP=$(http_body -X GET "${GATEWAY_URL}/api/v1/vehicles/${VIN}/status" \
        -H "Authorization: Bearer ${TOKEN}")

    UNLOCK_LOCKED=$(echo "$UNLOCK_RESP" | json_get is_locked)
    if [ "$UNLOCK_LOCKED" = "false" ]; then
        pass "GET /status shows is_locked=false after unlock"
    else
        fail "GET /status shows is_locked=false after unlock" "is_locked=$UNLOCK_LOCKED"
    fi
fi

# ═══════════════════════════════════════════════════════════════════════════════
# Additional: Test auth enforcement
# ═══════════════════════════════════════════════════════════════════════════════
printf "\n--- Additional: Auth Enforcement ---\n"

# Lock without token → 401
NO_AUTH_STATUS=$(http_status -X POST "${GATEWAY_URL}/api/v1/vehicles/${VIN}/lock")
if [ "$NO_AUTH_STATUS" = "401" ]; then
    pass "Lock without token returns 401 Unauthorized"
else
    fail "Lock without token returns 401 Unauthorized" "got: $NO_AUTH_STATUS"
fi

# Status with wrong token → 401
WRONG_TOKEN_STATUS=$(http_status -X GET "${GATEWAY_URL}/api/v1/vehicles/${VIN}/status" \
    -H "Authorization: Bearer wrong-token-12345")
if [ "$WRONG_TOKEN_STATUS" = "401" ]; then
    pass "Status with wrong token returns 401 Unauthorized"
else
    fail "Status with wrong token returns 401 Unauthorized" "got: $WRONG_TOKEN_STATUS"
fi

# ─── Summary ──────────────────────────────────────────────────────────────────
printf "\n=== Cloud Connectivity E2E Results ===\n"
printf "  ${GREEN}Passed${NC}: %d\n" "$passed"
printf "  ${RED}Failed${NC}: %d\n" "$failed"
printf "  ${YELLOW}Skipped${NC}: %d\n" "$skipped"
printf "  Total: %d\n\n" "$((passed + failed + skipped))"

if [ "$failed" -gt 0 ]; then
    printf "${RED}Some tests failed!${NC}\n"
    printf "Service logs are available at:\n"
    printf "  /tmp/cloud-gateway-e2e.log\n"
    printf "  /tmp/cloud-gateway-client-e2e.log\n"
    printf "  /tmp/locking-service-e2e.log\n"
    exit 1
fi

printf "${GREEN}All tests passed!${NC}\n"
exit 0
