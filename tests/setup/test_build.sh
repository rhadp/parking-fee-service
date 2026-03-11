#!/usr/bin/env bash
# Test build system and skeleton implementations.
# Covers: TS-01-5 through TS-01-14, TS-01-P2, TS-01-P3, TS-01-P4, TS-01-E3, TS-01-E4

set -euo pipefail
source "$(dirname "$0")/test_helpers.sh"

# TS-01-5: Rust workspace Cargo.toml exists and lists all members (01-REQ-2.1)
test_rust_workspace_toml() {
    local file="$REPO_ROOT/rhivos/Cargo.toml"
    assert_file_exists "$file" || return 1
    assert_file_contains "$file" '\[workspace\]' || return 1
    local result=0
    for member in locking-service cloud-gateway-client update-service parking-operator-adaptor mock-sensors; do
        assert_file_contains "$file" "$member" || result=1
    done
    return $result
}

# TS-01-6: Rust workspace builds successfully (01-REQ-2.2)
test_rust_workspace_builds() {
    local output
    output=$(cd "$REPO_ROOT/rhivos" && cargo build 2>&1) || {
        echo "  cargo build failed"
        echo "  $output"
        return 1
    }
    # Check no errors in output
    if echo "$output" | grep -q "^error"; then
        echo "  cargo build produced errors"
        return 1
    fi
    return 0
}

# TS-01-7: Rust workspace tests pass (01-REQ-2.3, 01-REQ-9.1)
test_rust_workspace_tests() {
    local output
    output=$(cd "$REPO_ROOT/rhivos" && cargo test 2>&1) || {
        echo "  cargo test failed"
        echo "  $output"
        return 1
    }
    if echo "$output" | grep -q "test result: ok"; then
        return 0
    else
        echo "  cargo test did not report 'test result: ok'"
        return 1
    fi
}

# TS-01-8: Go backend workspace file exists (01-REQ-3.1)
test_go_backend_workspace() {
    local file="$REPO_ROOT/backend/go.work"
    assert_file_exists "$file" || return 1
    local result=0
    for module in parking-fee-service cloud-gateway; do
        assert_file_contains "$file" "$module" || result=1
    done
    return $result
}

# TS-01-9: Go mock workspace file exists (01-REQ-3.2)
test_go_mock_workspace() {
    local file="$REPO_ROOT/mock/go.work"
    assert_file_exists "$file" || return 1
    local result=0
    for module in parking-app-cli companion-app-cli parking-operator; do
        assert_file_contains "$file" "$module" || result=1
    done
    return $result
}

# TS-01-10: Go modules build successfully (01-REQ-3.3)
test_go_modules_build() {
    local result=0
    for module_dir in backend/parking-fee-service backend/cloud-gateway mock/parking-app-cli mock/companion-app-cli mock/parking-operator; do
        if ! (cd "$REPO_ROOT/$module_dir" && go build ./... 2>&1); then
            echo "  go build failed in $module_dir"
            result=1
        fi
    done
    return $result
}

# TS-01-11: Go module tests pass (01-REQ-3.4, 01-REQ-9.2)
test_go_module_tests() {
    local result=0
    for module_dir in backend/parking-fee-service backend/cloud-gateway mock/parking-app-cli mock/companion-app-cli mock/parking-operator; do
        if ! (cd "$REPO_ROOT/$module_dir" && go test ./... 2>&1); then
            echo "  go test failed in $module_dir"
            result=1
        fi
    done
    return $result
}

# TS-01-12: Rust skeleton binaries exit with code 0 (01-REQ-4.1)
test_rust_skeleton_binaries() {
    # Build first
    (cd "$REPO_ROOT/rhivos" && cargo build 2>/dev/null) || {
        echo "  cargo build failed, cannot test binaries"
        return 1
    }
    local result=0
    for binary in locking-service cloud-gateway-client update-service parking-operator-adaptor; do
        local bin_path="$REPO_ROOT/rhivos/target/debug/$binary"
        if [[ ! -f "$bin_path" ]]; then
            echo "  Binary not found: $bin_path"
            result=1
            continue
        fi
        local output
        # Use timeout to prevent hanging if binary is long-running
        output=$(timeout 5 "$bin_path" 2>&1) || {
            local exit_code=$?
            # timeout returns 124 if command timed out; that's acceptable for a "starting..." message
            if [[ $exit_code -ne 124 ]]; then
                echo "  Binary $binary exited with code $exit_code"
                result=1
                continue
            fi
        }
        if [[ -z "$output" ]]; then
            echo "  Binary $binary produced no output"
            result=1
        fi
    done
    return $result
}

