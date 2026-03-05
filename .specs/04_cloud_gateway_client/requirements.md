# Requirements: CLOUD_GATEWAY_CLIENT (Spec 04)

> EARS-syntax requirements for the CLOUD_GATEWAY_CLIENT component.
> Derived from the PRD at `.specs/04_cloud_gateway_client/prd.md` and the master PRD at `.specs/prd.md`.

## Notation

Requirements use the EARS (Easy Approach to Requirements Syntax) patterns:

- **Ubiquitous:** `The <system> shall <action>.`
- **Event-driven:** `When <trigger>, the <system> shall <action>.`
- **State-driven:** `While <state>, the <system> shall <action>.`
- **Unwanted behavior:** `If <condition>, then the <system> shall <action>.`
- **Option:** `Where <feature>, the <system> shall <action>.`

## References

- Master PRD: `.specs/prd.md`
- Component PRD: `.specs/04_cloud_gateway_client/prd.md`
- Design: `.specs/04_cloud_gateway_client/design.md`

## Terminology

| Term | Definition |
|------|-----------|
| CLOUD_GATEWAY_CLIENT | Rust service running in the RHIVOS safety partition that bridges DATA_BROKER and CLOUD_GATEWAY via NATS |
| DATA_BROKER | Eclipse Kuksa Databroker instance deployed in the RHIVOS safety partition |
| CLOUD_GATEWAY | Cloud-based service that relays commands and telemetry between COMPANION_APPs and vehicles via NATS |
| NATS | Lightweight messaging system used for vehicle-to-cloud communication |
| VIN | Vehicle Identification Number, used to namespace NATS subjects |
| UDS | Unix Domain Socket, used for same-partition gRPC communication with DATA_BROKER |

## Requirements

### 04-REQ-1.1: NATS Connection on Startup

When the CLOUD_GATEWAY_CLIENT starts, it shall connect to the NATS server at the URL specified by the `NATS_URL` environment variable and subscribe to the subject `vehicles.{VIN}.commands`, where `{VIN}` is the vehicle identity configured via the `VIN` environment variable.

**Rationale:** The CLOUD_GATEWAY_CLIENT must establish connectivity to CLOUD_GATEWAY before it can receive commands or publish telemetry. Environment-variable-based configuration supports multiple virtual vehicles and different deployment environments.

**Acceptance criteria:**
- CLOUD_GATEWAY_CLIENT connects to the NATS server specified by `NATS_URL` (default: `nats://localhost:4222`).
- CLOUD_GATEWAY_CLIENT subscribes to `vehicles.{VIN}.commands` using the configured VIN.
- CLOUD_GATEWAY_CLIENT logs a message confirming successful connection and subscription.
- If `VIN` is not set, the service shall exit with a non-zero exit code and a descriptive error message.

---

### 04-REQ-1.2: NATS Reconnection

If the NATS connection is lost, the CLOUD_GATEWAY_CLIENT shall attempt to reconnect using the async-nats built-in reconnection mechanism and shall re-subscribe to all subjects once reconnected.

**Rationale:** Network interruptions between the vehicle and cloud are expected in real-world operation. The client must recover automatically without manual intervention.

**Acceptance criteria:**
- When the NATS server becomes unreachable, the client does not crash or exit.
- When the NATS server becomes reachable again, the client re-establishes the connection automatically.
- After reconnection, the client resumes receiving messages on `vehicles.{VIN}.commands`.
- Reconnection attempts are logged.

---

### 04-REQ-2.1: Command Reception and Validation

When a message is received on `vehicles.{VIN}.commands`, the CLOUD_GATEWAY_CLIENT shall parse the JSON payload and validate that it contains the required fields (`command_id`, `action`, `doors`, `source`, `vin`, `timestamp`) before writing the command to DATA_BROKER.

**Rationale:** Commands arriving from the cloud must be structurally validated before being forwarded into the safety-critical DATA_BROKER, preventing malformed data from reaching the LOCKING_SERVICE.

**Acceptance criteria:**
- Valid command JSON is parsed and all required fields are present.
- The `action` field contains either `"lock"` or `"unlock"`.
- Valid commands are written to `Vehicle.Command.Door.Lock` on DATA_BROKER as a JSON string via gRPC over UDS.
- The `command_id` from the incoming command is preserved in the DATA_BROKER write.

---

### 04-REQ-2.2: Malformed Command Rejection

If the CLOUD_GATEWAY_CLIENT receives a message on `vehicles.{VIN}.commands` whose JSON payload cannot be parsed or is missing required fields, the CLOUD_GATEWAY_CLIENT shall log a warning with details of the validation failure and discard the message without writing to DATA_BROKER.

