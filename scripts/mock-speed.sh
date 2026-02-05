#!/usr/bin/env bash
# Copyright 2024 SDV Parking Demo System
# Mock Speed Signal Generator
#
# This script generates mock vehicle speed signals and publishes them to the
# DATA_BROKER (Eclipse Kuksa Databroker) for testing and demos.
#
# Usage:
#   ./scripts/mock-speed.sh [OPTIONS]
#
# Options:
#   --speed KMH         Starting speed in km/h (default: 0)
#   --max-speed KMH     Maximum speed in km/h (default: 120)
#   --interval SECS     Update interval in seconds (default: 1)
#   --acceleration KMH  Speed change per update in km/h (default: 5)
#   --count N           Number of updates (default: infinite)
#   --pattern PATTERN   Speed pattern: constant, accelerate, decelerate, random, parking (default: random)
#   --address ADDR      Databroker address (default: localhost:55556)
#   --help              Show this help message
#
# Requirements: 7.4
#
# Examples:
#   ./scripts/mock-speed.sh                              # Random speed changes
#   ./scripts/mock-speed.sh --speed 60 --pattern constant  # Constant 60 km/h
#   ./scripts/mock-speed.sh --pattern parking            # Simulate parking (decel to 0)
#   ./scripts/mock-speed.sh --pattern accelerate --max-speed 100  # Accelerate to 100

set -euo pipefail

# Script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Default configuration
DEFAULT_SPEED=0              # Starting speed in km/h
DEFAULT_MAX_SPEED=120        # Maximum speed in km/h
DEFAULT_INTERVAL=1           # 1 second between updates
DEFAULT_ACCELERATION=5       # 5 km/h change per update
DEFAULT_COUNT=-1             # Infinite updates
DEFAULT_PATTERN="random"     # Random speed changes
DEFAULT_ADDRESS="localhost:55556"

# Current configuration
SPEED="${DEFAULT_SPEED}"
MAX_SPEED="${DEFAULT_MAX_SPEED}"
INTERVAL="${DEFAULT_INTERVAL}"
ACCELERATION="${DEFAULT_ACCELERATION}"
COUNT="${DEFAULT_COUNT}"
PATTERN="${DEFAULT_PATTERN}"
DATABROKER_ADDRESS="${SDV_DATABROKER_ADDRESS:-${DEFAULT_ADDRESS}}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SPEED]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

log_signal() {
    echo -e "${CYAN}[SIGNAL]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    # Check for grpcurl (preferred) or kuksa-client
    if command -v grpcurl &> /dev/null; then
        PUBLISH_METHOD="grpcurl"
        log_info "Using grpcurl for signal publishing"
    elif command -v kuksa-client &> /dev/null; then
        PUBLISH_METHOD="kuksa-client"
        log_info "Using kuksa-client for signal publishing"
    else
        log_warn "Neither grpcurl nor kuksa-client found"
        log_warn "Signals will be printed to stdout only (dry-run mode)"
        PUBLISH_METHOD="dry-run"
    fi
    
    # Check for bc (required for floating point math)
    if ! command -v bc &> /dev/null; then
        log_error "bc is required for speed calculations but not found"
        log_error "Install hint: brew install bc (macOS) or apt install bc (Linux)"
        exit 1
    fi
}

# Publish speed signal to databroker
publish_speed() {
    local speed_kmh="$1"
    local timestamp
    timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    
    log_signal "Speed: ${speed_kmh} km/h, timestamp=${timestamp}"
    
    case "${PUBLISH_METHOD}" in
        grpcurl)
            # Use grpcurl to call SetSignal RPC
            local payload
            payload=$(cat <<EOF
{
  "signal_path": "Vehicle.Speed",
  "signal": {
    "speed": {
      "speed_kmh": ${speed_kmh}
    }
  }
}
EOF
)
            if grpcurl -plaintext -d "${payload}" \
                "${DATABROKER_ADDRESS}" \
                sdv.services.databroker.DataBroker/SetSignal 2>/dev/null; then
                log_success "Published to ${DATABROKER_ADDRESS}"
            else
                log_warn "Failed to publish (databroker may not be running)"
            fi
            ;;
        kuksa-client)
            # Use kuksa-client to set value
            kuksa-client --address "${DATABROKER_ADDRESS}" \
                setValue "Vehicle.Speed" "${speed_kmh}" 2>/dev/null || true
            log_success "Published via kuksa-client"
            ;;
        dry-run)
            # Just print the signal (no actual publishing)
            echo "  Would publish: Vehicle.Speed = ${speed_kmh} km/h"
            ;;
    esac
}

