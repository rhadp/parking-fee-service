# Design: LOCKING_SERVICE (Spec 03)

> Design document for the LOCKING_SERVICE and mock sensor CLI tools.
> Implements the requirements defined in `.specs/03_locking_service/requirements.md`.

## 1. Architecture Overview

The LOCKING_SERVICE is a Rust binary running in the RHIVOS safety partition (ASIL-B). It communicates exclusively with DATA_BROKER via gRPC over Unix Domain Sockets (UDS). The service subscribes to command signals, validates safety constraints by reading vehicle state signals, and publishes lock state and command responses back to DATA_BROKER.

Three mock sensor CLIs (LOCATION_SENSOR, SPEED_SENSOR, DOOR_SENSOR) are standalone Rust binaries that write sensor values to DATA_BROKER via gRPC. They are on-demand tools used for testing, not long-running services.

```
+------------------------------------------------------------------+
|  RHIVOS Safety Partition                                         |
|                                                                  |
|  +--------------------+       gRPC/UDS       +-----------------+ |
|  | LOCKING_SERVICE    | <------------------> | DATA_BROKER     | |
|  |                    |  subscribe commands   | (Kuksa)         | |
|  |  - cmd handler     |  read sensor state    |                 | |
|  |  - safety validator|  write lock state     |                 | |
|  |  - response writer |  write responses      |                 | |
|  +--------------------+                       +-----------------+ |
|                                                     ^             |
+------------------------------------------------------------------+
                                                      | gRPC
                                              +-------+-------+
                                              |  Mock Sensors  |
                                              | (CLI tools)    |
                                              +----------------+
```

## 2. Module Structure

### LOCKING_SERVICE (`rhivos/locking-service/`)

```
rhivos/locking-service/
  Cargo.toml
  src/
    main.rs              # Entry point, gRPC client setup, subscription loop
    command.rs           # Command parsing and types (LockCommand, CommandResponse)
    validator.rs         # Safety constraint validation logic
    broker.rs            # DATA_BROKER gRPC client wrapper (read/write signals)
    config.rs            # Configuration (UDS path, speed threshold, signal paths)
    error.rs             # Error types
```

### Mock Sensor CLIs (`rhivos/mock-sensors/`)

```
rhivos/mock-sensors/
  Cargo.toml
  src/
    bin/
      location_sensor.rs  # LOCATION_SENSOR CLI entry point
      speed_sensor.rs      # SPEED_SENSOR CLI entry point
      door_sensor.rs       # DOOR_SENSOR CLI entry point
    broker_client.rs       # Shared DATA_BROKER gRPC client for sensor writes
```

## 3. gRPC Communication with DATA_BROKER

The LOCKING_SERVICE and mock sensors communicate with Eclipse Kuksa Databroker using the Kuksa gRPC API over UDS.

### Connection

- **Transport:** Unix Domain Socket
- **Default UDS path:** `/tmp/kuksa/databroker.sock` (configurable via `DATABROKER_UDS_PATH` environment variable)
- **Fallback for local development:** TCP at `localhost:55556` (configurable via `DATABROKER_ADDRESS` environment variable)
- **Library:** `tonic` crate with UDS support, using Kuksa protobuf definitions

### Operations Used

| Operation | Kuksa gRPC Method | Direction | Signal(s) |
|-----------|------------------|-----------|-----------|
| Subscribe to commands | `Subscribe` (streaming) | Read | `Vehicle.Command.Door.Lock` |
| Read vehicle speed | `GetCurrentValues` | Read | `Vehicle.Speed` |
| Read door state | `GetCurrentValues` | Read | `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` |
| Write lock state | `SetCurrentValues` | Write | `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` |
| Write command response | `SetCurrentValues` | Write | `Vehicle.Command.Door.Response` |
| Write sensor values | `SetCurrentValues` | Write | Various (mock sensors) |

## 4. Command Processing Pipeline

The LOCKING_SERVICE processes commands in a serial pipeline:

```
Subscribe ──> Parse ──> Validate ──> Execute ──> Respond
   (1)         (2)        (3)         (4)         (5)
```

### Step 1: Subscribe

The service subscribes to `Vehicle.Command.Door.Lock` via Kuksa's `Subscribe` streaming RPC. Each incoming message contains a JSON-encoded command payload.

### Step 2: Parse

The JSON payload is deserialized into a `LockCommand` struct:

```rust
struct LockCommand {
    command_id: String,          // UUID
    action: Action,              // "lock" | "unlock"
    doors: Vec<String>,          // ["driver"]
    source: String,              // "companion_app"
    vin: String,                 // Vehicle Identification Number
    timestamp: u64,              // Unix timestamp
}

enum Action {
    Lock,
    Unlock,
}
```

If parsing fails, the error is handled per 03-REQ-4.2.

### Step 3: Validate

Safety constraints are checked by reading current signal values from DATA_BROKER:

