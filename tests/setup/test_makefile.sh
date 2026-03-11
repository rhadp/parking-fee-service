#!/usr/bin/env bash
# Test Makefile targets and proto definitions.
# Covers: TS-01-15, TS-01-16, TS-01-17, TS-01-18, TS-01-19, TS-01-25, TS-01-P6

set -euo pipefail
source "$(dirname "$0")/test_helpers.sh"

# TS-01-15: Proto directory contains valid proto3 file (01-REQ-5.1, 01-REQ-5.2)
test_proto_valid() {
    # Find .proto files
    local proto_files
    proto_files=$(find "$REPO_ROOT/proto" -name "*.proto" 2>/dev/null)
    if [[ -z "$proto_files" ]]; then
        echo "  No .proto files found in proto/"
        return 1
    fi

    local result=0
    while IFS= read -r proto_file; do
        if ! grep -q 'syntax = "proto3"' "$proto_file"; then
            echo "  Missing proto3 syntax declaration in $proto_file"
            result=1
        fi
        if ! grep -q 'package ' "$proto_file"; then
            echo "  Missing package declaration in $proto_file"
            result=1
        fi
    done <<< "$proto_files"
    return $result
}

# TS-01-16: Root Makefile has all required targets (01-REQ-6.1)
test_makefile_targets() {
    local makefile="$REPO_ROOT/Makefile"
    assert_file_exists "$makefile" || return 1
    local result=0
    for target in build test lint clean infra-up infra-down; do
        if ! grep -q "^${target}:" "$makefile" && ! grep -q "^${target} *:" "$makefile"; then
            # Also check for targets defined with dependencies on same line
            if ! grep -qE "^${target}[[:space:]]*:" "$makefile"; then
                echo "  Missing target: $target"
                result=1
            fi
        fi
    done
    return $result
}

# TS-01-17: make build succeeds (01-REQ-6.2)
test_make_build() {
    local output
    output=$(cd "$REPO_ROOT" && make build 2>&1)
    local exit_code=$?
    if [[ $exit_code -ne 0 ]]; then
        echo "  make build failed with exit code $exit_code"
        echo "  $output"
        return 1
    fi
    return 0
}

# TS-01-18: make test succeeds (01-REQ-6.3)
test_make_test() {
    local output
    output=$(cd "$REPO_ROOT" && make test 2>&1)
    local exit_code=$?
    if [[ $exit_code -ne 0 ]]; then
        echo "  make test failed with exit code $exit_code"
        echo "  $output"
        return 1
    fi
    return 0
}

# TS-01-19: make clean removes build artifacts (01-REQ-6.4)
test_make_clean() {
    # Build first to create artifacts
    (cd "$REPO_ROOT" && make build 2>&1) >/dev/null || {
        echo "  make build failed (prerequisite for clean test)"
        return 1
    }

    # Verify target directory exists after build
    if [[ ! -d "$REPO_ROOT/rhivos/target" ]]; then
        echo "  rhivos/target/ not found after build"
        return 1
    fi

    # Clean
    (cd "$REPO_ROOT" && make clean 2>&1) >/dev/null || {
        echo "  make clean failed"
        return 1
    }

    # Verify target directory is removed
    if [[ -d "$REPO_ROOT/rhivos/target" ]]; then
        echo "  rhivos/target/ still exists after make clean"
        return 1
    fi
    return 0
}

# TS-01-25: make test runs all component tests (01-REQ-9.3)
test_make_test_all_components() {
    local output
    output=$(cd "$REPO_ROOT" && make test 2>&1)
    local exit_code=$?
    if [[ $exit_code -ne 0 ]]; then
        echo "  make test failed"
        return 1
    fi

    local result=0
    # Check for Rust test output
    if ! echo "$output" | grep -q "test result"; then
        echo "  make test output missing Rust test results"
        result=1
    fi

    # Check for Go test output
    if ! echo "$output" | grep -qi "ok\|PASS"; then
        echo "  make test output missing Go test results"
        result=1
    fi
    return $result
}

# TS-01-P6: Proto validity property (Property 6)
test_proto_validity_property() {
    local proto_files
    proto_files=$(find "$REPO_ROOT/proto" -name "*.proto" 2>/dev/null)
    if [[ -z "$proto_files" ]]; then
        echo "  No .proto files found"
        return 1
    fi

    local count=0
    local result=0
    while IFS= read -r f; do
        count=$((count + 1))
        if ! grep -q 'syntax = "proto3"' "$f"; then
            echo "  File $f is not proto3"
            result=1
        fi
    done <<< "$proto_files"

    if [[ $count -lt 1 ]]; then
        echo "  Expected at least 1 proto file, found $count"
        return 1
    fi
    return $result
}

# Run all tests
run_test "TS-01-15: Proto valid proto3 file" test_proto_valid
run_test "TS-01-16: Makefile has required targets" test_makefile_targets
run_test "TS-01-17: make build succeeds" test_make_build
run_test "TS-01-18: make test succeeds" test_make_test
run_test "TS-01-19: make clean removes artifacts" test_make_clean
run_test "TS-01-25: make test all components" test_make_test_all_components
run_test "TS-01-P6: Proto validity property" test_proto_validity_property

print_summary "test_makefile.sh"
