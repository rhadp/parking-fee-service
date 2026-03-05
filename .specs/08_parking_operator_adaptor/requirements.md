# Requirements: PARKING_OPERATOR_ADAPTOR (Spec 08)

> EARS-syntax requirements for the PARKING_OPERATOR_ADAPTOR and mock PARKING_OPERATOR.
> Derived from the PRD at `.specs/08_parking_operator_adaptor/prd.md` and the master PRD at `.specs/prd.md`.

## Notation

Requirements use the EARS (Easy Approach to Requirements Syntax) patterns:

- **Ubiquitous:** `The <system> shall <action>.`
- **Event-driven:** `When <trigger>, the <system> shall <action>.`
- **State-driven:** `While <state>, the <system> shall <action>.`
- **Unwanted behavior:** `If <condition>, then the <system> shall <action>.`
- **Option:** `Where <feature>, the <system> shall <action>.`

## PARKING_OPERATOR_ADAPTOR Requirements

### 08-REQ-1.1: Autonomous Session Start on Lock Event

When the DATA_BROKER publishes `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true`, the PARKING_OPERATOR_ADAPTOR shall call the PARKING_OPERATOR `POST /parking/start` with `{vehicle_id, zone_id, timestamp}` and, upon receiving a successful response, transition the session state to active.

**Rationale:** The PARKING_OPERATOR_ADAPTOR owns the parking session lifecycle and must autonomously start sessions in response to lock events, eliminating the need for manual user interaction.

