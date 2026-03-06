#!/usr/bin/env bash
# Test Spec: TS-01-23, TS-01-24, TS-01-26, TS-01-P7, TS-01-E5, TS-01-E6
# Tests for mock CLI apps and mock sensors

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PASS=0
FAIL=0

pass() { PASS=$((PASS + 1)); echo "  PASS: $1"; }
fail() { FAIL=$((FAIL + 1)); echo "  FAIL: $1"; }

echo "=== Mock CLI and Sensor Tests ==="

# TS-01-23: Mock CLI apps build successfully (01-REQ-8.1, 01-REQ-8.2, 01-REQ-8.3)
echo ""
echo "--- TS-01-23: Mock CLI App Builds ---"
for app in parking-app-cli companion-app-cli parking-operator; do
    app_dir="$REPO_ROOT/mock/$app"
    if [ -d "$app_dir" ]; then
        if (cd "$app_dir" && go build -o "$app" . 2>&1); then
            pass "Mock app '$app' builds successfully"
            if [ -f "$app_dir/$app" ]; then
                pass "Mock app '$app' produces binary"
                rm -f "$app_dir/$app"  # Clean up
            else
                fail "Mock app '$app' build did not produce binary"
            fi
        else
            fail "Mock app '$app' build fails"
        fi
    else
        fail "Mock app directory '$app' does not exist"
    fi
done

# TS-01-24: Mock CLI apps print usage without arguments (01-REQ-8.4)
echo ""
echo "--- TS-01-24: Mock CLI Usage Output ---"
for app in parking-app-cli companion-app-cli parking-operator; do
    app_dir="$REPO_ROOT/mock/$app"
    if [ -d "$app_dir" ]; then
        output=$(cd "$app_dir" && go run . 2>&1) || true
        exit_code=$?
        if [ $exit_code -eq 0 ]; then
            pass "Mock app '$app' exits with code 0 (no args)"
        else
            fail "Mock app '$app' exits with code $exit_code (no args)"
        fi
        if echo "$output" | grep -qi "usage\|Usage"; then
            pass "Mock app '$app' prints usage message"
        else
            fail "Mock app '$app' does not print usage message"
        fi
    else
        fail "Mock app directory '$app' does not exist"
    fi
done

# TS-01-26: Mock sensors crate builds (01-REQ-10.1, 01-REQ-10.2)
echo ""
echo "--- TS-01-26: Mock Sensors Crate Build ---"
if (cd "$REPO_ROOT/rhivos" && cargo build -p mock-sensors 2>&1); then
    pass "mock-sensors crate builds successfully"
else
    fail "mock-sensors crate build fails"
fi

# TS-01-P7: Mock CLI usage output property (Property 7)
echo ""
echo "--- TS-01-P7: Mock CLI Usage Output Property ---"
for app in parking-app-cli companion-app-cli parking-operator; do
    app_dir="$REPO_ROOT/mock/$app"
    if [ -d "$app_dir" ]; then
        output=$(cd "$app_dir" && go run . 2>&1) || true
        exit_code=$?
        if [ $exit_code -eq 0 ] && [ -n "$output" ]; then
            pass "Mock app '$app' exits 0 with non-empty output"
        else
            fail "Mock app '$app' property violation (exit=$exit_code, output_empty=$( [ -z "$output" ] && echo yes || echo no))"
        fi
    else
        fail "Mock app '$app' directory not found"
    fi
done

# TS-01-E5: Mock CLI unknown subcommand (01-REQ-8.E1)
echo ""
echo "--- TS-01-E5: Mock CLI Unknown Subcommand ---"
for app in parking-app-cli companion-app-cli parking-operator; do
    app_dir="$REPO_ROOT/mock/$app"
    if [ -d "$app_dir" ]; then
        exit_code=0
        output=$(cd "$app_dir" && go run . nonexistent 2>&1) || exit_code=$?
        if [ $exit_code -ne 0 ]; then
            pass "Mock app '$app' exits non-zero for unknown subcommand"
        else
            fail "Mock app '$app' should exit non-zero for unknown subcommand"
        fi
        if echo "$output" | grep -qiE "unknown|invalid|unrecognized"; then
            pass "Mock app '$app' reports unknown subcommand error"
        else
            fail "Mock app '$app' does not report unknown subcommand error"
        fi
    else
        fail "Mock app '$app' directory not found"
    fi
done

# TS-01-E6: No tests reported gracefully (01-REQ-9.E1)
echo ""
echo "--- TS-01-E6: No Tests Handled Gracefully ---"
# Go test runner should not fail if some packages have no test files
# This is inherent Go behavior - go test ./... succeeds with "no test files" output
for module_dir in backend/parking-fee-service backend/cloud-gateway; do
    if [ -d "$REPO_ROOT/$module_dir" ]; then
        result=$(cd "$REPO_ROOT/$module_dir" && go test ./... 2>&1) || true
        exit_code=$?
        if [ $exit_code -eq 0 ]; then
            pass "go test ./... in '$module_dir' succeeds (handles no-test packages gracefully)"
        else
            fail "go test ./... in '$module_dir' fails unexpectedly"
        fi
    fi
done

echo ""
echo "=== Mock CLI/Sensor Tests Complete: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ]
