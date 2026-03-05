#!/usr/bin/env bash
# Test Spec: TS-01-5 through TS-01-14, TS-01-P2, TS-01-P3, TS-01-P4, TS-01-E3, TS-01-E4
# Tests for build system, workspaces, and skeleton implementations

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PASS=0
FAIL=0

pass() { PASS=$((PASS + 1)); echo "  PASS: $1"; }
fail() { FAIL=$((FAIL + 1)); echo "  FAIL: $1"; }

echo "=== Build and Workspace Tests ==="

# TS-01-5: Rust workspace Cargo.toml exists and lists members (01-REQ-2.1)
echo ""
echo "--- TS-01-5: Rust Workspace Cargo.toml ---"
cargo_toml="$REPO_ROOT/rhivos/Cargo.toml"
if [ -f "$cargo_toml" ]; then
    pass "rhivos/Cargo.toml exists"
    if grep -q '\[workspace\]' "$cargo_toml"; then
        pass "Cargo.toml contains [workspace] section"
    else
        fail "Cargo.toml does not contain [workspace] section"
    fi
    for member in locking-service cloud-gateway-client update-service parking-operator-adaptor mock-sensors; do
        if grep -q "$member" "$cargo_toml"; then
            pass "Cargo.toml lists member '$member'"
        else
            fail "Cargo.toml does not list member '$member'"
        fi
    done
else
    fail "rhivos/Cargo.toml does not exist"
fi

# TS-01-6: Rust workspace builds (01-REQ-2.2)
echo ""
echo "--- TS-01-6: Rust Workspace Build ---"
if (cd "$REPO_ROOT/rhivos" && cargo build 2>&1); then
    pass "cargo build succeeds in rhivos/"
else
    fail "cargo build fails in rhivos/"
fi

# TS-01-7: Rust workspace tests pass (01-REQ-2.3, 01-REQ-9.1)
echo ""
echo "--- TS-01-7: Rust Workspace Tests ---"
rust_test_output=$(cd "$REPO_ROOT/rhivos" && cargo test 2>&1) || true
if echo "$rust_test_output" | grep -q "test result: ok"; then
    pass "cargo test passes in rhivos/"
else
    fail "cargo test does not pass in rhivos/"
fi

# TS-01-8: Go backend workspace file (01-REQ-3.1)
echo ""
echo "--- TS-01-8: Go Backend Workspace ---"
go_work="$REPO_ROOT/backend/go.work"
if [ -f "$go_work" ]; then
    pass "backend/go.work exists"
    for module in parking-fee-service cloud-gateway; do
        if grep -q "$module" "$go_work"; then
            pass "go.work lists module '$module'"
        else
            fail "go.work does not list module '$module'"
        fi
    done
else
    fail "backend/go.work does not exist"
fi

# TS-01-9: Go mock workspace file (01-REQ-3.2)
echo ""
echo "--- TS-01-9: Go Mock Workspace ---"
mock_work="$REPO_ROOT/mock/go.work"
if [ -f "$mock_work" ]; then
    pass "mock/go.work exists"
    for module in parking-app-cli companion-app-cli parking-operator; do
        if grep -q "$module" "$mock_work"; then
            pass "go.work lists module '$module'"
        else
            fail "go.work does not list module '$module'"
        fi
    done
else
    fail "mock/go.work does not exist"
fi

# TS-01-10: Go modules build (01-REQ-3.3)
echo ""
echo "--- TS-01-10: Go Modules Build ---"
for module_dir in backend/parking-fee-service backend/cloud-gateway mock/parking-app-cli mock/companion-app-cli mock/parking-operator; do
    if (cd "$REPO_ROOT/$module_dir" && go build ./... 2>&1); then
        pass "go build succeeds in $module_dir"
    else
        fail "go build fails in $module_dir"
    fi
done

