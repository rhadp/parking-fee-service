# PRD: CLOUD_GATEWAY_CLIENT (Phase 2.1)

> Extracted from the master PRD at `.specs/prd.md`. This spec covers the CLOUD_GATEWAY_CLIENT component of Phase 2.1.

## Scope

Implement the CLOUD_GATEWAY_CLIENT in Rust, running in the RHIVOS safety partition. This component bridges the vehicle's DATA_BROKER with the cloud-based CLOUD_GATEWAY via NATS messaging.

## Component Description

- Maintains secure connection to CLOUD_GATEWAY (NATS with TLS)
- Uses the async-nats Rust client crate for NATS connectivity
- Receives authenticated lock/unlock commands from COMPANION_APP via CLOUD_GATEWAY
- Validates command structure and bearer tokens
- Publishes validated commands to DATA_BROKER as command signals (`Vehicle.Command.Door.Lock`) -- does NOT call LOCKING_SERVICE directly
- Subscribes to DATA_BROKER for vehicle state (lock status, location, parking state)
- Publishes vehicle telemetry (location, door status, parking state) to CLOUD_GATEWAY
- Observes command response signals from DATA_BROKER (`Vehicle.Command.Door.Response`) and relays results to CLOUD_GATEWAY
- Communicates with DATA_BROKER via gRPC over UDS (same partition)
- Communicates with CLOUD_GATEWAY via NATS (with TLS in production, plain NATS for local dev)

## NATS Subject Model

- `vehicles.{VIN}.commands` — incoming commands from CLOUD_GATEWAY (subscribe)
- `vehicles.{VIN}.command_responses` — outgoing command responses (publish)
- `vehicles.{VIN}.telemetry` — outgoing vehicle telemetry (publish)

## Command Payload Format

```json
{
  "command_id": "<uuid>",
  "action": "lock" | "unlock",
  "doors": ["driver"],
  "source": "companion_app",
  "vin": "<vin>",
  "timestamp": <unix_ts>
}
```

## Vehicle Identity

- Self-created VINs, self-registration on startup
- Must support many virtual devices/cars simultaneously
- VIN is used as part of NATS subject hierarchy

## Tech Stack

- Language: Rust
- NATS client: async-nats crate
- gRPC: tonic (with UDS support for DATA_BROKER)

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 2 | 1 | Requires Rust workspace skeleton and local NATS infrastructure |
| 02_data_broker | 2 | 1 | Requires running DATA_BROKER with VSS signals configured |
