#!/usr/bin/env bash
# Copyright 2024 SDV Parking Demo System
# Mock Location Signal Generator
#
# This script generates mock location signals (latitude/longitude) and publishes
# them to the DATA_BROKER (Eclipse Kuksa Databroker) for testing and demos.
#
# Usage:
#   ./scripts/mock-location.sh [OPTIONS]
#
# Options:
#   --lat LAT           Starting latitude (default: 37.7749)
#   --lon LON           Starting longitude (default: -122.4194)
#   --interval SECS     Update interval in seconds (default: 1)
#   --drift METERS      Random drift per update in meters (default: 10)
#   --count N           Number of updates (default: infinite)
#   --route FILE        JSON file with waypoints for route simulation
#   --address ADDR      Databroker address (default: localhost:55556)
#   --help              Show this help message
#
# Requirements: 7.4
#
# Examples:
#   ./scripts/mock-location.sh                           # Default San Francisco location
#   ./scripts/mock-location.sh --lat 40.7128 --lon -74.0060  # New York
#   ./scripts/mock-location.sh --route routes/demo.json  # Follow a route
#   ./scripts/mock-location.sh --count 10 --interval 2   # 10 updates, 2s apart

set -euo pipefail

# Script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Default configuration
DEFAULT_LAT=37.7749          # San Francisco latitude
DEFAULT_LON=-122.4194        # San Francisco longitude
DEFAULT_INTERVAL=1           # 1 second between updates
DEFAULT_DRIFT=10             # 10 meters random drift
DEFAULT_COUNT=-1             # Infinite updates
DEFAULT_ADDRESS="localhost:55556"

# Current configuration
LAT="${DEFAULT_LAT}"
LON="${DEFAULT_LON}"
INTERVAL="${DEFAULT_INTERVAL}"
DRIFT="${DEFAULT_DRIFT}"
COUNT="${DEFAULT_COUNT}"
ROUTE_FILE=""
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
    echo -e "${GREEN}[LOCATION]${NC} $1"
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
    
    # Check for jq (optional, for route parsing)
    if [[ -n "${ROUTE_FILE}" ]] && ! command -v jq &> /dev/null; then
        log_error "jq is required for route simulation but not found"
        log_error "Install hint: brew install jq (macOS) or apt install jq (Linux)"
        exit 1
    fi
}

# Convert meters to degrees (approximate)
meters_to_degrees() {
    local meters="$1"
    # 1 degree ≈ 111,320 meters at equator
    echo "scale=10; ${meters} / 111320" | bc
}

# Generate random drift in range [-drift, +drift]
random_drift() {
    local max_drift="$1"
    local drift_degrees
    drift_degrees=$(meters_to_degrees "${max_drift}")
    
    # Generate random value between -1 and 1, multiply by drift
    local random_factor
    random_factor=$(echo "scale=10; (${RANDOM} - 16383.5) / 16383.5" | bc)
    echo "scale=10; ${random_factor} * ${drift_degrees}" | bc
}

