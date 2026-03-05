# Design: CLOUD_GATEWAY_CLIENT (Spec 04)

> Design document for the CLOUD_GATEWAY_CLIENT component.
> Implements requirements from `.specs/04_cloud_gateway_client/requirements.md`.

## References

- Master PRD: `.specs/prd.md`
- Component PRD: `.specs/04_cloud_gateway_client/prd.md`
- Requirements: `.specs/04_cloud_gateway_client/requirements.md`

## Architecture Overview

The CLOUD_GATEWAY_CLIENT is a Rust service running in the RHIVOS safety partition. It bridges the vehicle's DATA_BROKER (Eclipse Kuksa Databroker) with the cloud-based CLOUD_GATEWAY via NATS messaging. The service operates three concurrent pipelines:

1. **Command pipeline (inbound):** NATS -> validate -> DATA_BROKER
2. **Response relay pipeline (outbound):** DATA_BROKER -> NATS
3. **Telemetry pipeline (outbound):** DATA_BROKER -> NATS

```
                    NATS Server (CLOUD_GATEWAY)
                         |           ^           ^
          subscribe      |           | publish   | publish
   vehicles.{VIN}.commands   vehicles.{VIN}. vehicles.{VIN}.
                         |     command_responses  telemetry
                         v           |           |
                  +------+-----------+-----------+------+
                  |       CLOUD_GATEWAY_CLIENT          |
                  |                                     |
                  |  +------------+  +--------------+   |
                  |  | Command    |  | Response     |   |
                  |  | Processor  |  | Relay        |   |
                  |  +-----+------+  +------+-------+   |
                  |        |                |            |
                  |        v                ^            |
                  |  +-----+----------------+-------+   |
                  |  |     DATA_BROKER (gRPC/UDS)   |   |
                  |  +-----+------------------------+   |
                  |        ^                            |
                  |  +-----+-------+                    |
                  |  | Telemetry   |                    |
                  |  | Publisher   |                    |
                  |  +-------------+                    |
                  +-------------------------------------+
```

## Technology Stack

| Component | Technology | Purpose |
|-----------|-----------|---------|
| Language | Rust (edition 2021) | Safety-critical service implementation |
| Async runtime | tokio | Async I/O, task spawning, timers |
| NATS client | async-nats | Connection to CLOUD_GATEWAY |
| gRPC client | tonic | Connection to DATA_BROKER via UDS |
| JSON handling | serde, serde_json | Command parsing and telemetry serialization |
| Logging | tracing | Structured logging |
| UUID | uuid | Command ID validation |

## Module Structure

The crate lives at `rhivos/cloud-gateway-client/` within the Rust workspace.

```
rhivos/cloud-gateway-client/
  Cargo.toml
  src/
    main.rs              # Entry point: config loading, task spawning
    config.rs            # Environment variable parsing (VIN, NATS_URL, DATABROKER_UDS_PATH)
    nats_client.rs       # NATS connection management, subscribe/publish
    databroker_client.rs # DATA_BROKER gRPC client (tonic over UDS)
    command.rs           # Command data types, validation logic
    command_processor.rs # Inbound pipeline: NATS -> validate -> DATA_BROKER
    response_relay.rs    # Outbound pipeline: DATA_BROKER response -> NATS
    telemetry.rs         # Outbound pipeline: DATA_BROKER signals -> NATS telemetry
```

## Configuration

All configuration is via environment variables:

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `VIN` | Yes | (none) | Vehicle Identification Number; used in NATS subject hierarchy |
| `NATS_URL` | No | `nats://localhost:4222` | NATS server connection URL |
| `DATABROKER_UDS_PATH` | No | `/tmp/kuksa/databroker.sock` | Unix Domain Socket path for DATA_BROKER gRPC |

If `VIN` is not set, the service exits immediately with exit code 1 and a descriptive error message.

## NATS Subject Model

