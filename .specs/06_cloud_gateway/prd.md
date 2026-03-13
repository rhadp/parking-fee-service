# PRD: CLOUD_GATEWAY (Phase 2.2)

> Extracted from the master PRD at `.specs/prd.md`. This spec covers Phase 2.2: Vehicle-to-Cloud Connectivity.

## Scope

Implement the CLOUD_GATEWAY as a cloud-based Go service with two interfaces: a REST API towards COMPANION_APPs (HTTPS) for receiving lock/unlock commands and returning command status, and a NATS interface towards vehicles (CLOUD_GATEWAY_CLIENT) for relaying commands and receiving telemetry.

## Component Description

- Cloud-based service with two interfaces:
  - **REST API** towards COMPANION_APPs (HTTPS) -- receives lock/unlock commands, returns command status
  - **NATS** towards vehicles (CLOUD_GATEWAY_CLIENT) -- relays commands and receives telemetry
- Authenticates vehicles and COMPANION_APPs using bearer tokens
- Routes commands between mobile apps and vehicles, translating between REST and NATS protocols
- Supports multiple vehicles simultaneously (each identified by VIN)
- COMPANION_APP is paired with a specific VIN
- Deployed on Google Cloud infrastructure
- Uses containerized NATS server (nats-server) for local development

## REST API Endpoints

- `POST /vehicles/{vin}/commands` -- submit a lock/unlock command for a specific vehicle
- `GET /vehicles/{vin}/commands/{command_id}` -- query command status / get command response
- `GET /health` -- health check

## NATS Subject Hierarchy

- `vehicles.{vin}.commands` -- commands sent from CLOUD_GATEWAY to CLOUD_GATEWAY_CLIENT
- `vehicles.{vin}.command_responses` -- command responses sent from CLOUD_GATEWAY_CLIENT to CLOUD_GATEWAY
- `vehicles.{vin}.telemetry` -- telemetry data published by CLOUD_GATEWAY_CLIENT

## Command Flow

1. COMPANION_APP sends `POST /vehicles/{vin}/commands` with bearer token and command payload
2. CLOUD_GATEWAY validates the bearer token and VIN
3. CLOUD_GATEWAY publishes the command to NATS subject `vehicles.{vin}.commands`
4. CLOUD_GATEWAY_CLIENT receives the command via NATS subscription
5. CLOUD_GATEWAY_CLIENT writes the command to DATA_BROKER, LOCKING_SERVICE executes it
6. LOCKING_SERVICE writes the response to DATA_BROKER, CLOUD_GATEWAY_CLIENT observes it
7. CLOUD_GATEWAY_CLIENT publishes the response to NATS subject `vehicles.{vin}.command_responses`
8. CLOUD_GATEWAY receives the response and stores it
9. COMPANION_APP queries `GET /vehicles/{vin}/commands/{command_id}` to retrieve the result

## Command Payload

```json
{
  "command_id": "<uuid>",
  "type": "lock" | "unlock",
  "doors": ["driver"]
}
```

## Command Response Payload (from NATS)

```json
{
  "command_id": "<uuid>",
  "status": "success" | "failed",
  "reason": "<optional>"
}
```

## Authentication

- Bearer tokens for the demo
- Each COMPANION_APP token is associated with a specific VIN
- Tokens are validated on every REST request
- Invalid tokens receive HTTP 401

## Tech Stack

- Language: Go 1.22+
- HTTP framework: net/http standard library (Go 1.22 ServeMux)
- NATS client: nats.go (github.com/nats-io/nats.go)
- Local NATS: containerized nats-server
- Port: 8081

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 4 | 1 | Uses Go workspace and cloud-gateway skeleton from group 4; group 4 creates Go modules and skeleton binaries |
| 01_project_setup | 7 | 5 | Uses NATS infrastructure from group 7 for integration tests; group 7 creates compose.yml with nats-server |

## Clarifications

- **C1 (HTTP framework):** Use Go standard library `net/http` with Go 1.22 `ServeMux` pattern matching. No external HTTP router.
- **C2 (Command status storage):** Command responses are stored in an in-memory map keyed by command ID. No expiry or persistence — this is a demo service. The map is protected by a mutex for concurrent access.
- **C3 (Token-VIN mapping):** Bearer tokens are mapped to VINs via a JSON configuration file. The config includes a list of `{token, vin}` pairs. Default demo config includes at least one token-VIN pair. Config path is specified by `CONFIG_PATH` env var, defaulting to `config.json`.
- **C4 (NATS connection failure):** If NATS is unreachable at startup, the service retries with exponential backoff (1s, 2s, 4s) up to 5 attempts, then exits non-zero. If NATS disconnects at runtime, the nats.go client handles automatic reconnection.
- **C5 (Command timeout):** Commands have a configurable timeout (default: 30 seconds). If no response is received within the timeout, the command status becomes `"timeout"`. The timeout is set in the config file.
- **C6 (Telemetry handling):** The gateway subscribes to `vehicles.*.telemetry` and logs received telemetry. No storage or aggregation — fleet telemetry is out of scope for this phase.
- **C7 (Content-Type):** All REST responses use `Content-Type: application/json`.
- **C8 (Bearer token in NATS):** When publishing commands to NATS, the gateway includes the bearer token as a NATS message header (`Authorization: Bearer <token>`) so the CLOUD_GATEWAY_CLIENT can validate it.
- **C9 (Command ID):** The `command_id` field is required in the POST body. The server does not generate IDs — the client (COMPANION_APP) provides the UUID.
- **C10 (VIN validation):** VIN is extracted from the URL path. The gateway validates that the token is authorized for the specified VIN. A token not authorized for the given VIN receives HTTP 403.

## Clarifications from Master PRD

- **A1:** CLOUD_GATEWAY exposes REST towards COMPANION_APPs and NATS towards CLOUD_GATEWAY_CLIENT. COMPANION_APP uses REST exclusively. CLOUD_GATEWAY handles protocol translation.
- **AC3:** Each component defines its own NATS subjects. Pattern: `vehicles.{vin}.{action}`.
- **IA2:** Containerized NATS server (nats-server) for local dev. CLOUD_GATEWAY uses nats.go Go client.
- **IA6:** Self-created VINs. COMPANION_APP is paired with a specific VIN.
- **U4:** Bearer tokens for the demo.

## Out-of-Scope

- Real authentication/authorization beyond basic bearer tokens
- Aggregated fleet telemetry (optional, later phase)
- Production-grade security/encryption
- TLS termination (handled by infrastructure layer)