# Publish location signal to databroker
publish_location() {
    local lat="$1"
    local lon="$2"
    local timestamp
    timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    
    log_signal "Location: lat=${lat}, lon=${lon}, timestamp=${timestamp}"
    
    case "${PUBLISH_METHOD}" in
        grpcurl)
            # Use grpcurl to call SetSignal RPC
            local payload
            payload=$(cat <<EOF
{
  "signal_path": "Vehicle.CurrentLocation",
  "signal": {
    "location": {
      "latitude": ${lat},
      "longitude": ${lon}
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
                setValue "Vehicle.CurrentLocation.Latitude" "${lat}" 2>/dev/null || true
            kuksa-client --address "${DATABROKER_ADDRESS}" \
                setValue "Vehicle.CurrentLocation.Longitude" "${lon}" 2>/dev/null || true
            log_success "Published via kuksa-client"
            ;;
        dry-run)
            # Just print the signal (no actual publishing)
            echo "  Would publish: Vehicle.CurrentLocation.Latitude = ${lat}"
            echo "  Would publish: Vehicle.CurrentLocation.Longitude = ${lon}"
            ;;
    esac
}

# Simulate random walk from current position
simulate_random_walk() {
    local current_lat="${LAT}"
    local current_lon="${LON}"
    local iteration=0
    
    log_info "Starting random walk simulation"
    log_info "Starting position: lat=${current_lat}, lon=${current_lon}"
    log_info "Drift: ${DRIFT} meters, Interval: ${INTERVAL}s"
    
    while true; do
        # Check if we've reached the count limit
        if [[ "${COUNT}" -gt 0 ]] && [[ "${iteration}" -ge "${COUNT}" ]]; then
            log_info "Reached count limit (${COUNT})"
            break
        fi
        
        # Publish current location
        publish_location "${current_lat}" "${current_lon}"
        
        # Apply random drift
        local lat_drift lon_drift
        lat_drift=$(random_drift "${DRIFT}")
        lon_drift=$(random_drift "${DRIFT}")
        
        current_lat=$(echo "scale=6; ${current_lat} + ${lat_drift}" | bc)
        current_lon=$(echo "scale=6; ${current_lon} + ${lon_drift}" | bc)
        
        iteration=$((iteration + 1))
        
        # Wait for next interval
        sleep "${INTERVAL}"
    done
}

# Simulate route from waypoints file
simulate_route() {
    local route_file="$1"
    
    if [[ ! -f "${route_file}" ]]; then
        log_error "Route file not found: ${route_file}"
        exit 1
    fi
    
    log_info "Loading route from ${route_file}"
    
    local waypoints
    waypoints=$(jq -c '.waypoints[]' "${route_file}")
    local total
    total=$(echo "${waypoints}" | wc -l | tr -d ' ')
    
    log_info "Route has ${total} waypoints"
    
    local iteration=0
    while IFS= read -r waypoint; do
        # Check if we've reached the count limit
        if [[ "${COUNT}" -gt 0 ]] && [[ "${iteration}" -ge "${COUNT}" ]]; then
            log_info "Reached count limit (${COUNT})"
            break
        fi
        
        local lat lon
        lat=$(echo "${waypoint}" | jq -r '.lat')
        lon=$(echo "${waypoint}" | jq -r '.lon')
        
        publish_location "${lat}" "${lon}"
        
        iteration=$((iteration + 1))
        
        # Wait for next interval
        sleep "${INTERVAL}"
    done <<< "${waypoints}"
    
    log_info "Route simulation complete"
}

# Print usage information
usage() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS]

Generate mock location signals for SDV Parking Demo testing.

Options:
    --lat LAT           Starting latitude (default: ${DEFAULT_LAT})
    --lon LON           Starting longitude (default: ${DEFAULT_LON})
    --interval SECS     Update interval in seconds (default: ${DEFAULT_INTERVAL})
    --drift METERS      Random drift per update in meters (default: ${DEFAULT_DRIFT})
    --count N           Number of updates, -1 for infinite (default: ${DEFAULT_COUNT})
    --route FILE        JSON file with waypoints for route simulation
    --address ADDR      Databroker address (default: ${DEFAULT_ADDRESS})
    --help              Show this help message

Environment Variables:
    SDV_DATABROKER_ADDRESS    Override default databroker address

Examples:
    $(basename "$0")                                    # Default location
    $(basename "$0") --lat 40.7128 --lon -74.0060       # New York
    $(basename "$0") --route routes/parking-demo.json  # Follow route
    $(basename "$0") --count 10 --interval 2           # 10 updates

Route File Format (JSON):
    {
      "name": "Demo Route",
      "waypoints": [
        {"lat": 37.7749, "lon": -122.4194},
        {"lat": 37.7750, "lon": -122.4195}
      ]
    }

Signal Path:
    Vehicle.CurrentLocation.Latitude
    Vehicle.CurrentLocation.Longitude

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --lat)
                LAT="$2"
                shift 2
                ;;
            --lon)
                LON="$2"
                shift 2
                ;;
            --interval)
                INTERVAL="$2"
                shift 2
                ;;
            --drift)
                DRIFT="$2"
                shift 2
                ;;
            --count)
                COUNT="$2"
                shift 2
                ;;
            --route)
                ROUTE_FILE="$2"
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
    log_info "Stopping location signal generator"
}

# Main entry point
main() {
    parse_args "$@"
    check_prerequisites
    
    trap cleanup EXIT INT TERM
    
    log_info "Mock Location Signal Generator"
    log_info "Databroker address: ${DATABROKER_ADDRESS}"
    
    if [[ -n "${ROUTE_FILE}" ]]; then
        simulate_route "${ROUTE_FILE}"
    else
        simulate_random_walk
    fi
}

# Run main function
main "$@"
