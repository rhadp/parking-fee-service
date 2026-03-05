# Design: DATA_BROKER (Spec 02)

> Design document for deploying and configuring Eclipse Kuksa Databroker as the DATA_BROKER in the RHIVOS safety partition.

## References

- Master PRD: `.specs/prd.md`
- Component PRD: `.specs/02_data_broker/prd.md`
- Requirements: `.specs/02_data_broker/requirements.md`
- Eclipse Kuksa Databroker: https://github.com/eclipse-kuksa/kuksa-databroker
- COVESA VSS v5.1: https://covesa.github.io/vehicle_signal_specification/

## Architecture Overview

The DATA_BROKER is Eclipse Kuksa Databroker deployed as a pre-built container image. It is not a custom implementation. The deployment consists of:

1. **Kuksa Databroker container** -- pulled from the official `ghcr.io/eclipse-kuksa/kuksa-databroker` image and configured in the project's docker-compose file.
2. **VSS overlay file** -- a JSON file that defines custom signals not present in the standard VSS v5.1 tree. Kuksa Databroker loads this overlay at startup.
3. **Network configuration** -- dual access via Unix Domain Socket (same-partition) and TCP port 55556 (cross-partition).

```
+------------------------------------------------------------------+
|  RHIVOS Safety Partition (docker-compose network)                |
|                                                                  |
|  +--------------------+     +-------------------------------+    |
|  | LOCKING_SERVICE    |---->|                               |    |
|  +--------------------+ UDS |   KUKSA DATABROKER            |    |
|  +--------------------+     |   (ghcr.io/eclipse-kuksa/     |    |
|  | CLOUD_GATEWAY_     |---->|    kuksa-databroker)           |    |
|  | CLIENT             | UDS |                               |    |
|  +--------------------+     |   - VSS v5.1 standard signals |    |
|                             |   - Custom overlay signals    |    |
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

## Technology Stack

| Technology | Version / Reference | Purpose |
|------------|-------------------|---------|
| Eclipse Kuksa Databroker | Latest stable (`ghcr.io/eclipse-kuksa/kuksa-databroker`) | Vehicle signal broker |
| COVESA VSS | v5.1 | Standard vehicle signal tree |
| gRPC | As provided by Kuksa | Signal access protocol |
| Docker / Podman | As available in local dev env | Container runtime |
| docker-compose | As configured in 01_project_setup | Local infrastructure orchestration |

## VSS Overlay Configuration

Custom signals are defined in a VSS overlay JSON file mounted into the Kuksa Databroker container. The overlay extends the standard VSS tree with parking-demo-specific signals.

### Overlay File: `config/vss/vss_overlay.json`

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

All custom signals use `actuator` type because they represent values that are actively set by components (not passively sensed). The `string` type for command signals allows flexible JSON payloads without requiring proto/VSS schema changes for payload evolution.

## Network Configuration

### Unix Domain Socket (Same-Partition)

- **Path:** `/tmp/kuksa/databroker.sock` (mounted as a shared volume in docker-compose)
- **Protocol:** gRPC over UDS
- **Consumers:** LOCKING_SERVICE, CLOUD_GATEWAY_CLIENT
- **No TLS** for local development (UDS provides filesystem-level access control)

### Network TCP (Cross-Partition)

- **Host:** `0.0.0.0` (bind to all interfaces within the container)
- **Port:** `55556`
- **Protocol:** gRPC over HTTP/2
- **Consumers:** PARKING_APP, PARKING_OPERATOR_ADAPTOR, mock sensors, test harnesses
- **No TLS** for local development

### Kuksa Databroker Startup Arguments

```
databroker --address 0.0.0.0 --port 55556 --vss config/vss/vss.json --vss-overlay config/vss/vss_overlay.json
```

The `--vss` argument loads the standard VSS v5.1 JSON file (bundled in the container image or mounted). The `--vss-overlay` argument loads the custom overlay on top.

> Note: UDS support is configured via the `--uds` flag or environment variable. The exact flag depends on the Kuksa Databroker version. If native UDS is not available, consumers connect via TCP on localhost.

## Access Control Model

For the demo scope, Kuksa Databroker runs without authorization (no `--jwt-public-key` or token enforcement). All connected consumers have full read/write access to all signals.

The intended ownership model (for documentation and future enforcement) is:

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

## Docker-Compose Integration

The DATA_BROKER service is added to the docker-compose configuration established by `01_project_setup`. The service definition:

```yaml
services:
  databroker:
    image: ghcr.io/eclipse-kuksa/kuksa-databroker:latest
    container_name: databroker
    ports:
      - "55556:55556"
    volumes:
      - ./config/vss/vss_overlay.json:/config/vss/vss_overlay.json:ro
      - kuksa-uds:/tmp/kuksa
    command: >
      --address 0.0.0.0
      --port 55556
    healthcheck:
      test: ["CMD", "grpc_health_probe", "-addr=:55556"]
      interval: 5s
      timeout: 3s
      retries: 5
      start_period: 10s
    restart: unless-stopped

