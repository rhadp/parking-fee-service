#!/usr/bin/env bash
# test_structure.sh — Verify that all required directories exist.
# Validates Property 7 (Directory Completeness) and Requirements 01-REQ-1.1 through 01-REQ-1.7.

set -euo pipefail

# Resolve project root (one level up from tests/)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

failures=0
checks=0

assert_dir() {
    local dir="$1"
    local req="$2"
    checks=$((checks + 1))

    if [ -d "$PROJECT_ROOT/$dir" ]; then
        printf "${GREEN}✓${NC} %-50s [%s]\n" "$dir/" "$req"
    else
        printf "${RED}✗${NC} %-50s [%s] MISSING\n" "$dir/" "$req"
        failures=$((failures + 1))
    fi
}

assert_file() {
    local file="$1"
    local req="$2"
    checks=$((checks + 1))

    if [ -f "$PROJECT_ROOT/$file" ]; then
        printf "${GREEN}✓${NC} %-50s [%s]\n" "$file" "$req"
    else
        printf "${RED}✗${NC} %-50s [%s] MISSING\n" "$file" "$req"
        failures=$((failures + 1))
    fi
}

echo "Verifying project directory structure..."
echo "========================================="

# 01-REQ-1.1: rhivos/ directory with Rust service sub-directories
assert_dir "rhivos"                             "01-REQ-1.1"
assert_dir "rhivos/locking-service"             "01-REQ-1.1"
assert_dir "rhivos/cloud-gateway-client"        "01-REQ-1.1"
assert_dir "rhivos/parking-operator-adaptor"    "01-REQ-1.1"
assert_dir "rhivos/update-service"              "01-REQ-1.1"

# 01-REQ-1.2: backend/ directory with Go service sub-directories
assert_dir "backend"                            "01-REQ-1.2"
assert_dir "backend/parking-fee-service"        "01-REQ-1.2"
assert_dir "backend/cloud-gateway"              "01-REQ-1.2"

# 01-REQ-1.3: android/ directory with placeholder sub-directories
assert_dir "android"                            "01-REQ-1.3"
assert_dir "android/parking-app"                "01-REQ-1.3"
assert_dir "android/companion-app"              "01-REQ-1.3"
assert_file "android/parking-app/.gitkeep"      "01-REQ-1.3"
assert_file "android/companion-app/.gitkeep"    "01-REQ-1.3"

# 01-REQ-1.4: proto/ directory with sub-directories
assert_dir "proto"                              "01-REQ-1.4"
assert_dir "proto/services"                     "01-REQ-1.4"
assert_dir "proto/common"                       "01-REQ-1.4"

# 01-REQ-1.5: mock/ directory with mock CLI applications
assert_dir "mock"                               "01-REQ-1.5"
assert_dir "mock/parking-app-cli"               "01-REQ-1.5"
assert_dir "mock/companion-app-cli"             "01-REQ-1.5"

# 01-REQ-1.6: mock/sensors/ directory
assert_dir "mock/sensors"                       "01-REQ-1.6"

# 01-REQ-1.7: Supporting directories
assert_dir "containers"                         "01-REQ-1.7"
assert_dir "infra"                              "01-REQ-1.7"
assert_dir "scripts"                            "01-REQ-1.7"
assert_dir "docs"                               "01-REQ-1.7"
assert_dir "tests"                              "01-REQ-1.7"

# Additional structure from design doc
assert_dir "containers/rhivos"                  "design"
assert_dir "containers/backend"                 "design"
assert_dir "containers/mock"                    "design"
assert_dir "infra/config/mosquitto"             "design"
assert_dir "proto/gen/go"                       "design"

# Root files
assert_file "Makefile"                          "01-REQ-5"
assert_file "scripts/check-tools.sh"            "01-REQ-5.E1"

echo "========================================="
echo "$checks checks, $failures failures"

if [ "$failures" -ne 0 ]; then
    printf "\n${RED}FAIL:${NC} $failures directories or files are missing.\n"
    exit 1
fi

printf "\n${GREEN}PASS:${NC} All required directories and files exist.\n"
exit 0