# Clamp speed to valid range [0, max_speed]
clamp_speed() {
    local speed="$1"
    if (( $(echo "${speed} < 0" | bc -l) )); then
        echo "0"
    elif (( $(echo "${speed} > ${MAX_SPEED}" | bc -l) )); then
        echo "${MAX_SPEED}"
    else
        echo "${speed}"
    fi
}

# Generate random speed change
random_speed_change() {
    local current_speed="$1"
    # Random change between -ACCELERATION and +ACCELERATION
    local random_factor
    random_factor=$(echo "scale=2; (${RANDOM} - 16383.5) / 16383.5" | bc)
    local change
    change=$(echo "scale=2; ${random_factor} * ${ACCELERATION}" | bc)
    local new_speed
    new_speed=$(echo "scale=2; ${current_speed} + ${change}" | bc)
    clamp_speed "${new_speed}"
}

# Simulate constant speed
simulate_constant() {
    local current_speed="${SPEED}"
    local iteration=0
    
    log_info "Starting constant speed simulation at ${current_speed} km/h"
    
    while true; do
        if [[ "${COUNT}" -gt 0 ]] && [[ "${iteration}" -ge "${COUNT}" ]]; then
            log_info "Reached count limit (${COUNT})"
            break
        fi
        
        publish_speed "${current_speed}"
        iteration=$((iteration + 1))
        sleep "${INTERVAL}"
    done
}

# Simulate acceleration
simulate_accelerate() {
    local current_speed="${SPEED}"
    local iteration=0
    
    log_info "Starting acceleration simulation from ${current_speed} km/h to ${MAX_SPEED} km/h"
    
    while true; do
        if [[ "${COUNT}" -gt 0 ]] && [[ "${iteration}" -ge "${COUNT}" ]]; then
            log_info "Reached count limit (${COUNT})"
            break
        fi
        
        publish_speed "${current_speed}"
        
        # Accelerate
        current_speed=$(echo "scale=2; ${current_speed} + ${ACCELERATION}" | bc)
        current_speed=$(clamp_speed "${current_speed}")
        
        # Stop if we've reached max speed
        if (( $(echo "${current_speed} >= ${MAX_SPEED}" | bc -l) )); then
            publish_speed "${MAX_SPEED}"
            log_info "Reached maximum speed (${MAX_SPEED} km/h)"
            break
        fi
        
        iteration=$((iteration + 1))
        sleep "${INTERVAL}"
    done
}

# Simulate deceleration
simulate_decelerate() {
    local current_speed="${SPEED}"
    local iteration=0
    
    log_info "Starting deceleration simulation from ${current_speed} km/h to 0 km/h"
    
    while true; do
        if [[ "${COUNT}" -gt 0 ]] && [[ "${iteration}" -ge "${COUNT}" ]]; then
            log_info "Reached count limit (${COUNT})"
            break
        fi
        
        publish_speed "${current_speed}"
        
        # Decelerate
        current_speed=$(echo "scale=2; ${current_speed} - ${ACCELERATION}" | bc)
        current_speed=$(clamp_speed "${current_speed}")
        
        # Stop if we've reached 0
        if (( $(echo "${current_speed} <= 0" | bc -l) )); then
            publish_speed "0"
            log_info "Vehicle stopped (0 km/h)"
            break
        fi
        
        iteration=$((iteration + 1))
        sleep "${INTERVAL}"
    done
}

# Simulate random speed changes
simulate_random() {
    local current_speed="${SPEED}"
    local iteration=0
    
    log_info "Starting random speed simulation"
    log_info "Starting speed: ${current_speed} km/h, Max: ${MAX_SPEED} km/h"
    
    while true; do
        if [[ "${COUNT}" -gt 0 ]] && [[ "${iteration}" -ge "${COUNT}" ]]; then
            log_info "Reached count limit (${COUNT})"
            break
        fi
        
        publish_speed "${current_speed}"
        
        # Random speed change
        current_speed=$(random_speed_change "${current_speed}")
        
        iteration=$((iteration + 1))
        sleep "${INTERVAL}"
    done
}

