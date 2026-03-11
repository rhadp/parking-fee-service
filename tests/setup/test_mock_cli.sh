#!/usr/bin/env bash
# Test mock CLI apps and mock sensors.
# Covers: TS-01-23, TS-01-24, TS-01-26, TS-01-P7, TS-01-E5, TS-01-E6

set -euo pipefail
source "$(dirname "$0")/test_helpers.sh"

# TS-01-23: Mock CLI apps build successfully (01-REQ-8.1, 01-REQ-8.2, 01-REQ-8.3)
test_mock_cli_build() {
    local result=0
    for app in parking-app-cli companion-app-cli parking-operator; do
        local app_dir="$REPO_ROOT/mock/$app"
        if ! (cd "$app_dir" && go build -o "$app" . 2>&1); then
            echo "  Failed to build $app"
            result=1
            continue
        fi
        if [[ ! -f "$app_dir/$app" ]]; then
            echo "  Binary not produced: $app_dir/$app"
            result=1
        fi
        # Clean up binary
        rm -f "$app_dir/$app"
    done
    return $result
}

# TS-01-24: Mock CLI apps print usage without arguments (01-REQ-8.4)
test_mock_cli_usage() {
    local result=0
    for app in parking-app-cli companion-app-cli parking-operator; do
        local output
        output=$(cd "$REPO_ROOT/mock/$app" && go run . 2>&1)
        local exit_code=$?
        if [[ $exit_code -ne 0 ]]; then
            echo "  $app exited with code $exit_code (expected 0)"
            result=1
            continue
        fi
        if ! echo "$output" | grep -qi "usage\|Usage"; then
            echo "  $app did not print usage information"
            result=1
        fi
    done
    return $result
}

# TS-01-26: Mock sensors crate builds (01-REQ-10.1, 01-REQ-10.2)
test_mock_sensors_build() {
    local output
    output=$(cd "$REPO_ROOT/rhivos" && cargo build -p mock-sensors 2>&1)
    local exit_code=$?
    if [[ $exit_code -ne 0 ]]; then
        echo "  cargo build -p mock-sensors failed"
        echo "  $output"
        return 1
    fi
    return 0
}

# TS-01-P7: Mock CLI usage output property (Property 7)
test_mock_cli_usage_property() {
    local result=0
    for app in parking-app-cli companion-app-cli parking-operator; do
        local output
        output=$(cd "$REPO_ROOT/mock/$app" && go run . 2>&1)
        local exit_code=$?
        if [[ $exit_code -ne 0 ]]; then
            echo "  $app exited with non-zero code"
            result=1
            continue
        fi
        if [[ -z "$output" ]]; then
            echo "  $app produced no output"
            result=1
        fi
    done
    return $result
}

# TS-01-E5: Mock CLI unknown subcommand (01-REQ-8.E1)
test_mock_cli_unknown_subcommand() {
    local result=0
    for app in parking-app-cli companion-app-cli parking-operator; do
        local output
        output=$(cd "$REPO_ROOT/mock/$app" && go run . nonexistent 2>&1) || true
        local exit_code=$?
        # With go run, the exit code might be reported differently
        # Re-run to capture exit code properly
        (cd "$REPO_ROOT/mock/$app" && go run . nonexistent >/dev/null 2>&1) && {
            echo "  $app should exit non-zero for unknown subcommand"
            result=1
            continue
        }
        if ! echo "$output" | grep -qiE "unknown|invalid|error"; then
            echo "  $app did not report error for unknown subcommand"
            result=1
        fi
    done
    return $result
}

# TS-01-E6: No tests reported gracefully (01-REQ-9.E1)
test_no_tests_graceful() {
    # Go reports "no test files" for packages without tests - this is acceptable
    # Verify that go test ./... still succeeds even if some subpackages have no tests
    local result=0
    for module_dir in backend/parking-fee-service backend/cloud-gateway; do
        local output
        output=$(cd "$REPO_ROOT/$module_dir" && go test ./... 2>&1)
        local exit_code=$?
        if [[ $exit_code -ne 0 ]]; then
            echo "  go test ./... failed in $module_dir (should handle no-test packages gracefully)"
            result=1
        fi
    done
    return $result
}

# Run all tests
run_test "TS-01-23: Mock CLI apps build" test_mock_cli_build
run_test "TS-01-24: Mock CLI apps print usage" test_mock_cli_usage
run_test "TS-01-26: Mock sensors crate builds" test_mock_sensors_build
run_test "TS-01-P7: Mock CLI usage property" test_mock_cli_usage_property
run_test "TS-01-E5: Unknown subcommand handling" test_mock_cli_unknown_subcommand
run_test "TS-01-E6: No tests graceful" test_no_tests_graceful

print_summary "test_mock_cli.sh"