1. Read `Vehicle.Speed` from DATA_BROKER. If speed > 0.5 m/s, reject with `"vehicle_moving"`.
2. If action is `Lock`, read `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen`. If `true`, reject with `"door_ajar"`.
3. If action is `Unlock`, skip the door-ajar check.

### Step 4: Execute

If all constraints pass, write the new lock state to DATA_BROKER:

- Lock: set `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true`
- Unlock: set `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = false`

### Step 5: Respond

Publish a `CommandResponse` to `Vehicle.Command.Door.Response`:

```rust
struct CommandResponse {
    command_id: String,
    status: String,              // "success" | "failed"
    reason: Option<String>,      // Present when status == "failed"
    timestamp: u64,              // Unix timestamp
}
```

## 5. Safety Constraint Validation Logic

The validator module implements the following decision logic:

```
fn validate(command: &LockCommand) -> Result<(), RejectionReason> {
    // 1. Speed check (applies to both lock and unlock)
    let speed = read_signal("Vehicle.Speed");
    if speed > SPEED_THRESHOLD {          // SPEED_THRESHOLD = 0.5
        return Err(RejectionReason::VehicleMoving);
    }

    // 2. Door-ajar check (applies only to lock)
    if command.action == Action::Lock {
        let is_open = read_signal("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen");
        if is_open {
            return Err(RejectionReason::DoorAjar);
        }
    }

    Ok(())
}
```

The `SPEED_THRESHOLD` constant (0.5 m/s) is defined in `config.rs` and can be overridden via the `SPEED_THRESHOLD` environment variable for testing.

## 6. State Machine

The LOCKING_SERVICE itself is stateless with respect to lock state -- the lock state is persisted in DATA_BROKER. The service acts as a command processor that reads and writes DATA_BROKER signals.

The logical lock state transitions are:

```
                    lock (valid)
    UNLOCKED ──────────────────────> LOCKED
       ^                               |
       |        unlock (valid)         |
       +<──────────────────────────────+
```

**Transition rules:**
- `UNLOCKED -> LOCKED`: Requires speed <= 0.5 AND door not ajar.
- `LOCKED -> UNLOCKED`: Requires speed <= 0.5. No door-ajar constraint.
- Locking an already-locked door or unlocking an already-unlocked door is idempotent: the command succeeds and the current state is re-written to DATA_BROKER.

## 7. Mock Sensor CLI Design

Each mock sensor is a simple Rust binary that:

1. Parses command-line arguments using the `clap` crate.
2. Connects to DATA_BROKER via gRPC (UDS or TCP, same connection logic as LOCKING_SERVICE).
3. Writes the specified value(s) to the appropriate VSS signal(s).
4. Exits with code 0 on success, non-zero on failure.

### LOCATION_SENSOR

```
location-sensor --latitude 48.1351 --longitude 11.5820
```

Writes `Vehicle.CurrentLocation.Latitude` and `Vehicle.CurrentLocation.Longitude`.

### SPEED_SENSOR

```
speed-sensor --speed 0.0
```

Writes `Vehicle.Speed`.

### DOOR_SENSOR

```
door-sensor --open false
```

Writes `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen`.

### Shared Broker Client

All three sensors share a `broker_client` module that encapsulates the gRPC connection setup and `SetCurrentValues` call to DATA_BROKER. This avoids code duplication.

## 8. Correctness Properties

The following properties must hold for the LOCKING_SERVICE implementation:

### CP-1: Safety Gate Completeness

Every lock command passes through both the speed check and the door-ajar check before execution. Every unlock command passes through the speed check before execution. No command path bypasses safety validation.

### CP-2: Atomic State Update

When a command is accepted, the lock state write to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` and the response write to `Vehicle.Command.Door.Response` both complete before the next command is processed. If the lock state write fails, no success response is published.

### CP-3: Response Correlation

Every command response published to `Vehicle.Command.Door.Response` contains the exact `command_id` from the originating command. No response is published without a corresponding command.

### CP-4: Rejection Consistency

A command that is rejected due to a safety constraint violation results in no change to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`. The response status is `"failed"` with an appropriate reason string.

### CP-5: Command Serialization

Commands are processed strictly in FIFO order. No two commands are evaluated or executed concurrently. Each command reads fresh signal values from DATA_BROKER at the start of its processing.

### CP-6: Crash Resilience

An invalid or malformed command does not cause the LOCKING_SERVICE to panic or exit. The service continues processing subsequent commands after handling an invalid one.

### CP-7: Idempotent Lock State

Locking an already-locked door or unlocking an already-unlocked door succeeds and re-writes the current state. No error is produced for idempotent operations.

## 9. Error Handling

