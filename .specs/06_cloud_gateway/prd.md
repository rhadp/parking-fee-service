# PRD: CLOUD_GATEWAY (Phase 2.2)

> Extracted from the master PRD at `.specs/prd.md`. This spec covers Phase 2.2: Vehicle-to-Cloud Connectivity.

## Scope

Implement the CLOUD_GATEWAY as a cloud-based Go service with dual interfaces: REST API towards COMPANION_APPs and NATS towards vehicles (CLOUD_GATEWAY_CLIENT).

## Component Description

- Cloud-based service with two interfaces:
  - **REST API** towards COMPANION_APPs (HTTPS) — receives lock/unlock commands, returns command status
  - **NATS** towards vehicles (CLOUD_GATEWAY_CLIENT) — relays commands and receives telemetry
- Authenticates vehicles and COMPANION_APPs using bearer tokens
- Routes commands between mobile apps and vehicles, translating between REST and NATS protocols
- Uses containerized NATS server (nats-server) for local development
- Deployed on Google Cloud infrastructure

## REST API (towards COMPANION_APP)

- `POST /vehicles/{vin}/commands` — send lock/unlock command
  - Headers: `Authorization: Bearer <token>`
  - Body: `{"command_id": "<uuid>", "type": "lock"|"unlock", "doors": ["driver"]}`
  - Response: `200 OK {"command_id": "<uuid>", "status": "accepted"|"success"|"failed"}`
- `GET /vehicles/{vin}/status` — get vehicle status (lock state, parking state, location)

## NATS Interface (towards CLOUD_GATEWAY_CLIENT)

- Publish: `vehicles.{VIN}.commands` — relay commands to vehicle
- Subscribe: `vehicles.{VIN}.command_responses` — receive command responses from vehicle
- Subscribe: `vehicles.{VIN}.telemetry` — receive vehicle telemetry

## Protocol Translation

CLOUD_GATEWAY translates between REST and NATS:
1. COMPANION_APP sends REST POST with lock/unlock command
2. CLOUD_GATEWAY validates bearer token
3. CLOUD_GATEWAY publishes command to NATS subject `vehicles.{VIN}.commands`
4. CLOUD_GATEWAY_CLIENT receives command via NATS subscription
5. CLOUD_GATEWAY_CLIENT publishes response to `vehicles.{VIN}.command_responses`
6. CLOUD_GATEWAY receives response via NATS subscription
7. CLOUD_GATEWAY returns response to COMPANION_APP via REST

## Tech Stack

- Language: Go 1.22+
- HTTP: standard library net/http or chi router
- NATS client: nats.go
- Port: 8081 (HTTP)

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 2 | 1 | Requires Go module structure, build system, and local NATS infrastructure |