| Subject | Direction | Purpose |
|---------|-----------|---------|
| `vehicles.{VIN}.commands` | Subscribe (inbound) | Receive lock/unlock commands from CLOUD_GATEWAY |
| `vehicles.{VIN}.command_responses` | Publish (outbound) | Relay command execution results to CLOUD_GATEWAY |
| `vehicles.{VIN}.telemetry` | Publish (outbound) | Publish vehicle state updates to CLOUD_GATEWAY |

The `{VIN}` token is replaced with the value of the `VIN` environment variable at startup.

## Command Processing Pipeline

### Inbound: NATS -> Validate -> DATA_BROKER

1. A JSON message arrives on `vehicles.{VIN}.commands`.
2. The `command_processor` deserializes the payload into a `Command` struct.
3. Validation checks:
   - All required fields present: `command_id`, `action`, `doors`, `source`, `vin`, `timestamp`.
   - `action` is one of `"lock"` or `"unlock"`.
   - `command_id` is a valid UUID string.
4. If validation fails, the message is discarded and a warning is logged.
5. If validation passes, the command JSON is written to `Vehicle.Command.Door.Lock` on DATA_BROKER via gRPC `SetRequest`.

### Command Payload Format

```json
{
  "command_id": "<uuid>",
  "action": "lock",
  "doors": ["driver"],
  "source": "companion_app",
  "vin": "WVWZZZ3CZWE123456",
  "timestamp": 1700000000
}
```

## Response Relay Pipeline

### Outbound: DATA_BROKER -> NATS

1. The `response_relay` module subscribes to `Vehicle.Command.Door.Response` on DATA_BROKER via gRPC `SubscribeRequest`.
2. When a response signal update is received, the JSON string value is read.
3. The response JSON is published to `vehicles.{VIN}.command_responses` on NATS.

### Response Payload Format

```json
{
  "command_id": "<uuid>",
  "status": "success",
  "timestamp": 1700000001
}
```

Or on failure:

```json
{
  "command_id": "<uuid>",
  "status": "failed",
  "reason": "vehicle_moving",
  "timestamp": 1700000001
}
```

## Telemetry Pipeline

### Outbound: DATA_BROKER -> NATS

1. The `telemetry` module subscribes to the following DATA_BROKER signals via gRPC `SubscribeRequest`:
   - `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` (bool)
   - `Vehicle.CurrentLocation.Latitude` (double)
   - `Vehicle.CurrentLocation.Longitude` (double)
   - `Vehicle.Parking.SessionActive` (bool)
2. When any subscribed signal value changes, a telemetry JSON message is constructed and published to `vehicles.{VIN}.telemetry` on NATS.

### Telemetry Payload Format

```json
{
  "vin": "WVWZZZ3CZWE123456",
  "signal": "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
  "value": true,
  "timestamp": 1700000002
}
```

## Connection Management

### NATS Connection

- The async-nats client is created with the configured `NATS_URL`.
- async-nats provides built-in reconnection with backoff. The client leverages this default behavior.
- On successful connection and reconnection, the client logs the event.
- The command subscription is re-established automatically by async-nats after reconnection.

### DATA_BROKER Connection

- A tonic gRPC client connects to DATA_BROKER via Unix Domain Socket.
- If DATA_BROKER is unreachable at startup, the service retries with exponential backoff (1s, 2s, 4s, ..., max 30s).
- If DATA_BROKER becomes unreachable during operation, subscription streams will terminate; the service detects the broken stream, logs an error, and retries the connection and subscriptions.
- Commands received via NATS while DATA_BROKER is unreachable are logged and discarded.

## Startup Sequence

1. Parse environment variables (`VIN`, `NATS_URL`, `DATABROKER_UDS_PATH`). Exit if `VIN` is missing.
2. Connect to NATS server. Log success or retry on failure.
3. Connect to DATA_BROKER via gRPC/UDS. Retry with backoff if unreachable.
4. Spawn three concurrent tokio tasks:
   - `command_processor`: subscribes to NATS commands, validates, writes to DATA_BROKER.
   - `response_relay`: subscribes to DATA_BROKER `Vehicle.Command.Door.Response`, publishes to NATS.
   - `telemetry`: subscribes to DATA_BROKER telemetry signals, publishes to NATS.
