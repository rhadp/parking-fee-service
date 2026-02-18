# PRD: CLOUD_GATEWAY + CLOUD_GATEWAY_CLIENT + Mock COMPANION_APP (Phase 2.2)

> Extracted from the [main PRD](../prd.md). This spec covers Phase 2.2:
> vehicle-to-cloud connectivity via MQTT, the CLOUD_GATEWAY REST API, the
> CLOUD_GATEWAY_CLIENT Rust service, and the mock COMPANION_APP CLI.

## Scope

From the main PRD, Phase 2.2:

- Implement CLOUD_GATEWAY (Go): REST API for companion apps, MQTT client for
  vehicle communication, vehicle state management, and pairing.
- Implement CLOUD_GATEWAY_CLIENT (Rust): MQTT client connecting to Mosquitto,
  receives commands, writes to DATA_BROKER, subscribes to results, publishes
  telemetry.
- Implement mock COMPANION_APP CLI (Go): real REST client replacing the spec 01
  skeleton.
- Integration test of bi-directional CLOUD_GATEWAY ↔ CLOUD_GATEWAY_CLIENT
  communication.

### Components in scope

| Component | Work | Language |
|-----------|------|----------|
| CLOUD_GATEWAY | Full implementation | Go |
| CLOUD_GATEWAY_CLIENT | Full implementation | Rust |
| Mock COMPANION_APP CLI | Real implementation (replace spec 01 skeleton) | Go |

### End-to-end command flow (corrected from PRD Flow 2)

```
COMPANION_APP                CLOUD_GATEWAY              Mosquitto
     │                            │                        │
     │ POST /vehicles/{vin}/lock  │                        │
     │ ────────────────────────►  │                        │
     │  ◄─── 202 {cmd_id}        │                        │
     │                            │ PUBLISH commands (QoS2)│
     │                            │ ──────────────────────►│
     │                            │                        │
                                                           │
     CLOUD_GATEWAY_CLIENT        DATA_BROKER (Kuksa)       │
           │                        │                      │
           │ ◄── MQTT command ──────┤                      │
           │                        │                      │
           │ Set(Command.Door.Lock) │                      │
           │ ──────────────────────►│                      │
           │                        │──► LOCKING_SERVICE   │
           │                        │    validates & writes │
           │                        │    IsLocked + Result  │
           │                        │◄──                    │
           │ Subscribe(LockResult)  │                      │
           │ ◄─────────────────────┤                      │
           │                        │                      │
           │ PUBLISH cmd_response ──────────────────────►  │
           │                        │                      │
                                                           │
     COMPANION_APP                CLOUD_GATEWAY            │
     │                            │ ◄── MQTT response ────┤
     │                            │ (updates vehicle state)│
     │ GET /vehicles/{vin}/status │                        │
     │ ────────────────────────►  │                        │
     │  ◄─── 200 {state}         │                        │
```

### REST-MQTT decoupling

The REST interface and MQTT communication are **decoupled**. The VIN serves
as a correlation ID to map between both sides within CLOUD_GATEWAY:

- `POST /lock` or `POST /unlock` returns immediately with `202 Accepted` and
  a `command_id`. It does NOT block on the MQTT round-trip.
- The MQTT response arrives asynchronously and updates the vehicle's internal
  state in CLOUD_GATEWAY.
- `GET /status` returns the latest known vehicle state (from cached telemetry
  or status responses).

### Vehicle pairing

A basic VIN-to-companion-app pairing mechanism keeps the implementation
realistic:

1. CLOUD_GATEWAY_CLIENT generates a VIN and a 6-digit pairing PIN on first
   startup. Both are logged and persisted.
2. CLOUD_GATEWAY_CLIENT registers with CLOUD_GATEWAY via MQTT.
3. COMPANION_APP pairs by calling `POST /api/v1/pair` with `{vin, pin}` and
   receives a bearer token.
4. All subsequent REST commands require the bearer token.
5. CLOUD_GATEWAY validates the token against its pairing database.

### Telemetry

CLOUD_GATEWAY_CLIENT periodically reads vehicle signals from DATA_BROKER and
publishes them as telemetry to CLOUD_GATEWAY via MQTT (QoS 0). This provides
ongoing state updates without explicit requests.

For on-demand status queries, CLOUD_GATEWAY uses a request-response pattern
via MQTT (QoS 2), where CLOUD_GATEWAY_CLIENT reads current DATA_BROKER state
and responds immediately.

### MQTT QoS

- **QoS 2 (exactly-once):** commands, command responses, status
  requests/responses, registration.
- **QoS 0 (fire-and-forget):** telemetry data.

## Dependencies

| Spec | Relationship | Notes |
|------|-------------|-------|
| 01_repo_setup | Depends on | Rust workspace, Go modules, Mosquitto infra, mock CLI skeletons |
| 02_locking_service | Depends on | Kuksa config (custom signals), Kuksa client library, LOCKING_SERVICE (processes commands written by CLOUD_GATEWAY_CLIENT) |

## Clarifications

### Architecture

- **A1 (REST-MQTT decoupling):** REST and MQTT are decoupled. VIN is the
  correlation ID. Lock/unlock endpoints return immediately with 202 Accepted.
  The MQTT response updates internal state asynchronously.

- **A2 (Status mechanism):** GET /status returns cached state from telemetry.
  If stale or unavailable, CLOUD_GATEWAY triggers an MQTT status
  request-response to CLOUD_GATEWAY_CLIENT.

- **A3 (Result forwarding):** CLOUD_GATEWAY_CLIENT subscribes to
  `Vehicle.Command.Door.LockResult` on DATA_BROKER and forwards changes to
  CLOUD_GATEWAY via MQTT `command_responses` topic.

### Implementation

- **U1 (MQTT message schemas):** Defined in the design document.

- **U2 (MQTT topics):** `vehicles/{vin}/commands`, `commands_responses`,
  `status_request`, `status_response`, `telemetry`, `registration`.

- **U3 (Token validation):** Realistic pairing: COMPANION_APP must be paired
  with a VIN via PIN. CLOUD_GATEWAY generates and validates bearer tokens per
  pairing.

- **U4 (VIN creation):** Generated on first startup of CLOUD_GATEWAY_CLIENT.
  Persisted to a configurable data directory. Pairing PIN generated alongside.

- **U5 (Telemetry):** Included in this spec. Periodic publishing of vehicle
  signals (location, speed, door state, lock state, parking session).

- **U6 (MQTT QoS):** QoS 2 for commands/responses. QoS 0 for telemetry.