volumes:
  kuksa-uds:
```

> Note: The exact image tag, command-line flags, and health check method should be validated against the Kuksa Databroker version used. The `grpc_health_probe` binary may need to be installed in the container, or an alternative health check (e.g., `grpcurl`) may be used. An alternative is a simple TCP check: `test: ["CMD-SHELL", "nc -z localhost 55556"]`.

## Correctness Properties

The following properties must hold for a correctly deployed and configured DATA_BROKER:

### CP-1: Signal Registration Completeness

All 8 signals (5 standard VSS + 3 custom overlay) SHALL be registered in the DATA_BROKER signal tree after startup. A `ListMetadata` or equivalent query must return entries for every signal path listed in the requirements.

### CP-2: Signal Read/Write Consistency

When a value is written to a signal via `SetRequest`, a subsequent `GetRequest` for the same signal SHALL return the most recently written value. The DATA_BROKER does not buffer history; only the latest value is retained.

### CP-3: Pub/Sub Delivery Guarantee

When a consumer subscribes to a signal and another consumer writes a new value to that signal, the subscriber SHALL receive the updated value on the subscription stream. Delivery is best-effort; if the subscriber disconnects, updates are lost (no durable queues).

### CP-4: Type Enforcement

The DATA_BROKER SHALL reject writes where the provided value type does not match the signal's registered data type (e.g., writing a string to a boolean signal). The rejection is communicated via a gRPC error response.

### CP-5: Dual-Interface Availability

Both the UDS endpoint and the TCP:55556 endpoint SHALL be available simultaneously after startup. A consumer connecting via either transport must be able to perform identical operations.

### CP-6: Overlay Merge Correctness

Custom signals from the overlay file SHALL coexist with standard VSS signals without conflicts. Standard signal paths must remain accessible and unmodified after overlay application.

## Error Handling

| Error Scenario | Behavior |
|---------------|----------|
| Overlay file missing or malformed | Databroker fails to start; container exits with non-zero code; docker-compose health check reports unhealthy |
| Invalid signal path in get/set request | gRPC NOT_FOUND error returned to client |
| Type mismatch on write | gRPC INVALID_ARGUMENT error returned to client |
| Consumer connection while databroker starting | gRPC UNAVAILABLE; consumer should implement retry with backoff |
| TCP port 55556 already in use | Container fails to start; port conflict reported in logs |
| UDS path not writable | Databroker logs error; UDS endpoint unavailable; TCP endpoint still works |

## Testing Strategy

Testing validates that the DATA_BROKER is correctly deployed and configured, not that Kuksa Databroker itself works (it is third-party software).

### What We Test

1. **Deployment** -- container starts, becomes healthy, exposes the expected endpoints.
2. **Signal registration** -- all 8 signals are present with correct types after overlay is applied.
3. **Basic operations** -- write a value, read it back, subscribe and receive updates. This validates our configuration, not Kuksa internals.
4. **Cross-partition access** -- TCP port 55556 is reachable and functional from outside the container.

### What We Do Not Test

- Kuksa Databroker internal correctness (covered by upstream tests).
- Performance benchmarks.
- TLS/authorization (not configured for demo scope).

### Test Implementation

Tests are written in Go, located in `tests/setup/`, and use the Kuksa gRPC client library (or raw gRPC) to connect to the DATA_BROKER.

- **Spec tests:** `cd tests/setup && go test -run TestDataBroker -v`
- **Infrastructure:** `make infra-up` / `make infra-down`

See `test_spec.md` for detailed test specifications.

## Definition of Done

1. Kuksa Databroker container starts successfully via `make infra-up`.
2. Health check passes within 30 seconds.
3. All 5 standard VSS signals are queryable via gRPC on port 55556.
4. All 3 custom signals from the overlay are queryable via gRPC on port 55556.
5. Signal write/read round-trip works for each data type (bool, float, double, string).
6. Pub/sub subscription delivers value updates to subscribers.
7. All spec tests in `tests/setup/` pass: `cd tests/setup && go test -run TestDataBroker -v`.
