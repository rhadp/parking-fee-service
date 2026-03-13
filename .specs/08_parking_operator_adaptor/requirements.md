# Requirements Document

## Introduction

This document specifies the requirements for the PARKING_OPERATOR_ADAPTOR component (Phase 2.3) of the SDV Parking Demo System. The adaptor is a containerized Rust application running in the RHIVOS QM partition that bridges the in-vehicle PARKING_APP (via gRPC) with a PARKING_OPERATOR backend (via REST), and autonomously manages parking sessions based on lock/unlock events from DATA_BROKER.

## Glossary

- **PARKING_OPERATOR_ADAPTOR:** A containerized Rust application that interfaces between the vehicle and a specific parking operator.
- **PARKING_APP:** An Android Automotive OS application that requests adapter installation and manages the parking workflow.
- **PARKING_OPERATOR:** A backend REST API representing a parking service provider.
- **DATA_BROKER:** Eclipse Kuksa Databroker providing VSS-compliant gRPC pub/sub for vehicle signals.
- **Session:** A parking session started when the vehicle locks and stopped when it unlocks, tracked by the PARKING_OPERATOR.
- **Autonomous operation:** The adaptor automatically starts/stops sessions based on lock/unlock events without user intervention.
- **Override:** The PARKING_APP manually starting or stopping a session via gRPC, overriding autonomous behavior.
- **Lock event:** A change in `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` signal to `true`.
- **Unlock event:** A change in `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` signal to `false`.
- **SessionActive signal:** `Vehicle.Parking.SessionActive` VSS signal written to DATA_BROKER indicating whether a parking session is currently active.
- **Rate:** Pricing information for parking: either per-hour or flat-fee, with amount and currency.
- **BrokerClient:** A gRPC client for the kuksa.val.v1 API used to read/write/subscribe to vehicle signals.

## Requirements

### Requirement 1: Autonomous Session Start

**User Story:** As a vehicle system, I want the adaptor to automatically start a parking session when the vehicle locks, so that parking fees are tracked without user intervention.

#### Acceptance Criteria

1. [08-REQ-1.1] WHEN the `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` signal changes to `true` AND no session is currently active, THE adaptor SHALL call `POST /parking/start` on the PARKING_OPERATOR with `vehicle_id`, `zone_id`, and `timestamp`.
2. [08-REQ-1.2] WHEN the PARKING_OPERATOR returns a successful start response, THE adaptor SHALL store the `session_id`, `zone_id`, `start_time`, and `rate` in the in-memory session state.
3. [08-REQ-1.3] WHEN a session is successfully started, THE adaptor SHALL write `Vehicle.Parking.SessionActive = true` to DATA_BROKER.

#### Edge Cases

1. [08-REQ-1.E1] IF a lock event is received while a session is already active, THEN THE adaptor SHALL log an info message and take no action (idempotent).
2. [08-REQ-1.E2] IF the PARKING_OPERATOR start call fails after 3 retries with exponential backoff (1s, 2s, 4s), THEN THE adaptor SHALL log the error and not update session state.

### Requirement 2: Autonomous Session Stop

**User Story:** As a vehicle system, I want the adaptor to automatically stop a parking session when the vehicle unlocks, so that parking fees are finalized.

#### Acceptance Criteria

1. [08-REQ-2.1] WHEN the `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` signal changes to `false` AND a session is currently active, THE adaptor SHALL call `POST /parking/stop` on the PARKING_OPERATOR with `session_id` and `timestamp`.
2. [08-REQ-2.2] WHEN the PARKING_OPERATOR returns a successful stop response, THE adaptor SHALL clear the in-memory session state and mark the session as inactive.
3. [08-REQ-2.3] WHEN a session is successfully stopped, THE adaptor SHALL write `Vehicle.Parking.SessionActive = false` to DATA_BROKER.

#### Edge Cases

1. [08-REQ-2.E1] IF an unlock event is received while no session is active, THEN THE adaptor SHALL log an info message and take no action (idempotent).
2. [08-REQ-2.E2] IF the PARKING_OPERATOR stop call fails after 3 retries, THEN THE adaptor SHALL log the error and not update session state.

### Requirement 3: Manual Session Override

**User Story:** As a PARKING_APP, I want to manually start or stop a parking session, so that I can override autonomous behavior when needed.

#### Acceptance Criteria

