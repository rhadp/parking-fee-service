#!/usr/bin/env bash
# Copyright 2024 SDV Parking Demo System
# Local Development Environment Orchestration Script
#
# This script manages the complete local development environment for the
# SDV Parking Demo System. It starts/stops infrastructure containers and
# RHIVOS native processes.
#
# Usage:
#   ./scripts/local-dev.sh start   # Start all services
#   ./scripts/local-dev.sh stop    # Stop all services
#   ./scripts/local-dev.sh status  # Check service health
#   ./scripts/local-dev.sh test    # Run integration tests
#   ./scripts/local-dev.sh logs    # Tail all service logs
#
# Requirements: 1.4, 1.5, 2.5

set -e

# Script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Directories
LOGS_DIR="${PROJECT_ROOT}/logs"
PIDS_DIR="${LOGS_DIR}/pids"
COMPOSE_DIR="${PROJECT_ROOT}/infra/compose"
RHIVOS_DIR="${PROJECT_ROOT}/rhivos"

# Timeouts (seconds)
HEALTH_TIMEOUT=60
SHUTDOWN_TIMEOUT=5

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

#------------------------------------------------------------------------------
# Logging Functions
#------------------------------------------------------------------------------

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

log_status() {
    local service="$1"
    local status="$2"
    local port="$3"
    if [[ "$status" == "healthy" ]]; then
        printf "  ${GREEN}%-30s${NC} :%s ${GREEN}%s${NC}\n" "$service" "$port" "[healthy]"
    else
        printf "  ${RED}%-30s${NC} :%s ${RED}%s${NC}\n" "$service" "$port" "[unhealthy]"
    fi
}

#------------------------------------------------------------------------------
# Utility Functions
#------------------------------------------------------------------------------

# Check if a port is listening
check_port() {
    local port="$1"
    nc -z localhost "$port" 2>/dev/null
}

# Check HTTP health endpoint
check_http_health() {
    local url="$1"
    curl -sf "$url" >/dev/null 2>&1
}

# Wait for a port to be listening
wait_for_port() {
    local port="$1"
    local service="$2"
    local timeout="${3:-$HEALTH_TIMEOUT}"
    local elapsed=0

    while ! check_port "$port"; do
        if [[ $elapsed -ge $timeout ]]; then
            log_error "$service: Port $port not listening after ${timeout}s"
            return 1
        fi
        sleep 1
        ((elapsed++))
    done
    return 0
}

# Wait for HTTP health endpoint
wait_for_health() {
    local url="$1"
    local service="$2"
    local timeout="${3:-$HEALTH_TIMEOUT}"
    local elapsed=0

    while ! check_http_health "$url"; do
        if [[ $elapsed -ge $timeout ]]; then
            log_error "$service: Health check failed after ${timeout}s ($url)"
            return 1
        fi
        sleep 1
        ((elapsed++))
    done
    return 0
}

# Ensure directories exist
ensure_dirs() {
    mkdir -p "$LOGS_DIR" "$PIDS_DIR"
}

# Check if service is running via PID file
is_service_running() {
    local service="$1"
    local pidfile="${PIDS_DIR}/${service}.pid"

    if [[ -f "$pidfile" ]]; then
        local pid
        pid=$(cat "$pidfile")
        if kill -0 "$pid" 2>/dev/null; then
            return 0
        fi
    fi
    return 1
}

#------------------------------------------------------------------------------
# Infrastructure Management (Containers)
#------------------------------------------------------------------------------

start_infrastructure() {
    log_info "Starting infrastructure containers..."

    cd "$COMPOSE_DIR"

    # Start containers
    if ! podman-compose up -d 2>&1; then
        log_error "Failed to start infrastructure containers"
        return 1
    fi

    # Wait for services to be healthy
    log_info "Waiting for infrastructure services..."

    # MOSQUITTO (port check)
    if wait_for_port 1883 "MOSQUITTO" 30; then
        log_success "MOSQUITTO is ready on :1883"
    else
        return 1
    fi

    # KUKSA_DATABROKER (port check - container has no shell for health check)
    if wait_for_port 55556 "KUKSA_DATABROKER" 30; then
        log_success "KUKSA_DATABROKER is ready on :55556"
    else
        return 1
    fi

    # MOCK_PARKING_OPERATOR (HTTP health)
    if wait_for_health "http://localhost:8080/health" "MOCK_PARKING_OPERATOR" 30; then
        log_success "MOCK_PARKING_OPERATOR is ready on :8080"
    else
        return 1
    fi

    # PARKING_FEE_SERVICE (HTTP health)
    if wait_for_health "http://localhost:8081/health" "PARKING_FEE_SERVICE" 30; then
        log_success "PARKING_FEE_SERVICE is ready on :8081"
    else
        return 1
    fi

    # CLOUD_GATEWAY (HTTP health)
    if wait_for_health "http://localhost:8082/health" "CLOUD_GATEWAY" 30; then
        log_success "CLOUD_GATEWAY is ready on :8082"
    else
        return 1
    fi

    cd "$PROJECT_ROOT"
    return 0
}

