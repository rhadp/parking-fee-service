# Requirements: DATA_BROKER (Spec 02)

> EARS-syntax requirements for Eclipse Kuksa Databroker deployment and configuration in the RHIVOS safety partition.

## References

- Master PRD: `.specs/prd.md`
- Component PRD: `.specs/02_data_broker/prd.md`
- Design: `.specs/02_data_broker/design.md`

## Terminology

| Term | Definition |
|------|-----------|
| DATA_BROKER | Eclipse Kuksa Databroker instance deployed in the RHIVOS safety partition |
| VSS | COVESA Vehicle Signal Specification, version 5.1 |
| UDS | Unix Domain Socket, used for same-partition gRPC communication |
| Custom signal | A VSS signal defined in an overlay file, not part of the standard VSS tree |
| Consumer | Any component that reads from or writes to the DATA_BROKER via gRPC |

## Requirements

### 02-REQ-1: Databroker Container Deployment

**02-REQ-1.1** The system SHALL deploy Eclipse Kuksa Databroker as a container in the local development infrastructure managed by docker-compose.

**02-REQ-1.2** When the local infrastructure is started via `make infra-up`, the DATA_BROKER container SHALL reach a healthy state within 30 seconds.

**02-REQ-1.3** When the DATA_BROKER container fails to start, the system SHALL report the failure through the container runtime health check mechanism so that dependent services do not attempt to connect.

### 02-REQ-2: Standard VSS Signal Configuration

**02-REQ-2.1** The DATA_BROKER SHALL register the following standard VSS v5.1 signals on startup:

| Signal Path | Data Type |
|-------------|-----------|
| `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` | bool |
| `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` | bool |
| `Vehicle.CurrentLocation.Latitude` | double |
| `Vehicle.CurrentLocation.Longitude` | double |
| `Vehicle.Speed` | float |

**02-REQ-2.2** When a consumer reads a standard VSS signal that has not yet been written to, the DATA_BROKER SHALL return the signal metadata with no current value (not-yet-set state).

### 02-REQ-3: Custom Signal Configuration

**02-REQ-3.1** The DATA_BROKER SHALL register the following custom signals on startup, defined via a VSS overlay file:

| Signal Path | Data Type | Description |
|-------------|-----------|-------------|
| `Vehicle.Parking.SessionActive` | bool | Parking session state |
| `Vehicle.Command.Door.Lock` | string | Lock/unlock command request (JSON payload) |
| `Vehicle.Command.Door.Response` | string | Command execution result (JSON payload) |

**02-REQ-3.2** When a consumer attempts to get or set a signal path that does not exist in the registered VSS tree or overlay, the DATA_BROKER SHALL return a gRPC NOT_FOUND error.

### 02-REQ-4: Same-Partition Access via UDS

**02-REQ-4.1** The DATA_BROKER SHALL expose a gRPC endpoint over a Unix Domain Socket so that same-partition consumers (LOCKING_SERVICE, CLOUD_GATEWAY_CLIENT) can connect without using the network stack.

**02-REQ-4.2** When a same-partition consumer connects to the UDS endpoint, it SHALL be able to perform get, set, and subscribe operations on all registered signals.

**02-REQ-4.3** When the UDS path is not accessible (e.g., permission denied, path does not exist), the consumer SHALL receive a connection error rather than silently failing.

### 02-REQ-5: Cross-Partition Access via Network TCP

**02-REQ-5.1** The DATA_BROKER SHALL expose a gRPC endpoint over network TCP on port 55556 so that cross-partition consumers (PARKING_APP, PARKING_OPERATOR_ADAPTOR) can connect.

**02-REQ-5.2** When a cross-partition consumer connects to `localhost:55556`, it SHALL be able to perform get, set, and subscribe operations on all registered signals.

**02-REQ-5.3** When the DATA_BROKER is unreachable on port 55556 (e.g., container not running, port not exposed), the consumer SHALL receive a gRPC UNAVAILABLE error on connection attempt.

### 02-REQ-6: Signal Read/Write Access Control

**02-REQ-6.1** The DATA_BROKER SHALL allow any connected consumer to read (get or subscribe to) any registered signal.

**02-REQ-6.2** The DATA_BROKER SHALL allow any connected consumer to write (set) any registered signal.

> **Note:** For the demo scope, Kuksa Databroker is deployed without authorization. Full read/write access control (restricting which component can write which signal) is deferred to production configuration. The access control model describing intended ownership is documented in the design document for reference.

**02-REQ-6.3** When a consumer writes a value to a signal, all active subscriptions on that signal SHALL receive the updated value.

## Edge Cases

| ID | Scenario | Expected Behavior |
|----|----------|-------------------|
| 02-EDGE-1 | Consumer writes to a non-existent signal path | DATA_BROKER returns gRPC NOT_FOUND error |
| 02-EDGE-2 | Consumer reads a signal that has never been written | DATA_BROKER returns signal metadata with no current value |
| 02-EDGE-3 | Consumer connects while DATA_BROKER is starting up | Consumer receives gRPC UNAVAILABLE; consumer is expected to retry |
| 02-EDGE-4 | Multiple consumers subscribe to the same signal | All subscribers receive the update when the signal value changes |
| 02-EDGE-5 | Consumer writes a value with wrong type (e.g., string to bool signal) | DATA_BROKER returns gRPC INVALID_ARGUMENT error |
| 02-EDGE-6 | Network TCP connection drops mid-subscription | gRPC stream terminates; consumer is expected to re-subscribe |
