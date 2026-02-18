# Requirements Document: PARKING_OPERATOR_ADAPTOR + UPDATE_SERVICE

## Introduction

This specification defines the RHIVOS QM partition services: the
PARKING_OPERATOR_ADAPTOR that manages parking sessions (event-driven via
DATA_BROKER lock events and REST calls to a PARKING_OPERATOR), the
UPDATE_SERVICE that manages adapter container lifecycle via podman, a mock
PARKING_OPERATOR backend, and the mock PARKING_APP CLI.

## Glossary

| Term | Definition |
|------|-----------|
| Adapter | A PARKING_OPERATOR_ADAPTOR container instance managed by UPDATE_SERVICE |
| DATA_BROKER | Eclipse Kuksa Databroker (specs 01–02) |
| Flat fee | A fixed parking rate per session regardless of duration |
| IsLocked | VSS signal `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` |
| Offloading | Automatic removal of an adapter container after a period of inactivity |
| PARKING_OPERATOR | External parking service that the adaptor communicates with via REST |
| PARKING_OPERATOR_ADAPTOR | Rust service implementing the common parking adapter interface |
| Per-minute rate | A parking rate calculated as `rate_amount × duration_minutes` |
| Podman | Container management tool used by UPDATE_SERVICE |
| SessionActive | VSS signal `Vehicle.Parking.SessionActive` |
| UPDATE_SERVICE | Rust service managing adapter container lifecycle |
| Zone | A parking area identified by a `zone_id`, associated with a PARKING_OPERATOR |

## Requirements

### Requirement 1: Event-Driven Session Management

**User Story:** As a vehicle owner, I want parking sessions to start
automatically when I lock my car and stop when I unlock, so that I don't
need to interact with a parking app.

#### Acceptance Criteria

