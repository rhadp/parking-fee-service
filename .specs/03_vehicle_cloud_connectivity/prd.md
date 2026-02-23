# PRD: Vehicle-to-Cloud Connectivity (Phase 2.2)

> Extracted from the main PRD (`.specs/prd.md`) — Phase 2.2: Vehicle-to-Cloud
> Connectivity.

## Context

This specification covers the vehicle-to-cloud connectivity layer of the SDV
Parking Demo System. It implements the CLOUD_GATEWAY backend service — the
bridge between the mobile COMPANION_APP and the vehicle's CLOUD_GATEWAY_CLIENT.
The CLOUD_GATEWAY exposes a REST API for mobile applications and translates
those requests into MQTT messages for the vehicle side. It also upgrades the
mock COMPANION_APP CLI from a stub to a functional tool that exercises the
CLOUD_GATEWAY REST API end-to-end.

## Scope

From the main PRD, Phase 2.2:

- Implementation of the V2X connectivity:
  - **CLOUD_GATEWAY** (Go) — dual-interface cloud service:
    - REST API towards COMPANION_APPs: `POST /vehicles/{vin}/commands`
      (lock/unlock), `GET /vehicles/{vin}/status`, `GET /health`. Bearer token
      authentication.
    - MQTT client connecting to Eclipse Mosquitto broker: publishes commands to
      `vehicles/{vin}/commands` topic, subscribes to
      `vehicles/{vin}/telemetry` and `vehicles/{vin}/command_responses` topics.
    - Bridges REST requests to MQTT messages and MQTT responses back to REST
      responses.
  - **Mock COMPANION_APP CLI** — the skeleton mock CLI from Phase 1.2 gets real
    functionality: `lock`, `unlock`, `status` commands now actually call the
    CLOUD_GATEWAY REST API.
  - **Integration test** — end-to-end: COMPANION_APP CLI -> CLOUD_GATEWAY REST
    -> MQTT -> (simulated subscriber) and back.

## Key PRD Flows Covered

### Flow 2: Remote Unlock via Companion App (steps 1-2, 8-9)

This spec implements the cloud portion of Flow 2:

1. COMPANION_APP -> CLOUD_GATEWAY (REST POST)
   `POST /vehicles/VIN12345/commands`
   Headers: `Authorization: Bearer <token>`
   Body: `{"command_id": "<uuid>", "type": "unlock", "doors": ["driver"]}`

2. CLOUD_GATEWAY validates bearer token and translates to MQTT
   PUBLISH `vehicles/VIN12345/commands`
   `{"command_id": "<uuid>", "action": "unlock", "doors": ["driver"], "source": "companion_app"}`

   _(Steps 3-7 are handled by CLOUD_GATEWAY_CLIENT and LOCKING_SERVICE in
   specs 02 and later)_

8. CLOUD_GATEWAY receives MQTT response on `vehicles/VIN12345/command_responses`
   `{"command_id": "<uuid>", "status": "success"}`

9. CLOUD_GATEWAY -> COMPANION_APP (REST response)
   `200 OK {"command_id": "<uuid>", "status": "success"}`

## Components

### CLOUD_GATEWAY

- Cloud-based Go service with two interfaces:
  - **REST API** (HTTPS) towards COMPANION_APPs — receives lock/unlock
    commands, returns vehicle status
  - **MQTT client** towards Eclipse Mosquitto broker — relays commands and
    receives telemetry/responses
- Authenticates COMPANION_APPs using bearer tokens
- Routes commands between mobile apps and vehicles, translating between REST
  and MQTT protocols
- Uses containerized Eclipse Mosquitto for local development (already
  provisioned by spec 01)
- Each vehicle is identified by a unique VIN
- Commands are correlated using a `command_id` (UUID) that flows through the
  entire request-response cycle

### Mock COMPANION_APP CLI (enhanced)

- CLI commands `lock`, `unlock`, `status` call CLOUD_GATEWAY REST API
- Paired with a specific VIN (via `--vin` flag)
- Authenticates with bearer token (via `--token` flag)
- Generates `command_id` UUIDs for each command
- Displays command responses to the user

## Communication Protocols

| Source            | Target          | Protocol   | Direction          |
|-------------------|-----------------|------------|--------------------|
| COMPANION_APP CLI | CLOUD_GATEWAY   | HTTPS/REST | Request/Response   |
| CLOUD_GATEWAY     | MQTT Broker     | MQTT       | Bidirectional      |

## MQTT Topic Structure

| Topic                                 | Publisher         | Subscriber(s)          | Payload                          |
|---------------------------------------|-------------------|------------------------|----------------------------------|
| `vehicles/{vin}/commands`             | CLOUD_GATEWAY     | CLOUD_GATEWAY_CLIENT   | Lock/unlock command JSON         |
| `vehicles/{vin}/command_responses`    | CLOUD_GATEWAY_CLIENT | CLOUD_GATEWAY       | Command result JSON              |
| `vehicles/{vin}/telemetry`            | CLOUD_GATEWAY_CLIENT | CLOUD_GATEWAY       | Vehicle state JSON               |

## REST API

| Method | Path                          | Auth          | Request Body                    | Response                         |
|--------|-------------------------------|---------------|--------------------------------|----------------------------------|
| POST   | `/vehicles/{vin}/commands`    | Bearer token  | `{"command_id","type","doors"}` | `{"command_id","status"}`        |
| GET    | `/vehicles/{vin}/status`      | Bearer token  | —                              | `{"vin","locked","timestamp"}`   |
| GET    | `/health`                     | None          | —                              | `{"status":"ok"}`                |

## Relevant PRD Clarifications

- **A1:** COMPANION_APP uses REST exclusively. CLOUD_GATEWAY handles protocol
  translation between REST and MQTT.
- **I1:** Confirmed: COMPANION_APP uses REST towards CLOUD_GATEWAY.
- **IA2:** Containerized Eclipse Mosquitto for local dev.
- **IA6:** Self-created VINs. COMPANION_APP is paired with a specific VIN.
- **U4:** Bearer tokens for the demo.

## Out-of-Scope for This Spec

- CLOUD_GATEWAY_CLIENT implementation (spec 02_rhivos_safety)
- LOCKING_SERVICE or DATA_BROKER interactions (spec 02_rhivos_safety)
- Real TLS/mTLS configuration (demo uses plaintext locally)
- Token validation beyond string matching (no JWT, no OIDC)
- Fleet telemetry aggregation (optional future phase)
- COMPANION_APP mobile application (spec 06, Phase 2.6)
- Vehicle status data beyond what is received via MQTT telemetry

## Dependencies

| Spec | Relationship | Notes |
|------|-------------|-------|
| 01_project_setup | Depends on | Uses Go module skeleton (`backend/cloud-gateway/`), mock CLI skeleton (`mock/companion-app-cli/`), local infra (Mosquitto on :1883), Makefile, Go workspace |
| 02_rhivos_safety | Integrates with | CLOUD_GATEWAY_CLIENT subscribes to MQTT topics published by CLOUD_GATEWAY; publishes to topics CLOUD_GATEWAY subscribes to. Integration test simulates CLOUD_GATEWAY_CLIENT behavior without depending on spec 02 implementation. |