# Simulate parking scenario (approach, slow down, stop)
simulate_parking() {
    local current_speed="${SPEED}"
    if (( $(echo "${current_speed} <= 0" | bc -l) )); then
        current_speed=50  # Start at 50 km/h if not specified
    fi
    
    log_info "Starting parking simulation from ${current_speed} km/h"
    log_info "Simulating: approach -> slow down -> stop"
    
    # Phase 1: Maintain speed briefly (approaching parking area)
    log_info "Phase 1: Approaching parking area..."
    for i in {1..3}; do
        publish_speed "${current_speed}"
        sleep "${INTERVAL}"
    done
    
    # Phase 2: Gradual deceleration
    log_info "Phase 2: Slowing down..."
    while (( $(echo "${current_speed} > 5" | bc -l) )); do
        current_speed=$(echo "scale=2; ${current_speed} - ${ACCELERATION}" | bc)
        current_speed=$(clamp_speed "${current_speed}")
        publish_speed "${current_speed}"
        sleep "${INTERVAL}"
    done
    
    # Phase 3: Final stop
    log_info "Phase 3: Stopping..."
    for speed in 3 1 0; do
        publish_speed "${speed}"
        sleep "${INTERVAL}"
    done
    
    log_info "Vehicle parked (0 km/h)"
}

# Print usage information
usage() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS]

Generate mock vehicle speed signals for SDV Parking Demo testing.

Options:
    --speed KMH         Starting speed in km/h (default: ${DEFAULT_SPEED})
    --max-speed KMH     Maximum speed in km/h (default: ${DEFAULT_MAX_SPEED})
    --interval SECS     Update interval in seconds (default: ${DEFAULT_INTERVAL})
    --acceleration KMH  Speed change per update in km/h (default: ${DEFAULT_ACCELERATION})
    --count N           Number of updates, -1 for infinite (default: ${DEFAULT_COUNT})
    --pattern PATTERN   Speed pattern (default: ${DEFAULT_PATTERN})
                        Patterns: constant, accelerate, decelerate, random, parking
    --address ADDR      Databroker address (default: ${DEFAULT_ADDRESS})
    --help              Show this help message

Environment Variables:
    SDV_DATABROKER_ADDRESS    Override default databroker address

Patterns:
    constant     Maintain constant speed
    accelerate   Gradually increase speed to max
    decelerate   Gradually decrease speed to 0
    random       Random speed changes within bounds
    parking      Simulate parking scenario (approach, slow, stop)

Examples:
    $(basename "$0")                                    # Random speed changes
    $(basename "$0") --speed 60 --pattern constant      # Constant 60 km/h
    $(basename "$0") --pattern parking --speed 50       # Parking from 50 km/h
    $(basename "$0") --pattern accelerate --max-speed 100  # Accelerate to 100

Signal Path:
    Vehicle.Speed

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --speed)
                SPEED="$2"
                shift 2
                ;;
            --max-speed)
                MAX_SPEED="$2"
                shift 2
                ;;
            --interval)
                INTERVAL="$2"
                shift 2
                ;;
            --acceleration)
                ACCELERATION="$2"
                shift 2
                ;;
            --count)
                COUNT="$2"
                shift 2
                ;;
            --pattern)
                PATTERN="$2"
                shift 2
                ;;
            --address)
                DATABROKER_ADDRESS="$2"
                shift 2
                ;;
            --help|-h)
                usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done
}

# Cleanup on exit
cleanup() {
    log_info "Stopping speed signal generator"
}

# Main entry point
main() {
    parse_args "$@"
    check_prerequisites
    
    trap cleanup EXIT INT TERM
    
    log_info "Mock Speed Signal Generator"
    log_info "Databroker address: ${DATABROKER_ADDRESS}"
    
    case "${PATTERN}" in
        constant)
            simulate_constant
            ;;
        accelerate)
            simulate_accelerate
            ;;
        decelerate)
            simulate_decelerate
            ;;
        random)
            simulate_random
            ;;
        parking)
            simulate_parking
            ;;
        *)
            log_error "Unknown pattern: ${PATTERN}"
            log_error "Valid patterns: constant, accelerate, decelerate, random, parking"
            exit 1
            ;;
    esac
}

# Run main function
main "$@"