**Rationale:** Malformed commands must not propagate to the safety partition. Logging provides observability for debugging while discarding prevents undefined behavior in downstream services.

**Acceptance criteria:**
- Invalid JSON does not cause a panic or service crash.
- Messages with missing required fields are not written to DATA_BROKER.
- A warning-level log entry is emitted for each rejected message, including the reason for rejection.
- The service continues processing subsequent messages after rejecting a malformed one.

---

### 04-REQ-3.1: Command Response Relay

When `Vehicle.Command.Door.Response` changes on DATA_BROKER, the CLOUD_GATEWAY_CLIENT shall read the response JSON and publish it to the NATS subject `vehicles.{VIN}.command_responses`.

**Rationale:** The COMPANION_APP expects command execution results relayed through CLOUD_GATEWAY. The CLOUD_GATEWAY_CLIENT bridges the DATA_BROKER response signal to the NATS subject so the result reaches the user.

**Acceptance criteria:**
- CLOUD_GATEWAY_CLIENT subscribes to `Vehicle.Command.Door.Response` on DATA_BROKER via gRPC over UDS.
- When a response signal is received, the JSON payload is published to `vehicles.{VIN}.command_responses` on NATS.
- The `command_id` and `status` fields from the DATA_BROKER response are preserved in the NATS message.

---

### 04-REQ-4.1: Telemetry Publishing

While the CLOUD_GATEWAY_CLIENT is connected to both NATS and DATA_BROKER, it shall subscribe to vehicle state signals on DATA_BROKER (`Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`, `Vehicle.CurrentLocation.Latitude`, `Vehicle.CurrentLocation.Longitude`, `Vehicle.Parking.SessionActive`) and publish state changes as JSON to the NATS subject `vehicles.{VIN}.telemetry`.

**Rationale:** Cloud services and the COMPANION_APP need visibility into the vehicle's current state (lock status, location, parking state) for fleet monitoring and user notifications.

**Acceptance criteria:**
- CLOUD_GATEWAY_CLIENT subscribes to the listed DATA_BROKER signals on startup.
- When any subscribed signal value changes, a JSON telemetry message is published to `vehicles.{VIN}.telemetry`.
- The telemetry message includes the signal name, value, and a timestamp.
- Telemetry is only published when signal values actually change (not on a periodic schedule).

---

### 04-REQ-5.1: DATA_BROKER Connectivity

When the CLOUD_GATEWAY_CLIENT starts, it shall connect to DATA_BROKER via gRPC over UDS at the socket path specified by the `DATABROKER_UDS_PATH` environment variable (default: the Kuksa UDS path configured in the infrastructure).

**Rationale:** Same-partition communication uses UDS for performance and isolation. The connection must be established before command processing or telemetry publishing can begin.

**Acceptance criteria:**
- CLOUD_GATEWAY_CLIENT connects to DATA_BROKER via gRPC over UDS on startup.
- If DATA_BROKER is unreachable at startup, the service retries connection with backoff and logs each failed attempt.
- If DATA_BROKER becomes unreachable during operation, the service logs an error and retries until the connection is restored.
- Commands received via NATS while DATA_BROKER is unreachable are logged and discarded (not silently lost).

## Edge Cases

| ID | Scenario | Expected Behavior |
|----|----------|-------------------|
| 04-EDGE-1 | NATS connection lost during operation | Client uses async-nats reconnection; no crash; resumes processing after reconnect |
| 04-EDGE-2 | Malformed JSON received on command subject | Warning logged; message discarded; service continues |
| 04-EDGE-3 | Command with valid JSON but unknown `action` value | Message rejected; warning logged; not written to DATA_BROKER |
| 04-EDGE-4 | DATA_BROKER unreachable when command arrives via NATS | Command logged and discarded; error logged; service does not crash |
| 04-EDGE-5 | DATA_BROKER unreachable at startup | Service retries with backoff; logs each failed attempt |
| 04-EDGE-6 | VIN environment variable not set | Service exits immediately with non-zero code and descriptive error |
| 04-EDGE-7 | Multiple rapid commands arrive on NATS | All valid commands are processed in order; none are dropped |

## Traceability

| Requirement | PRD Section |
|-------------|-------------|
| 04-REQ-1.1 | NATS connectivity, Vehicle Identity |
| 04-REQ-1.2 | NATS connectivity (resilience) |
| 04-REQ-2.1 | Command reception, Command Payload Format |
| 04-REQ-2.2 | Command reception (error handling) |
| 04-REQ-3.1 | Command response relay |
| 04-REQ-4.1 | Telemetry publishing |
| 04-REQ-5.1 | DATA_BROKER communication via gRPC over UDS |
