# PRD: PARKING_OPERATOR_ADAPTOR (Phase 2.3)

> Extracted from the master PRD at `.specs/prd.md`. This spec covers the PARKING_OPERATOR_ADAPTOR and mock PARKING_OPERATOR of Phase 2.3.

## Scope

Implement the PARKING_OPERATOR_ADAPTOR in Rust, running in the RHIVOS QM partition. Also implement a mock PARKING_OPERATOR (Go REST service) for testing.

## PARKING_OPERATOR_ADAPTOR

- Containerized application running in the RHIVOS QM partition
- Implements a common gRPC interface towards the PARKING_APP (StartSession, StopSession, GetStatus, GetRate)
- Implements a proprietary REST interface towards its PARKING_OPERATOR
- Subscribes to lock/unlock state signals from DATA_BROKER (cross-partition, network gRPC)
- Autonomously starts parking sessions when a lock event is detected, and stops sessions on unlock events
- The PARKING_APP can override the autonomous session behavior (e.g., manual stop, prevent auto-start)
- Writes parking session state (`Vehicle.Parking.SessionActive`) to DATA_BROKER
- Local development port: 50052

### gRPC Interface (towards PARKING_APP)

Defined in `proto/parking_adaptor.proto`:

- `StartSession(zone_id)` — manually start a parking session (override)
- `StopSession(session_id)` — manually stop a parking session (override)
- `GetStatus()` — get current session status
- `GetRate()` — get current parking rate

### REST Interface (towards PARKING_OPERATOR)

- `POST /parking/start` — start parking session with operator
  - Body: `{vehicle_id, zone_id, timestamp}`
  - Response: `{session_id, status}`
- `POST /parking/stop` — stop parking session
  - Body: `{session_id, timestamp}`
  - Response: `{session_id, duration, fee, status}`

### Autonomous Session Management

1. Subscribes to DATA_BROKER for `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`
2. On lock event (IsLocked = true): calls PARKING_OPERATOR `POST /parking/start`
3. On unlock event (IsLocked = false): calls PARKING_OPERATOR `POST /parking/stop`
4. Updates `Vehicle.Parking.SessionActive` on DATA_BROKER accordingly

## Mock PARKING_OPERATOR

- Mock service that receives start/stop parking events from PARKING_OPERATOR_ADAPTOR
- Simulates a real parking operator's REST API
- Returns mock session IDs, durations, and fees
- Language: Go (aligns with backend services)

## Tech Stack

- PARKING_OPERATOR_ADAPTOR: Rust, tonic (gRPC), reqwest (HTTP client)
- Mock PARKING_OPERATOR: Go, net/http

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 2 | 1 | Requires Rust workspace skeleton, proto definitions, and build system |
| 02_data_broker | 2 | 1 | Requires running DATA_BROKER with VSS signals configured |
