#!/usr/bin/env bash
# Test local infrastructure (Podman compose).
# Covers: TS-01-20, TS-01-21, TS-01-22, TS-01-P5

set -euo pipefail
source "$(dirname "$0")/test_helpers.sh"

# TS-01-20: Compose file defines NATS and Kuksa services (01-REQ-7.1)
test_compose_file_services() {
    local compose="$REPO_ROOT/deployments/compose.yml"
    assert_file_exists "$compose" || return 1
    local result=0
    assert_file_contains "$compose" "nats" || result=1
    # Check for kuksa or databroker
    if ! grep -q "kuksa" "$compose" && ! grep -q "databroker" "$compose"; then
        echo "  Missing kuksa/databroker service in compose.yml"
        result=1
    fi
    assert_file_contains "$compose" "4222" || result=1
    assert_file_contains "$compose" "55556" || result=1
    return $result
}

# TS-01-21: Infrastructure starts and services are reachable (01-REQ-7.2)
# NOTE: This test requires Podman to be installed and running.
test_infra_starts() {
    # Check if podman is available
    if ! command -v podman &>/dev/null; then
        echo "  SKIP: podman not installed"
        return 0
    fi

    # Start infrastructure
    (cd "$REPO_ROOT" && make infra-up 2>&1) >/dev/null || {
        echo "  make infra-up failed"
        return 1
    }

    # Wait for services to start (up to 30 seconds)
    local result=0
    local max_wait=30
    local waited=0

    # Check NATS on port 4222
    while ! nc -z localhost 4222 2>/dev/null; do
        sleep 1
        waited=$((waited + 1))
        if [[ $waited -ge $max_wait ]]; then
            echo "  NATS not reachable on port 4222 after ${max_wait}s"
            result=1
            break
        fi
    done

    # Check Kuksa on port 55556
    waited=0
    while ! nc -z localhost 55556 2>/dev/null; do
        sleep 1
        waited=$((waited + 1))
        if [[ $waited -ge $max_wait ]]; then
            echo "  Kuksa Databroker not reachable on port 55556 after ${max_wait}s"
            result=1
            break
        fi
    done

    # Cleanup
    (cd "$REPO_ROOT" && make infra-down 2>&1) >/dev/null || true

    return $result
}

# TS-01-22: Infrastructure stops cleanly (01-REQ-7.3)
test_infra_stops() {
    if ! command -v podman &>/dev/null; then
        echo "  SKIP: podman not installed"
        return 0
    fi

    # Start
    (cd "$REPO_ROOT" && make infra-up 2>&1) >/dev/null || {
        echo "  make infra-up failed"
        return 1
    }
    sleep 2

    # Stop
    (cd "$REPO_ROOT" && make infra-down 2>&1) >/dev/null || {
        echo "  make infra-down failed"
        return 1
    }

    # Verify no containers remain
    local remaining
    remaining=$(podman compose -f "$REPO_ROOT/deployments/compose.yml" ps -q 2>/dev/null | tr -d '[:space:]')
    if [[ -n "$remaining" ]]; then
        echo "  Containers still running after infra-down"
        return 1
    fi
    return 0
}

# TS-01-P5: Infrastructure lifecycle property (Property 5)
test_infra_lifecycle() {
    if ! command -v podman &>/dev/null; then
        echo "  SKIP: podman not installed"
        return 0
    fi

    (cd "$REPO_ROOT" && make infra-up 2>&1) >/dev/null || {
        echo "  make infra-up failed"
        return 1
    }
    sleep 2
    (cd "$REPO_ROOT" && make infra-down 2>&1) >/dev/null || {
        echo "  make infra-down failed"
        return 1
    }

    local remaining
    remaining=$(podman compose -f "$REPO_ROOT/deployments/compose.yml" ps -q 2>/dev/null | tr -d '[:space:]')
    if [[ -n "$remaining" ]]; then
        echo "  Orphaned containers found after lifecycle"
        return 1
    fi
    return 0
}

# Run all tests
run_test "TS-01-20: Compose file defines services" test_compose_file_services
run_test "TS-01-21: Infrastructure starts" test_infra_starts
run_test "TS-01-22: Infrastructure stops cleanly" test_infra_stops
run_test "TS-01-P5: Infrastructure lifecycle" test_infra_lifecycle

print_summary "test_infra.sh"
