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
| `GRPC_PORT` | `50052` | Port for the adaptor's own gRPC service |
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
| 01_project_setup | 2 | 1 | Uses repo structure and Rust project skeleton from group 2 |
| 02_data_broker | 3 | 1 | Subscribes to lock/unlock signals, writes session state |
| 07_update_service | 4 | 1 | Managed by UPDATE_SERVICE (installed/started as container) |