stop_infrastructure() {
    log_info "Stopping infrastructure containers..."

    cd "$COMPOSE_DIR"
    podman-compose down 2>&1 || true
    cd "$PROJECT_ROOT"

    log_success "Infrastructure containers stopped"
}

#------------------------------------------------------------------------------
# RHIVOS Service Management (Native Processes)
#------------------------------------------------------------------------------

# Generic function to start a RHIVOS service
start_rhivos_service() {
    local service="$1"
    local binary="$2"
    local port="$3"
    shift 3
    local env_vars=("$@")

    local pidfile="${PIDS_DIR}/${service}.pid"
    local logfile="${LOGS_DIR}/${service}.log"

    # Check if already running
    if is_service_running "$service"; then
        log_warn "$service is already running"
        return 0
    fi

    # Check binary exists
    if [[ ! -x "$binary" ]]; then
        log_error "$service: Binary not found at $binary"
        return 1
    fi

    log_info "Starting $service..."

    # Export environment variables and start service
    (
        for env_var in "${env_vars[@]}"; do
            export "$env_var"
        done
        nohup "$binary" > "$logfile" 2>&1 &
        echo $! > "$pidfile"
    )

    # Wait for service to be ready
    if [[ -n "$port" ]]; then
        if wait_for_port "$port" "$service" 30; then
            log_success "$service is ready on :$port"
        else
            log_error "$service failed to start (see $logfile)"
            return 1
        fi
    else
        # For services without a port, just check the PID
        sleep 2
        if is_service_running "$service"; then
            log_success "$service is running"
        else
            log_error "$service failed to start (see $logfile)"
            return 1
        fi
    fi

    return 0
}

start_locking_service() {
    start_rhivos_service \
        "locking-service" \
        "${RHIVOS_DIR}/target/debug/locking-service" \
        "50053" \
        "RUST_LOG=debug" \
        "LOCKING_SOCKET_PATH=tcp://0.0.0.0:50053" \
        "DATA_BROKER_SOCKET=localhost:55556"
}

start_update_service() {
    start_rhivos_service \
        "update-service" \
        "${RHIVOS_DIR}/target/debug/update-service" \
        "50051" \
        "RUST_LOG=debug" \
        "UPDATE_SERVICE_LISTEN_ADDR=0.0.0.0:50051" \
        "DATA_BROKER_SOCKET=localhost:55556" \
        "UPDATE_SERVICE_STORAGE_PATH=/tmp/sdv-adapters"
}

start_parking_operator_adaptor() {
    start_rhivos_service \
        "parking-operator-adaptor" \
        "${RHIVOS_DIR}/target/debug/parking-operator-adaptor" \
        "50052" \
        "RUST_LOG=debug" \
        "LISTEN_ADDR=0.0.0.0:50052" \
        "DATA_BROKER_SOCKET=localhost:55556" \
        "OPERATOR_BASE_URL=http://localhost:8080/api/v1" \
        "PARKING_FEE_SERVICE_URL=http://localhost:8081/api/v1" \
        "STORAGE_PATH=/tmp/sdv-parking-session.json"
}

start_cloud_gateway_client() {
    start_rhivos_service \
        "cloud-gateway-client" \
        "${RHIVOS_DIR}/target/debug/cloud-gateway-client" \
        "" \
        "RUST_LOG=debug" \
        "CGC_VIN=DEMO_VIN_001" \
        "CGC_MQTT_BROKER_URL=mqtt://localhost:1883" \
        "CGC_LOCKING_SERVICE_SOCKET=localhost:50053" \
        "CGC_DATA_BROKER_SOCKET=localhost:55556"
}

