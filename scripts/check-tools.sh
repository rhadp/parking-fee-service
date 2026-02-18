#!/usr/bin/env bash
# check-tools.sh — Verify that required development tools are installed.
# Exits non-zero if any required tool is missing.

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

missing=0

check_tool() {
    local tool="$1"
    local version_flag="${2:---version}"

    if command -v "$tool" &>/dev/null; then
        version=$("$tool" $version_flag 2>&1 | head -n1)
        printf "${GREEN}✓${NC} %-25s %s\n" "$tool" "$version"
    else
        printf "${RED}✗${NC} %-25s %s\n" "$tool" "NOT FOUND"
        missing=1
    fi
}

echo "Checking required development tools..."
echo "======================================="

# Rust toolchain
check_tool cargo

# Go toolchain
check_tool go version

# Protocol Buffers compiler
check_tool protoc

# Go protobuf plugins
check_tool protoc-gen-go

check_tool protoc-gen-go-grpc

# Container runtime: accept either podman or docker
if command -v podman &>/dev/null; then
    version=$(podman --version 2>&1 | head -n1)
    printf "${GREEN}✓${NC} %-25s %s\n" "podman" "$version"
elif command -v docker &>/dev/null; then
    version=$(docker --version 2>&1 | head -n1)
    printf "${GREEN}✓${NC} %-25s %s\n" "docker (fallback)" "$version"
else
    printf "${RED}✗${NC} %-25s %s\n" "podman/docker" "NOT FOUND"
    missing=1
fi

echo "======================================="

if [ "$missing" -ne 0 ]; then
    echo ""
    printf "${RED}ERROR:${NC} One or more required tools are missing. Install them before proceeding.\n"
    exit 1
fi

echo ""
printf "${GREEN}All required tools are installed.${NC}\n"
exit 0
