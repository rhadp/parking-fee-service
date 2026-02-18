#!/usr/bin/env bash
# test_proto.sh — Verify that all .proto files are syntactically valid and
# that generated Go bindings compile.
# Validates Property 2 (Proto-Binding Consistency) and Requirements 01-REQ-4.1
# through 01-REQ-4.6.

set -euo pipefail

# Resolve project root (one level up from tests/)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

failures=0
checks=0

pass() {
    local msg="$1"
    local req="$2"
    checks=$((checks + 1))
    printf "${GREEN}✓${NC} %-60s [%s]\n" "$msg" "$req"
}

fail() {
    local msg="$1"
    local req="$2"
    checks=$((checks + 1))
    printf "${RED}✗${NC} %-60s [%s] FAILED\n" "$msg" "$req"
    failures=$((failures + 1))
}

echo "Verifying proto definitions..."
echo "========================================="

# ─── Check proto files exist ──────────────────────────────────────────────

COMMON_PROTO="$PROJECT_ROOT/proto/common/common.proto"
UPDATE_PROTO="$PROJECT_ROOT/proto/services/update_service.proto"
ADAPTER_PROTO="$PROJECT_ROOT/proto/services/parking_adapter.proto"

for proto_file in "$COMMON_PROTO" "$UPDATE_PROTO" "$ADAPTER_PROTO"; do
    rel_path="${proto_file#$PROJECT_ROOT/}"
    if [ -f "$proto_file" ]; then
        pass "$rel_path exists" "01-REQ-4"
    else
        fail "$rel_path exists" "01-REQ-4"
    fi
done

# ─── Check proto3 syntax ─────────────────────────────────────────────────

for proto_file in "$COMMON_PROTO" "$UPDATE_PROTO" "$ADAPTER_PROTO"; do
    rel_path="${proto_file#$PROJECT_ROOT/}"
    if grep -q 'syntax = "proto3"' "$proto_file" 2>/dev/null; then
        pass "$rel_path uses proto3 syntax" "01-REQ-4.6"
    else
        fail "$rel_path uses proto3 syntax" "01-REQ-4.6"
    fi
done

# ─── Check common.proto defines required types ──────────────────────────

for msg_type in "Location" "VehicleId" "AdapterInfo" "ErrorDetails"; do
    if grep -q "message $msg_type" "$COMMON_PROTO" 2>/dev/null; then
        pass "common.proto defines message $msg_type" "01-REQ-4.3"
    else
        fail "common.proto defines message $msg_type" "01-REQ-4.3"
    fi
done

if grep -q "enum AdapterState" "$COMMON_PROTO" 2>/dev/null; then
    pass "common.proto defines enum AdapterState" "01-REQ-4.3"
else
    fail "common.proto defines enum AdapterState" "01-REQ-4.3"
fi

# ─── Check update_service.proto defines required RPCs ───────────────────

for rpc_name in "InstallAdapter" "WatchAdapterStates" "ListAdapters" "RemoveAdapter" "GetAdapterStatus"; do
    if grep -q "rpc $rpc_name" "$UPDATE_PROTO" 2>/dev/null; then
        pass "update_service.proto defines RPC $rpc_name" "01-REQ-4.1"
    else
        fail "update_service.proto defines RPC $rpc_name" "01-REQ-4.1"
    fi
done

# Check WatchAdapterStates is server streaming
if grep -q "returns (stream" "$UPDATE_PROTO" 2>/dev/null; then
    pass "update_service.proto has server-streaming RPC" "01-REQ-4.1"
else
    fail "update_service.proto has server-streaming RPC" "01-REQ-4.1"
fi

# ─── Check parking_adapter.proto defines required RPCs ──────────────────

for rpc_name in "StartSession" "StopSession" "GetStatus" "GetRate"; do
    if grep -q "rpc $rpc_name" "$ADAPTER_PROTO" 2>/dev/null; then
        pass "parking_adapter.proto defines RPC $rpc_name" "01-REQ-4.2"
    else
        fail "parking_adapter.proto defines RPC $rpc_name" "01-REQ-4.2"
    fi
done

# ─── Validate protoc compilation ────────────────────────────────────────

echo ""
echo "Running protoc syntax validation..."

if protoc \
    --proto_path="$PROJECT_ROOT/proto" \
    --descriptor_set_out=/dev/null \
    "$COMMON_PROTO" "$UPDATE_PROTO" "$ADAPTER_PROTO" 2>&1; then
    pass "protoc compiles all .proto files without errors" "01-REQ-4.E1"
else
    fail "protoc compiles all .proto files without errors" "01-REQ-4.E1"
fi

# ─── Validate generated Go packages compile ─────────────────────────────

echo ""
echo "Checking generated Go packages..."

GO_GEN_DIR="$PROJECT_ROOT/proto/gen/go"

# Check generated directories exist
for gen_dir in "common" "services/update" "services/adapter"; do
    if [ -d "$GO_GEN_DIR/$gen_dir" ]; then
        pass "Generated Go package $gen_dir/ exists" "01-REQ-4.4"
    else
        fail "Generated Go package $gen_dir/ exists" "01-REQ-4.4"
    fi
done

# Check generated Go files exist
for gen_file in "common/common.pb.go" \
    "services/update/update_service.pb.go" "services/update/update_service_grpc.pb.go" \
    "services/adapter/parking_adapter.pb.go" "services/adapter/parking_adapter_grpc.pb.go"; do
    if [ -f "$GO_GEN_DIR/$gen_file" ]; then
        pass "Generated file $gen_file exists" "01-REQ-4.4"
    else
        fail "Generated file $gen_file exists" "01-REQ-4.4"
    fi
done

# Verify generated Go packages compile
if [ -f "$GO_GEN_DIR/go.mod" ]; then
    if (cd "$GO_GEN_DIR" && go build ./... 2>&1); then
        pass "Generated Go packages compile successfully" "01-REQ-4.4"
    else
        fail "Generated Go packages compile successfully" "01-REQ-4.4"
    fi
else
    fail "go.mod exists in proto/gen/go/" "01-REQ-4.4"
fi

# ─── Summary ────────────────────────────────────────────────────────────

echo ""
echo "========================================="
echo "$checks checks, $failures failures"

if [ "$failures" -ne 0 ]; then
    printf "\n${RED}FAIL:${NC} $failures checks failed.\n"
    exit 1
fi

printf "\n${GREEN}PASS:${NC} All proto definition checks passed.\n"
exit 0