stop_rhivos_service() {
    local service="$1"
    local pidfile="${PIDS_DIR}/${service}.pid"

    if [[ ! -f "$pidfile" ]]; then
        return 0
    fi

    local pid
    pid=$(cat "$pidfile")

    if kill -0 "$pid" 2>/dev/null; then
        log_info "Stopping $service (PID: $pid)..."
        kill -TERM "$pid" 2>/dev/null || true

        # Wait for graceful shutdown
        local elapsed=0
        while kill -0 "$pid" 2>/dev/null && [[ $elapsed -lt $SHUTDOWN_TIMEOUT ]]; do
            sleep 1
            ((elapsed++))
        done

        # Force kill if still running
        if kill -0 "$pid" 2>/dev/null; then
            log_warn "$service did not stop gracefully, sending SIGKILL"
            kill -KILL "$pid" 2>/dev/null || true
        fi
    fi

    rm -f "$pidfile"
}

stop_rhivos_services() {
    log_info "Stopping RHIVOS services..."

    stop_rhivos_service "cloud-gateway-client"
    stop_rhivos_service "parking-operator-adaptor"
    stop_rhivos_service "update-service"
    stop_rhivos_service "locking-service"

    log_success "RHIVOS services stopped"
}

#------------------------------------------------------------------------------
# Command Implementations
#------------------------------------------------------------------------------

cmd_start() {
    log_info "Starting local development environment..."
    ensure_dirs

    # Start infrastructure
    start_infrastructure || exit 1

    # Start RHIVOS services
    start_locking_service || exit 1
    start_update_service || exit 1
    start_parking_operator_adaptor || exit 1
    start_cloud_gateway_client || exit 1

    echo ""
    log_success "Local development environment is ready!"
    echo ""
    echo "CLI Simulator Configuration:"
    echo "  export CLOUD_GATEWAY_URL=http://localhost:8082"
    echo "  export DATA_BROKER_ADDR=localhost:55556"
    echo "  export PARKING_FEE_SERVICE_URL=http://localhost:8081"
    echo "  export UPDATE_SERVICE_ADDR=localhost:50051"
    echo "  export PARKING_ADAPTOR_ADDR=localhost:50052"
    echo "  export LOCKING_SERVICE_ADDR=localhost:50053"
    echo ""
    echo "Or run: source scripts/dev-env.sh local_insecure"
    echo ""
}

cmd_stop() {
    log_info "Stopping local development environment..."

    # Stop RHIVOS first
    stop_rhivos_services

    # Stop infrastructure
    stop_infrastructure

    echo ""
    log_success "Local development environment stopped"
}

cmd_status() {
    echo ""
    echo "Service Health Status:"
    echo "======================"
    echo ""

    local all_healthy=true

    # Infrastructure services
    echo "Infrastructure (Containers):"

    if check_port 1883; then
        log_status "MOSQUITTO" "healthy" "1883"
    else
        log_status "MOSQUITTO" "unhealthy" "1883"
        all_healthy=false
    fi

    if check_port 55556; then
        log_status "KUKSA_DATABROKER" "healthy" "55556"
    else
        log_status "KUKSA_DATABROKER" "unhealthy" "55556"
        all_healthy=false
    fi

    if check_http_health "http://localhost:8080/health"; then
        log_status "MOCK_PARKING_OPERATOR" "healthy" "8080"
    else
        log_status "MOCK_PARKING_OPERATOR" "unhealthy" "8080"
        all_healthy=false
    fi

    if check_http_health "http://localhost:8081/health"; then
        log_status "PARKING_FEE_SERVICE" "healthy" "8081"
    else
        log_status "PARKING_FEE_SERVICE" "unhealthy" "8081"
        all_healthy=false
    fi

    if check_http_health "http://localhost:8082/health"; then
        log_status "CLOUD_GATEWAY" "healthy" "8082"
    else
        log_status "CLOUD_GATEWAY" "unhealthy" "8082"
        all_healthy=false
    fi

    echo ""
    echo "RHIVOS Services (Native):"

    if check_port 50053; then
        log_status "LOCKING_SERVICE" "healthy" "50053"
    else
        log_status "LOCKING_SERVICE" "unhealthy" "50053"
        all_healthy=false
    fi

    if check_port 50051; then
        log_status "UPDATE_SERVICE" "healthy" "50051"
    else
        log_status "UPDATE_SERVICE" "unhealthy" "50051"
        all_healthy=false
    fi

    if check_port 50052; then
        log_status "PARKING_OPERATOR_ADAPTOR" "healthy" "50052"
    else
        log_status "PARKING_OPERATOR_ADAPTOR" "unhealthy" "50052"
        all_healthy=false
    fi

    if is_service_running "cloud-gateway-client"; then
        printf "  ${GREEN}%-30s${NC} ${GREEN}%s${NC}\n" "CLOUD_GATEWAY_CLIENT" "[running]"
    else
        printf "  ${RED}%-30s${NC} ${RED}%s${NC}\n" "CLOUD_GATEWAY_CLIENT" "[not running]"
        all_healthy=false
    fi

    echo ""

    if $all_healthy; then
        log_success "All services are healthy"
        return 0
    else
        log_error "Some services are unhealthy"
        return 1
    fi
}

