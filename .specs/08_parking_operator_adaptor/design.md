# Design: PARKING_OPERATOR_ADAPTOR (Spec 08)

> Design document for the PARKING_OPERATOR_ADAPTOR (Rust gRPC service).
> Implements requirements from `.specs/08_parking_operator_adaptor/requirements.md`.

## References

- Master PRD: `.specs/prd.md`
- Component PRD: `.specs/08_parking_operator_adaptor/prd.md`
- Requirements: `.specs/08_parking_operator_adaptor/requirements.md`
- DATA_BROKER Design: `.specs/02_data_broker/design.md`

## Architecture Overview

The PARKING_OPERATOR_ADAPTOR is an async Rust service running in the RHIVOS QM partition. It bridges three interfaces:

1. **DATA_BROKER subscription** (inbound) -- subscribes to lock/unlock events for autonomous session management and writes session state.
2. **gRPC service** (inbound, port 50052) -- exposes `StartSession`, `StopSession`, `GetStatus`, `GetRate` RPCs for PARKING_APP override control.
3. **PARKING_OPERATOR REST client** (outbound) -- calls the operator's proprietary REST API to start/stop parking sessions and query status.

```
+----------------------------------------------------------------------+
|  RHIVOS QM Partition                                                 |
|                                                                      |
|  +----------------------------------------------------------------+  |
|  | PARKING_OPERATOR_ADAPTOR (Rust)                                 |  |
|  |                                                                 |  |
|  |  +----------------------+     +-------------------------+      |  |
|  |  | DATA_BROKER          |     | gRPC Service            |      |  |
|  |  | Subscriber           |     | (port 50052)            |      |  |
|  |  | (lock/unlock events) |     | StartSession            |      |  |
|  |  +----------+-----------+     | StopSession             |      |  |
|  |             |                 | GetStatus               |      |  |
|  |             v                 | GetRate                  |      |  |
|  |  +----------------------+     +------------+------------+      |  |
|  |  | Session State        |<----------------+                    |  |
|  |  | Machine              |                                      |  |
|  |  +----------+-----------+                                      |  |
|  |             |                                                   |  |
|  |             v                                                   |  |
|  |  +----------------------+     +-------------------------+      |  |
|  |  | Operator REST        |     | DATA_BROKER             |      |  |
|  |  | Client (reqwest)     |     | Publisher                |      |  |
|  |  | POST /parking/start  |     | (SessionActive)         |      |  |
|  |  | POST /parking/stop   |     +-------------------------+      |  |
|  |  | GET /parking/status  |                                      |  |
|  |  +----------------------+                                      |  |
|  +----------------------------------------------------------------+  |
|                    |                          ^                       |
+--------------------+--------------------------+----------------------+
                     | HTTP REST                | gRPC (network TCP)
                     v                          |
          +---------------------+     +---------------------+
          | PARKING_OPERATOR    |     | DATA_BROKER         |
          | (mock or real)      |     | (Safety Partition)  |
          | POST /parking/start |     | port 55556          |
          | POST /parking/stop  |     +---------------------+
          | GET /parking/status |
          +---------------------+

                                      +---------------------+
                                      | PARKING_APP         |
                                      | (AAOS IVI)          |
                                      | gRPC client         |
                                      | -> port 50052       |
                                      +---------------------+
```

## Technology Stack

| Technology | Version / Reference | Purpose |
|------------|-------------------|---------|
| Rust | Stable (edition 2021) | Primary language |
| tokio | Latest stable | Async runtime |
| tonic | Latest stable | gRPC server framework |
| prost | Latest stable | Protocol buffer code generation |
| reqwest | Latest stable (with `json` feature) | HTTP client for operator REST API |
| serde / serde_json | Latest stable | JSON serialization/deserialization |
| tracing | Latest stable | Structured logging |

## Module Structure

```
rhivos/parking-operator-adaptor/
  Cargo.toml
  build.rs                          # tonic-build for proto compilation
  src/
    main.rs                         # Entry point, service wiring
    config.rs                       # Configuration (env vars, ports, URLs)
    grpc/
      mod.rs
      service.rs                    # ParkingAdaptorService impl (tonic)
    operator/
      mod.rs
      client.rs                     # REST client for PARKING_OPERATOR
      models.rs                     # Request/response structs (serde)
    broker/
      mod.rs
      subscriber.rs                 # DATA_BROKER subscription (lock events)
      publisher.rs                  # DATA_BROKER write (SessionActive)
    session/
      mod.rs
      state.rs                      # Session state machine
```

## gRPC Service Definition

Defined in `proto/parking_adaptor.proto`:

```protobuf
syntax = "proto3";
package parking_adaptor;

service ParkingAdaptor {
  rpc StartSession(StartSessionRequest) returns (StartSessionResponse);
  rpc StopSession(StopSessionRequest) returns (StopSessionResponse);
  rpc GetStatus(GetStatusRequest) returns (GetStatusResponse);
  rpc GetRate(GetRateRequest) returns (GetRateResponse);
}

message StartSessionRequest {
  string zone_id = 1;
}

message StartSessionResponse {
  string session_id = 1;
  string status = 2;
}

message StopSessionRequest {}

message StopSessionResponse {
  string session_id = 1;
  int64 duration_seconds = 2;
  double fee = 3;
  string status = 4;
}

message GetStatusRequest {}

message GetStatusResponse {
  string state = 1;       // "idle", "starting", "active", "stopping"
  string session_id = 2;  // empty if idle
  string zone_id = 3;     // empty if no zone configured
}

message GetRateRequest {}

message GetRateResponse {
  string rate_type = 1;   // "per_hour" or "flat_fee"
  double rate_amount = 2;
  string currency = 3;
  string zone_id = 4;
}
```

## Session State Machine

The adaptor maintains an internal session state machine that governs all transitions. Both autonomous (DATA_BROKER events) and manual (gRPC overrides) operations go through this state machine.

```
             lock event / StartSession RPC
                    |
                    v
  +-------+    +-----------+    operator 200 OK    +--------+
  | idle  |--->| starting  |--------------------->| active |
  +-------+    +-----------+                       +--------+
      ^            |                                  |
      |            | operator error                   | unlock event / StopSession RPC
      |            v                                  v
      |        +-------+                         +-----------+
      |        | idle  |                         | stopping  |
      |        +-------+                         +-----------+
      |                                               |
      |                     operator 200 OK           |
      +-----------------------------------------------+
      |                     operator error            |
      +-----------------------------------------------+
```

### State Descriptions

| State | Description |
|-------|-------------|
| `idle` | No active parking session. Waiting for lock event or manual start. |
| `starting` | Lock event or StartSession received; REST call to operator in progress. |
| `active` | Operator confirmed session start. SessionActive = true published to DATA_BROKER. |
| `stopping` | Unlock event or StopSession received; REST call to operator in progress. |

### Transition Rules

1. `idle -> starting`: On lock event (autonomous) or `StartSession` RPC (manual). If already in `active`, the event is ignored (idempotent).
2. `starting -> active`: On operator `POST /parking/start` returning `200 OK`. Write `SessionActive = true` to DATA_BROKER.
3. `starting -> idle`: On operator error (unreachable, non-200 response). Log error, do not update DATA_BROKER.
4. `active -> stopping`: On unlock event (autonomous) or `StopSession` RPC (manual). If already in `idle`, the event is ignored (idempotent).
5. `stopping -> idle`: On operator `POST /parking/stop` returning `200 OK`. Write `SessionActive = false` to DATA_BROKER.
6. `stopping -> idle`: On operator error. Log error. Transition to idle to avoid stuck state; `SessionActive` remains `true` (stale, but the operator did not confirm stop).

### Concurrency

The session state is protected by a `tokio::sync::Mutex`. All state transitions are serialized. The DATA_BROKER subscriber and gRPC handlers acquire the lock before inspecting or modifying state.

## Autonomous Session Management

### DATA_BROKER Subscription

At startup, the adaptor establishes a gRPC subscription to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` on the DATA_BROKER (TCP, port 55556). The subscription uses the Kuksa Databroker `Subscribe` RPC and receives a streaming response.

For each update on the subscription stream:

1. Extract the `IsLocked` boolean value.
2. If `IsLocked = true` and state is `idle`: trigger session start.
3. If `IsLocked = false` and state is `active`: trigger session stop.
4. All other combinations are ignored (idempotent behavior).

### Override Handling

Manual gRPC calls (`StartSession`, `StopSession`) use the same state machine as autonomous operations. This means:

- A manual `StartSession` while idle triggers the same `idle -> starting -> active` flow.
- A manual `StopSession` while active triggers the same `active -> stopping -> idle` flow.
- A subsequent lock event after a manual start is ignored (session already active).
- A subsequent unlock event after a manual stop is ignored (session already idle).

## PARKING_OPERATOR REST Client

### Endpoints

| Method | Path | Request Body | Response Body |
|--------|------|-------------|---------------|
| POST | `/parking/start` | `{"vehicle_id": "string", "zone_id": "string", "timestamp": int64}` | `{"session_id": "string", "status": "string"}` |
| POST | `/parking/stop` | `{"session_id": "string", "timestamp": int64}` | `{"session_id": "string", "duration": int64, "fee": float64, "status": "string"}` |
| GET | `/parking/status/{session_id}` | -- | `{"session_id": "string", "status": "string", "rate_type": "string", "rate_amount": float64, "currency": "string"}` |

### HTTP Client Behavior

- Timeout: 5 seconds per request.
- No automatic retries (demo scope).
- Content-Type: `application/json`.
- On connection error: return `Err` with context; caller decides state transition.

## Configuration Schema

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `PARKING_OPERATOR_URL` | `http://localhost:8080` | Base URL of the PARKING_OPERATOR REST API |
| `DATA_BROKER_ADDR` | `http://localhost:55556` | DATA_BROKER gRPC address (network TCP) |
| `GRPC_PORT` | `50052` | Port for the adaptor's own gRPC service |
| `VEHICLE_ID` | `DEMO-VIN-001` | Vehicle identifier sent in operator requests |
| `ZONE_ID` | `zone-demo-1` | Default parking zone identifier |

