# Design Document: DATA_BROKER (Spec 02)

## Overview

The DATA_BROKER is Eclipse Kuksa Databroker deployed as a pre-built binary. No custom code wraps or reimplements any part of it. The deployment consists of: (1) the Kuksa Databroker binary, (2) a VSS overlay JSON file for custom signals, and (3) configuration for dual listeners (UDS + TCP). This design document covers architecture, configuration, and correctness properties.

## Architecture

The DATA_BROKER runs in the RHIVOS safety partition and serves as the central vehicle signal hub. Same-partition consumers (LOCKING_SERVICE, CLOUD_GATEWAY_CLIENT) connect via Unix Domain Sockets. Cross-partition consumers (PARKING_APP, PARKING_OPERATOR_ADAPTOR, mock sensors) connect via network TCP on port 55556.

```
+------------------------------------------------------------------+
|  RHIVOS Safety Partition                                         |
|                                                                  |
|  +--------------------+     +-------------------------------+    |
|  | LOCKING_SERVICE    |---->|                               |    |
|  +--------------------+ UDS |   KUKSA DATABROKER            |    |
|  +--------------------+     |   (pre-built binary)          |    |
|  | CLOUD_GATEWAY_     |---->|                               |    |
|  | CLIENT             | UDS |   - VSS v5.1 standard signals |    |
|  +--------------------+     |   - Custom overlay signals    |    |
|                             |   - gRPC API                  |    |
|                             +-------------------------------+    |
|                                  |              |                |
+----------------------------------|--------------|-----------------+
                                   | UDS          | TCP:55556
                                   |              |
                          same-partition     cross-partition
                          consumers          consumers
                                             (PARKING_APP,
                                              PARKING_OPERATOR_ADAPTOR,
                                              mock sensors)
```

### Module Responsibilities

1. **Kuksa Databroker binary** -- manages the VSS signal tree, handles gRPC connections, enforces signal types, delivers subscription updates.
2. **VSS overlay file** (`config/vss/vss_overlay.json`) -- extends the standard VSS v5.1 tree with custom parking and command signals.
3. **Startup configuration** -- command-line arguments and environment variables controlling listeners, VSS paths, and overlay loading.

## Components and Interfaces

### Kuksa Databroker gRPC API

The DATA_BROKER exposes the standard Kuksa Databroker gRPC API (defined by the `kuksa.val.v2` proto package or equivalent). Key operations:

| Operation | Description |
|-----------|-------------|
| `GetValue` / `GetRequest` | Read the current value of one or more signals |
| `SetValue` / `SetRequest` | Write a value to one or more signals |
| `Subscribe` | Open a server-streaming subscription for signal updates |
| `ListMetadata` / `GetMetadata` | Query signal metadata (path, type, description) |

The exact method names depend on the Kuksa Databroker version. The tests must adapt to the actual API surface.

### Endpoints

| Endpoint | Transport | Address | Consumers |
|----------|-----------|---------|-----------|
| UDS | Unix Domain Socket | `/tmp/kuksa/databroker.sock` | LOCKING_SERVICE, CLOUD_GATEWAY_CLIENT |
| TCP | Network TCP (gRPC/HTTP2) | `0.0.0.0:55556` | PARKING_APP, PARKING_OPERATOR_ADAPTOR, mock sensors, test harnesses |

## Data Models

### VSS Overlay File Format

The overlay file uses the Kuksa/COVESA VSS JSON format. It is placed at `config/vss/vss_overlay.json` and mounted or passed to the Databroker binary at startup.

#### Overlay Content: `config/vss/vss_overlay.json`

```json
{
  "Vehicle": {
    "type": "branch",
    "children": {
      "Parking": {
        "type": "branch",
        "description": "Parking-related signals for the parking demo.",
        "children": {
          "SessionActive": {
            "type": "actuator",
            "datatype": "boolean",
            "description": "Indicates whether a parking session is currently active. Written by PARKING_OPERATOR_ADAPTOR."
          }
        }
      },
      "Command": {
        "type": "branch",
        "children": {
          "Door": {
            "type": "branch",
            "children": {
              "Lock": {
                "type": "actuator",
                "datatype": "string",
                "description": "Lock/unlock command request as JSON. Written by CLOUD_GATEWAY_CLIENT. Payload: {command_id, action, doors, source, vin, timestamp}."
              },
              "Response": {
                "type": "actuator",
                "datatype": "string",
                "description": "Command execution result as JSON. Written by LOCKING_SERVICE. Payload: {command_id, status, reason, timestamp}."
              }
            }
          }
        }
      }
    }
  }
}
```

