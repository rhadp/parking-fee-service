# Requirements: PARKING_OPERATOR_ADAPTOR (Spec 08)

> EARS-syntax requirements for the PARKING_OPERATOR_ADAPTOR.
> Derived from the PRD at `.specs/08_parking_operator_adaptor/prd.md` and the master PRD at `.specs/prd.md`.

## Introduction

The PARKING_OPERATOR_ADAPTOR is a containerized Rust application running in the RHIVOS QM partition. It autonomously manages parking sessions by subscribing to lock/unlock events from DATA_BROKER, communicating with a PARKING_OPERATOR via REST, and exposing a gRPC interface for manual override by the PARKING_APP. It publishes session state to DATA_BROKER so that downstream consumers (PARKING_APP, CLOUD_GATEWAY_CLIENT) can observe parking status.

## Glossary

| Term | Definition |
|------|-----------|
| DATA_BROKER | Eclipse Kuksa Databroker running in the RHIVOS safety partition; provides VSS-compliant gRPC pub/sub for vehicle signals |
| PARKING_OPERATOR | External (or mock) REST service that manages parking sessions on behalf of a parking provider |
| PARKING_APP | Android Automotive OS application on the IVI that can override autonomous session behavior |
| SessionActive | Custom VSS signal (`Vehicle.Parking.SessionActive`) indicating whether a parking session is currently active |
| IsLocked | VSS signal (`Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`) representing the driver-side door lock state |
| Rate type | Billing model: either `per_hour` (charged by duration) or `flat_fee` (fixed amount per session) |
| zone_id | Identifier for the parking zone the vehicle is parked in |
| vehicle_id | Unique vehicle identifier (VIN) sent to the PARKING_OPERATOR |

## Requirements

### Requirement 1: gRPC Service Interface

**User Story:** As a PARKING_APP, I want to query adaptor status and manually control parking sessions, so that I can display session information and override autonomous behavior.

#### Acceptance Criteria

1. **08-REQ-1.1** The PARKING_OPERATOR_ADAPTOR shall expose a gRPC service on a static port (default 50052) with RPCs: `StartSession(zone_id)`, `StopSession()`, `GetStatus()`, and `GetRate()`.
2. **08-REQ-1.2** WHEN a `StartSession` RPC is called and no session is active, the PARKING_OPERATOR_ADAPTOR shall start a parking session with the PARKING_OPERATOR and return the `session_id` and `status`.
3. **08-REQ-1.3** WHEN a `StopSession` RPC is called and a session is active, the PARKING_OPERATOR_ADAPTOR shall stop the parking session with the PARKING_OPERATOR and return the `session_id`, `duration_seconds`, `fee`, and `status`.

#### Edge Cases

1. **08-REQ-1.E1** IF `StartSession` is called while a session is already active, THEN the PARKING_OPERATOR_ADAPTOR shall return a gRPC `ALREADY_EXISTS` error.
2. **08-REQ-1.E2** IF `StopSession` is called while no session is active, THEN the PARKING_OPERATOR_ADAPTOR shall return a gRPC `NOT_FOUND` error.

### Requirement 2: DATA_BROKER Subscription for Lock/Unlock Events

**User Story:** As a vehicle system, I want the adaptor to subscribe to lock/unlock events, so that parking sessions can be managed autonomously without user interaction.

#### Acceptance Criteria

