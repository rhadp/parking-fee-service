# Requirements Document: RHIVOS QM Partition (Phase 2.3)

## Introduction

This document specifies the requirements for the RHIVOS QM partition services
of the SDV Parking Demo System. The QM partition hosts two primary Rust
services — PARKING_OPERATOR_ADAPTOR and UPDATE_SERVICE — along with supporting
mock services and CLI enhancements for testing. The PARKING_OPERATOR_ADAPTOR
autonomously manages parking sessions by subscribing to lock/unlock events from
the safety partition's DATA_BROKER and communicating with external parking
operators via REST. The UPDATE_SERVICE manages the lifecycle of containerized
adapters, including OCI image pulling, checksum verification, and automatic
offloading.

## Glossary

| Term | Definition |
|------|-----------|
| PARKING_OPERATOR_ADAPTOR | Containerized Rust service in the QM partition that bridges DATA_BROKER lock events and a PARKING_OPERATOR REST API for autonomous parking session management. |
| UPDATE_SERVICE | Rust service that manages the lifecycle of containerized adapters: download, install, run, stop, offload, and remove. |
| PARKING_OPERATOR | External (or mock) REST service representing a real-world parking operator. |
| DATA_BROKER | Eclipse Kuksa Databroker running in the safety partition, providing VSS-compliant gRPC pub/sub for vehicle signals. |
| Adapter | A containerized PARKING_OPERATOR_ADAPTOR instance specific to a parking operator. |
| Autonomous session | A parking session started/stopped automatically by the adaptor in response to lock/unlock events, without explicit user action. |
| Override | The ability for PARKING_APP to manually start or stop a session, overriding autonomous behavior. |
| Offloading | Removing an adapter container and its resources after a period of inactivity or under resource pressure. |
| OCI | Open Container Initiative — standard for container image format and distribution. |
| Cross-partition | Communication between RHIVOS QM and Safety partitions, using network gRPC over TCP. |

## Requirements

### Requirement 1: PARKING_OPERATOR_ADAPTOR gRPC Interface

**User Story:** As the PARKING_APP, I want to interact with the
PARKING_OPERATOR_ADAPTOR via a common gRPC interface, so that I can query
session status, parking rates, and override autonomous session behavior.

#### Acceptance Criteria

1. THE PARKING_OPERATOR_ADAPTOR SHALL expose a gRPC service on a configurable
   network address implementing the `ParkingAdaptor` service defined in
   `parking_adaptor.proto`. `04-REQ-1.1`
2. WHEN `StartSession` is called with a valid `vehicle_id` and `zone_id`,
   THE service SHALL start a parking session with the configured
   PARKING_OPERATOR and return a `StartSessionResponse` containing a
   `session_id` and `status`. `04-REQ-1.2`
3. WHEN `StopSession` is called with a valid `session_id`, THE service SHALL
   stop the corresponding parking session with the PARKING_OPERATOR and return
   a `StopSessionResponse` containing fee, duration, and currency. `04-REQ-1.3`
4. WHEN `GetStatus` is called with a valid `session_id`, THE service SHALL
   return a `GetStatusResponse` reflecting the current session state including
   active status, start time, and current accumulated fee. `04-REQ-1.4`
5. WHEN `GetRate` is called with a `zone_id`, THE service SHALL query the
   PARKING_OPERATOR for the rate and return a `GetRateResponse` containing
   `rate_per_hour`, `currency`, and `zone_name`. `04-REQ-1.5`

#### Edge Cases

1. IF `StartSession` is called while a session is already active, THEN the
   service SHALL return gRPC status `ALREADY_EXISTS` with a message indicating
   a session is already in progress. `04-REQ-1.E1`
2. IF `StopSession` is called with a `session_id` that does not correspond to
   an active session, THEN the service SHALL return gRPC status `NOT_FOUND`.
   `04-REQ-1.E2`
3. IF the PARKING_OPERATOR REST endpoint is unreachable when `StartSession` is
   called, THEN the service SHALL return gRPC status `UNAVAILABLE` with details
   indicating the operator is unreachable. `04-REQ-1.E3`

---

### Requirement 2: Autonomous Session Management

**User Story:** As a vehicle owner, I want the parking session to start
automatically when I lock the vehicle and stop when I unlock it, so that I do
not need to manually interact with a parking app.

#### Acceptance Criteria

