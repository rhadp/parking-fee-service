#!/usr/bin/env bash
# Test Spec: TS-01-1, TS-01-2, TS-01-3, TS-01-4, TS-01-E1, TS-01-E2, TS-01-P1
# Tests for directory structure requirements

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PASS=0
FAIL=0

pass() { PASS=$((PASS + 1)); echo "  PASS: $1"; }
fail() { FAIL=$((FAIL + 1)); echo "  FAIL: $1"; }

echo "=== Directory Structure Tests ==="

# TS-01-1: Top-level directories exist (01-REQ-1.1)
echo ""
echo "--- TS-01-1: Top-Level Directory Structure ---"
for dir in rhivos backend android mobile mock proto deployments; do
    if [ -d "$REPO_ROOT/$dir" ]; then
        pass "Top-level directory '$dir' exists"
    else
        fail "Top-level directory '$dir' does not exist"
    fi
done

# TS-01-2: Rust service subdirectories (01-REQ-1.2)
echo ""
echo "--- TS-01-2: Rust Service Subdirectories ---"
for dir in locking-service cloud-gateway-client update-service parking-operator-adaptor mock-sensors; do
    if [ -d "$REPO_ROOT/rhivos/$dir" ]; then
        pass "Rust service directory 'rhivos/$dir' exists"
    else
        fail "Rust service directory 'rhivos/$dir' does not exist"
    fi
done

# TS-01-3: Go backend subdirectories (01-REQ-1.3)
echo ""
echo "--- TS-01-3: Go Backend Subdirectories ---"
for dir in parking-fee-service cloud-gateway; do
    if [ -d "$REPO_ROOT/backend/$dir" ]; then
        pass "Backend directory 'backend/$dir' exists"
    else
        fail "Backend directory 'backend/$dir' does not exist"
    fi
done

# TS-01-4: Mock CLI subdirectories (01-REQ-1.4)
echo ""
echo "--- TS-01-4: Mock CLI Subdirectories ---"
for dir in parking-app-cli companion-app-cli parking-operator; do
    if [ -d "$REPO_ROOT/mock/$dir" ]; then
        pass "Mock directory 'mock/$dir' exists"
    else
        fail "Mock directory 'mock/$dir' does not exist"
    fi
done

# TS-01-E1: Android placeholder (01-REQ-1.E1)
echo ""
echo "--- TS-01-E1: Android Placeholder ---"
if [ -d "$REPO_ROOT/android" ]; then
    entries=$(ls -A "$REPO_ROOT/android" | wc -l | tr -d ' ')
    if [ -f "$REPO_ROOT/android/README.md" ] && [ "$entries" -eq 1 ]; then
        pass "android/ contains only README.md"
    else
        fail "android/ should contain only README.md (found $entries entries)"
    fi
else
    fail "android/ directory does not exist"
fi

# TS-01-E2: Mobile placeholder (01-REQ-1.E2)
echo ""
echo "--- TS-01-E2: Mobile Placeholder ---"
if [ -d "$REPO_ROOT/mobile" ]; then
    entries=$(ls -A "$REPO_ROOT/mobile" | wc -l | tr -d ' ')
    if [ -f "$REPO_ROOT/mobile/README.md" ] && [ "$entries" -eq 1 ]; then
        pass "mobile/ contains only README.md"
    else
        fail "mobile/ should contain only README.md (found $entries entries)"
    fi
else
    fail "mobile/ directory does not exist"
fi

# TS-01-P1: Directory completeness property (Property 1)
echo ""
echo "--- TS-01-P1: Directory Completeness Property ---"
all_dirs=(
    "rhivos" "backend" "android" "mobile" "mock" "proto" "deployments"
    "rhivos/locking-service" "rhivos/cloud-gateway-client" "rhivos/update-service"
    "rhivos/parking-operator-adaptor" "rhivos/mock-sensors"
    "backend/parking-fee-service" "backend/cloud-gateway"
    "mock/parking-app-cli" "mock/companion-app-cli" "mock/parking-operator"
)
for dir in "${all_dirs[@]}"; do
    if [ -d "$REPO_ROOT/$dir" ]; then
        count=$(ls -A "$REPO_ROOT/$dir" | wc -l | tr -d ' ')
        if [ "$count" -gt 0 ]; then
            pass "Directory '$dir' exists and is non-empty"
        else
            fail "Directory '$dir' exists but is empty"
        fi
    else
        fail "Directory '$dir' does not exist"
    fi
done

echo ""
echo "=== Directory Tests Complete: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ]
