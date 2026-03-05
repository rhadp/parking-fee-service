#!/usr/bin/env bash
# Test Spec: TS-01-20, TS-01-21, TS-01-22, TS-01-P5
# Tests for local infrastructure (docker-compose)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PASS=0
FAIL=0

pass() { PASS=$((PASS + 1)); echo "  PASS: $1"; }
fail() { FAIL=$((FAIL + 1)); echo "  FAIL: $1"; }

echo "=== Infrastructure Tests ==="

# TS-01-20: Compose file defines NATS and Kuksa services (01-REQ-7.1)
echo ""
echo "--- TS-01-20: Compose File Contents ---"
compose_file="$REPO_ROOT/deployments/docker-compose.yml"
if [ -f "$compose_file" ]; then
    pass "deployments/docker-compose.yml exists"
    content=$(cat "$compose_file")
    if echo "$content" | grep -qi "nats"; then
        pass "Compose file defines NATS service"
    else
        fail "Compose file does not define NATS service"
    fi
    if echo "$content" | grep -qi "kuksa\|databroker"; then
        pass "Compose file defines Kuksa Databroker service"
    else
        fail "Compose file does not define Kuksa Databroker service"
    fi
    if echo "$content" | grep -q "4222"; then
        pass "Compose file exposes NATS port 4222"
    else
        fail "Compose file does not expose NATS port 4222"
    fi
    if echo "$content" | grep -q "55556"; then
        pass "Compose file exposes Kuksa port 55556"
    else
        fail "Compose file does not expose Kuksa port 55556"
    fi
else
    fail "deployments/docker-compose.yml does not exist"
fi

# Check if Docker/Podman is available and running for live tests
DOCKER_CMD=""
if command -v podman &>/dev/null && podman info >/dev/null 2>&1; then
    DOCKER_CMD="podman"
elif command -v docker &>/dev/null && docker info >/dev/null 2>&1; then
    DOCKER_CMD="docker"
fi

if [ -z "$DOCKER_CMD" ]; then
    echo ""
    echo "  SKIP: Docker/Podman not available, skipping live infrastructure tests"
    echo "  (TS-01-21, TS-01-22, TS-01-P5 require container runtime)"
else
    # TS-01-21: Infrastructure starts and services are reachable (01-REQ-7.2)
    echo ""
    echo "--- TS-01-21: Infrastructure Startup ---"
    if [ -f "$compose_file" ]; then
        # Start infrastructure
        if (cd "$REPO_ROOT" && make infra-up 2>&1); then
            pass "make infra-up succeeds"

            # Wait for services to be reachable (up to 30 seconds)
            nats_ok=false
            kuksa_ok=false
            for i in $(seq 1 30); do
                if ! $nats_ok && (echo > /dev/tcp/localhost/4222) 2>/dev/null; then
                    nats_ok=true
                fi
                if ! $kuksa_ok && (echo > /dev/tcp/localhost/55556) 2>/dev/null; then
                    kuksa_ok=true
                fi
                if $nats_ok && $kuksa_ok; then
                    break
                fi
                sleep 1
            done

            if $nats_ok; then
                pass "NATS reachable on port 4222"
            else
                fail "NATS not reachable on port 4222 within 30 seconds"
            fi
            if $kuksa_ok; then
                pass "Kuksa Databroker reachable on port 55556"
            else
                fail "Kuksa Databroker not reachable on port 55556 within 30 seconds"
            fi
        else
            fail "make infra-up fails"
        fi

        # TS-01-22: Infrastructure stops cleanly (01-REQ-7.3)
        echo ""
        echo "--- TS-01-22: Infrastructure Shutdown ---"
        if (cd "$REPO_ROOT" && make infra-down 2>&1); then
            pass "make infra-down succeeds"
        else
            fail "make infra-down fails"
        fi

        # Check no containers remain
        remaining=$($DOCKER_CMD compose -f "$compose_file" ps -q 2>/dev/null | wc -l | tr -d ' ')
        if [ "$remaining" -eq 0 ]; then
            pass "No containers remain after infra-down"
        else
            fail "$remaining containers still running after infra-down"
            # Clean up
            $DOCKER_CMD compose -f "$compose_file" down 2>/dev/null || true
        fi

        # TS-01-P5: Infrastructure lifecycle property (Property 5)
        echo ""
        echo "--- TS-01-P5: Infrastructure Lifecycle Property ---"
        (cd "$REPO_ROOT" && make infra-up 2>/dev/null) || true
        (cd "$REPO_ROOT" && make infra-down 2>/dev/null) || true
        remaining=$($DOCKER_CMD compose -f "$compose_file" ps -q 2>/dev/null | wc -l | tr -d ' ')
        if [ "$remaining" -eq 0 ]; then
            pass "Infrastructure lifecycle leaves no orphaned containers"
        else
            fail "Infrastructure lifecycle leaves $remaining orphaned containers"
            $DOCKER_CMD compose -f "$compose_file" down 2>/dev/null || true
        fi
    else
        fail "Compose file not found, cannot test infrastructure"
    fi
fi

echo ""
echo "=== Infrastructure Tests Complete: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ]
