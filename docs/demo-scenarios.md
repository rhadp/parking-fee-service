# Demo Scenarios

This document describes the three main demo scenarios for the SDV Parking Demo System and how to configure the environment to demonstrate each scenario.

## Overview

The SDV Parking Demo System demonstrates mixed-criticality communication between an Android parking app and ASIL-B door locking services on RHIVOS, with dynamic parking operator adapters that download on-demand based on vehicle location.

**Requirements: 7.1, 7.2, 7.3, 7.4, 7.5**

## Prerequisites

Before running any demo scenario, ensure the local infrastructure is running:

```bash
# Start local development infrastructure
make infra-up

# Verify services are healthy
make infra-status
```

## Demo Scenario 1: Happy Path

**Description:** The vehicle enters a parking zone, the appropriate parking operator adapter is downloaded and installed, and a parking session is successfully started and stopped.

### Scenario Flow

1. Vehicle approaches parking zone (location signal changes)
2. PARKING_APP detects parking zone via DATA_BROKER subscription
3. PARKING_APP requests adapter installation from UPDATE_SERVICE
4. UPDATE_SERVICE downloads adapter from registry
5. UPDATE_SERVICE installs and starts PARKING_OPERATOR_ADAPTOR
6. PARKING_APP starts parking session via PARKING_OPERATOR_ADAPTOR
7. User parks vehicle (speed drops to 0)
8. User locks doors via LOCKING_SERVICE
9. Later, user unlocks doors and ends parking session
10. PARKING_APP displays parking fee

### Configuration

```bash
# Source the demo profile
source scripts/dev-env.sh
eval "$(grep -A20 'demo_happy_path:' infra/config/development.yaml | \
        grep -A10 'script:' | tail -n+2 | sed 's/^      //')"

# Or set environment variables manually
export SDV_LOG_LEVEL=info
export SDV_TLS_SKIP_VERIFY=true
export SDV_SIMULATE_REGISTRY_UNAVAILABLE=false
export SDV_SIMULATE_NETWORK_FAILURE=false
```

### Running the Demo

```bash
# Terminal 1: Start mock signal generators
./scripts/mock-location.sh --lat 37.7749 --lon -122.4194 --interval 2

# Terminal 2: Simulate vehicle approaching and parking
./scripts/mock-speed.sh --pattern parking --speed 30

# Terminal 3: Simulate door operations
./scripts/mock-door.sh --pattern unlock-open-close-lock

# Terminal 4: Watch service logs
make logs
```

### Expected Behavior

- UPDATE_SERVICE successfully downloads adapter from registry
- PARKING_OPERATOR_ADAPTOR starts and becomes available
- Parking session starts successfully
- Door lock/unlock commands execute successfully
- Parking fee is calculated and displayed

---

## Demo Scenario 2: Adapter Already Installed

**Description:** The vehicle enters a parking zone where the parking operator adapter is already installed from a previous visit. No download is needed.

### Scenario Flow

1. Vehicle approaches parking zone (location signal changes)
2. PARKING_APP detects parking zone via DATA_BROKER subscription
3. PARKING_APP queries UPDATE_SERVICE for installed adapters
4. UPDATE_SERVICE reports adapter is already installed and running
5. PARKING_APP starts parking session immediately (no download delay)
6. Parking session proceeds as in Scenario 1

### Configuration

```bash
# Pre-install the demo adapter before starting
make install-demo-adapter

# Source the demo profile
eval "$(grep -A20 'demo_adapter_installed:' infra/config/development.yaml | \
        grep -A10 'script:' | tail -n+2 | sed 's/^      //')"

# Or set environment variables manually
export SDV_LOG_LEVEL=info
export SDV_TLS_SKIP_VERIFY=true
```

### Running the Demo

```bash
# Verify adapter is pre-installed
grpcurl -plaintext localhost:50051 sdv.services.update.UpdateService/ListAdapters

# Terminal 1: Start mock signal generators
./scripts/mock-location.sh --lat 37.7749 --lon -122.4194 --interval 2

# Terminal 2: Simulate parking
./scripts/mock-speed.sh --pattern parking --speed 30
```

### Expected Behavior

- UPDATE_SERVICE reports adapter is already installed
- No download occurs (faster startup)
- Parking session starts immediately
- All other operations proceed normally

---

## Demo Scenario 3: Error Handling

**Description:** The vehicle enters a parking zone but the adapter cannot be downloaded due to registry unavailability or network issues. The system demonstrates graceful error handling.

### Variant A: Registry Unavailable

The OCI registry is unavailable, preventing adapter download.

#### Configuration

```bash
# Source the demo profile
eval "$(grep -A20 'demo_registry_error:' infra/config/development.yaml | \
        grep -A10 'script:' | tail -n+2 | sed 's/^      //')"

# Or set environment variables manually
export SDV_LOG_LEVEL=debug
export SDV_TLS_SKIP_VERIFY=true
export SDV_SIMULATE_REGISTRY_UNAVAILABLE=true
```

#### Expected Behavior

- UPDATE_SERVICE attempts to download adapter
- Download fails with `UNAVAILABLE` error code
- PARKING_APP displays user-friendly error message
- System suggests retry or manual parking payment
- No crash or undefined behavior

### Variant B: Network Failure

Complete network connectivity loss prevents all external communication.

#### Configuration

