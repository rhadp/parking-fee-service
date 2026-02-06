#!/usr/bin/env bash
# Test: Service Health Verification
#
# Verifies all services are healthy and responding.
# Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5
#
# Usage:
#   ./test-service-health.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

pass() { echo -e "${GREEN}PASS${NC}: $1"; }
fail() { echo -e "${RED}FAIL${NC}: $1"; exit 1; }

echo "=== Service Health Tests ==="
echo ""

# Test 1: MOSQUITTO (port 1883)
echo -n "Testing MOSQUITTO... "
if nc -z localhost 1883 2>/dev/null; then
    pass "MOSQUITTO is listening on port 1883"
else
    fail "MOSQUITTO is not responding on port 1883"
fi

# Test 2: KUKSA_DATABROKER (port 55556)
echo -n "Testing KUKSA_DATABROKER... "
if nc -z localhost 55556 2>/dev/null; then
    pass "KUKSA_DATABROKER is listening on port 55556"
else
    fail "KUKSA_DATABROKER is not responding on port 55556"
fi

# Test 3: MOCK_PARKING_OPERATOR (HTTP health)
echo -n "Testing MOCK_PARKING_OPERATOR... "
if curl -sf http://localhost:8080/health >/dev/null 2>&1; then
    pass "MOCK_PARKING_OPERATOR health check passed"
else
    fail "MOCK_PARKING_OPERATOR health check failed"
fi

# Test 4: PARKING_FEE_SERVICE (HTTP health)
echo -n "Testing PARKING_FEE_SERVICE... "
if curl -sf http://localhost:8081/health >/dev/null 2>&1; then
    pass "PARKING_FEE_SERVICE health check passed"
else
    fail "PARKING_FEE_SERVICE health check failed"
fi

# Test 5: CLOUD_GATEWAY (HTTP health)
echo -n "Testing CLOUD_GATEWAY... "
if curl -sf http://localhost:8082/health >/dev/null 2>&1; then
    pass "CLOUD_GATEWAY health check passed"
else
    fail "CLOUD_GATEWAY health check failed"
fi

# Test 6: LOCKING_SERVICE (port 50053)
echo -n "Testing LOCKING_SERVICE... "
if nc -z localhost 50053 2>/dev/null; then
    pass "LOCKING_SERVICE is listening on port 50053"
else
    fail "LOCKING_SERVICE is not responding on port 50053"
fi

# Test 7: UPDATE_SERVICE (port 50051)
echo -n "Testing UPDATE_SERVICE... "
if nc -z localhost 50051 2>/dev/null; then
    pass "UPDATE_SERVICE is listening on port 50051"
else
    fail "UPDATE_SERVICE is not responding on port 50051"
fi

# Test 8: PARKING_OPERATOR_ADAPTOR (port 50052)
echo -n "Testing PARKING_OPERATOR_ADAPTOR... "
if nc -z localhost 50052 2>/dev/null; then
    pass "PARKING_OPERATOR_ADAPTOR is listening on port 50052"
else
    fail "PARKING_OPERATOR_ADAPTOR is not responding on port 50052"
fi

echo ""
echo "=== All service health tests passed ==="