## Correctness Properties

| ID | Property | Description |
|----|----------|-------------|
| CP-1 | Autonomous session lifecycle | When a lock event is followed by an unlock event and the operator is reachable, exactly one `POST /parking/start` and one `POST /parking/stop` call shall be made to the operator, and `Vehicle.Parking.SessionActive` shall transition from `false -> true -> false`. |
| CP-2 | Idempotent lock handling | When multiple consecutive lock events are received without an intervening unlock, only the first lock event shall trigger a `POST /parking/start` call. Subsequent lock events shall be ignored. |
| CP-3 | Idempotent unlock handling | When multiple consecutive unlock events are received without an intervening lock, only the first unlock event (if a session is active) shall trigger a `POST /parking/stop` call. Subsequent unlock events shall be ignored. |
| CP-4 | Override consistency | A manual `StartSession` or `StopSession` via gRPC shall produce the same state transitions, DATA_BROKER updates, and operator REST calls as the equivalent autonomous operation. |
| CP-5 | State-signal consistency | `Vehicle.Parking.SessionActive` on DATA_BROKER shall always reflect the adaptor's internal session state: `true` when state is `active`, `false` when state is `idle`. The signal is not updated during transient states (`starting`, `stopping`). |
| CP-6 | Failure isolation | An operator REST failure (unreachable, non-200 response) during session start shall leave the session in `idle` state and shall not update `Vehicle.Parking.SessionActive`. The adaptor shall remain functional and ready for subsequent events. |

## Error Handling

| Error Scenario | Component | Behavior |
|---------------|-----------|----------|
| PARKING_OPERATOR unreachable (connection refused) | REST client | Log error; session state unchanged; gRPC caller gets `UNAVAILABLE` |
| PARKING_OPERATOR returns HTTP 4xx/5xx | REST client | Log error with status code and body; session state unchanged; gRPC caller gets `INTERNAL` |
| PARKING_OPERATOR request timeout (>5s) | REST client | Log timeout; session state unchanged; gRPC caller gets `DEADLINE_EXCEEDED` |
| DATA_BROKER unreachable at startup | Subscriber | Log error and retry connection with backoff; service starts but autonomous mode inactive until connected |
| DATA_BROKER write failure (SessionActive) | Publisher | Log error; session state already transitioned internally; signal may be stale |
| StartSession called while session active | gRPC service | Return gRPC `ALREADY_EXISTS` with message "session already active" |
| StopSession called while no session active | gRPC service | Return gRPC `NOT_FOUND` with message "no active session" |
| GetRate called with no zone configured | gRPC service | Return gRPC `FAILED_PRECONDITION` with message "no zone configured" |

## Definition of Done

1. PARKING_OPERATOR_ADAPTOR builds without warnings: `cd rhivos && cargo clippy -p parking-operator-adaptor`.
2. PARKING_OPERATOR_ADAPTOR unit tests pass: `cd rhivos && cargo test -p parking-operator-adaptor`.
3. Session state machine handles all transitions correctly, including double lock/unlock and operator errors.
4. Autonomous session management works: lock event triggers start, unlock event triggers stop.
5. gRPC service exposes all four RPCs (`StartSession`, `StopSession`, `GetStatus`, `GetRate`) on port 50052.
6. Manual overrides produce the same state transitions as autonomous operations.
7. `Vehicle.Parking.SessionActive` is correctly published to DATA_BROKER on session start/stop.
8. PARKING_OPERATOR REST client correctly sends start/stop/status requests and parses responses.
9. Rate information (per_hour or flat_fee) is returned by `GetRate`.
10. Integration tests pass with DATA_BROKER, mock operator, and adaptor running together.

## Testing Strategy

### Unit Tests (Rust)

Located in `rhivos/parking-operator-adaptor/src/` as `#[cfg(test)]` modules.

- **Session state machine:** Test all state transitions, including edge cases (double lock, double unlock, error recovery).
- **Operator client:** Test request serialization and response deserialization using mock HTTP responses (e.g., `wiremock` crate).
- **gRPC service:** Test RPC handlers with mocked session state and operator client.

Command: `cd rhivos && cargo test -p parking-operator-adaptor`

### Integration Tests

Located in `tests/` directory.

- **Lock-to-session flow:** Start DATA_BROKER + mock operator + adaptor. Write `IsLocked = true` to DATA_BROKER. Verify `SessionActive = true` appears on DATA_BROKER. Verify mock operator received start call.
- **Unlock-to-session flow:** With active session, write `IsLocked = false`. Verify `SessionActive = false`. Verify mock operator received stop call with correct session_id.
- **gRPC override:** Call `StartSession` via gRPC. Verify session active. Call `StopSession`. Verify session idle.
- **Operator down:** Stop mock operator. Write `IsLocked = true`. Verify session remains idle. Verify `SessionActive` not written.

### Lint

Command: `cd rhivos && cargo clippy -p parking-operator-adaptor`
