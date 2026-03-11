#!/usr/bin/env bash
# Run all setup verification tests.
# Usage: bash tests/setup/run_all.sh [--skip-infra]
#
# Options:
#   --skip-infra   Skip infrastructure tests (requires Podman)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SKIP_INFRA=false

for arg in "$@"; do
    case "$arg" in
        --skip-infra) SKIP_INFRA=true ;;
    esac
done

OVERALL_RESULT=0

echo "============================================"
echo "  Project Setup Verification Tests"
echo "============================================"
echo ""

# Run each test suite
run_suite() {
    local name="$1"
    local script="$2"
    echo "--- $name ---"
    if bash "$script"; then
        echo ""
    else
        OVERALL_RESULT=1
        echo ""
    fi
}

run_suite "Directory Structure" "$SCRIPT_DIR/test_directories.sh"
run_suite "Build & Skeletons" "$SCRIPT_DIR/test_build.sh"
run_suite "Makefile & Proto" "$SCRIPT_DIR/test_makefile.sh"
run_suite "Mock CLI Apps" "$SCRIPT_DIR/test_mock_cli.sh"

if [[ "$SKIP_INFRA" == "false" ]]; then
    run_suite "Infrastructure" "$SCRIPT_DIR/test_infra.sh"
else
    echo "--- Infrastructure (SKIPPED) ---"
    echo ""
fi

echo "============================================"
if [[ $OVERALL_RESULT -eq 0 ]]; then
    echo "  ALL SUITES PASSED"
else
    echo "  SOME SUITES FAILED"
fi
echo "============================================"

exit $OVERALL_RESULT