```bash
# Source the demo profile
eval "$(grep -A20 'demo_network_error:' infra/config/development.yaml | \
        grep -A10 'script:' | tail -n+2 | sed 's/^      //')"

# Or set environment variables manually
export SDV_LOG_LEVEL=debug
export SDV_TLS_SKIP_VERIFY=true
export SDV_SIMULATE_NETWORK_FAILURE=true
```

#### Expected Behavior

- All network calls fail with `UNAVAILABLE` error
- Local services (DATA_BROKER, LOCKING_SERVICE) continue to work
- PARKING_APP shows offline mode indicator
- User can still lock/unlock doors locally

### Variant C: Partial Network Partition

Only specific services are unreachable (e.g., registry down but MQTT works).

#### Configuration

```bash
# Source the demo profile
eval "$(grep -A20 'demo_partial_partition:' infra/config/development.yaml | \
        grep -A10 'script:' | tail -n+2 | sed 's/^      //')"

# Or set environment variables manually
export SDV_LOG_LEVEL=debug
export SDV_TLS_SKIP_VERIFY=true
export SDV_SIMULATE_PARTITION_SERVICES=registry
```

#### Expected Behavior

- Registry calls fail, adapter download fails
- MQTT communication continues to work
- Cloud gateway can still send/receive messages
- Partial functionality maintained

---

## Mock Data Generators

The following scripts generate mock VSS signals for demo scenarios:

### Location Signals

```bash
# Generate location signals (San Francisco default)
./scripts/mock-location.sh

# Custom starting location (New York)
./scripts/mock-location.sh --lat 40.7128 --lon -74.0060

# Follow a predefined route
./scripts/mock-location.sh --route routes/parking-demo.json

# Limited updates for scripted demo
./scripts/mock-location.sh --count 20 --interval 2
```

### Speed Signals

```bash
# Random speed changes
./scripts/mock-speed.sh

# Constant speed (highway driving)
./scripts/mock-speed.sh --speed 100 --pattern constant

# Simulate parking (decelerate to stop)
./scripts/mock-speed.sh --pattern parking --speed 50

# Accelerate from stop
./scripts/mock-speed.sh --pattern accelerate --max-speed 60
```

### Door Signals

```bash
# Static door state (locked)
./scripts/mock-door.sh

# Toggle lock state
./scripts/mock-door.sh --pattern toggle-lock

# Full door cycle (unlock -> open -> close -> lock)
./scripts/mock-door.sh --pattern unlock-open-close-lock

# All doors simultaneously
./scripts/mock-door.sh --door all --pattern toggle-lock
```

---

## Failure Simulation Environment Variables

The following environment variables control failure simulation:

| Variable | Description | Default |
|----------|-------------|---------|
| `SDV_SIMULATE_REGISTRY_UNAVAILABLE` | Simulate registry unavailable | `false` |
| `SDV_SIMULATE_REGISTRY_TIMEOUT_SEC` | Registry timeout in seconds | `0` |
| `SDV_SIMULATE_NETWORK_FAILURE` | Simulate network failure | `false` |
| `SDV_SIMULATE_PARTITION_SERVICES` | Services to partition (comma-separated) | `""` |
| `SDV_SIMULATE_CHECKSUM_MISMATCH` | Simulate checksum failure | `false` |
| `SDV_SIMULATE_ADAPTER_INSTALL_FAILURE` | Simulate adapter install failure | `false` |
| `SDV_SIMULATE_MQTT_FAILURE` | Simulate MQTT failure | `false` |
| `SDV_SIMULATE_INTERMITTENT_RATE` | Intermittent failure rate (0-100%) | `0` |
| `SDV_SIMULATE_LATENCY_MS` | Artificial latency in ms | `0` |
| `SDV_SIMULATE_FAILURE_RATE` | General failure rate (0.0-1.0) | `0.0` |

### Example: Combined Failure Simulation

```bash
# Simulate flaky network with 20% failure rate and 100ms latency
export SDV_SIMULATE_INTERMITTENT_RATE=20
export SDV_SIMULATE_LATENCY_MS=100
export SDV_LOG_LEVEL=debug

# Run demo and observe retry behavior
./scripts/mock-location.sh --count 10
```

---

## Troubleshooting

### Services Not Starting

```bash
# Check service health
make infra-status

# View service logs
make logs

# Restart infrastructure
make infra-down && make infra-up
```

### Mock Signals Not Publishing

```bash
# Verify databroker is running
grpcurl -plaintext localhost:55556 list

# Check if grpcurl is installed
which grpcurl || echo "Install grpcurl: brew install grpcurl"

# Run in dry-run mode to verify script
./scripts/mock-location.sh --count 1
```

### Failure Simulation Not Working

```bash
# Verify environment variables are set
env | grep SDV_SIMULATE

# Check service is reading the variables
export SDV_LOG_LEVEL=debug
# Restart service and check logs
```

---

## Demo Checklist

Before presenting a demo, verify:

- [ ] Local infrastructure is running (`make infra-up`)
- [ ] All services are healthy (`make infra-status`)
- [ ] Correct demo profile is loaded (check `env | grep SDV`)
- [ ] Mock signal generators are ready
- [ ] Terminal windows are arranged for visibility
- [ ] For Scenario 2: Adapter is pre-installed
- [ ] For Scenario 3: Failure simulation is enabled

---

## Related Documentation

- [Local Infrastructure Setup](local-infrastructure.md)
- [Development Configuration](../infra/config/development.yaml)
- [Service Endpoints](../infra/config/endpoints.yaml)
- [Protocol Buffer Definitions](../proto/README.md)