5. Log "CLOUD_GATEWAY_CLIENT started for VIN={VIN}".
6. Await all tasks; if any task exits with an error, log and attempt restart of the failed task.

## Correctness Properties

| ID | Property | Description |
|----|----------|-------------|
| CP-1 | Command relay integrity | Every valid command received on NATS is written to DATA_BROKER exactly once with all fields preserved. No valid command is silently dropped. |
| CP-2 | Malformed command isolation | Malformed or invalid commands are never written to DATA_BROKER. They are logged and discarded. |
| CP-3 | Response relay completeness | Every command response that appears on `Vehicle.Command.Door.Response` in DATA_BROKER is published to `vehicles.{VIN}.command_responses` on NATS. |
| CP-4 | Telemetry accuracy | Telemetry messages published to NATS accurately reflect the current signal values from DATA_BROKER. No stale or fabricated values are published. |
| CP-5 | VIN isolation | The client only subscribes to and publishes on NATS subjects scoped to its configured VIN. It never reads or writes subjects belonging to other VINs. |
| CP-6 | NATS reconnection resilience | A NATS disconnection does not crash the service. The service automatically reconnects and resumes all pipelines. |
| CP-7 | DATA_BROKER failure tolerance | DATA_BROKER unavailability does not crash the service. The service retries connections and discards inbound commands that cannot be forwarded. |

## Error Handling

| Error Condition | Behavior | Log Level |
|----------------|----------|-----------|
| `VIN` env var not set | Exit with code 1 and descriptive error | ERROR |
| NATS unreachable at startup | Retry with async-nats built-in reconnection | WARN |
| NATS connection lost during operation | Automatic reconnection via async-nats; log event | WARN |
| DATA_BROKER unreachable at startup | Retry with exponential backoff (max 30s) | WARN |
| DATA_BROKER connection lost during operation | Detect broken stream; reconnect and re-subscribe | ERROR |
| Malformed command JSON on NATS | Discard message; log validation failure details | WARN |
| Command with invalid `action` field | Discard message; log rejected action value | WARN |
| DATA_BROKER write fails for valid command | Log error; command is lost (no retry to avoid reordering) | ERROR |
| NATS publish fails for response/telemetry | Log error; message is lost (DATA_BROKER is source of truth) | ERROR |

## Testing Strategy

### Unit Tests

Unit tests cover individual modules in isolation:

- `config.rs`: Env var parsing, missing VIN, defaults.
- `command.rs`: Command struct validation (valid, missing fields, invalid action, malformed JSON).
- `telemetry.rs`: Telemetry message construction and serialization.

Run with: `cd rhivos && cargo test -p cloud-gateway-client`

### Integration Tests

Integration tests require running NATS and DATA_BROKER infrastructure:

- Full command pipeline: publish command on NATS -> verify signal written to DATA_BROKER.
- Full response relay: write response to DATA_BROKER -> verify published on NATS.
- Full telemetry pipeline: write signal to DATA_BROKER -> verify published on NATS.
- VIN scoping: verify messages only flow on the correct VIN subject.

Requires: `make infra-up` (starts NATS + Kuksa)

### Lint

Run with: `cd rhivos && cargo clippy -p cloud-gateway-client`

## Definition of Done

1. All requirements in `requirements.md` are implemented and covered by tests.
2. `cargo build -p cloud-gateway-client` compiles without errors or warnings.
3. `cargo clippy -p cloud-gateway-client` passes with no warnings.
4. `cargo test -p cloud-gateway-client` passes all unit tests.
5. Integration tests pass against running NATS + DATA_BROKER infrastructure.
6. The service starts successfully with `VIN` and default configuration, connects to NATS and DATA_BROKER, and processes commands end-to-end.
7. Structured logging is present for all connection events, command processing, and error conditions.
