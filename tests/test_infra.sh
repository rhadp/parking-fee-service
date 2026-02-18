#!/usr/bin/env bash
# test_infra.sh — Infrastructure smoke test for local development services.
#
# Validates:
#   - make infra-up starts Kuksa Databroker and Mosquitto
#   - Kuksa responds on port 55555
#   - Mosquitto responds on port 1883
#   - make infra-down stops and removes containers
#   - Property 4: Infrastructure Lifecycle Idempotency
#   - Requirements: 01-REQ-6.1, 01-REQ-6.2, 01-REQ-6.3, 01-REQ-6.4, 01-REQ-6.5
#
# Usage:
#   ./tests/test_infra.sh
#
# Prerequisites:
#   - podman or docker must be installed and running
#   - Ports 55555 and 1883 must be available

set -euo pipefail

# Resolve project root (parent of tests/ directory)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

passed=0
failed=0
skipped=0

pass() {
    printf "  ${GREEN}PASS${NC} %s\n" "$1"
    passed=$((passed + 1))
}

fail() {
    printf "  ${RED}FAIL${NC} %s\n" "$1"
    failed=$((failed + 1))
}

skip() {
    printf "  ${YELLOW}SKIP${NC} %s\n" "$1"
    skipped=$((skipped + 1))
}

# Detect container runtime
CONTAINER_RUNTIME=""
if command -v podman &>/dev/null; then
    CONTAINER_RUNTIME="podman"
elif command -v docker &>/dev/null; then
    CONTAINER_RUNTIME="docker"
fi

if [ -z "$CONTAINER_RUNTIME" ]; then
    echo "No container runtime found (podman or docker). Skipping infrastructure tests."
    exit 0
fi

echo "Infrastructure Smoke Tests (using $CONTAINER_RUNTIME)"
echo "======================================================"

# Ensure cleanup on exit
cleanup() {
    echo ""
    echo "Cleaning up infrastructure..."
    cd "$PROJECT_ROOT" && make infra-down 2>/dev/null || true
}
trap cleanup EXIT

# ─── Test: compose.yaml exists ──────────────────────────────────────────────

if [ -f "$PROJECT_ROOT/infra/compose.yaml" ]; then
    pass "infra/compose.yaml exists"
else
    fail "infra/compose.yaml exists"
fi

# ─── Test: mosquitto.conf exists ────────────────────────────────────────────

if [ -f "$PROJECT_ROOT/infra/config/mosquitto/mosquitto.conf" ]; then
    pass "infra/config/mosquitto/mosquitto.conf exists"
else
    fail "infra/config/mosquitto/mosquitto.conf exists"
fi

# ─── Test: mosquitto.conf contains listener and allow_anonymous ─────────────

if grep -q "listener 1883" "$PROJECT_ROOT/infra/config/mosquitto/mosquitto.conf" && \
   grep -q "allow_anonymous true" "$PROJECT_ROOT/infra/config/mosquitto/mosquitto.conf"; then
    pass "mosquitto.conf has listener 1883 and allow_anonymous true"
else
    fail "mosquitto.conf has listener 1883 and allow_anonymous true"
fi

# ─── Test: compose.yaml defines databroker and mosquitto services ───────────

if grep -q "databroker:" "$PROJECT_ROOT/infra/compose.yaml" && \
   grep -q "mosquitto:" "$PROJECT_ROOT/infra/compose.yaml"; then
    pass "compose.yaml defines databroker and mosquitto services"
else
    fail "compose.yaml defines databroker and mosquitto services"
fi

# ─── Test: compose.yaml maps correct ports ──────────────────────────────────

if grep -q '"55555:55555"' "$PROJECT_ROOT/infra/compose.yaml" && \
   grep -q '"1883:1883"' "$PROJECT_ROOT/infra/compose.yaml"; then
    pass "compose.yaml maps ports 55555 and 1883"
else
    fail "compose.yaml maps ports 55555 and 1883"
fi

# ─── Check if container runtime daemon is available ─────────────────────────

DAEMON_AVAILABLE=true
if ! $CONTAINER_RUNTIME info &>/dev/null; then
    DAEMON_AVAILABLE=false
    echo ""
    printf "  ${YELLOW}WARNING${NC}: $CONTAINER_RUNTIME daemon is not running. Skipping live infrastructure tests.\n"
    echo ""