# TS-01-11: Go module tests pass (01-REQ-3.4, 01-REQ-9.2)
echo ""
echo "--- TS-01-11: Go Module Tests ---"
for module_dir in backend/parking-fee-service backend/cloud-gateway mock/parking-app-cli mock/companion-app-cli mock/parking-operator; do
    if (cd "$REPO_ROOT/$module_dir" && go test ./... 2>&1); then
        pass "go test passes in $module_dir"
    else
        fail "go test fails in $module_dir"
    fi
done

# TS-01-12: Rust skeleton binaries exit with code 0 (01-REQ-4.1)
echo ""
echo "--- TS-01-12: Rust Skeleton Binary Exit Codes ---"
# Build first
(cd "$REPO_ROOT/rhivos" && cargo build 2>/dev/null) || true
for binary in locking-service cloud-gateway-client update-service parking-operator-adaptor; do
    bin_path="$REPO_ROOT/rhivos/target/debug/$binary"
    if [ -x "$bin_path" ]; then
        output=$("$bin_path" 2>&1) || true
        exit_code=$?
        if [ $exit_code -eq 0 ] && [ -n "$output" ]; then
            pass "Binary '$binary' exits with code 0 and produces output"
        else
            fail "Binary '$binary' exit_code=$exit_code output_len=${#output}"
        fi
    else
        fail "Binary '$binary' not found at $bin_path"
    fi
done

# TS-01-13: Go skeleton binaries exit with code 0 (01-REQ-4.2)
echo ""
echo "--- TS-01-13: Go Skeleton Binary Exit Codes ---"
for module_dir in backend/parking-fee-service backend/cloud-gateway; do
    output=$(cd "$REPO_ROOT/$module_dir" && go run . 2>&1) || true
    exit_code=$?
    if [ $exit_code -eq 0 ] && [ -n "$output" ]; then
        pass "Go binary in '$module_dir' exits with code 0 and produces output"
    else
        fail "Go binary in '$module_dir' exit_code=$exit_code output_len=${#output}"
    fi
done

# TS-01-14: Each skeleton has at least one passing test (01-REQ-4.3)
echo ""
echo "--- TS-01-14: Skeleton Test Coverage ---"
for crate in locking-service cloud-gateway-client update-service parking-operator-adaptor mock-sensors; do
    test_output=$(cd "$REPO_ROOT/rhivos" && cargo test -p "$crate" 2>&1) || true
    if echo "$test_output" | grep -qE "test result: ok\. [1-9]"; then
        pass "Rust crate '$crate' has at least one passing test"
    else
        fail "Rust crate '$crate' has no passing tests"
    fi
done

for module_dir in backend/parking-fee-service backend/cloud-gateway mock/parking-app-cli mock/companion-app-cli mock/parking-operator; do
    test_output=$(cd "$REPO_ROOT/$module_dir" && go test -v ./... 2>&1) || true
    if echo "$test_output" | grep -q "PASS"; then
        pass "Go module '$module_dir' has at least one passing test"
    else
        fail "Go module '$module_dir' has no passing tests"
    fi
done

# TS-01-P2: Build determinism property (Property 2)
echo ""
echo "--- TS-01-P2: Build Determinism ---"
if [ -f "$REPO_ROOT/Makefile" ]; then
    (cd "$REPO_ROOT" && make clean 2>/dev/null) || true
    result1=$(cd "$REPO_ROOT" && make build 2>&1 && echo "OK" || echo "FAIL")
    result2=$(cd "$REPO_ROOT" && make build 2>&1 && echo "OK" || echo "FAIL")
    if [ "$result1" = "OK" ] && [ "$result2" = "OK" ]; then
        pass "Two consecutive make build runs succeed (deterministic)"
    else
        fail "Build is not deterministic (run1=$result1, run2=$result2)"
    fi
else
    fail "Makefile does not exist, cannot test build determinism"
fi

# TS-01-P3: Test discoverability property (Property 3)
echo ""
echo "--- TS-01-P3: Test Discoverability ---"
for crate in locking-service cloud-gateway-client update-service parking-operator-adaptor mock-sensors; do
    test_output=$(cd "$REPO_ROOT/rhivos" && cargo test -p "$crate" 2>&1) || true
    if echo "$test_output" | grep -qE "test result: ok\. [1-9]"; then
        pass "Rust crate '$crate' discovers and passes tests"
    else
        fail "Rust crate '$crate' test discovery failed"
    fi
