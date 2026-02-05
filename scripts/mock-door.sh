#!/usr/bin/env bash
# Copyright 2024 SDV Parking Demo System
# Mock Door Sensor Signal Generator
#
# This script generates mock door sensor signals (lock state, open state) and
# publishes them to the DATA_BROKER (Eclipse Kuksa Databroker) for testing and demos.
#
# Usage:
#   ./scripts/mock-door.sh [OPTIONS]
#
# Options:
#   --door DOOR         Door to simulate: driver, passenger, rear_left, rear_right, all (default: driver)
#   --locked BOOL       Initial lock state: true/false (default: true)
#   --open BOOL         Initial open state: true/false (default: false)
#   --interval SECS     Update interval in seconds (default: 1)
#   --count N           Number of updates (default: infinite)
#   --pattern PATTERN   Door pattern: static, toggle-lock, toggle-open, unlock-open-close-lock, random (default: static)
#   --address ADDR      Databroker address (default: localhost:55556)
#   --help              Show this help message
#
# Requirements: 7.4
#
# Examples:
#   ./scripts/mock-door.sh                                    # Static driver door locked
#   ./scripts/mock-door.sh --door all --pattern toggle-lock   # Toggle all doors lock
#   ./scripts/mock-door.sh --pattern unlock-open-close-lock   # Full door cycle
#   ./scripts/mock-door.sh --locked false --open true         # Door unlocked and open

set -euo pipefail

# Script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Default configuration
DEFAULT_DOOR="driver"        # Driver door
DEFAULT_LOCKED="true"        # Door is locked
DEFAULT_OPEN="false"         # Door is closed
DEFAULT_INTERVAL=1           # 1 second between updates
DEFAULT_COUNT=-1             # Infinite updates
DEFAULT_PATTERN="static"     # Static state
DEFAULT_ADDRESS="localhost:55556"

# Current configuration
DOOR="${DEFAULT_DOOR}"
LOCKED="${DEFAULT_LOCKED}"
OPEN="${DEFAULT_OPEN}"
INTERVAL="${DEFAULT_INTERVAL}"
COUNT="${DEFAULT_COUNT}"
PATTERN="${DEFAULT_PATTERN}"
DATABROKER_ADDRESS="${SDV_DATABROKER_ADDRESS:-${DEFAULT_ADDRESS}}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[DOOR]${NC} $1"
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

log_state() {
    local locked="$1"
    local open="$2"
    local state_icon=""
    local state_color=""
    
    if [[ "${locked}" == "true" ]] && [[ "${open}" == "false" ]]; then
        state_icon="🔒"
        state_color="${GREEN}"
    elif [[ "${locked}" == "false" ]] && [[ "${open}" == "false" ]]; then
        state_icon="🔓"
        state_color="${YELLOW}"
    elif [[ "${locked}" == "false" ]] && [[ "${open}" == "true" ]]; then
        state_icon="🚪"
        state_color="${MAGENTA}"
    else
        state_icon="⚠️"
        state_color="${RED}"
    fi
    
    echo -e "${state_color}${state_icon}${NC}"
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
}

# Get VSS signal path for door
get_door_signal_path() {
    local door="$1"
    case "${door}" in
        driver)
            echo "Vehicle.Cabin.Door.Row1.DriverSide"
            ;;
        passenger)
            echo "Vehicle.Cabin.Door.Row1.PassengerSide"
            ;;
        rear_left)
            echo "Vehicle.Cabin.Door.Row2.DriverSide"
            ;;
        rear_right)
            echo "Vehicle.Cabin.Door.Row2.PassengerSide"
            ;;
        *)
            log_error "Unknown door: ${door}"
            exit 1
            ;;
    esac
}

