#!/usr/bin/env bash
# Test runner entry point for all project setup spec tests
# Runs all test scripts and reports combined results

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Load .env if present
if [ -f "$PROJECT_ROOT/.env" ]; then
    set -a
    # shellcheck source=/dev/null
    . "$PROJECT_ROOT/.env"
    set +a
fi
TOTAL_PASS=0
TOTAL_FAIL=0
SCRIPTS_RUN=0
SCRIPTS_FAILED=0

echo "========================================"
echo "  Project Setup Spec Tests"
echo "========================================"

# Define test scripts in order (infra tests last since they need Docker)
TEST_SCRIPTS=(
    "test_directories.sh"
    "test_build.sh"
    "test_makefile.sh"
    "test_mock_cli.sh"
    "test_infra.sh"
)

for script in "${TEST_SCRIPTS[@]}"; do
    script_path="$SCRIPT_DIR/$script"
    if [ -f "$script_path" ]; then
        echo ""
        echo "----------------------------------------"
        echo "Running: $script"
        echo "----------------------------------------"
        SCRIPTS_RUN=$((SCRIPTS_RUN + 1))
        if bash "$script_path"; then
            echo "  >>> $script: ALL PASSED"
        else
            SCRIPTS_FAILED=$((SCRIPTS_FAILED + 1))
            echo "  >>> $script: SOME FAILURES"
        fi
    else
        echo ""
        echo "  WARNING: $script not found"
    fi
done

echo ""
echo "========================================"
echo "  Summary"
echo "========================================"
echo "  Scripts run:    $SCRIPTS_RUN"
echo "  Scripts failed: $SCRIPTS_FAILED"
echo "========================================"

if [ "$SCRIPTS_FAILED" -gt 0 ]; then
    echo "  RESULT: FAIL"
    exit 1
else
    echo "  RESULT: PASS"
    exit 0
fi