1. WHEN the PARKING_OPERATOR_ADAPTOR receives a
   `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true` event from DATA_BROKER,
   THE adaptor SHALL autonomously start a parking session by calling the
   PARKING_OPERATOR's `POST /parking/start` endpoint. `04-REQ-2.1`
2. WHEN the PARKING_OPERATOR_ADAPTOR receives a
   `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = false` event from
   DATA_BROKER, THE adaptor SHALL autonomously stop the active parking session
   by calling the PARKING_OPERATOR's `POST /parking/stop` endpoint. `04-REQ-2.2`
3. AFTER autonomously starting a parking session, THE adaptor SHALL write
   `Vehicle.Parking.SessionActive = true` to DATA_BROKER. `04-REQ-2.3`
4. AFTER autonomously stopping a parking session, THE adaptor SHALL write
   `Vehicle.Parking.SessionActive = false` to DATA_BROKER. `04-REQ-2.4`
5. WHEN a `StartSession` or `StopSession` gRPC call from PARKING_APP overrides
   the autonomous behavior, THE adaptor SHALL respect the override and update
   `Vehicle.Parking.SessionActive` accordingly. `04-REQ-2.5`

#### Edge Cases

1. IF an unlock event is received but no session is currently active, THEN the
   adaptor SHALL ignore the event and not call the PARKING_OPERATOR.
   `04-REQ-2.E1`
2. IF the PARKING_OPERATOR is unreachable during an autonomous session start,
   THEN the adaptor SHALL log the error and NOT write
   `Vehicle.Parking.SessionActive = true` to DATA_BROKER. `04-REQ-2.E2`
3. IF a lock event is received while a session is already active, THEN the
   adaptor SHALL ignore the duplicate lock event and not start a second
   session. `04-REQ-2.E3`

---

### Requirement 3: DATA_BROKER Subscription

**User Story:** As a PARKING_OPERATOR_ADAPTOR, I need to subscribe to vehicle
signals from DATA_BROKER over cross-partition network gRPC, so that I can
react to lock/unlock events and read location data for zone context.

#### Acceptance Criteria

1. THE PARKING_OPERATOR_ADAPTOR SHALL connect to DATA_BROKER using network gRPC
   (TCP) at a configurable address. `04-REQ-3.1`
2. THE PARKING_OPERATOR_ADAPTOR SHALL subscribe to
   `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` for lock/unlock events.
   `04-REQ-3.2`
3. THE PARKING_OPERATOR_ADAPTOR SHALL read
   `Vehicle.CurrentLocation.Latitude` and `Vehicle.CurrentLocation.Longitude`
   from DATA_BROKER to determine the vehicle's zone context. `04-REQ-3.3`
4. THE PARKING_OPERATOR_ADAPTOR SHALL write `Vehicle.Parking.SessionActive`
   to DATA_BROKER to publish parking session state. `04-REQ-3.4`

#### Edge Cases

1. IF DATA_BROKER is unreachable at startup, THEN the adaptor SHALL retry the
   connection with exponential backoff and log each retry attempt. `04-REQ-3.E1`

---

### Requirement 4: UPDATE_SERVICE Adapter Lifecycle Management

**User Story:** As the PARKING_APP, I want to install, monitor, list, query,
and remove adapters via UPDATE_SERVICE, so that I can manage the parking
operator adapter lifecycle on the vehicle.

#### Acceptance Criteria

1. THE UPDATE_SERVICE SHALL expose a gRPC service on a configurable network
   address implementing the `UpdateService` service defined in
   `update_service.proto`. `04-REQ-4.1`
2. WHEN `InstallAdapter` is called with an `image_ref` and
   `checksum_sha256`, THE service SHALL initiate an adapter download and return
   an `InstallAdapterResponse` containing `job_id`, `adapter_id`, and initial
   `state` of `DOWNLOADING`. `04-REQ-4.2`
3. WHEN `WatchAdapterStates` is called, THE service SHALL return a
   server-streaming response emitting `AdapterStateEvent` messages whenever any
   adapter transitions between lifecycle states. `04-REQ-4.3`
4. WHEN `ListAdapters` is called, THE service SHALL return a
   `ListAdaptersResponse` containing all known adapters with their current
   states. `04-REQ-4.4`
5. WHEN `RemoveAdapter` is called with a valid `adapter_id`, THE service SHALL
   stop and remove the adapter container and return a `RemoveAdapterResponse`.
   `04-REQ-4.5`
6. WHEN `GetAdapterStatus` is called with a valid `adapter_id`, THE service
   SHALL return a `GetAdapterStatusResponse` containing the adapter's current
   `AdapterInfo`. `04-REQ-4.6`

