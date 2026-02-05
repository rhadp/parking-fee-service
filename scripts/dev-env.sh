#!/usr/bin/env bash
# Copyright 2024 SDV Parking Demo System
# Development Environment Setup Script
#
# This script sets up environment variables for local development.
# Source this script to configure your shell for development.
#
# Usage:
#   source scripts/dev-env.sh [profile]
#
# Profiles:
#   local_tls     - Local development with self-signed TLS (default)
#   local_insecure - Local development without TLS
#   debug         - Debug mode with verbose logging
#   mock          - Mock mode for isolated testing
#   chaos         - Chaos testing with simulated failures
#
# Requirements: 5.6

# Script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

# Profile: local_tls
# Local development with TLS using self-signed certificates
setup_local_tls() {
    log_info "Setting up local_tls profile..."
    
    export SDV_TLS_SKIP_VERIFY=true
    export SDV_TLS_CA_FILE="${PROJECT_ROOT}/infra/certs/ca/ca.crt"
    export SDV_TLS_CERT_FILE="${PROJECT_ROOT}/infra/certs/client/client.crt"
    export SDV_TLS_KEY_FILE="${PROJECT_ROOT}/infra/certs/client/client.key"
    export SDV_LOG_LEVEL=debug
    
    # Service addresses for local development
    export SDV_DATABROKER_ADDRESS="localhost:55556"
    export SDV_UPDATE_SERVICE_ADDRESS="localhost:50051"
    export SDV_PARKING_ADAPTOR_ADDRESS="localhost:50052"
    export SDV_LOCKING_SERVICE_ADDRESS="localhost:50053"
    export SDV_MQTT_BROKER_ADDRESS="localhost:8883"
    
    log_warn "TLS verification is DISABLED. Use only in development."
    log_success "local_tls profile activated"
}

# Profile: local_insecure
# Local development without TLS (plain text)
setup_local_insecure() {
    log_info "Setting up local_insecure profile..."
    
    export SDV_TLS_DISABLED=true
    export SDV_LOG_LEVEL=debug
    
    # Service addresses for local development (non-TLS ports)
    export SDV_DATABROKER_ADDRESS="localhost:55556"
    export SDV_UPDATE_SERVICE_ADDRESS="localhost:50051"
    export SDV_PARKING_ADAPTOR_ADDRESS="localhost:50052"
    export SDV_LOCKING_SERVICE_ADDRESS="localhost:50053"
    export SDV_MQTT_BROKER_ADDRESS="localhost:1883"
    
    log_warn "TLS is DISABLED. All traffic is unencrypted!"
    log_success "local_insecure profile activated"
}

# Profile: debug
# Debug mode with maximum verbosity
setup_debug() {
    log_info "Setting up debug profile..."
    
    export SDV_LOG_LEVEL=trace
    export GRPC_VERBOSITY=DEBUG
    export GRPC_TRACE=all
    export RUST_BACKTRACE=1
    export RUST_LOG=debug
    
    log_success "debug profile activated"
}

# Profile: mock
# Mock mode for isolated testing
setup_mock() {
    log_info "Setting up mock profile..."
    
    export SDV_MOCK_MODE=true
    export SDV_LOG_LEVEL=debug
    
    log_success "mock profile activated"
}

# Profile: chaos
# Chaos testing with simulated failures and latency
setup_chaos() {
    log_info "Setting up chaos profile..."
    
    export SDV_SIMULATE_LATENCY_MS=100
    export SDV_SIMULATE_FAILURE_RATE=0.1
    export SDV_LOG_LEVEL=debug
    
    log_warn "Chaos mode enabled: 10% failure rate, 100ms latency"
    log_success "chaos profile activated"
}

# Clear all SDV environment variables
clear_env() {
    log_info "Clearing SDV environment variables..."
    
    unset SDV_TLS_SKIP_VERIFY
    unset SDV_TLS_DISABLED
    unset SDV_TLS_CA_FILE
    unset SDV_TLS_CERT_FILE
    unset SDV_TLS_KEY_FILE
    unset SDV_DATABROKER_ADDRESS
    unset SDV_UPDATE_SERVICE_ADDRESS
    unset SDV_PARKING_ADAPTOR_ADDRESS
    unset SDV_LOCKING_SERVICE_ADDRESS
    unset SDV_MQTT_BROKER_ADDRESS
    unset SDV_LOG_LEVEL
    unset SDV_MOCK_MODE
    unset SDV_SIMULATE_LATENCY_MS
    unset SDV_SIMULATE_FAILURE_RATE
    unset GRPC_VERBOSITY
    unset GRPC_TRACE
    
    log_success "Environment cleared"
}

# Print current environment
print_env() {
    echo ""
    echo "Current SDV Environment Variables:"
    echo "=================================="
    env | grep -E "^SDV_|^GRPC_" | sort || echo "  (none set)"
    echo ""
}

# Print usage information
usage() {
    cat << EOF
Usage: source $(basename "$0") [PROFILE]

Set up development environment for SDV Parking Demo System.

Profiles:
    local_tls       Local development with self-signed TLS (default)
    local_insecure  Local development without TLS (INSECURE)
    debug           Debug mode with verbose logging
    mock            Mock mode for isolated testing
    chaos           Chaos testing with simulated failures
    clear           Clear all SDV environment variables
    show            Show current environment variables

Examples:
    source $(basename "$0")              # Use default (local_tls)
    source $(basename "$0") debug        # Enable debug logging
    source $(basename "$0") clear        # Clear environment

⚠️  WARNING: Some profiles disable security features.
             Use only in development environments.

EOF
}

# Main entry point
main() {
    local profile="${1:-local_tls}"
    
    case "$profile" in
        local_tls)
            setup_local_tls
            ;;
        local_insecure)
            setup_local_insecure
            ;;
        debug)
            setup_debug
            ;;
        mock)
            setup_mock
            ;;
        chaos)
            setup_chaos
            ;;
        clear)
            clear_env
            ;;
        show)
            print_env
            ;;
        help|--help|-h)
            usage
            ;;
        *)
            log_error "Unknown profile: $profile"
            usage
            return 1
            ;;
    esac
    
    # Always show current environment after setup
    if [[ "$profile" != "show" && "$profile" != "help" && "$profile" != "--help" && "$profile" != "-h" ]]; then
        print_env
    fi
}

# Run main function if script is sourced
# (Don't run if just being parsed for functions)
if [[ "${BASH_SOURCE[0]}" != "${0}" ]]; then
    main "$@"
else
    echo "This script should be sourced, not executed directly."
    echo "Usage: source $(basename "$0") [profile]"
    exit 1
fi
