# Requirements: LOCKING_SERVICE (Spec 03)

> EARS-syntax requirements for the LOCKING_SERVICE and mock sensor CLI tools.
> Derived from the PRD at `.specs/03_locking_service/prd.md` and the master PRD at `.specs/prd.md`.

## Notation

Requirements use the EARS (Easy Approach to Requirements Syntax) patterns:

- **Ubiquitous:** `The <system> shall <action>.`
- **Event-driven:** `When <trigger>, the <system> shall <action>.`
- **State-driven:** `While <state>, the <system> shall <action>.`
- **Unwanted behavior:** `If <condition>, then the <system> shall <action>.`
- **Option:** `Where <feature>, the <system> shall <action>.`

## LOCKING_SERVICE Requirements

### 03-REQ-1.1: Command Signal Subscription

When the LOCKING_SERVICE starts, the LOCKING_SERVICE shall subscribe to the `Vehicle.Command.Door.Lock` signal on DATA_BROKER via gRPC over UDS and continuously process incoming command messages.

**Rationale:** The LOCKING_SERVICE receives remote lock/unlock requests published by CLOUD_GATEWAY_CLIENT through DATA_BROKER, following the indirect pub/sub architecture (no direct service calls).

**Acceptance criteria:**
- LOCKING_SERVICE establishes a gRPC subscription to `Vehicle.Command.Door.Lock` on startup.
- LOCKING_SERVICE processes each incoming command message containing a JSON payload with fields: `command_id`, `action`, `doors`, `source`, `vin`, `timestamp`.

---

### 03-REQ-2.1: Safety Validation — Vehicle Must Be Stationary

When a lock or unlock command is received, the LOCKING_SERVICE shall read `Vehicle.Speed` from DATA_BROKER and reject the command if the vehicle speed is greater than 0.5 m/s.

**Rationale:** Locking or unlocking doors while the vehicle is moving is a safety hazard. A near-zero threshold (0.5 m/s) accounts for sensor noise while the vehicle is effectively stationary.

**Acceptance criteria:**
- A lock command is rejected with reason `"vehicle_moving"` when `Vehicle.Speed > 0.5`.
- An unlock command is rejected with reason `"vehicle_moving"` when `Vehicle.Speed > 0.5`.
- A command is accepted when `Vehicle.Speed <= 0.5`.

---

### 03-REQ-2.2: Safety Validation — Door Must Not Be Ajar to Lock

When a lock command is received and `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` is `true`, the LOCKING_SERVICE shall reject the lock command with reason `"door_ajar"`.

**Rationale:** Locking a door that is physically open is mechanically invalid and could damage the locking mechanism.

**Acceptance criteria:**
- A lock command is rejected with reason `"door_ajar"` when `IsOpen == true`.
- A lock command is accepted when `IsOpen == false` (and all other constraints pass).
- An unlock command is accepted regardless of the `IsOpen` value (no door-ajar constraint for unlock).

---

### 03-REQ-3.1: Lock State Publication

When the LOCKING_SERVICE successfully executes a lock or unlock command, the LOCKING_SERVICE shall write the resulting lock state to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` on DATA_BROKER via gRPC over UDS.

**Rationale:** Downstream consumers (PARKING_OPERATOR_ADAPTOR, PARKING_APP, CLOUD_GATEWAY_CLIENT) depend on the lock state signal to trigger parking session lifecycle events and report vehicle status.

**Acceptance criteria:**
- After a successful lock command, `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` is set to `true`.
- After a successful unlock command, `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` is set to `false`.
- The lock state is not modified when a command is rejected.

---

### 03-REQ-4.1: Command Response Publication

When the LOCKING_SERVICE finishes processing a command (whether successful or rejected), the LOCKING_SERVICE shall write a JSON response to `Vehicle.Command.Door.Response` on DATA_BROKER containing `command_id`, `status` (`"success"` or `"failed"`), an optional `reason`, and a `timestamp`.

**Rationale:** CLOUD_GATEWAY_CLIENT observes command responses via DATA_BROKER and relays results back to the COMPANION_APP through CLOUD_GATEWAY.

**Acceptance criteria:**
- A success response is published: `{"command_id": "<uuid>", "status": "success", "timestamp": <unix_ts>}`.
- A failure response is published: `{"command_id": "<uuid>", "status": "failed", "reason": "<reason>", "timestamp": <unix_ts>}`.
- The `command_id` in the response matches the `command_id` from the originating command.

---

### 03-REQ-4.2: Invalid Command Handling

If the LOCKING_SERVICE receives a command signal whose JSON payload cannot be parsed or is missing required fields (`command_id`, `action`), the LOCKING_SERVICE shall log a warning and, if a `command_id` is extractable, publish a failure response with reason `"invalid_command"`.

**Rationale:** Malformed commands must not cause the service to crash or enter an undefined state. Graceful rejection ensures system resilience.

**Acceptance criteria:**
- Malformed JSON does not cause a panic or service crash.
- If `command_id` can be extracted, a failure response with reason `"invalid_command"` is published.
- If `command_id` cannot be extracted, the error is logged but no response is published (no valid command to correlate).

---

### 03-REQ-4.3: Concurrent Command Serialization

While the LOCKING_SERVICE is processing a command, if another command arrives, the LOCKING_SERVICE shall queue the incoming command and process it after the current command completes.

**Rationale:** Concurrent lock/unlock operations on the same door could produce inconsistent state. Serializing commands ensures each command sees a consistent view of vehicle state.

**Acceptance criteria:**
- Commands received during processing of another command are not dropped.
- Commands are processed in arrival order (FIFO).
- Each command reads fresh signal values from DATA_BROKER at the time it begins processing.

---

## Mock Sensor Requirements

### 03-REQ-5.1: LOCATION_SENSOR CLI

The LOCATION_SENSOR CLI shall accept latitude and longitude values as command-line arguments and write them to `Vehicle.CurrentLocation.Latitude` (double) and `Vehicle.CurrentLocation.Longitude` (double) on DATA_BROKER via gRPC.

**Rationale:** Mock location data is needed for integration testing of location-dependent features (parking operator discovery) without real GPS hardware.

**Acceptance criteria:**
- Usage: `location-sensor --latitude <f64> --longitude <f64>`.
- Both signals are written to DATA_BROKER in a single invocation.
- The CLI exits with code 0 on success and non-zero on failure (e.g., DATA_BROKER unreachable).

---

### 03-REQ-5.2: SPEED_SENSOR CLI

The SPEED_SENSOR CLI shall accept a speed value as a command-line argument and write it to `Vehicle.Speed` (float) on DATA_BROKER via gRPC.

**Rationale:** Mock speed data is needed for integration testing of the LOCKING_SERVICE safety constraint that requires the vehicle to be stationary.

**Acceptance criteria:**
- Usage: `speed-sensor --speed <f32>`.
- The signal is written to DATA_BROKER.
- The CLI exits with code 0 on success and non-zero on failure.

---

### 03-REQ-5.3: DOOR_SENSOR CLI

The DOOR_SENSOR CLI shall accept a boolean open/closed value as a command-line argument and write it to `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` (bool) on DATA_BROKER via gRPC.

**Rationale:** Mock door ajar data is needed for integration testing of the LOCKING_SERVICE safety constraint that prevents locking an open door.

**Acceptance criteria:**
- Usage: `door-sensor --open <true|false>`.
- The signal is written to DATA_BROKER.
- The CLI exits with code 0 on success and non-zero on failure.
