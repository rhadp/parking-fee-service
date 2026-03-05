# Design: LOCKING_SERVICE (Spec 03)

> Design document for the LOCKING_SERVICE component.
> Implements requirements from `.specs/03_locking_service/requirements.md`.

## References

- Master PRD: `.specs/prd.md`
- Component PRD: `.specs/03_locking_service/prd.md`
- Requirements: `.specs/03_locking_service/requirements.md`

## Architecture Overview

The LOCKING_SERVICE is a Rust service running in the RHIVOS safety partition. It communicates exclusively with DATA_BROKER via gRPC over Unix Domain Sockets. The service operates a single processing pipeline: subscribe to command signals, validate, check safety constraints, execute, and respond.

```
                  +-------------------------------------+
                  |          LOCKING_SERVICE             |
                  |                                     |
                  |  +-------------+                    |
                  |  | Command     |                    |
                  |  | Handler     |                    |
                  |  +------+------+                    |
                  |         |                           |
                  |         v                           |
                  |  +------+------+                    |
                  |  | Safety      |                    |
                  |  | Validator   |                    |
                  |  +------+------+                    |
                  |         |                           |
                  |         v                           |
                  |  +------+------+                    |
                  |  | State       |                    |
                  |  | Writer      |                    |
                  |  +------+------+                    |
                  |         |                           |
                  +---------|---------------------------+
                            |
                            v
                  +---------------------+
                  | DATA_BROKER         |
                  | (gRPC over UDS)     |
                  +---------------------+
```

## Technology Stack

| Component | Technology | Purpose |
|-----------|-----------|---------|
| Language | Rust (edition 2021) | Safety-critical service implementation |
| Async runtime | tokio | Async I/O, task spawning, signal handling |
| gRPC client | tonic | Connection to DATA_BROKER via UDS |
| JSON handling | serde, serde_json | Command parsing and response serialization |
| Logging | tracing | Structured logging |
| UUID | uuid | Command ID validation |

## Module Structure

The crate lives at `rhivos/locking-service/` within the Rust workspace.

```
rhivos/locking-service/
  Cargo.toml
  src/
    main.rs              # Entry point: config loading, task spawning, signal handling
    config.rs            # Environment variable parsing (DATABROKER_UDS_PATH)
    databroker_client.rs # DATA_BROKER gRPC client (tonic over UDS)
    command.rs           # Command data types, JSON parsing, validation logic
    safety.rs            # Safety constraint checks (speed, door ajar)
    executor.rs          # Lock/unlock execution and response writing
```

## Configuration

All configuration is via environment variables:

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DATABROKER_UDS_PATH` | No | `/tmp/kuksa/databroker.sock` | Unix Domain Socket path for DATA_BROKER gRPC |

## VSS Signals Used

### Subscriptions (read)

| Signal Path | Data Type | Purpose |
|-------------|-----------|---------|
| `Vehicle.Command.Door.Lock` | string (JSON) | Incoming lock/unlock command requests |

### Reads on demand

| Signal Path | Data Type | Purpose |
|-------------|-----------|---------|
| `Vehicle.Speed` | float | Safety check: vehicle must be stationary |
| `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` | bool | Safety check: door must not be ajar |

### Writes

| Signal Path | Data Type | Purpose |
|-------------|-----------|---------|
| `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` | bool | Updated lock state after command execution |
| `Vehicle.Command.Door.Response` | string (JSON) | Command execution result |

## Command Processing Flow

```
  Vehicle.Command.Door.Lock (subscription stream)
              |
              v
  +---[ Parse JSON ]---+
  |                     |
  | (malformed)         | (valid JSON)
  v                     v
  Write failure     +---[ Validate fields ]---+
  response          |                         |
  (invalid_command) | (missing fields)        | (all fields present)
                    v                         v
                    Write failure         +---[ Check action ]---+
                    response              |                      |
                    (invalid_command)      | (unknown action)     | (lock or unlock)
                                          v                      v
                                          Write failure      +---[ Safety checks ]---+
                                          response           |                       |
                                          (invalid_action)   | (constraint violated) | (all pass)
                                                             v                       v
                                                             Write failure       Execute lock/unlock
                                                             response            Write state
                                                             (vehicle_moving     Write success
                                                              or door_ajar)      response