#### Edge Cases

1. IF `InstallAdapter` is called with an `image_ref` for an adapter that is
   already installed and running, THEN the service SHALL return gRPC status
   `ALREADY_EXISTS`. `04-REQ-4.E1`
2. IF `RemoveAdapter` or `GetAdapterStatus` is called with an `adapter_id`
   that does not exist, THEN the service SHALL return gRPC status `NOT_FOUND`.
   `04-REQ-4.E2`
3. IF the container fails to start during installation, THEN the adapter SHALL
   transition to the `ERROR` state and the failure reason SHALL be included in
   the `AdapterStateEvent`. `04-REQ-4.E3`

---

### Requirement 5: OCI Container Pulling and Checksum Verification

**User Story:** As the UPDATE_SERVICE, I need to pull OCI containers from a
registry and verify their integrity, so that only trusted adapter images are
installed on the vehicle.

#### Acceptance Criteria

1. WHEN `InstallAdapter` is invoked, THE UPDATE_SERVICE SHALL pull the OCI
   container image specified by `image_ref` from the configured registry.
   `04-REQ-5.1`
2. AFTER pulling the OCI manifest, THE UPDATE_SERVICE SHALL compute the SHA-256
   digest of the manifest and compare it against the `checksum_sha256` provided
   in the `InstallAdapterRequest`. `04-REQ-5.2`
3. IF the checksum matches, THE UPDATE_SERVICE SHALL proceed to extract and
   install the container, transitioning the adapter state from `DOWNLOADING` to
   `INSTALLING`. `04-REQ-5.3`

#### Edge Cases

1. IF the computed SHA-256 digest does not match the provided
   `checksum_sha256`, THEN the UPDATE_SERVICE SHALL transition the adapter to
   the `ERROR` state, discard the downloaded image, and include a
   "checksum mismatch" detail in the `AdapterStateEvent`. `04-REQ-5.E1`
2. IF the registry is unreachable or returns an error during the pull, THEN the
   UPDATE_SERVICE SHALL transition the adapter to the `ERROR` state and include
   the failure reason in the `AdapterStateEvent`. `04-REQ-5.E2`

---

### Requirement 6: Adapter Offloading

**User Story:** As the vehicle platform, I want unused adapters to be
automatically offloaded after a configurable period of inactivity, so that
storage and system resources are freed.

#### Acceptance Criteria

1. THE UPDATE_SERVICE SHALL support a configurable inactivity timeout (default:
   24 hours) after which a stopped adapter is automatically offloaded.
   `04-REQ-6.1`
2. WHEN an adapter has been in the `STOPPED` state for longer than the
   configured inactivity timeout, THE UPDATE_SERVICE SHALL transition the
   adapter to `OFFLOADING`, remove the container and its resources, and then
   remove the adapter from the known adapters list. `04-REQ-6.2`
3. THE UPDATE_SERVICE SHALL emit `AdapterStateEvent` messages during
   offloading transitions so that watchers are notified. `04-REQ-6.3`

#### Edge Cases

1. IF an adapter is re-started (via `InstallAdapter`) while it is in the
   `OFFLOADING` state, THEN the UPDATE_SERVICE SHALL cancel the offload and
   re-download the adapter. `04-REQ-6.E1`

---

### Requirement 7: Adapter State Machine

**User Story:** As a developer, I want adapter lifecycle states to follow a
well-defined state machine, so that state transitions are predictable and
observable.

#### Acceptance Criteria

1. THE UPDATE_SERVICE SHALL enforce the following valid state transitions:
   `UNKNOWN -> DOWNLOADING`, `DOWNLOADING -> INSTALLING`,
   `DOWNLOADING -> ERROR`, `INSTALLING -> RUNNING`, `INSTALLING -> ERROR`,
   `RUNNING -> STOPPED`, `STOPPED -> OFFLOADING`, `STOPPED -> DOWNLOADING`
   (re-install), `OFFLOADING -> UNKNOWN` (removed), `ERROR -> DOWNLOADING`
   (retry). `04-REQ-7.1`
2. THE UPDATE_SERVICE SHALL reject any state transition not in the allowed set
   and log a warning. `04-REQ-7.2`

#### Edge Cases

(None beyond the state transition rules above.)

---

### Requirement 8: Mock PARKING_OPERATOR

**User Story:** As a developer, I want a mock PARKING_OPERATOR service that
simulates a real parking operator's REST API, so that I can test the
PARKING_OPERATOR_ADAPTOR without a real operator backend.

