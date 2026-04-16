# Requirements Document

## Introduction

This document specifies the requirements for the PARKING_OPERATOR_ADAPTOR component (Phase 2.3) of the SDV Parking Demo System. The PARKING_OPERATOR_ADAPTOR is a containerized Rust application running in the RHIVOS QM partition that bridges the in-vehicle PARKING_APP (via gRPC) with a PARKING_OPERATOR backend (via REST), and autonomously manages parking sessions based on lock/unlock events received from DATA_BROKER.

## Glossary

- **PARKING_OPERATOR_ADAPTOR:** A containerized Rust service running in the RHIVOS QM partition that manages parking sessions autonomously and exposes a gRPC interface.
- **PARKING_OPERATOR:** An external parking operator backend service exposing a REST API for session management.
- **DATA_BROKER:** Eclipse Kuksa Databroker — a VSS-compliant vehicle signal broker used for lock/unlock event subscription and session state publication.
- **PARKING_APP:** An Android Automotive OS application (or mock CLI app) that interacts with the adaptor via gRPC for manual session control.
- **gRPC:** Remote procedure call framework used for the adaptor's service interface and DATA_BROKER communication.
- **VSS:** Vehicle Signal Specification (COVESA standard) — a taxonomy for vehicle data signals.
- **Lock event:** A change of `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` to `true` in DATA_BROKER, indicating the vehicle has been locked.
- **Unlock event:** A change of `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` to `false` in DATA_BROKER, indicating the vehicle has been unlocked.
- **Session:** An in-memory record of an active parking session including session_id, zone_id, start_time, rate, and active flag.
- **Override:** A manual StartSession or StopSession gRPC call that takes precedence over autonomous lock/unlock behavior. Autonomous behavior resumes on the next lock/unlock cycle.
- **Rate model:** Pricing information for a parking session — either `per_hour` (charged by the hour) or `flat_fee` (fixed fee per session).
- **Exponential backoff:** Retry strategy with increasing delays: 1s, 2s, 4s.
- **Idempotent:** An operation that produces the same result regardless of how many times it is applied (e.g., lock while session active = no-op).
- **tonic:** A Rust gRPC framework used for the adaptor's gRPC server and DATA_BROKER client.
- **reqwest:** A Rust HTTP client library used for REST communication with the PARKING_OPERATOR.

## Requirements

### Requirement 1: gRPC Service Interface

**User Story:** As a PARKING_APP developer, I want the adaptor to expose gRPC RPCs for session management and status queries, so that the app can interact with the parking session.

#### Acceptance Criteria

1. [08-REQ-1.1] WHEN the PARKING_OPERATOR_ADAPTOR starts, THE service SHALL listen for gRPC connections on the port specified by the `GRPC_PORT` environment variable, defaulting to `50053`.
2. [08-REQ-1.2] THE service SHALL implement the `StartSession(zone_id)` RPC that starts a parking session with the PARKING_OPERATOR and SHALL return the session_id, status, and rate on success.
3. [08-REQ-1.3] THE service SHALL implement the `StopSession()` RPC that stops the active parking session with the PARKING_OPERATOR and SHALL return the session_id, status, duration_seconds, total_amount, and currency on success.
4. [08-REQ-1.4] THE service SHALL implement the `GetStatus()` RPC that returns the current session state: session_id, active, zone_id, start_time, and rate. WHEN no session is active, THE service SHALL return `active: false` with empty fields.
5. [08-REQ-1.5] THE service SHALL implement the `GetRate()` RPC that returns the cached rate from the active session. WHEN no session is active, THE service SHALL return an empty rate response.

#### Edge Cases

1. [08-REQ-1.E1] IF `StartSession` is called WHEN a session is already active, THEN THE service SHALL return an `ALREADY_EXISTS` gRPC error with the existing session_id.
2. [08-REQ-1.E2] IF `StopSession` is called WHEN no session is active, THEN THE service SHALL return a `FAILED_PRECONDITION` gRPC error.

### Requirement 2: PARKING_OPERATOR REST Client

**User Story:** As a system integrator, I want the adaptor to communicate with the PARKING_OPERATOR via REST, so that parking sessions are registered with the operator backend.

