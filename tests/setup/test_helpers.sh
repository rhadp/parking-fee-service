#!/usr/bin/env bash
# Test helper functions for setup verification tests.
# Source this file from other test scripts.

set -euo pipefail

# Determine repo root (two levels up from tests/setup/)
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Colors (if terminal supports them)
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    NC='\033[0m' # No Color
else
    RED=''
    GREEN=''
    YELLOW=''
    NC=''
fi

# Run a test case. Usage: run_test "test name" test_function
run_test() {
    local name="$1"
    local func="$2"
    TESTS_RUN=$((TESTS_RUN + 1))
    if $func; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "${GREEN}PASS${NC}: $name"
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo -e "${RED}FAIL${NC}: $name"
    fi
}

# Assert a directory exists
assert_dir_exists() {
    local dir="$1"
    if [[ -d "$dir" ]]; then
        return 0
    else
        echo "  Expected directory to exist: $dir"
        return 1
    fi
}

# Assert a file exists
assert_file_exists() {
    local file="$1"
    if [[ -f "$file" ]]; then
        return 0
    else
        echo "  Expected file to exist: $file"
        return 1
    fi
}

# Assert a string is found in a file
assert_file_contains() {
    local file="$1"
    local pattern="$2"
    if grep -q "$pattern" "$file" 2>/dev/null; then
        return 0
    else
        echo "  Expected '$pattern' in $file"
        return 1
    fi
}

# Assert a command succeeds (exit code 0)
assert_command_succeeds() {
    local desc="$1"
    shift
    if "$@" >/dev/null 2>&1; then
        return 0
    else
        echo "  Command failed: $desc"
        return 1
    fi
}

# Assert a command fails (non-zero exit code)
assert_command_fails() {
    local desc="$1"
    shift
    if "$@" >/dev/null 2>&1; then
        echo "  Expected command to fail: $desc"
        return 1
    else
        return 0
    fi
}

# Assert a directory has exactly N entries (files/dirs, excluding . and ..)
assert_dir_entry_count() {
    local dir="$1"
    local expected="$2"
    local actual
    actual=$(ls -1A "$dir" 2>/dev/null | wc -l | tr -d ' ')
    if [[ "$actual" -eq "$expected" ]]; then
        return 0
    else
        echo "  Expected $expected entries in $dir, got $actual"
        return 1
    fi
}

# Assert a directory is non-empty
assert_dir_not_empty() {
    local dir="$1"
    local count
    count=$(ls -1A "$dir" 2>/dev/null | wc -l | tr -d ' ')
    if [[ "$count" -gt 0 ]]; then
        return 0
    else
        echo "  Expected non-empty directory: $dir"
        return 1
    fi
}

# Assert stdout of a command contains a pattern
assert_output_contains() {
    local pattern="$1"
    local output="$2"
    if echo "$output" | grep -qi "$pattern"; then
        return 0
    else
        echo "  Expected output to contain '$pattern'"
        return 1
    fi
}

# Print test summary and exit with appropriate code
print_summary() {
    local script_name="${1:-tests}"
    echo ""
    echo "=========================================="
    echo "$script_name: $TESTS_PASSED/$TESTS_RUN passed, $TESTS_FAILED failed"
    echo "=========================================="
    if [[ "$TESTS_FAILED" -gt 0 ]]; then
        return 1
    fi
    return 0
}