### Signal Type Rationale

All custom signals use `actuator` type because they represent values actively set by components (not passively sensed). The `string` type for command signals allows flexible JSON payloads without requiring proto/VSS schema changes for payload evolution.

### Complete Signal Inventory

| Signal Path | Data Type | Source | Standard/Custom |
|-------------|-----------|--------|-----------------|
| `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` | bool | Standard VSS v5.1 | Standard |
| `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` | bool | Standard VSS v5.1 | Standard |
| `Vehicle.CurrentLocation.Latitude` | double | Standard VSS v5.1 | Standard |
| `Vehicle.CurrentLocation.Longitude` | double | Standard VSS v5.1 | Standard |
| `Vehicle.Speed` | float | Standard VSS v5.1 | Standard |
| `Vehicle.Parking.SessionActive` | boolean | Overlay | Custom |
| `Vehicle.Command.Door.Lock` | string | Overlay | Custom |
| `Vehicle.Command.Door.Response` | string | Overlay | Custom |

## Network Configuration

### Unix Domain Socket (Same-Partition)

- **Path:** `/tmp/kuksa/databroker.sock`
- **Protocol:** gRPC over UDS
- **Consumers:** LOCKING_SERVICE, CLOUD_GATEWAY_CLIENT
- **No TLS** for local development (UDS provides filesystem-level access control)

### Network TCP (Cross-Partition)

- **Host:** `0.0.0.0` (bind to all interfaces)
- **Port:** `55556`
- **Protocol:** gRPC over HTTP/2
- **Consumers:** PARKING_APP, PARKING_OPERATOR_ADAPTOR, mock sensors, test harnesses
- **No TLS** for local development

### Kuksa Databroker Startup Command

```
databroker --address 0.0.0.0 --port 55556 --vss config/vss/vss.json --vss-overlay config/vss/vss_overlay.json
```

The `--vss` argument loads the standard VSS v5.1 JSON file. The `--vss-overlay` argument merges the custom overlay on top. UDS support is configured via the `--uds` flag or environment variable (version-dependent).

## Access Control Model

For the demo scope, Kuksa Databroker runs without authorization (no `--jwt-public-key` or token enforcement). All connected consumers have full read/write access to all signals.

The intended ownership model (for documentation and future enforcement):

| Signal Path | Intended Writer | Intended Readers |
|-------------|----------------|-----------------|
| `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` | LOCKING_SERVICE | PARKING_OPERATOR_ADAPTOR, PARKING_APP, CLOUD_GATEWAY_CLIENT |
| `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` | DOOR_SENSOR (mock) | LOCKING_SERVICE |
| `Vehicle.CurrentLocation.Latitude` | LOCATION_SENSOR (mock) | PARKING_APP, CLOUD_GATEWAY_CLIENT |
| `Vehicle.CurrentLocation.Longitude` | LOCATION_SENSOR (mock) | PARKING_APP, CLOUD_GATEWAY_CLIENT |
| `Vehicle.Speed` | SPEED_SENSOR (mock) | LOCKING_SERVICE |
| `Vehicle.Parking.SessionActive` | PARKING_OPERATOR_ADAPTOR | PARKING_APP, CLOUD_GATEWAY_CLIENT |
| `Vehicle.Command.Door.Lock` | CLOUD_GATEWAY_CLIENT | LOCKING_SERVICE |
| `Vehicle.Command.Door.Response` | LOCKING_SERVICE | CLOUD_GATEWAY_CLIENT |

In production, Kuksa's JWT-based access control would enforce this ownership. For the demo, all consumers can read and write all signals.

## Correctness Properties

### Property 1: Signal Registration Completeness

*For any* startup of the DATA_BROKER with the standard VSS v5.1 file and the custom overlay, the DATA_BROKER SHALL register all 8 signals (5 standard + 3 custom) in its signal tree, each with the correct data type.

**Validates: 02-REQ-2.1, 02-REQ-2.2, 02-REQ-3.1**

### Property 2: Signal Read/Write Consistency

*For any* signal in the registered tree and *for any* value of the correct type written to that signal, a subsequent read of that signal SHALL return the most recently written value.

**Validates: 02-REQ-6.1, 02-REQ-6.2**

### Property 3: Subscription Delivery

*For any* active subscription on a signal and *for any* new value written to that signal, the subscriber SHALL receive the updated value on the subscription stream.