# Publish door signal to databroker
publish_door_state() {
    local door="$1"
    local is_locked="$2"
    local is_open="$3"
    local timestamp
    timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    local signal_path
    signal_path=$(get_door_signal_path "${door}")
    local state_icon
    state_icon=$(log_state "${is_locked}" "${is_open}")
    
    log_signal "Door: ${door}, locked=${is_locked}, open=${is_open} ${state_icon}"
    
    case "${PUBLISH_METHOD}" in
        grpcurl)
            # Use grpcurl to call SetSignal RPC
            local payload
            payload=$(cat <<EOF
{
  "signal_path": "${signal_path}",
  "signal": {
    "door_state": {
      "is_locked": ${is_locked},
      "is_open": ${is_open}
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
            # Use kuksa-client to set values
            kuksa-client --address "${DATABROKER_ADDRESS}" \
                setValue "${signal_path}.IsLocked" "${is_locked}" 2>/dev/null || true
            kuksa-client --address "${DATABROKER_ADDRESS}" \
                setValue "${signal_path}.IsOpen" "${is_open}" 2>/dev/null || true
            log_success "Published via kuksa-client"
            ;;
        dry-run)
            # Just print the signal (no actual publishing)
            echo "  Would publish: ${signal_path}.IsLocked = ${is_locked}"
            echo "  Would publish: ${signal_path}.IsOpen = ${is_open}"
            ;;
    esac
}

# Publish state for all doors
publish_all_doors() {
    local is_locked="$1"
    local is_open="$2"
    
    for door in driver passenger rear_left rear_right; do
        publish_door_state "${door}" "${is_locked}" "${is_open}"
    done
}

# Toggle boolean value
toggle_bool() {
    local value="$1"
    if [[ "${value}" == "true" ]]; then
        echo "false"
    else
        echo "true"
    fi
}

# Simulate static door state
simulate_static() {
    local current_locked="${LOCKED}"
    local current_open="${OPEN}"
    local iteration=0
    
    log_info "Starting static door state simulation"
    log_info "Door: ${DOOR}, Locked: ${current_locked}, Open: ${current_open}"
    
    while true; do
        if [[ "${COUNT}" -gt 0 ]] && [[ "${iteration}" -ge "${COUNT}" ]]; then
            log_info "Reached count limit (${COUNT})"
            break
        fi
        
        if [[ "${DOOR}" == "all" ]]; then
            publish_all_doors "${current_locked}" "${current_open}"
        else
            publish_door_state "${DOOR}" "${current_locked}" "${current_open}"
        fi
        
        iteration=$((iteration + 1))
        sleep "${INTERVAL}"
    done
}

# Simulate toggling lock state
simulate_toggle_lock() {
    local current_locked="${LOCKED}"
    local current_open="${OPEN}"
    local iteration=0
    
    log_info "Starting toggle lock simulation"
    log_info "Door: ${DOOR}, Starting locked: ${current_locked}"
    
    while true; do
        if [[ "${COUNT}" -gt 0 ]] && [[ "${iteration}" -ge "${COUNT}" ]]; then
            log_info "Reached count limit (${COUNT})"
            break
        fi
        
        if [[ "${DOOR}" == "all" ]]; then
            publish_all_doors "${current_locked}" "${current_open}"
        else
            publish_door_state "${DOOR}" "${current_locked}" "${current_open}"
        fi
        
        # Toggle lock state
        current_locked=$(toggle_bool "${current_locked}")
        
        iteration=$((iteration + 1))
        sleep "${INTERVAL}"
    done
}

# Simulate toggling open state
simulate_toggle_open() {
    local current_locked="${LOCKED}"
    local current_open="${OPEN}"
    local iteration=0
    
    log_info "Starting toggle open simulation"
    log_info "Door: ${DOOR}, Starting open: ${current_open}"
    
    # Door must be unlocked to open
    if [[ "${current_locked}" == "true" ]]; then
        log_warn "Door is locked, unlocking first..."
        current_locked="false"
    fi
    
    while true; do
        if [[ "${COUNT}" -gt 0 ]] && [[ "${iteration}" -ge "${COUNT}" ]]; then
            log_info "Reached count limit (${COUNT})"
            break
        fi
        
        if [[ "${DOOR}" == "all" ]]; then
            publish_all_doors "${current_locked}" "${current_open}"
        else
            publish_door_state "${DOOR}" "${current_locked}" "${current_open}"
        fi
        
        # Toggle open state
        current_open=$(toggle_bool "${current_open}")
        
        iteration=$((iteration + 1))
        sleep "${INTERVAL}"
    done
}

# Simulate full door cycle: unlock -> open -> close -> lock
simulate_full_cycle() {
    log_info "Starting full door cycle simulation"
    log_info "Door: ${DOOR}"
    log_info "Cycle: locked -> unlock -> open -> close -> lock"
    
    local publish_func
    if [[ "${DOOR}" == "all" ]]; then
        publish_func="publish_all_doors"
    else
        publish_func="publish_door_state ${DOOR}"
    fi
    
    # State 1: Locked and closed (initial)
    log_info "State 1: Door locked and closed"
    if [[ "${DOOR}" == "all" ]]; then
        publish_all_doors "true" "false"
    else
        publish_door_state "${DOOR}" "true" "false"
    fi
    sleep "${INTERVAL}"
    
    # State 2: Unlocked and closed
    log_info "State 2: Door unlocked"
    if [[ "${DOOR}" == "all" ]]; then
        publish_all_doors "false" "false"
    else
        publish_door_state "${DOOR}" "false" "false"
    fi
    sleep "${INTERVAL}"
    
    # State 3: Unlocked and open
    log_info "State 3: Door opened"
    if [[ "${DOOR}" == "all" ]]; then
        publish_all_doors "false" "true"
    else
        publish_door_state "${DOOR}" "false" "true"
    fi
    sleep "${INTERVAL}"
    sleep "${INTERVAL}"  # Extra time with door open
    
    # State 4: Unlocked and closed
    log_info "State 4: Door closed"
    if [[ "${DOOR}" == "all" ]]; then
        publish_all_doors "false" "false"
    else
        publish_door_state "${DOOR}" "false" "false"
    fi
    sleep "${INTERVAL}"
    
    # State 5: Locked and closed (final)
    log_info "State 5: Door locked"
    if [[ "${DOOR}" == "all" ]]; then
        publish_all_doors "true" "false"
    else
        publish_door_state "${DOOR}" "true" "false"
    fi
    
    log_info "Full door cycle complete"
}

# Simulate random door state changes
simulate_random() {
    local current_locked="${LOCKED}"
    local current_open="${OPEN}"
    local iteration=0
    
    log_info "Starting random door state simulation"
    log_info "Door: ${DOOR}"
    
    while true; do
        if [[ "${COUNT}" -gt 0 ]] && [[ "${iteration}" -ge "${COUNT}" ]]; then
            log_info "Reached count limit (${COUNT})"
            break
        fi
        
        if [[ "${DOOR}" == "all" ]]; then
            publish_all_doors "${current_locked}" "${current_open}"
        else
            publish_door_state "${DOOR}" "${current_locked}" "${current_open}"
        fi
        
        # Random state change (weighted towards realistic transitions)
        local random_action=$((RANDOM % 10))
        
        if [[ "${current_locked}" == "true" ]]; then
            # If locked, can only unlock (30% chance)
            if [[ "${random_action}" -lt 3 ]]; then
                current_locked="false"
            fi
        elif [[ "${current_open}" == "true" ]]; then
            # If open, can close (40% chance) or stay open
            if [[ "${random_action}" -lt 4 ]]; then
                current_open="false"
            fi
        else
            # If unlocked and closed, can open (30%) or lock (30%)
            if [[ "${random_action}" -lt 3 ]]; then
                current_open="true"
            elif [[ "${random_action}" -lt 6 ]]; then
                current_locked="true"
            fi
        fi
        
        iteration=$((iteration + 1))
        sleep "${INTERVAL}"
    done
}

# Print usage information
usage() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS]

Generate mock door sensor signals for SDV Parking Demo testing.

Options:
    --door DOOR         Door to simulate (default: ${DEFAULT_DOOR})
                        Options: driver, passenger, rear_left, rear_right, all
    --locked BOOL       Initial lock state: true/false (default: ${DEFAULT_LOCKED})
    --open BOOL         Initial open state: true/false (default: ${DEFAULT_OPEN})
    --interval SECS     Update interval in seconds (default: ${DEFAULT_INTERVAL})
    --count N           Number of updates, -1 for infinite (default: ${DEFAULT_COUNT})
    --pattern PATTERN   Door pattern (default: ${DEFAULT_PATTERN})
                        Patterns: static, toggle-lock, toggle-open, 
                                  unlock-open-close-lock, random
    --address ADDR      Databroker address (default: ${DEFAULT_ADDRESS})
    --help              Show this help message

Environment Variables:
    SDV_DATABROKER_ADDRESS    Override default databroker address

Patterns:
    static                  Maintain constant door state
    toggle-lock             Toggle lock state each interval
    toggle-open             Toggle open state each interval (unlocks first)
    unlock-open-close-lock  Full door cycle: unlock -> open -> close -> lock
    random                  Random realistic state transitions

Door State Icons:
    🔒  Locked and closed (secure)
    🔓  Unlocked and closed
    🚪  Unlocked and open
    ⚠️   Invalid state (locked and open)

Examples:
    $(basename "$0")                                        # Static driver door locked
    $(basename "$0") --door all --pattern toggle-lock       # Toggle all doors
    $(basename "$0") --pattern unlock-open-close-lock       # Full door cycle
    $(basename "$0") --locked false --open true             # Door unlocked and open
    $(basename "$0") --door passenger --pattern random      # Random passenger door

Signal Paths:
    Vehicle.Cabin.Door.Row1.DriverSide.IsLocked
    Vehicle.Cabin.Door.Row1.DriverSide.IsOpen
    Vehicle.Cabin.Door.Row1.PassengerSide.IsLocked
    Vehicle.Cabin.Door.Row1.PassengerSide.IsOpen
    Vehicle.Cabin.Door.Row2.DriverSide.IsLocked
    Vehicle.Cabin.Door.Row2.DriverSide.IsOpen
    Vehicle.Cabin.Door.Row2.PassengerSide.IsLocked
    Vehicle.Cabin.Door.Row2.PassengerSide.IsOpen

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --door)
                DOOR="$2"
                shift 2
                ;;
            --locked)
                LOCKED="$2"
                shift 2
                ;;
            --open)
                OPEN="$2"
                shift 2
                ;;
            --interval)
                INTERVAL="$2"
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
    
    # Validate door option
    case "${DOOR}" in
        driver|passenger|rear_left|rear_right|all)
            ;;
        *)
            log_error "Invalid door: ${DOOR}"
            log_error "Valid options: driver, passenger, rear_left, rear_right, all"
            exit 1
            ;;
    esac
    
    # Validate boolean options
    if [[ "${LOCKED}" != "true" ]] && [[ "${LOCKED}" != "false" ]]; then
        log_error "Invalid locked value: ${LOCKED} (must be true or false)"
        exit 1
    fi
    
    if [[ "${OPEN}" != "true" ]] && [[ "${OPEN}" != "false" ]]; then
        log_error "Invalid open value: ${OPEN} (must be true or false)"
        exit 1
    fi
    
    # Warn about invalid state (locked and open)
    if [[ "${LOCKED}" == "true" ]] && [[ "${OPEN}" == "true" ]]; then
        log_warn "Invalid state: door cannot be locked and open simultaneously"
        log_warn "Setting open=false"
        OPEN="false"
    fi
}

# Cleanup on exit
cleanup() {
    log_info "Stopping door signal generator"
}

# Main entry point
main() {
    parse_args "$@"
    check_prerequisites
    
    trap cleanup EXIT INT TERM
    
    log_info "Mock Door Signal Generator"
    log_info "Databroker address: ${DATABROKER_ADDRESS}"
    
    case "${PATTERN}" in
        static)
            simulate_static
            ;;
        toggle-lock)
            simulate_toggle_lock
            ;;
        toggle-open)
            simulate_toggle_open
            ;;
        unlock-open-close-lock)
            simulate_full_cycle
            ;;
        random)
            simulate_random
            ;;
        *)
            log_error "Unknown pattern: ${PATTERN}"
            log_error "Valid patterns: static, toggle-lock, toggle-open, unlock-open-close-lock, random"
            exit 1
            ;;
    esac
}

# Run main function
main "$@"