#### Acceptance Criteria

1. [08-REQ-2.1] WHEN starting a session, THE service SHALL send `POST /parking/start` with JSON body `{vehicle_id, zone_id, timestamp}` to the PARKING_OPERATOR at the URL specified by `PARKING_OPERATOR_URL`, defaulting to `http://localhost:8080`.
2. [08-REQ-2.2] WHEN stopping a session, THE service SHALL send `POST /parking/stop` with JSON body `{session_id, timestamp}` to the PARKING_OPERATOR.
3. [08-REQ-2.3] THE service SHALL parse the start response `{session_id, status, rate: {type, amount, currency}}` and store the session_id and rate in the in-memory session state.
4. [08-REQ-2.4] THE service SHALL parse the stop response `{session_id, status, duration_seconds, total_amount, currency}` and return these values to the caller.

#### Edge Cases

1. [08-REQ-2.E1] IF the PARKING_OPERATOR REST API call fails, THEN THE service SHALL retry up to 3 times with exponential backoff (1s, 2s, 4s). After all retries fail, THE service SHALL return an `UNAVAILABLE` gRPC error and SHALL NOT update session state.
2. [08-REQ-2.E2] IF the PARKING_OPERATOR returns a non-200 HTTP status, THEN THE service SHALL treat it as a failure and apply retry logic per [08-REQ-2.E1].

### Requirement 3: DATA_BROKER Lock Event Subscription

**User Story:** As a vehicle system, I want the adaptor to subscribe to lock/unlock events from DATA_BROKER, so that parking sessions start and stop autonomously when the vehicle is locked or unlocked.

#### Acceptance Criteria

1. [08-REQ-3.1] WHEN the service starts, THE service SHALL connect to DATA_BROKER at the address specified by `DATA_BROKER_ADDR`, defaulting to `http://localhost:55556`.
2. [08-REQ-3.2] THE service SHALL subscribe to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` signal on DATA_BROKER via gRPC.
3. [08-REQ-3.3] WHEN `IsLocked` changes to `true`, THE service SHALL start a parking session with the PARKING_OPERATOR using the zone_id from `ZONE_ID` env var (default `zone-demo-1`) and the vehicle_id from `VEHICLE_ID` env var (default `DEMO-VIN-001`).
4. [08-REQ-3.4] WHEN `IsLocked` changes to `false`, THE service SHALL stop the active parking session with the PARKING_OPERATOR.

#### Edge Cases

1. [08-REQ-3.E1] IF a lock event is received WHEN a session is already active, THEN THE service SHALL treat it as a no-op and log an info message.
2. [08-REQ-3.E2] IF an unlock event is received WHEN no session is active, THEN THE service SHALL treat it as a no-op and log an info message.
3. [08-REQ-3.E3] IF DATA_BROKER is unreachable on startup, THEN THE service SHALL retry connection with exponential backoff (1s, 2s, 4s) up to 5 attempts, then exit with a non-zero code.

### Requirement 4: Session State Publication

**User Story:** As a PARKING_APP developer, I want the adaptor to publish session state to DATA_BROKER, so that the UI can display whether a parking session is active.

#### Acceptance Criteria

1. [08-REQ-4.1] WHEN a parking session starts successfully, THE service SHALL set `Vehicle.Parking.SessionActive` to `true` in DATA_BROKER.
2. [08-REQ-4.2] WHEN a parking session stops successfully, THE service SHALL set `Vehicle.Parking.SessionActive` to `false` in DATA_BROKER.
3. [08-REQ-4.3] WHEN the service starts, THE service SHALL set `Vehicle.Parking.SessionActive` to `false` in DATA_BROKER (initial state).

#### Edge Cases

1. [08-REQ-4.E1] IF publishing `Vehicle.Parking.SessionActive` to DATA_BROKER fails, THEN THE service SHALL log the error and continue operation. The session state in memory remains authoritative.

### Requirement 5: Override Mechanism

**User Story:** As a PARKING_APP user, I want to manually start or stop a parking session via the app, overriding the autonomous lock/unlock behavior.

#### Acceptance Criteria

1. [08-REQ-5.1] WHEN `StartSession` is called via gRPC, THE service SHALL start a session with the PARKING_OPERATOR regardless of the current lock state.
2. [08-REQ-5.2] WHEN `StopSession` is called via gRPC, THE service SHALL stop the session with the PARKING_OPERATOR regardless of the current lock state.
3. [08-REQ-5.3] AFTER a manual override, THE service SHALL resume autonomous lock/unlock session management on the next lock or unlock event cycle. There SHALL be no persistent "disable auto-session" flag.

#### Edge Cases

1. [08-REQ-5.E1] IF a manual `StopSession` is followed by a lock event, THE service SHALL start a new session autonomously (override does not persist).

### Requirement 6: In-Memory Session State

**User Story:** As a developer, I want the adaptor to maintain session state in memory, so that status queries can be answered without calling the PARKING_OPERATOR.

#### Acceptance Criteria

1. [08-REQ-6.1] THE service SHALL maintain an in-memory session record containing: `session_id` (String), `zone_id` (String), `start_time` (i64, Unix timestamp), `rate` (type: String, amount: f64, currency: String), and `active` (bool).
2. [08-REQ-6.2] WHEN a session starts successfully, THE service SHALL populate all fields of the session record from the PARKING_OPERATOR start response and set `active` to `true`.
3. [08-REQ-6.3] WHEN a session stops successfully, THE service SHALL set `active` to `false` and clear the session record.

#### Edge Cases

1. [08-REQ-6.E1] IF the service restarts, THEN session state SHALL be lost. The service SHALL start with no active session.

### Requirement 7: Configuration

**User Story:** As a deployment engineer, I want the adaptor to read its configuration from environment variables, so that it can be deployed in different environments without code changes.

#### Acceptance Criteria

1. [08-REQ-7.1] THE service SHALL read `PARKING_OPERATOR_URL` from the environment, defaulting to `http://localhost:8080`.
2. [08-REQ-7.2] THE service SHALL read `DATA_BROKER_ADDR` from the environment, defaulting to `http://localhost:55556`.
3. [08-REQ-7.3] THE service SHALL read `GRPC_PORT` from the environment, defaulting to `50053`.
4. [08-REQ-7.4] THE service SHALL read `VEHICLE_ID` from the environment, defaulting to `DEMO-VIN-001`.
5. [08-REQ-7.5] THE service SHALL read `ZONE_ID` from the environment, defaulting to `zone-demo-1`.