done
for module_dir in backend/parking-fee-service backend/cloud-gateway mock/parking-app-cli mock/companion-app-cli mock/parking-operator; do
    test_output=$(cd "$REPO_ROOT/$module_dir" && go test -v ./... 2>&1) || true
    if echo "$test_output" | grep -q "PASS"; then
        pass "Go module '$module_dir' discovers and passes tests"
    else
        fail "Go module '$module_dir' test discovery failed"
    fi
done

# TS-01-P4: Skeleton exit behavior property (Property 4)
echo ""
echo "--- TS-01-P4: Skeleton Exit Behavior ---"
# Rust binaries
(cd "$REPO_ROOT/rhivos" && cargo build 2>/dev/null) || true
for binary in locking-service cloud-gateway-client update-service parking-operator-adaptor; do
    bin_path="$REPO_ROOT/rhivos/target/debug/$binary"
    if [ -x "$bin_path" ]; then
        output=$("$bin_path" 2>&1)
        exit_code=$?
        if [ $exit_code -eq 0 ] && [ -n "$output" ]; then
            pass "Rust binary '$binary' exits 0 with stdout output"
        else
            fail "Rust binary '$binary' exit=$exit_code output_empty=$( [ -z "$output" ] && echo yes || echo no)"
        fi
    else
        fail "Rust binary '$binary' not built"
    fi
done
# Go binaries
for module_dir in backend/parking-fee-service backend/cloud-gateway; do
    output=$(cd "$REPO_ROOT/$module_dir" && go run . 2>&1)
    exit_code=$?
    if [ $exit_code -eq 0 ] && [ -n "$output" ]; then
        pass "Go binary in '$module_dir' exits 0 with stdout output"
    else
        fail "Go binary in '$module_dir' exit=$exit_code"
    fi
done

# TS-01-E3: Skeleton binary without config (01-REQ-4.E1)
echo ""
echo "--- TS-01-E3: Skeleton Binary Without Config ---"
for binary in locking-service cloud-gateway-client update-service parking-operator-adaptor; do
    bin_path="$REPO_ROOT/rhivos/target/debug/$binary"
    if [ -x "$bin_path" ]; then
        stderr_output=$(env -i "$bin_path" 2>&1 1>/dev/null) || true
        exit_code=$?
        if [ $exit_code -eq 0 ]; then
            if echo "$stderr_output" | grep -qi "panic"; then
                fail "Binary '$binary' panics without config"
            else
                pass "Binary '$binary' exits cleanly without config"
            fi
        else
            fail "Binary '$binary' exits with code $exit_code without config"
        fi
    else
        fail "Binary '$binary' not built"
    fi
done

# TS-01-E4: make build reports failure clearly (01-REQ-6.E1)
echo ""
echo "--- TS-01-E4: Build Failure Reporting ---"
if [ -f "$REPO_ROOT/Makefile" ]; then
    # Inject a syntax error into a Rust source file
    rust_main="$REPO_ROOT/rhivos/locking-service/src/main.rs"
    if [ -f "$rust_main" ]; then
        cp "$rust_main" "$rust_main.bak"
        echo "THIS IS NOT VALID RUST" >> "$rust_main"
        build_output=$(cd "$REPO_ROOT" && make build 2>&1) || true
        build_exit=$?
        # Restore
        mv "$rust_main.bak" "$rust_main"
        if [ $build_exit -ne 0 ]; then
            pass "make build exits non-zero on component failure"
        else
            fail "make build should exit non-zero on component failure"
        fi
        if echo "$build_output" | grep -qi "error"; then
            pass "make build reports error message on failure"
        else
            fail "make build does not report error message on failure"
        fi
    else
        fail "Cannot test build failure: $rust_main does not exist"
    fi
else
    fail "Makefile does not exist, cannot test failure reporting"
fi

echo ""
echo "=== Build Tests Complete: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ]