cmd_test() {
    log_info "Running integration tests..."

    # First check status
    if ! cmd_status; then
        log_error "Cannot run tests - some services are unhealthy"
        log_info "Run 'make dev-up' to start the environment first"
        exit 1
    fi

    echo ""
    log_info "Executing TMT tests..."

    # Check if tmt is available
    if ! command -v tmt &> /dev/null; then
        log_warn "TMT is not installed. Running basic connectivity tests instead..."

        # Basic connectivity tests
        echo ""
        echo "Basic Connectivity Tests:"
        echo "========================="

        # Test companion-cli
        if [[ -x "${PROJECT_ROOT}/backend/bin/companion-cli" ]]; then
            if "${PROJECT_ROOT}/backend/bin/companion-cli" -c "help" &>/dev/null; then
                log_success "companion-cli: help command works"
            else
                log_error "companion-cli: help command failed"
            fi
        else
            log_warn "companion-cli not built - run 'make build-cli'"
        fi

        # Test parking-cli
        if [[ -x "${PROJECT_ROOT}/backend/bin/parking-cli" ]]; then
            if "${PROJECT_ROOT}/backend/bin/parking-cli" -c "help" &>/dev/null; then
                log_success "parking-cli: help command works"
            else
                log_error "parking-cli: help command failed"
            fi
        else
            log_warn "parking-cli not built - run 'make build-cli'"
        fi

        echo ""
        return 0
    fi

    # Run TMT tests
    cd "${PROJECT_ROOT}/tests/integration"
    tmt run --all
    cd "$PROJECT_ROOT"
}

cmd_logs() {
    log_info "Tailing service logs (Ctrl+C to exit)..."
    echo ""

    if [[ ! -d "$LOGS_DIR" ]] || [[ -z "$(ls -A "$LOGS_DIR"/*.log 2>/dev/null)" ]]; then
        log_warn "No log files found in $LOGS_DIR"
        log_info "Showing container logs instead..."
        cd "$COMPOSE_DIR"
        podman-compose logs -f
        return
    fi

    # Tail all log files with service name prefix
    tail -f "$LOGS_DIR"/*.log 2>/dev/null || log_warn "No log files to tail"
}

usage() {
    cat << EOF
SDV Parking Demo - Local Development Environment

Usage: $(basename "$0") <command>

Commands:
    start    Start all services (infrastructure + RHIVOS)
    stop     Stop all services
    status   Check service health status
    test     Run integration tests
    logs     Tail all service logs (Ctrl+C to exit)
    help     Show this help message

Examples:
    $(basename "$0") start     # Start the complete environment
    $(basename "$0") status    # Check if services are healthy
    $(basename "$0") stop      # Stop everything

Environment:
    After 'start', configure CLI simulators with:
    source scripts/dev-env.sh local_insecure

Port Assignments:
    MOSQUITTO:              1883 (MQTT), 8883 (MQTT/TLS)
    KUKSA_DATABROKER:       55556 (gRPC)
    MOCK_PARKING_OPERATOR:  8080 (HTTP)
    PARKING_FEE_SERVICE:    8081 (HTTP)
    CLOUD_GATEWAY:          8082 (HTTP)
    UPDATE_SERVICE:         50051 (gRPC)
    PARKING_OPERATOR_ADAPTOR: 50052 (gRPC)
    LOCKING_SERVICE:        50053 (gRPC)

EOF
}

#------------------------------------------------------------------------------
# Main Entry Point
#------------------------------------------------------------------------------

main() {
    local command="${1:-help}"

    case "$command" in
        start)
            cmd_start
            ;;
        stop)
            cmd_stop
            ;;
        status)
            cmd_status
            ;;
        test)
            cmd_test
            ;;
        logs)
            cmd_logs
            ;;
        help|--help|-h)
            usage
            ;;
        *)
            log_error "Unknown command: $command"
            usage
            exit 1
            ;;
    esac
}

main "$@"