fi

# ─── Live tests (require running container daemon) ──────────────────────────

# Wait for a port to become available; returns 0 on success, 1 on timeout
wait_for_port() {
    local port="$1"
    local timeout="${2:-30}"
    local elapsed=0

    while [ "$elapsed" -lt "$timeout" ]; do
        if (echo > /dev/tcp/localhost/"$port") 2>/dev/null; then
            return 0
        fi
        sleep 1
        ((elapsed++))
    done
    return 1
}

if [ "$DAEMON_AVAILABLE" = true ]; then
    # ─── Test: make infra-up starts containers ──────────────────────────────

    echo ""
    echo "Starting infrastructure (make infra-up)..."
    cd "$PROJECT_ROOT"
    if make infra-up 2>&1; then
        pass "make infra-up completes without error"
    else
        fail "make infra-up completes without error"
    fi

    # ─── Test: Kuksa Databroker responds on port 55555 ──────────────────────

    echo ""
    echo "Waiting for Kuksa Databroker on port 55555..."
    if wait_for_port 55555 30; then
        pass "Kuksa Databroker responds on port 55555"
    else
        fail "Kuksa Databroker responds on port 55555"
    fi

    # ─── Test: Mosquitto responds on port 1883 ──────────────────────────────

    echo "Waiting for Mosquitto on port 1883..."
    if wait_for_port 1883 30; then
        pass "Mosquitto responds on port 1883"
    else
        fail "Mosquitto responds on port 1883"
    fi

    # ─── Test: make infra-status shows running containers ───────────────────

    echo ""
    echo "Checking infrastructure status (make infra-status)..."
    if make infra-status 2>&1; then
        pass "make infra-status completes without error"
    else
        fail "make infra-status completes without error"
    fi

    # ─── Test: Idempotency — running infra-up again should succeed ──────────

    echo ""
    echo "Testing idempotency (make infra-up again)..."
    if make infra-up 2>&1; then
        pass "make infra-up is idempotent (second call succeeds)"
    else
        fail "make infra-up is idempotent (second call succeeds)"
    fi

    # Verify services are still available after second infra-up
    if wait_for_port 55555 10 && wait_for_port 1883 10; then
        pass "services still available after idempotent infra-up"
    else
        fail "services still available after idempotent infra-up"
    fi

    # ─── Test: make infra-down stops containers ─────────────────────────────

    echo ""
    echo "Stopping infrastructure (make infra-down)..."
    if make infra-down 2>&1; then
        pass "make infra-down completes without error"
    else
        fail "make infra-down completes without error"
    fi

    # ─── Test: ports are no longer in use after infra-down ──────────────────

    # Give containers a moment to fully stop
    sleep 2

    if ! (echo > /dev/tcp/localhost/55555) 2>/dev/null; then
        pass "port 55555 is free after infra-down"
    else
        fail "port 55555 is free after infra-down"
    fi

    if ! (echo > /dev/tcp/localhost/1883) 2>/dev/null; then
        pass "port 1883 is free after infra-down"
    else
        fail "port 1883 is free after infra-down"
    fi

    # Disable the EXIT trap since we already cleaned up
    trap - EXIT

else
    # Skip live tests
    skip "make infra-up (container daemon not running)"
    skip "Kuksa Databroker responds on port 55555 (container daemon not running)"
    skip "Mosquitto responds on port 1883 (container daemon not running)"
    skip "make infra-status (container daemon not running)"
    skip "make infra-up idempotency (container daemon not running)"
    skip "services available after idempotent infra-up (container daemon not running)"
    skip "make infra-down (container daemon not running)"
    skip "port 55555 free after infra-down (container daemon not running)"
    skip "port 1883 free after infra-down (container daemon not running)"
fi

# ─── Summary ────────────────────────────────────────────────────────────────

echo ""
echo "======================================================"
printf "Results: ${GREEN}%d passed${NC}" "$passed"
if [ "$failed" -gt 0 ]; then
    printf ", ${RED}%d failed${NC}" "$failed"
fi
if [ "$skipped" -gt 0 ]; then
    printf ", ${YELLOW}%d skipped${NC}" "$skipped"
fi
echo ""

if [ "$failed" -gt 0 ]; then
    exit 1
fi

exit 0