1. **08-REQ-2.1** WHEN the PARKING_OPERATOR_ADAPTOR starts, it shall subscribe to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` on DATA_BROKER via network gRPC (cross-partition, TCP).
2. **08-REQ-2.2** The PARKING_OPERATOR_ADAPTOR shall maintain the DATA_BROKER subscription for the lifetime of the process.

#### Edge Cases

1. **08-REQ-2.E1** IF the DATA_BROKER is unreachable at startup, THEN the PARKING_OPERATOR_ADAPTOR shall log the error and retry connection with backoff; autonomous mode shall remain inactive until connected.

### Requirement 3: Autonomous Session Start on Lock Event

**User Story:** As a vehicle owner, I want the parking session to start automatically when I lock my car, so that I do not need to interact with any app.

#### Acceptance Criteria

1. **08-REQ-3.1** WHEN the DATA_BROKER publishes `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true` and no session is active, the PARKING_OPERATOR_ADAPTOR shall call the PARKING_OPERATOR `POST /parking/start` with `{vehicle_id, zone_id, timestamp}` and, upon a successful response, transition the session state to active.

#### Edge Cases

1. **08-REQ-3.E1** IF a lock event is received while a session is already active (double lock), THEN the PARKING_OPERATOR_ADAPTOR shall ignore the event and not create a duplicate session.

### Requirement 4: Autonomous Session Stop on Unlock Event

**User Story:** As a vehicle owner, I want the parking session to stop automatically when I unlock my car, so that I am charged only for the actual parking duration.

#### Acceptance Criteria

1. **08-REQ-4.1** WHEN the DATA_BROKER publishes `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = false` and a session is active, the PARKING_OPERATOR_ADAPTOR shall call the PARKING_OPERATOR `POST /parking/stop` with `{session_id, timestamp}` and, upon a successful response, transition the session state to idle.

#### Edge Cases

1. **08-REQ-4.E1** IF an unlock event is received while no session is active (double unlock), THEN the PARKING_OPERATOR_ADAPTOR shall ignore the event and not call the PARKING_OPERATOR.

### Requirement 5: PARKING_APP Override Capability

**User Story:** As a PARKING_APP, I want to manually start or stop sessions and prevent auto-start, so that I can give the driver explicit control over parking.

#### Acceptance Criteria

1. **08-REQ-5.1** Manual `StartSession` and `StopSession` RPCs shall produce the same state transitions, DATA_BROKER updates, and PARKING_OPERATOR REST calls as the equivalent autonomous operations.
2. **08-REQ-5.2** WHEN a session has been manually started via `StartSession`, a subsequent lock event from DATA_BROKER shall be ignored (session already active).

### Requirement 6: Session State Publication to DATA_BROKER

**User Story:** As a PARKING_APP or CLOUD_GATEWAY_CLIENT, I want to observe the parking session state on DATA_BROKER, so that I can display status or relay telemetry.

#### Acceptance Criteria

1. **08-REQ-6.1** WHEN the session state transitions to active (after a successful operator start), the PARKING_OPERATOR_ADAPTOR shall write `Vehicle.Parking.SessionActive = true` to DATA_BROKER.
2. **08-REQ-6.2** WHEN the session state transitions to idle (after a successful operator stop), the PARKING_OPERATOR_ADAPTOR shall write `Vehicle.Parking.SessionActive = false` to DATA_BROKER.

#### Edge Cases

1. **08-REQ-6.E1** IF the DATA_BROKER write fails, THEN the PARKING_OPERATOR_ADAPTOR shall log the error; the internal session state shall remain as transitioned (signal may be stale).

### Requirement 7: PARKING_OPERATOR REST Client

**User Story:** As a system integrator, I want the adaptor to communicate with the PARKING_OPERATOR via REST, so that it can start/stop sessions and query status using the operator's proprietary API.

#### Acceptance Criteria

1. **08-REQ-7.1** The PARKING_OPERATOR_ADAPTOR shall send `POST /parking/start` with JSON body `{vehicle_id, zone_id, timestamp}` and parse the response `{session_id, status}`.
2. **08-REQ-7.2** The PARKING_OPERATOR_ADAPTOR shall send `POST /parking/stop` with JSON body `{session_id, timestamp}` and parse the response `{session_id, duration, fee, status}`.
3. **08-REQ-7.3** The PARKING_OPERATOR_ADAPTOR shall send `GET /parking/status/{session_id}` and parse the response to obtain current session status from the operator.

#### Edge Cases

1. **08-REQ-7.E1** IF the PARKING_OPERATOR is unreachable (connection refused or timeout > 5s), THEN the PARKING_OPERATOR_ADAPTOR shall log the error, leave the session state unchanged, and return gRPC `UNAVAILABLE` to any pending RPC caller.
2. **08-REQ-7.E2** IF the PARKING_OPERATOR returns a non-200 HTTP status, THEN the PARKING_OPERATOR_ADAPTOR shall log the error with status code and body, leave the session state unchanged, and return gRPC `INTERNAL` to any pending RPC caller.

### Requirement 8: Configuration

**User Story:** As a deployer, I want the adaptor to be configurable via environment variables, so that it can connect to the correct DATA_BROKER, PARKING_OPERATOR, and expose its gRPC service on the right port.

#### Acceptance Criteria

1. **08-REQ-8.1** The PARKING_OPERATOR_ADAPTOR shall read configuration from environment variables: `DATA_BROKER_ADDR`, `PARKING_OPERATOR_URL`, `GRPC_PORT`, `ZONE_ID`, and `VEHICLE_ID`, with sensible defaults as specified in the PRD.

### Requirement 9: Rate Information

**User Story:** As a PARKING_APP, I want to query the parking rate, so that I can display the cost to the driver.

#### Acceptance Criteria

1. **08-REQ-9.1** WHEN `GetRate` is called, the PARKING_OPERATOR_ADAPTOR shall return the rate information including rate type (`per_hour` or `flat_fee`), rate amount, and currency.

#### Edge Cases

1. **08-REQ-9.E1** IF no zone is configured or rate information is unavailable, THEN the PARKING_OPERATOR_ADAPTOR shall return a gRPC `FAILED_PRECONDITION` error.

## Traceability

| Requirement | PRD Section |
|-------------|-------------|
| 08-REQ-1 | gRPC Interface (towards PARKING_APP) |
| 08-REQ-2 | Subscribes to lock/unlock state signals from DATA_BROKER |
| 08-REQ-3 | Autonomous Session Management: lock event |
| 08-REQ-4 | Autonomous Session Management: unlock event |
| 08-REQ-5 | PARKING_APP override capability |
| 08-REQ-6 | Writes parking session state to DATA_BROKER |
| 08-REQ-7 | REST Interface (towards PARKING_OPERATOR) |
| 08-REQ-8 | Configuration |
| 08-REQ-9 | Rate Model |