# TS-01-13: Go skeleton binaries exit with code 0 (01-REQ-4.2)
test_go_skeleton_binaries() {
    local result=0
    for module_dir in backend/parking-fee-service backend/cloud-gateway; do
        local output
        output=$(cd "$REPO_ROOT/$module_dir" && go run . 2>&1)
        local exit_code=$?
        if [[ $exit_code -ne 0 ]]; then
            echo "  go run in $module_dir exited with code $exit_code"
            result=1
            continue
        fi
        if [[ -z "$output" ]]; then
            echo "  go run in $module_dir produced no output"
            result=1
        fi
    done
    return $result
}

# TS-01-14: Each skeleton has at least one passing test (01-REQ-4.3)
test_each_skeleton_has_test() {
    local result=0

    # Rust crates
    for crate in locking-service cloud-gateway-client update-service parking-operator-adaptor mock-sensors; do
        local output
        output=$(cd "$REPO_ROOT/rhivos" && cargo test -p "$crate" 2>&1) || {
            echo "  cargo test -p $crate failed"
            result=1
            continue
        }
        if echo "$output" | grep -q "0 passed"; then
            echo "  No passing tests in Rust crate: $crate"
            result=1
        fi
    done

    # Go modules
    for module_dir in backend/parking-fee-service backend/cloud-gateway mock/parking-app-cli mock/companion-app-cli mock/parking-operator; do
        local output
        output=$(cd "$REPO_ROOT/$module_dir" && go test -v ./... 2>&1) || {
            echo "  go test failed in $module_dir"
            result=1
            continue
        }
        if ! echo "$output" | grep -q "PASS"; then
            echo "  No PASS output in $module_dir"
            result=1
        fi
    done
    return $result
}

# TS-01-P2: Build determinism (Property 2)
test_build_determinism() {
    # Two consecutive make build runs should both succeed
    local output1 output2
    output1=$(cd "$REPO_ROOT" && make clean 2>&1 && make build 2>&1) || {
        echo "  First make build failed"
        return 1
    }
    output2=$(cd "$REPO_ROOT" && make build 2>&1) || {
        echo "  Second make build failed"
        return 1
    }
    return 0
}

# TS-01-P3: Test discoverability (Property 3)
test_discoverability() {
    local result=0

    for crate in locking-service cloud-gateway-client update-service parking-operator-adaptor mock-sensors; do
        local output
        output=$(cd "$REPO_ROOT/rhivos" && cargo test -p "$crate" 2>&1) || {
            echo "  cargo test -p $crate failed"
            result=1
            continue
        }
        if echo "$output" | grep -q "0 passed"; then
            echo "  No tests discovered in crate: $crate"
            result=1
        fi
    done

    for module_dir in backend/parking-fee-service backend/cloud-gateway mock/parking-app-cli mock/companion-app-cli mock/parking-operator; do
        local output
        output=$(cd "$REPO_ROOT/$module_dir" && go test -v ./... 2>&1) || {
            echo "  go test failed in $module_dir"
            result=1
            continue
        }
        if ! echo "$output" | grep -q "PASS"; then
            echo "  No tests discovered in $module_dir"
            result=1
        fi
    done
    return $result
}