**Acceptance criteria:**
- PARKING_OPERATOR_ADAPTOR subscribes to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` on DATA_BROKER via gRPC (network TCP, port 55556) at startup.
- On receiving `IsLocked = true`, the adaptor sends `POST /parking/start` with a JSON body containing `vehicle_id`, `zone_id`, and `timestamp`.
- On a `200 OK` response from the operator, the session state transitions from idle to active and the returned `session_id` is stored.
- If a session is already active (double lock), the adaptor shall not create a duplicate session and shall ignore the redundant lock event.

---

### 08-REQ-1.2: Autonomous Session Stop on Unlock Event

When the DATA_BROKER publishes `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = false`, the PARKING_OPERATOR_ADAPTOR shall call the PARKING_OPERATOR `POST /parking/stop` with `{session_id, timestamp}` and, upon receiving a successful response, transition the session state to idle.

**Rationale:** Unlock events signal the end of a parking session. The adaptor must autonomously stop the session and retrieve the fee and duration from the operator.

**Acceptance criteria:**
- On receiving `IsLocked = false`, the adaptor sends `POST /parking/stop` with the active `session_id` and current `timestamp`.
- On a `200 OK` response, the session state transitions from active to idle and the returned `duration` and `fee` are stored.
- If no session is currently active (double unlock), the adaptor shall ignore the redundant unlock event and not call the operator.

---

### 08-REQ-2.1: Session State Publication to DATA_BROKER

When the session state transitions to active, the PARKING_OPERATOR_ADAPTOR shall write `Vehicle.Parking.SessionActive = true` to DATA_BROKER. When the session state transitions to idle, the PARKING_OPERATOR_ADAPTOR shall write `Vehicle.Parking.SessionActive = false` to DATA_BROKER.

**Rationale:** Downstream consumers (PARKING_APP, CLOUD_GATEWAY_CLIENT) depend on the `Vehicle.Parking.SessionActive` signal to display session status and relay vehicle telemetry.

**Acceptance criteria:**
- After a successful session start (operator returns `200 OK`), `Vehicle.Parking.SessionActive` is set to `true` on DATA_BROKER.
- After a successful session stop (operator returns `200 OK`), `Vehicle.Parking.SessionActive` is set to `false` on DATA_BROKER.
- The signal is not updated when an operator call fails (state remains unchanged).

---

### 08-REQ-3.1: gRPC Manual Override Interface

The PARKING_OPERATOR_ADAPTOR shall expose a gRPC service on port 50052 with the following RPCs: `StartSession(zone_id)`, `StopSession(session_id)`, `GetStatus()`, and `GetRate()`, allowing the PARKING_APP to manually override autonomous session behavior.

**Rationale:** The PARKING_APP must be able to override the autonomous session lifecycle, for example to manually start a session before locking or to stop a session before unlocking.

**Acceptance criteria:**
- `StartSession(zone_id)` calls `POST /parking/start` on the operator and returns the `session_id` and `status`. If a session is already active, the RPC returns a gRPC `ALREADY_EXISTS` error.
- `StopSession(session_id)` calls `POST /parking/stop` on the operator and returns the `session_id`, `duration`, `fee`, and `status`. If no session is active, the RPC returns a gRPC `NOT_FOUND` error.
- `GetStatus()` returns the current session state (idle, starting, active, stopping), and if active, the `session_id`.
- `GetRate()` returns the current parking rate for the active zone. If no zone is configured, the RPC returns a gRPC `FAILED_PRECONDITION` error.
- Manual `StartSession` and `StopSession` calls update `Vehicle.Parking.SessionActive` on DATA_BROKER identically to autonomous operations.

---

### 08-REQ-4.1: PARKING_OPERATOR REST Client

The PARKING_OPERATOR_ADAPTOR shall communicate with the PARKING_OPERATOR via HTTP REST, sending `POST /parking/start` with `{vehicle_id, zone_id, timestamp}` and `POST /parking/stop` with `{session_id, timestamp}`, and parsing the JSON responses.

**Rationale:** The operator exposes a proprietary REST API. The adaptor must translate between its internal gRPC/event-driven model and the operator's REST interface.

**Acceptance criteria:**
- The adaptor sends well-formed JSON POST requests with `Content-Type: application/json`.
- The adaptor correctly parses `{session_id, status}` from start responses and `{session_id, duration, fee, status}` from stop responses.
- If the PARKING_OPERATOR is unreachable (connection refused, timeout), the adaptor shall log the error and return a gRPC `UNAVAILABLE` status to any pending RPC caller. The session state shall remain unchanged.
- If the PARKING_OPERATOR returns a non-200 HTTP status, the adaptor shall log the error and return a gRPC `INTERNAL` status. The session state shall remain unchanged.
- The operator base URL shall be configurable via environment variable (`PARKING_OPERATOR_URL`).

---

### 08-REQ-5.1: Mock PARKING_OPERATOR

The mock PARKING_OPERATOR shall be a Go HTTP server that implements `POST /parking/start` and `POST /parking/stop`, returning mock session IDs, durations, and fees.

**Rationale:** A mock operator is needed to test the PARKING_OPERATOR_ADAPTOR without a real parking operator backend.

**Acceptance criteria:**
- `POST /parking/start` accepts `{vehicle_id, zone_id, timestamp}` and returns `{session_id, status: "active"}` with a generated UUID session ID and HTTP 200.
- `POST /parking/stop` accepts `{session_id, timestamp}` and returns `{session_id, duration, fee, status: "completed"}` with HTTP 200. The `duration` is calculated from the start timestamp. The `fee` is computed as `duration_minutes * rate_per_minute` (configurable, default 0.05 EUR/min).
- If `POST /parking/stop` is called with an unknown `session_id`, the server returns HTTP 404 with `{error: "session not found"}`.
- If the request body is malformed or missing required fields, the server returns HTTP 400 with `{error: "bad request"}`.
- The mock server listens on a configurable port (default 8080).

---

### 08-REQ-6.1: Operator Unreachable Resilience

If the PARKING_OPERATOR is unreachable when the PARKING_OPERATOR_ADAPTOR attempts to start or stop a session (either autonomously or via manual override), the PARKING_OPERATOR_ADAPTOR shall log the failure, leave the session state unchanged, and — for autonomous operations — not retry automatically.

**Rationale:** The demo scope excludes complex retry logic for network failures. Failing gracefully and preserving consistent state is sufficient.

**Acceptance criteria:**
- A connection error during autonomous session start (lock event) results in the session remaining in idle state; `Vehicle.Parking.SessionActive` is not written.
- A connection error during autonomous session stop (unlock event) results in the session remaining in active state; `Vehicle.Parking.SessionActive` remains `true`.
- A connection error during a manual gRPC `StartSession` or `StopSession` call returns gRPC `UNAVAILABLE` to the caller.
- The error is logged with sufficient context (operator URL, error message, operation attempted).