| Error Condition | Behavior | Response |
|-----------------|----------|----------|
| DATA_BROKER unreachable on startup | Retry connection with exponential backoff (max 30s). Log error on each retry. Exit after configurable max retries (default: 10). | N/A (no commands received) |
| DATA_BROKER disconnects during operation | Attempt reconnection with exponential backoff. Queue any pending responses. | Pending command gets a failed response if reconnection does not succeed within 10s. |
| `Vehicle.Speed` signal not available | Treat as speed = 0.0 (fail-open for demo; in production, fail-closed). Log warning. | Command proceeds with speed = 0.0. |
| `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` signal not available | Treat as door closed (IsOpen = false) for demo. Log warning. | Lock command proceeds assuming door is closed. |
| Invalid JSON in command payload | Log warning with raw payload. | Failed response with reason `"invalid_command"` if `command_id` extractable; otherwise log-only. |
| Unknown `action` value (not "lock" or "unlock") | Treat as invalid command. | Failed response with reason `"invalid_command"`. |
| Lock state write fails | Log error. Do not publish success response. | Failed response with reason `"internal_error"`. |
| Mock sensor CLI cannot connect to DATA_BROKER | Print error message to stderr. | Exit with non-zero code. |
| Mock sensor CLI receives invalid arguments | Print usage help to stderr. | Exit with non-zero code. |

## 10. Technology Stack

| Component | Technology | Version / Crate |
|-----------|-----------|-----------------|
| Language | Rust | 1.75+ (edition 2021) |
| Async runtime | Tokio | `tokio` 1.x |
| gRPC framework | tonic | `tonic` 0.12+ |
| gRPC UDS transport | tonic + tower | `tower` 0.4+, `hyper-util` |
| Protobuf codegen | prost | `prost` 0.13+, `tonic-build` |
| CLI argument parsing | clap | `clap` 4.x (derive) |
| JSON serialization | serde + serde_json | `serde` 1.x, `serde_json` 1.x |
| UUID generation | uuid | `uuid` 1.x (for timestamps/IDs if needed) |
| Logging | tracing | `tracing` 0.1, `tracing-subscriber` 0.3 |
| Testing | cargo test | Built-in |

### Kuksa Protobuf Definitions

The Kuksa Databroker gRPC API protobuf definitions are used to generate Rust client code via `tonic-build`. The proto files are sourced from the Eclipse Kuksa project and placed at `proto/kuksa/` in the repository.

## 11. Definition of Done

The LOCKING_SERVICE and mock sensor CLIs are considered complete when:

1. **All requirements met:** Every requirement in `requirements.md` has passing tests.
2. **Unit tests pass:** `cd rhivos && cargo test -p locking-service` passes with no failures.
3. **Mock sensor tests pass:** `cd rhivos && cargo test -p mock-sensors` passes with no failures.
4. **Integration tests pass:** Integration tests demonstrating end-to-end command flow through DATA_BROKER pass.
5. **Lint clean:** `cd rhivos && cargo clippy -p locking-service -p mock-sensors -- -D warnings` produces no warnings.
6. **Format clean:** `cd rhivos && cargo fmt -p locking-service -p mock-sensors -- --check` produces no diffs.
7. **Documentation:** Public functions and modules have doc comments.
8. **Safety constraints verified:** All safety validation edge cases (speed > threshold, door ajar, concurrent commands) have dedicated tests.

## 12. Testing Strategy

### Unit Tests

Located in each module within `rhivos/locking-service/src/`. Focus on:

- **command.rs:** Parsing valid and invalid JSON payloads, edge cases (missing fields, unknown action).
- **validator.rs:** Safety constraint logic with mocked signal values (speed above/below threshold, door open/closed, combinations).
- **broker.rs:** Mock gRPC client interactions (not integration -- mocked at the trait level).

Run: `cd rhivos && cargo test -p locking-service`

### Integration Tests

Located in `rhivos/locking-service/tests/` or `tests/integration/`. Require a running DATA_BROKER instance. Focus on:

- Full command pipeline: write command to DATA_BROKER, observe lock state change and response.
- Safety rejection: set speed > 0.5 via SPEED_SENSOR, send lock command, verify rejection.
- Mock sensors: verify each sensor CLI writes the correct signal value to DATA_BROKER.
- Concurrent commands: send multiple commands in quick succession, verify FIFO ordering.

Run: `cd rhivos && cargo test -p locking-service --test integration` (or via a test harness script that starts DATA_BROKER first).

### Manual Testing

Use mock sensor CLIs to set up preconditions, then use a gRPC client (e.g., `grpcurl` or a test script) to write a command to `Vehicle.Command.Door.Lock` and observe the results in DATA_BROKER.

```bash
# Set preconditions
speed-sensor --speed 0.0
door-sensor --open false

# Write a lock command to DATA_BROKER (via grpcurl or kuksa-client CLI)
# Observe Vehicle.Cabin.Door.Row1.DriverSide.IsLocked becomes true
# Observe Vehicle.Command.Door.Response contains success
```
