# PRD: PARKING_OPERATOR_ADAPTOR (Phase 2.3)

> Extracted from the master PRD at `.specs/prd.md`. This spec covers the PARKING_OPERATOR_ADAPTOR component of Phase 2.3.

## Scope

Implement the PARKING_OPERATOR_ADAPTOR as a containerized Rust application running in the RHIVOS QM partition. The adaptor bridges the in-vehicle PARKING_APP (via gRPC) with a PARKING_OPERATOR backend (via REST), and autonomously manages parking sessions based on lock/unlock events received from DATA_BROKER.

## Component Description

- Containerized application running in the RHIVOS QM partition
- Implements a common gRPC interface towards the PARKING_APP:
  - `StartSession(zone_id)` -- manually start a parking session
  - `StopSession()` -- manually stop a parking session
  - `GetStatus()` -- get current session status
  - `GetRate()` -- get current parking rate
- Implements a proprietary REST interface towards its PARKING_OPERATOR:
  - `POST /parking/start` with `{vehicle_id, zone_id, timestamp}`
  - `POST /parking/stop` with `{session_id, timestamp}`
  - `GET /parking/status/{session_id}`
- Subscribes to lock/unlock state signals from DATA_BROKER (cross-partition, network gRPC)
  - Signal: `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`
- Autonomously starts parking sessions when a lock event is detected, and stops sessions on unlock events
- The PARKING_APP can override the autonomous session behavior (e.g., manual stop, prevent auto-start)
- Writes parking session state (`Vehicle.Parking.SessionActive`) to DATA_BROKER
- Static gRPC port from configuration
- Only one adaptor runs at a time per vehicle

## Rate Model

Two rate types are supported:

- **per_hour**: Charged by the hour (e.g., 2.50 EUR/hr)
- **flat_fee**: A fixed fee per parking session (e.g., 5.00 EUR)

Rate information is obtained from the PARKING_OPERATOR via the REST API.

## Session Lifecycle

Session ownership belongs to the PARKING_OPERATOR_ADAPTOR:

1. **Setup phase:** The PARKING_APP discovers operators via PARKING_FEE_SERVICE, selects one, and triggers adapter installation via UPDATE_SERVICE.
2. **Autonomous operation:** Once running, the PARKING_OPERATOR_ADAPTOR subscribes to lock/unlock events from DATA_BROKER and autonomously starts/stops parking sessions with the PARKING_OPERATOR.
3. **Override:** The PARKING_APP can override the session state (e.g., manually stop a session before unlocking, or prevent auto-start).

## Configuration

The adaptor reads its configuration from environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `PARKING_OPERATOR_URL` | `http://localhost:8080` | Base URL of the PARKING_OPERATOR REST API |
| `DATA_BROKER_ADDR` | `http://localhost:55556` | DATA_BROKER gRPC address (network TCP) |
| `GRPC_PORT` | `50053` | Port for the adaptor's own gRPC service |
| `VEHICLE_ID` | `DEMO-VIN-001` | Vehicle identifier sent in operator requests |
| `ZONE_ID` | `zone-demo-1` | Default parking zone identifier |

## Tech Stack

- Language: Rust (edition 2021)
- gRPC framework: tonic
- HTTP client: reqwest
- Async runtime: tokio
- Serialization: serde / serde_json
- Logging: tracing

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 3 | 1 | Uses Rust workspace and parking-operator-adaptor skeleton from group 3; group 3 creates Cargo workspace with the crate |
| 01_project_setup | 6 | 1 | Uses proto definitions (parking_adaptor.proto, common.proto) from group 6; group 6 creates proto files with ParkingAdaptor RPCs |
| 02_data_broker | 2 | 5 | Uses Kuksa Databroker compose service for integration tests; group 2 creates compose.yml with databroker |

## Clarifications

- **C1 (gRPC port):** Default gRPC port is 50053 (not 50052, which is used by UPDATE_SERVICE). Configurable via `GRPC_PORT` env var.
- **C2 (BrokerClient pattern):** DATA_BROKER connection uses the same kuksa.val.v1 gRPC pattern as LOCKING_SERVICE and CLOUD_GATEWAY_CLIENT. Proto files are vendored per-crate into `rhivos/parking-operator-adaptor/proto/`.
- **C3 (Session state):** The adaptor maintains an in-memory session state: `session_id` (from operator), `zone_id`, `start_time` (Unix timestamp), `rate` (type + amount + currency), `active` (bool). On service restart, session state is lost (demo simplification).
- **C4 (Override mechanism):** `StartSession` and `StopSession` gRPC RPCs override autonomous behavior. A manual `StopSession` call stops the session regardless of lock state. Autonomous behavior resumes on the next lock/unlock cycle — there is no persistent "disable auto-session" flag.
- **C5 (Lock event semantics):** IsLocked changes to `true` → start session (if no session active). IsLocked changes to `false` → stop session (if session active). Events are processed sequentially; latest state wins.
- **C6 (Idempotent sessions):** Lock event while session is already active = no-op (log info). Unlock event while no session is active = no-op (log info).
- **C7 (Operator REST failure):** If the PARKING_OPERATOR REST API call fails, the adaptor retries up to 3 times with exponential backoff (1s, 2s, 4s). After all retries fail, the operation is considered failed and the error is logged. Session state is not updated on failure.
- **C8 (Rate source):** The rate is returned in the PARKING_OPERATOR's start session response as `{rate_type, amount, currency}`. The adaptor caches the rate in session state. `GetRate` returns the cached rate (or empty if no session active).
- **C9 (GetStatus response):** Returns `{session_id, active, zone_id, start_time, rate}`. If no session is active, returns `{active: false}` with empty fields.
- **C10 (SessionActive signal):** The adaptor writes `Vehicle.Parking.SessionActive = true` to DATA_BROKER when a session starts, and `false` when it stops. This allows PARKING_APP to subscribe for UI updates.
- **C11 (Operator start response):** The operator's `POST /parking/start` response includes `{session_id, status, rate: {type, amount, currency}}`.
- **C12 (Operator stop response):** The operator's `POST /parking/stop` response includes `{session_id, status, duration_seconds, total_amount, currency}`.

## Out-of-Scope

- Persistent session state across restarts
- Multiple concurrent sessions
- Rate negotiation or dynamic pricing
- Payment processing
- Adapter-to-adapter communication
