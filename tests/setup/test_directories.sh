#!/usr/bin/env bash
# Test directory structure requirements.
# Covers: TS-01-1, TS-01-2, TS-01-3, TS-01-4, TS-01-E1, TS-01-E2, TS-01-P1

set -euo pipefail
source "$(dirname "$0")/test_helpers.sh"

# TS-01-1: Top-level directory structure exists (01-REQ-1.1)
test_top_level_directories() {
    local result=0
    for dir in rhivos backend android mobile mock proto deployments; do
        assert_dir_exists "$REPO_ROOT/$dir" || result=1
    done
    return $result
}

# TS-01-2: Rust service subdirectories exist (01-REQ-1.2)
test_rust_service_directories() {
    local result=0
    for dir in locking-service cloud-gateway-client update-service parking-operator-adaptor mock-sensors; do
        assert_dir_exists "$REPO_ROOT/rhivos/$dir" || result=1
    done
    return $result
}

# TS-01-3: Go backend subdirectories exist (01-REQ-1.3)
test_go_backend_directories() {
    local result=0
    for dir in parking-fee-service cloud-gateway; do
        assert_dir_exists "$REPO_ROOT/backend/$dir" || result=1
    done
    return $result
}

# TS-01-4: Mock CLI subdirectories exist (01-REQ-1.4)
test_mock_cli_directories() {
    local result=0
    for dir in parking-app-cli companion-app-cli parking-operator; do
        assert_dir_exists "$REPO_ROOT/mock/$dir" || result=1
    done
    return $result
}

# TS-01-E1: Android placeholder directory (01-REQ-1.E1)
test_android_placeholder() {
    assert_dir_exists "$REPO_ROOT/android" || return 1
    assert_file_exists "$REPO_ROOT/android/README.md" || return 1
    assert_dir_entry_count "$REPO_ROOT/android" 1 || return 1
}

# TS-01-E2: Mobile placeholder directory (01-REQ-1.E2)
test_mobile_placeholder() {
    assert_dir_exists "$REPO_ROOT/mobile" || return 1
    assert_file_exists "$REPO_ROOT/mobile/README.md" || return 1
    assert_dir_entry_count "$REPO_ROOT/mobile" 1 || return 1
}

# TS-01-P1: Directory completeness property (Property 1)
test_directory_completeness() {
    local result=0
    local all_dirs=(
        "rhivos" "backend" "android" "mobile" "mock" "proto" "deployments"
        "rhivos/locking-service" "rhivos/cloud-gateway-client" "rhivos/update-service"
        "rhivos/parking-operator-adaptor" "rhivos/mock-sensors"
        "backend/parking-fee-service" "backend/cloud-gateway"
        "mock/parking-app-cli" "mock/companion-app-cli" "mock/parking-operator"
    )
    for dir in "${all_dirs[@]}"; do
        assert_dir_exists "$REPO_ROOT/$dir" || result=1
        assert_dir_not_empty "$REPO_ROOT/$dir" || result=1
    done
    return $result
}

# Run all tests
run_test "TS-01-1: Top-level directories exist" test_top_level_directories
run_test "TS-01-2: Rust service subdirectories exist" test_rust_service_directories
run_test "TS-01-3: Go backend subdirectories exist" test_go_backend_directories
run_test "TS-01-4: Mock CLI subdirectories exist" test_mock_cli_directories
run_test "TS-01-E1: Android placeholder" test_android_placeholder
run_test "TS-01-E2: Mobile placeholder" test_mobile_placeholder
run_test "TS-01-P1: Directory completeness" test_directory_completeness

print_summary "test_directories.sh"