#### Acceptance Criteria

1. THE mock PARKING_OPERATOR SHALL expose an HTTP server on a configurable
   port (default: 8090). `04-REQ-8.1`
2. WHEN `POST /parking/start` is called with a JSON body containing
   `vehicle_id`, `zone_id`, and `timestamp`, THE mock SHALL create a session,
   store it in memory, and return a JSON response with `session_id` and
   `status` of "active". `04-REQ-8.2`
3. WHEN `POST /parking/stop` is called with a JSON body containing
   `session_id`, THE mock SHALL calculate the fee based on elapsed time and zone
   rate, mark the session as stopped, and return a JSON response with
   `session_id`, `fee`, `duration_seconds`, and `currency`. `04-REQ-8.3`
4. WHEN `GET /parking/{session_id}/status` is called, THE mock SHALL return
   the session's current status including `session_id`, `active`, `start_time`,
   `current_fee`, and `currency`. `04-REQ-8.4`
5. WHEN `GET /rate/{zone_id}` is called, THE mock SHALL return the parking rate
   for the zone including `rate_per_hour`, `currency`, and `zone_name`.
   `04-REQ-8.5`

#### Edge Cases

1. IF `POST /parking/stop` is called with a `session_id` that does not exist,
   THEN the mock SHALL return HTTP 404 with a descriptive error message.
   `04-REQ-8.E1`
2. IF `GET /parking/{session_id}/status` is called with a `session_id` that
   does not exist, THEN the mock SHALL return HTTP 404. `04-REQ-8.E2`
3. IF `GET /rate/{zone_id}` is called with an unknown `zone_id`, THEN the mock
   SHALL return HTTP 404 with a descriptive error message. `04-REQ-8.E3`

---

### Requirement 9: Mock PARKING_APP CLI Enhancements

**User Story:** As a developer, I want the mock PARKING_APP CLI to have
working commands for adapter installation and session management, so that I can
integration-test UPDATE_SERVICE and PARKING_OPERATOR_ADAPTOR without the real
Android app.

#### Acceptance Criteria

1. THE `install` command SHALL call UPDATE_SERVICE `InstallAdapter` with the
   provided `--image-ref` and `--checksum` flags and print the response
   (`job_id`, `adapter_id`, `state`). `04-REQ-9.1`
2. THE `watch` command SHALL call UPDATE_SERVICE `WatchAdapterStates` and
   print each `AdapterStateEvent` as it arrives on the stream, until the stream
   ends or the user interrupts. `04-REQ-9.2`
3. THE `list` command SHALL call UPDATE_SERVICE `ListAdapters` and print a
   table of all adapters with their IDs and states. `04-REQ-9.3`
4. THE `start-session` command SHALL call PARKING_OPERATOR_ADAPTOR
   `StartSession` with the provided `--vehicle-id` and `--zone-id` flags and
   print the response (`session_id`, `status`). `04-REQ-9.4`
5. THE `stop-session` command SHALL call PARKING_OPERATOR_ADAPTOR `StopSession`
   with the provided `--session-id` flag and print the response (`session_id`,
   `fee`, `duration`, `currency`). `04-REQ-9.5`

#### Edge Cases

1. IF the UPDATE_SERVICE or PARKING_OPERATOR_ADAPTOR gRPC endpoint is
   unreachable, THEN the CLI command SHALL print an error message including the
   target address and exit with a non-zero exit code. `04-REQ-9.E1`

---

### Requirement 10: Integration Tests

**User Story:** As a developer, I want integration tests that verify
end-to-end communication between QM partition services, DATA_BROKER, and mock
services, so that I can validate the system works correctly before deployment.

#### Acceptance Criteria

1. AN integration test SHALL verify that a lock event published to DATA_BROKER
   triggers the PARKING_OPERATOR_ADAPTOR to autonomously start a parking
   session with the mock PARKING_OPERATOR. `04-REQ-10.1`
2. AN integration test SHALL verify that the mock PARKING_APP CLI can trigger
   UPDATE_SERVICE adapter lifecycle operations (install, list, get-status).
   `04-REQ-10.2`
3. AN integration test SHALL verify that the PARKING_OPERATOR_ADAPTOR
   communicates correctly with the mock PARKING_OPERATOR REST API (start
   session, stop session, get rate). `04-REQ-10.3`

#### Edge Cases

(Integration test edge cases are covered by the individual component
requirements above.)