1. **04-REQ-1.1** WHEN PARKING_OPERATOR_ADAPTOR starts, THE service SHALL
   subscribe to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` on DATA_BROKER
   via gRPC streaming.

2. **04-REQ-1.2** WHEN `IsLocked` changes to `true` AND no parking session
   is currently active, THE PARKING_OPERATOR_ADAPTOR SHALL call
   `POST /parking/start` on the configured PARKING_OPERATOR with
   `{vehicle_id, zone_id, timestamp}`.

3. **04-REQ-1.3** WHEN the PARKING_OPERATOR responds with a session
   confirmation, THE PARKING_OPERATOR_ADAPTOR SHALL write
   `Vehicle.Parking.SessionActive = true` to DATA_BROKER.

4. **04-REQ-1.4** WHEN `IsLocked` changes to `false` AND a parking session
   is active, THE PARKING_OPERATOR_ADAPTOR SHALL call `POST /parking/stop`
   on the PARKING_OPERATOR with `{session_id, timestamp}`.

5. **04-REQ-1.5** WHEN the stop response is received, THE
   PARKING_OPERATOR_ADAPTOR SHALL write
   `Vehicle.Parking.SessionActive = false` to DATA_BROKER.

#### Edge Cases

1. **04-REQ-1.E1** IF the PARKING_OPERATOR is unreachable when starting a
   session, THEN the adaptor SHALL log the error, NOT set SessionActive, and
   retry on the next lock event.

2. **04-REQ-1.E2** IF `IsLocked` changes to `true` while a session is already
   active, THEN the adaptor SHALL ignore the event (no duplicate session).

3. **04-REQ-1.E3** IF `IsLocked` changes to `false` while no session is
   active, THEN the adaptor SHALL ignore the event.

---

### Requirement 2: PARKING_OPERATOR_ADAPTOR gRPC Interface

**User Story:** As the PARKING_APP, I want to query parking session status
and rates, and manually control sessions when needed, so that I can display
information and offer override options to the user.

#### Acceptance Criteria

1. **04-REQ-2.1** THE PARKING_OPERATOR_ADAPTOR SHALL expose a gRPC server
   implementing the `ParkingAdapter` service from
   `parking_adapter.proto` (spec 01).

2. **04-REQ-2.2** WHEN `StartSession` is called via gRPC, THE adaptor SHALL
   start a parking session with the PARKING_OPERATOR using the provided
   `vehicle_id` and `zone_id`, write `SessionActive = true`, and return the
   session info.

3. **04-REQ-2.3** WHEN `StopSession` is called via gRPC, THE adaptor SHALL
   stop the active session, write `SessionActive = false`, and return the
   fee summary.

4. **04-REQ-2.4** WHEN `GetStatus` is called, THE adaptor SHALL return the
   current session state (active/inactive, session_id, start_time,
   current_fee).

5. **04-REQ-2.5** WHEN `GetRate` is called, THE adaptor SHALL query
   `GET /parking/rate` on the PARKING_OPERATOR and return the rate info
   for the specified zone.

6. **04-REQ-2.6** THE adaptor SHALL accept configuration via environment
   variables: `DATABROKER_ADDR`, `PARKING_OPERATOR_URL`, `ZONE_ID`,
   `VEHICLE_VIN`, `LISTEN_ADDR` (default: `0.0.0.0:50054`).

#### Edge Cases

1. **04-REQ-2.E1** IF `StartSession` is called while a session is already
   active, THEN the adaptor SHALL return the existing session info without
   starting a new one.

2. **04-REQ-2.E2** IF `StopSession` is called with an unknown `session_id`,
   THEN the adaptor SHALL return a gRPC `NOT_FOUND` error.

---

### Requirement 3: UPDATE_SERVICE Container Lifecycle

**User Story:** As the vehicle platform, I want UPDATE_SERVICE to manage
adapter containers using podman, so that adapters can be installed, started,
stopped, and removed dynamically.

#### Acceptance Criteria

1. **04-REQ-3.1** WHEN `InstallAdapter` is called with an `image_ref`, THE
   UPDATE_SERVICE SHALL create and start a container using `podman` and
   transition the adapter state to `RUNNING`.

2. **04-REQ-3.2** THE UPDATE_SERVICE SHALL pass adapter configuration
   (DATABROKER_ADDR, PARKING_OPERATOR_URL, ZONE_ID, VEHICLE_VIN,
   LISTEN_ADDR) as environment variables to the container.

3. **04-REQ-3.3** WHEN `RemoveAdapter` is called, THE UPDATE_SERVICE SHALL
   stop and remove the container and transition the adapter state to
   `STOPPED`.

4. **04-REQ-3.4** THE adapter state machine SHALL enforce valid transitions:
   `UNKNOWN → INSTALLING → RUNNING`, `RUNNING → STOPPED`,
   `RUNNING → OFFLOADING`, `RUNNING → ERROR`, `ERROR → INSTALLING`.

5. **04-REQ-3.5** THE UPDATE_SERVICE SHALL persist adapter states to a JSON
   file in a configurable data directory, so that state survives restarts.

6. **04-REQ-3.6** WHEN UPDATE_SERVICE starts, THE service SHALL load
   persisted adapter states and reconcile with actual podman container state.

#### Edge Cases

1. **04-REQ-3.E1** IF `podman create` or `podman start` fails, THEN the
   adapter state SHALL transition to `ERROR` and the gRPC response SHALL
   include the error details.

2. **04-REQ-3.E2** IF `InstallAdapter` is called for an adapter that is
   already `RUNNING`, THEN UPDATE_SERVICE SHALL return the existing adapter
   info with state `RUNNING` (no duplicate install).

3. **04-REQ-3.E3** IF the container image is not found locally, THEN
   `podman create` SHALL fail and the adapter SHALL transition to `ERROR`
   with a descriptive message.

---

### Requirement 4: UPDATE_SERVICE gRPC Interface

**User Story:** As the PARKING_APP, I want to install, monitor, and manage
parking operator adapters via gRPC, so that I can provision the right adapter
for the current parking zone.

#### Acceptance Criteria

1. **04-REQ-4.1** THE UPDATE_SERVICE SHALL expose a gRPC server implementing
   the `UpdateService` from `update_service.proto` (spec 01).

2. **04-REQ-4.2** `InstallAdapter(image_ref, checksum)` SHALL return an
   `InstallAdapterResponse` with `job_id`, `adapter_id`, and current
   `state`.

3. **04-REQ-4.3** `WatchAdapterStates()` SHALL return a server-streaming
   response emitting `AdapterStateEvent` messages for every state
   transition.

4. **04-REQ-4.4** `ListAdapters()` SHALL return all known adapters with
   their current `AdapterInfo` and `AdapterState`.

5. **04-REQ-4.5** `GetAdapterStatus(adapter_id)` SHALL return the
   `AdapterInfo` and `AdapterState` for a specific adapter.

6. **04-REQ-4.6** THE UPDATE_SERVICE SHALL accept configuration via env vars
   or CLI flags: `LISTEN_ADDR` (default: `0.0.0.0:50053`), `DATA_DIR`
   (default: `./data`), `OFFLOAD_TIMEOUT` (default: `5m`).

#### Edge Cases

1. **04-REQ-4.E1** IF `GetAdapterStatus` is called with an unknown
   `adapter_id`, THEN UPDATE_SERVICE SHALL return gRPC `NOT_FOUND`.

2. **04-REQ-4.E2** IF `RemoveAdapter` is called with an unknown
   `adapter_id`, THEN UPDATE_SERVICE SHALL return gRPC `NOT_FOUND`.

---

### Requirement 5: Adapter Offloading

**User Story:** As the vehicle platform, I want unused adapters to be
automatically removed after a period of inactivity, so that system resources
are freed.

#### Acceptance Criteria

1. **04-REQ-5.1** WHEN a parking session ends (adapter notifies
   UPDATE_SERVICE or SessionActive becomes false), THE UPDATE_SERVICE SHALL
   start an offloading timer for that adapter.

2. **04-REQ-5.2** WHEN the offloading timer expires AND no new session has
   started, THE UPDATE_SERVICE SHALL stop and remove the adapter container
   and transition its state to `OFFLOADING` then `UNKNOWN`.

3. **04-REQ-5.3** WHEN a new session starts before the timer expires, THE
   UPDATE_SERVICE SHALL cancel the offloading timer.

4. **04-REQ-5.4** THE offloading timeout SHALL be configurable via
   `OFFLOAD_TIMEOUT` env var, defaulting to `5m` (5 minutes) for the demo.

#### Edge Cases

1. **04-REQ-5.E1** IF the adapter is manually removed (via `RemoveAdapter`)
   before the timer expires, THEN the timer SHALL be cancelled.

---

### Requirement 6: Mock PARKING_OPERATOR

**User Story:** As a developer, I want a mock parking operator that simulates
a real parking service, so that I can test the PARKING_OPERATOR_ADAPTOR
without external dependencies.

#### Acceptance Criteria

1. **04-REQ-6.1** THE mock PARKING_OPERATOR SHALL expose
   `POST /parking/start` accepting `{vehicle_id, zone_id, timestamp}` and
   returning `{session_id, status, rate}`.

2. **04-REQ-6.2** THE mock PARKING_OPERATOR SHALL expose
   `POST /parking/stop` accepting `{session_id, timestamp}` and returning
   `{session_id, status, total_fee, duration_seconds, currency}`.

3. **04-REQ-6.3** THE mock PARKING_OPERATOR SHALL expose
   `GET /parking/sessions/{id}` returning session details including current
   fee if active.

4. **04-REQ-6.4** THE mock PARKING_OPERATOR SHALL expose
   `GET /parking/rate` returning `{zone_id, rate_type, rate_amount, currency}`
   where `rate_type` is `"per_minute"` or `"flat"`.

5. **04-REQ-6.5** THE mock PARKING_OPERATOR SHALL calculate fees:
   for `per_minute`, `rate_amount × ceil(duration_minutes)`;
   for `flat`, the fixed `rate_amount`.

6. **04-REQ-6.6** THE mock SHALL accept `--listen-addr` (default: `:8082`)
   and `--rate-type` / `--rate-amount` / `--currency` configuration flags.

#### Edge Cases

1. **04-REQ-6.E1** IF `POST /parking/stop` is called with an unknown
   `session_id`, THEN the mock SHALL return `404 Not Found`.

2. **04-REQ-6.E2** IF `POST /parking/start` is called while a session for
   the same `vehicle_id` is already active, THEN the mock SHALL return the
   existing session.

---

### Requirement 7: Mock PARKING_APP CLI

**User Story:** As a developer, I want a CLI tool that simulates PARKING_APP
gRPC calls to UPDATE_SERVICE and PARKING_OPERATOR_ADAPTOR, so that I can
test both services without a real Android app.

#### Acceptance Criteria

1. **04-REQ-7.1** THE mock `parking-app-cli` SHALL implement subcommands
   for all UPDATE_SERVICE RPCs: `install-adapter`, `list-adapters`,
   `remove-adapter`, `adapter-status`, `watch-adapters`.

2. **04-REQ-7.2** THE mock `parking-app-cli` SHALL implement subcommands
   for all PARKING_OPERATOR_ADAPTOR RPCs: `start-session`, `stop-session`,
   `get-status`, `get-rate`.

3. **04-REQ-7.3** THE `watch-adapters` subcommand SHALL stream
   `AdapterStateEvent` messages to stdout until interrupted.

4. **04-REQ-7.4** THE mock CLI SHALL accept flags:
   `--update-service-addr` (default: `localhost:50053`),
   `--adapter-addr` (default: `localhost:50054`).

#### Edge Cases

1. **04-REQ-7.E1** IF the target service is unreachable, THEN the CLI SHALL
   print an error and exit with a non-zero exit code.

---

### Requirement 8: Integration Verification

**User Story:** As a developer, I want automated integration tests that verify
the full parking session flow (lock → adaptor → PARKING_OPERATOR → session)
and the adapter lifecycle (install → run → offload), so that I can validate
the QM partition services work correctly.

#### Acceptance Criteria

1. **04-REQ-8.1** THE integration test SHALL verify: setting `IsLocked = true`
   via mock-sensors triggers PARKING_OPERATOR_ADAPTOR to start a session with
   the mock PARKING_OPERATOR, and `SessionActive` becomes `true` in
   DATA_BROKER.

2. **04-REQ-8.2** THE integration test SHALL verify: setting `IsLocked = false`
   stops the session, `SessionActive` becomes `false`, and the mock
   PARKING_OPERATOR records the completed session with a calculated fee.

3. **04-REQ-8.3** THE integration test SHALL verify: `InstallAdapter` via
   the mock CLI creates and starts a container, and `ListAdapters` shows the
   adapter as `RUNNING`.

4. **04-REQ-8.4** THE integration test SHALL verify: after a session ends
   and the offloading timeout elapses, the adapter is removed and its state
   becomes `UNKNOWN`.

#### Edge Cases

1. **04-REQ-8.E1** IF required infrastructure is unavailable, THEN the test
   SHALL skip with a clear message.