#### Edge Cases

1. [08-REQ-7.E1] IF `GRPC_PORT` contains a non-numeric value, THEN THE service SHALL exit with a non-zero code and log an error message.

### Requirement 8: Startup and Shutdown

**User Story:** As an operator, I want the adaptor to start cleanly and shut down gracefully, so that it integrates well with container orchestration.

#### Acceptance Criteria

1. [08-REQ-8.1] WHEN the service starts, THE service SHALL log its version, PARKING_OPERATOR_URL, DATA_BROKER_ADDR, GRPC_PORT, VEHICLE_ID, and ZONE_ID.
2. [08-REQ-8.2] WHEN the service is ready to accept gRPC requests and has subscribed to DATA_BROKER, THE service SHALL log a "ready" message.
3. [08-REQ-8.3] WHEN SIGTERM or SIGINT is received, THE service SHALL complete any in-flight operation and exit with code 0.

#### Edge Cases

1. [08-REQ-8.E1] IF SIGTERM is received during an in-flight REST call to the PARKING_OPERATOR, THEN THE service SHALL wait for the call to complete (or timeout) before exiting.

### Requirement 9: Events Processed Sequentially

**User Story:** As a system architect, I want lock/unlock events and gRPC calls to be processed sequentially, so that there are no race conditions in session state management.

#### Acceptance Criteria

1. [08-REQ-9.1] THE service SHALL process lock/unlock events and gRPC session commands sequentially. THE service SHALL NOT process a new event or command while a previous one is in-flight.
2. [08-REQ-9.2] WHEN multiple events arrive while an operation is in-flight, THE service SHALL process them in order. The latest state wins.

#### Edge Cases

1. [08-REQ-9.E1] IF a lock event and a manual StopSession arrive concurrently, THEN THE service SHALL serialize them and process each in order, applying the session state rules for each.