```

## Module Responsibilities

### `command.rs` -- Command Handler

- Defines `Command` struct with serde deserialization:
  - `command_id: String` (UUID)
  - `action: String` ("lock" or "unlock")
  - `doors: Vec<String>`
  - `source: String`
  - `vin: String`
  - `timestamp: u64`
- Defines `CommandResponse` struct for serialization:
  - `command_id: String`
  - `status: String` ("success" or "failed")
  - `reason: Option<String>`
  - `timestamp: u64`
- Validates required fields are present and non-empty.
- Validates `action` is `"lock"` or `"unlock"`.
- Returns structured validation errors on failure.

### `safety.rs` -- Safety Validator

- Reads `Vehicle.Speed` from DATA_BROKER via `databroker_client`.
- Reads `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` from DATA_BROKER via `databroker_client`.
- Returns `Ok(())` if all safety constraints pass.
- Returns `Err(reason)` with the specific constraint violated:
  - `"vehicle_moving"` if speed >= 1.0 km/h.
  - `"door_ajar"` if `IsOpen == true`.
- If a safety signal has not been set (no current value), the check passes (safe default for demo: assume stationary, door closed).

### `executor.rs` -- State Writer

- On `"lock"` action: writes `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true` to DATA_BROKER.
- On `"unlock"` action: writes `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = false` to DATA_BROKER.
- Writes the `CommandResponse` JSON to `Vehicle.Command.Door.Response` on DATA_BROKER.
- Handles DATA_BROKER write failures by logging the error.

### `databroker_client.rs` -- gRPC Client

- Creates a tonic gRPC client connecting to DATA_BROKER via Unix Domain Socket.
- Provides methods:
  - `subscribe_signal(path) -> Stream<SignalUpdate>` for subscribing to signal changes.
  - `get_signal(path) -> Option<Value>` for reading a signal's current value.
  - `set_signal(path, value)` for writing a signal value (bool or string).
- Handles connection errors with retry and exponential backoff (1s, 2s, 4s, ..., max 30s).

## gRPC Client Configuration

- **Transport:** Unix Domain Socket (UDS)
- **Default UDS path:** `/tmp/kuksa/databroker.sock`
- **Protocol:** Kuksa Databroker gRPC API (kuksa.val.v1)
- **Serialization:** Protocol Buffers

## Correctness Properties

| ID | Property | Description |
|----|----------|-------------|
| CP-1 | Safety constraints always checked | Every command that reaches the execution step has passed both the speed check and the door ajar check. No command bypasses safety validation. |
| CP-2 | Response always sent | Every command received on `Vehicle.Command.Door.Lock` results in exactly one response on `Vehicle.Command.Door.Response`, whether success or failure. |
| CP-3 | State always updated on success | A successful lock command always sets `IsLocked = true`; a successful unlock command always sets `IsLocked = false`. The state write and response write are both performed. |
| CP-4 | Invalid commands never execute | A command that fails JSON parsing, field validation, action validation, or safety checks never modifies `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`. |
| CP-5 | Command ID preserved | The `command_id` in the response always matches the `command_id` from the incoming command. |

## Error Handling

| Error Condition | Behavior | Requirement | Log Level |
|----------------|----------|-------------|-----------|
| DATA_BROKER unreachable at startup | Retry with exponential backoff (max 30s) | 03-REQ-1.2 | WARN |
| DATA_BROKER connection lost during operation | Detect broken stream; reconnect and re-subscribe | 03-REQ-8.1 | ERROR |
| Malformed command JSON | Discard; write failure response with reason `"invalid_command"` | 03-REQ-2.2 | WARN |
| Missing required fields in command | Discard; write failure response with reason `"invalid_command"` | 03-REQ-2.2 | WARN |
| Unknown action value | Discard; write failure response with reason `"invalid_action"` | 03-REQ-6.1 | WARN |
| Vehicle speed >= 1.0 km/h | Reject command; write failure response with reason `"vehicle_moving"` | 03-REQ-3.3 | INFO |
| Door is ajar (IsOpen == true) | Reject command; write failure response with reason `"door_ajar"` | 03-REQ-3.3 | INFO |
| DATA_BROKER write fails for state update | Log error; response may still be attempted | 03-REQ-4.1 | ERROR |
| DATA_BROKER write fails for response | Log error; response is lost | 03-REQ-5.1 | ERROR |
| SIGTERM/SIGINT received | Cancel subscriptions; close gRPC connection; exit 0 | 03-REQ-7.1 | INFO |

## Startup Sequence

1. Parse environment variables (`DATABROKER_UDS_PATH`). Apply defaults.
2. Connect to DATA_BROKER via gRPC/UDS. Retry with backoff if unreachable.
3. Subscribe to `Vehicle.Command.Door.Lock` on DATA_BROKER.
4. Log "LOCKING_SERVICE started".
5. Enter command processing loop: for each signal update, run the full processing pipeline (parse -> validate -> safety check -> execute -> respond).
6. On SIGTERM/SIGINT: cancel subscriptions, close connection, exit 0.

## Testing Strategy

### Unit Tests

Unit tests cover individual modules in isolation:

- `config.rs`: Env var parsing, defaults.
- `command.rs`: Command struct parsing (valid, missing fields, invalid action, malformed JSON). Response serialization.
- `safety.rs`: Safety constraint evaluation with mock signal values (speed check, door ajar check, combined checks).

Run with: `cd rhivos && cargo test -p locking-service`

### Integration Tests

Integration tests require a running DATA_BROKER:

- Full command pipeline: write command to `Vehicle.Command.Door.Lock` -> verify state updated and response written.
- Safety constraint rejection: set speed > 0 -> send command -> verify rejection response.
- Door ajar rejection: set door open -> send command -> verify rejection response.
- Invalid command handling: write malformed JSON -> verify failure response.

Requires: `make infra-up` (starts Kuksa Databroker)

### Lint

Run with: `cd rhivos && cargo clippy -p locking-service`

## Definition of Done

1. All requirements in `requirements.md` are implemented and covered by tests.
2. `cargo build -p locking-service` compiles without errors or warnings.
3. `cargo clippy -p locking-service` passes with no warnings.
4. `cargo test -p locking-service` passes all unit tests.
5. Integration tests pass against running DATA_BROKER infrastructure.
6. The service starts successfully, connects to DATA_BROKER, and processes lock/unlock commands end-to-end.
7. Safety constraints are enforced: commands are rejected when the vehicle is moving or the door is ajar.
8. Structured logging is present for all command processing, safety checks, and error conditions.