**Validates: 02-REQ-7.1, 02-REQ-7.2**

### Property 4: Type Enforcement

*For any* signal in the registered tree and *for any* value whose type does not match the signal's registered data type, a write attempt SHALL be rejected with an error.

**Validates: 02-REQ-6.E2**

### Property 5: Dual-Interface Availability

*For any* signal operation (get, set, subscribe) and *for any* registered signal, the operation SHALL produce identical results whether the consumer connects via UDS or via TCP on port 55556.

**Validates: 02-REQ-4.1, 02-REQ-4.2, 02-REQ-5.1, 02-REQ-5.2**

### Property 6: Overlay Merge Correctness

*For any* startup with both the standard VSS file and the overlay file, custom signal paths SHALL coexist with standard signal paths without conflicts. Standard signal paths SHALL remain accessible and unmodified after overlay application.

**Validates: 02-REQ-2.2, 02-REQ-3.1**

## Startup and Shutdown Procedure

### Startup Sequence

1. The Kuksa Databroker binary is started with the configured arguments (address, port, VSS file, overlay file).
2. The binary loads the standard VSS v5.1 signal tree.
3. The binary loads and merges the custom overlay signals.
4. The binary opens the TCP listener on port 55556.
5. The binary opens the UDS listener (if configured).
6. The binary begins accepting gRPC connections on both endpoints.
7. Health check reports ready.

### Shutdown Sequence

1. The binary receives a termination signal (SIGTERM or SIGINT).
2. Active gRPC streams are closed.
3. Listeners are closed.
4. The binary exits with code 0.

## Error Handling

| Error Condition | Behavior | Requirement |
|----------------|----------|-------------|
| Overlay file missing or malformed JSON | Databroker fails to start with non-zero exit code | 02-REQ-2.E1 |
| Binary fails to start (any reason) | Non-zero exit code, error logged | 02-REQ-1.E1 |
| Get/set on non-existent signal path | gRPC NOT_FOUND error returned | 02-REQ-6.E1 |
| Type mismatch on write | gRPC INVALID_ARGUMENT error returned | 02-REQ-6.E2 |
| Consumer connects while starting up | gRPC UNAVAILABLE; consumer retries | 02-REQ-5.E1 |
| TCP port 55556 already in use | Binary fails to start; port conflict in logs | 02-REQ-1.E1 |
| UDS path not writable | UDS endpoint unavailable; TCP still works | 02-REQ-4.E1 |
| Subscriber disconnects mid-stream | gRPC stream terminates; updates discarded | 02-REQ-7.E1 |

## Technology Stack

| Technology | Version / Reference | Purpose |
|------------|-------------------|---------|
| Eclipse Kuksa Databroker | Latest stable binary from [GitHub releases](https://github.com/eclipse-kuksa/kuksa-databroker) | Vehicle signal broker |
| COVESA VSS | v5.1 | Standard vehicle signal tree |
| gRPC | As provided by Kuksa | Signal access protocol |
| Docker / Podman | As available in local dev env | Container runtime (for local dev) |
| docker-compose | As configured in 01_project_setup | Local infrastructure orchestration |

## Definition of Done

A task group is complete when ALL of the following are true:

1. All subtasks within the group are checked off (`[x]`)
2. All spec tests (`test_spec.md` entries) for the task group pass
3. All property tests for the task group pass
4. All previously passing tests still pass (no regressions)
5. No linter warnings or errors introduced
6. Code is committed on a feature branch and pushed to remote
7. Feature branch is merged back to `develop`
8. `tasks.md` checkboxes are updated to reflect completion

## Testing Strategy

Testing validates that the DATA_BROKER is correctly deployed and configured, not that Kuksa Databroker itself works (it is third-party software).

### What We Test

1. **Deployment** -- binary starts, becomes healthy, exposes the expected endpoints.
2. **Signal registration** -- all 8 signals are present with correct types after overlay is applied.
3. **Basic operations** -- write a value, read it back, subscribe and receive updates. This validates our configuration, not Kuksa internals.
4. **Dual-interface access** -- both UDS and TCP endpoints are functional.

### What We Do Not Test

- Kuksa Databroker internal correctness (covered by upstream tests).
- Performance benchmarks.
- TLS/authorization (not configured for demo scope).

### Test Implementation

Tests are written in Go, located in `tests/setup/`, and use gRPC to connect to the DATA_BROKER.

- **Spec tests:** `cd tests/setup && go test -run TestDataBroker -v`
- **Infrastructure:** `make infra-up` / `make infra-down`