1. [08-REQ-3.1] WHEN a `StartSession(zone_id)` gRPC request is received AND no session is currently active, THE adaptor SHALL call `POST /parking/start` on the PARKING_OPERATOR and start a session, identical to the autonomous flow.
2. [08-REQ-3.2] WHEN a `StopSession()` gRPC request is received AND a session is currently active, THE adaptor SHALL call `POST /parking/stop` on the PARKING_OPERATOR and stop the session, regardless of the current lock state.
3. [08-REQ-3.3] AFTER a manual override, THE adaptor SHALL resume autonomous behavior on the next lock/unlock cycle.

#### Edge Cases

1. [08-REQ-3.E1] IF `StartSession` is called while a session is already active, THEN THE adaptor SHALL return gRPC `ALREADY_EXISTS` with a descriptive error message.
2. [08-REQ-3.E2] IF `StopSession` is called while no session is active, THEN THE adaptor SHALL return gRPC `NOT_FOUND` with a descriptive error message.

### Requirement 4: Session Status Query

**User Story:** As a PARKING_APP, I want to query the current session status, so that I can display parking information to the driver.

#### Acceptance Criteria

1. [08-REQ-4.1] WHEN a `GetStatus()` gRPC request is received AND a session is active, THE adaptor SHALL return the current session information including `session_id`, `active: true`, `zone_id`, `start_time`, and `rate`.
2. [08-REQ-4.2] WHEN a `GetStatus()` gRPC request is received AND no session is active, THE adaptor SHALL return `{active: false}` with empty fields.

### Requirement 5: Rate Query

**User Story:** As a PARKING_APP, I want to query the current parking rate, so that I can display pricing to the driver.

#### Acceptance Criteria

1. [08-REQ-5.1] WHEN a `GetRate()` gRPC request is received AND a session is active, THE adaptor SHALL return the cached rate information including `rate_type`, `amount`, and `currency`.
2. [08-REQ-5.2] WHEN a `GetRate()` gRPC request is received AND no session is active, THE adaptor SHALL return gRPC `NOT_FOUND` with a message indicating no active session.

### Requirement 6: DATA_BROKER Integration

**User Story:** As a vehicle system, I want the adaptor to subscribe to lock events and publish session state via DATA_BROKER, so that all vehicle components stay in sync.

#### Acceptance Criteria

1. [08-REQ-6.1] WHEN the adaptor starts, THE adaptor SHALL subscribe to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` via DATA_BROKER gRPC.
2. [08-REQ-6.2] WHEN the adaptor starts or stops a session, THE adaptor SHALL write the updated `Vehicle.Parking.SessionActive` value to DATA_BROKER.

#### Edge Cases

1. [08-REQ-6.E1] IF the DATA_BROKER is unreachable at startup, THEN THE adaptor SHALL retry connection with exponential backoff (1s, 2s, 4s) up to 5 attempts, then exit non-zero.
2. [08-REQ-6.E2] IF writing `Vehicle.Parking.SessionActive` to DATA_BROKER fails, THEN THE adaptor SHALL log the error and continue operating (session state is not affected).

### Requirement 7: Configuration

**User Story:** As a developer, I want the adaptor to load configuration from environment variables, so that I can configure it for different environments.

#### Acceptance Criteria

1. [08-REQ-7.1] THE adaptor SHALL read configuration from environment variables: `PARKING_OPERATOR_URL`, `DATA_BROKER_ADDR`, `GRPC_PORT`, `VEHICLE_ID`, `ZONE_ID`.
2. [08-REQ-7.2] THE adaptor SHALL use default values when environment variables are not set: operator URL `http://localhost:8080`, databroker `http://localhost:55556`, port `50053`, vehicle ID `DEMO-VIN-001`, zone ID `zone-demo-1`.

### Requirement 8: Graceful Lifecycle

**User Story:** As an operator, I want the adaptor to start and stop cleanly.

#### Acceptance Criteria

1. [08-REQ-8.1] WHEN the adaptor starts, THE adaptor SHALL log its version, configured port, operator URL, DATA_BROKER address, vehicle ID, and a ready message.
2. [08-REQ-8.2] WHEN the adaptor receives SIGTERM or SIGINT, THE adaptor SHALL stop any active session with the PARKING_OPERATOR, close the DATA_BROKER connection, shut down the gRPC server, and exit with code 0.