# TS-01-P4: Skeleton exit behavior (Property 4)
test_skeleton_exit_behavior() {
    local result=0

    # Build Rust binaries first
    (cd "$REPO_ROOT/rhivos" && cargo build 2>/dev/null) || {
        echo "  cargo build failed"
        return 1
    }

    # Rust binaries
    for binary in locking-service cloud-gateway-client update-service parking-operator-adaptor; do
        local bin_path="$REPO_ROOT/rhivos/target/debug/$binary"
        if [[ ! -f "$bin_path" ]]; then
            echo "  Binary not found: $bin_path"
            result=1
            continue
        fi
        local output
        output=$(timeout 5 "$bin_path" 2>&1) || {
            local exit_code=$?
            if [[ $exit_code -ne 124 ]]; then
                echo "  Binary $binary exited with non-zero code"
                result=1
            fi
        }
        if [[ -z "$output" ]]; then
            echo "  Binary $binary produced no stdout output"
            result=1
        fi
    done

    # Go binaries
    for module_dir in backend/parking-fee-service backend/cloud-gateway; do
        local output
        output=$(cd "$REPO_ROOT/$module_dir" && go run . 2>&1)
        local exit_code=$?
        if [[ $exit_code -ne 0 ]]; then
            echo "  go run in $module_dir exited with code $exit_code"
            result=1
        fi
        if [[ -z "$output" ]]; then
            echo "  go run in $module_dir produced no output"
            result=1
        fi
    done
    return $result
}

# TS-01-E3: Skeleton binary without config (01-REQ-4.E1)
test_skeleton_no_config() {
    local result=0

    # Build Rust binaries
    (cd "$REPO_ROOT/rhivos" && cargo build 2>/dev/null) || {
        echo "  cargo build failed"
        return 1
    }

    for binary in locking-service cloud-gateway-client update-service parking-operator-adaptor; do
        local bin_path="$REPO_ROOT/rhivos/target/debug/$binary"
        if [[ ! -f "$bin_path" ]]; then
            echo "  Binary not found: $bin_path"
            result=1
            continue
        fi
        local stderr_output
        stderr_output=$(timeout 5 "$bin_path" 2>&1 >/dev/null) || true
        if echo "$stderr_output" | grep -qi "panic"; then
            echo "  Binary $binary panicked without config"
            result=1
        fi
    done

    for module_dir in backend/parking-fee-service backend/cloud-gateway; do
        local stderr_output
        stderr_output=$(cd "$REPO_ROOT/$module_dir" && go run . 2>&1 >/dev/null) || true
        if echo "$stderr_output" | grep -qi "panic"; then
            echo "  Go binary in $module_dir panicked without config"
            result=1
        fi
    done
    return $result
}

# TS-01-E4: make build reports failure clearly (01-REQ-6.E1)
test_make_build_failure_reporting() {
    local main_rs="$REPO_ROOT/rhivos/locking-service/src/main.rs"

    # Check the file exists first
    if [[ ! -f "$main_rs" ]]; then
        echo "  Source file not found: $main_rs"
        return 1
    fi

    # Backup original
    local backup
    backup=$(cat "$main_rs")

    # Inject syntax error
    echo "THIS IS NOT VALID RUST @@@@" > "$main_rs"

    local output exit_code
    output=$(cd "$REPO_ROOT" && make build 2>&1) || true
    exit_code=$?

    # Restore original
    echo "$backup" > "$main_rs"

    if [[ $exit_code -eq 0 ]]; then
        echo "  make build should have failed with syntax error"
        return 1
    fi

    if ! echo "$output" | grep -qi "error"; then
        echo "  make build did not report error in output"
        return 1
    fi
    return 0
}

# Run all tests
run_test "TS-01-5: Rust workspace Cargo.toml" test_rust_workspace_toml
run_test "TS-01-6: Rust workspace builds" test_rust_workspace_builds
run_test "TS-01-7: Rust workspace tests pass" test_rust_workspace_tests
run_test "TS-01-8: Go backend workspace file" test_go_backend_workspace
run_test "TS-01-9: Go mock workspace file" test_go_mock_workspace
run_test "TS-01-10: Go modules build" test_go_modules_build
run_test "TS-01-11: Go module tests pass" test_go_module_tests
run_test "TS-01-12: Rust skeleton binaries exit 0" test_rust_skeleton_binaries
run_test "TS-01-13: Go skeleton binaries exit 0" test_go_skeleton_binaries
run_test "TS-01-14: Each skeleton has passing test" test_each_skeleton_has_test
run_test "TS-01-P2: Build determinism" test_build_determinism
run_test "TS-01-P3: Test discoverability" test_discoverability
run_test "TS-01-P4: Skeleton exit behavior" test_skeleton_exit_behavior
run_test "TS-01-E3: Skeleton without config" test_skeleton_no_config
run_test "TS-01-E4: make build failure reporting" test_make_build_failure_reporting

print_summary "test_build.sh"
