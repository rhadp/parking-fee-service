# PRD: CLOUD_GATEWAY_CLIENT (Phase 2.1)

> Extracted from the master PRD at `.specs/prd.md`. This spec covers the CLOUD_GATEWAY_CLIENT component of Phase 2.1: RHIVOS Safety Partition.

## Scope

Implement the CLOUD_GATEWAY_CLIENT as a Rust service running in the RHIVOS safety partition. This component bridges the vehicle's DATA_BROKER (Eclipse Kuksa Databroker) with the cloud-based CLOUD_GATEWAY via NATS messaging. It receives authenticated lock/unlock commands, validates them, writes them to DATA_BROKER as command signals, and relays command responses and vehicle telemetry back to the cloud.

## Component Description

- Rust service running in the RHIVOS safety partition
- Maintains secure connection to CLOUD_GATEWAY (NATS with TLS for production, plain NATS for local dev)
- Uses the async-nats Rust client crate for NATS connectivity
- Receives authenticated lock/unlock commands from COMPANION_APP via CLOUD_GATEWAY
- Validates command structure and bearer tokens
- Publishes validated commands to DATA_BROKER as command signals (`Vehicle.Command.Door.Lock`) -- does NOT call LOCKING_SERVICE directly
- Subscribes to DATA_BROKER for vehicle state (lock status, location, parking state)
- Publishes vehicle telemetry (location, door status, parking state) to CLOUD_GATEWAY via NATS
- Observes command response signals from DATA_BROKER (`Vehicle.Command.Door.Response`) and relays results to CLOUD_GATEWAY
- Communicates with DATA_BROKER via gRPC over UDS (same partition)

## NATS Subject Model

- `vehicles.{VIN}.commands` -- incoming commands from CLOUD_GATEWAY (subscribe)
- `vehicles.{VIN}.command_responses` -- outgoing command responses to CLOUD_GATEWAY (publish)
- `vehicles.{VIN}.telemetry` -- outgoing vehicle telemetry to CLOUD_GATEWAY (publish)

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

## Command Response Format

```json
{
  "command_id": "<uuid>",
  "status": "success" | "failed",
  "reason": "<optional>",
  "timestamp": <unix_ts>
}
```

## VSS Signals Used

- `Vehicle.Command.Door.Lock` (string, JSON) -- command request, written by CLOUD_GATEWAY_CLIENT
- `Vehicle.Command.Door.Response` (string, JSON) -- command result, written by LOCKING_SERVICE, observed by CLOUD_GATEWAY_CLIENT
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` (bool) -- lock state, read for telemetry
- `Vehicle.CurrentLocation.Latitude` (double) -- vehicle latitude, read for telemetry
- `Vehicle.CurrentLocation.Longitude` (double) -- vehicle longitude, read for telemetry
- `Vehicle.Parking.SessionActive` (bool) -- parking state, read for telemetry

## Vehicle Identity

- Self-created VINs, self-registration on startup
- Must support many virtual devices/cars simultaneously
- VIN is used as part of NATS subject hierarchy
- Configured via the `VIN` environment variable

## Tech Stack

- Language: Rust (edition 2021)
- Async runtime: tokio
- NATS client: async-nats
- gRPC client: tonic (with UDS support for DATA_BROKER)
- Serialization: serde, serde_json
- Logging: tracing

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 2 | 1 | Uses repo structure and Rust project skeleton from group 2 |
| 02_data_broker | 3 | 1 | Requires DATA_BROKER with VSS overlay for command/state signals |

## Clarifications from Master PRD

- **A1:** CLOUD_GATEWAY exposes REST towards COMPANION_APPs and NATS towards CLOUD_GATEWAY_CLIENT. The CLOUD_GATEWAY_CLIENT uses NATS exclusively.
- **A2:** CLOUD_GATEWAY_CLIENT publishes validated commands to DATA_BROKER as custom VSS command signals. LOCKING_SERVICE subscribes to DATA_BROKER for command signals. There are no direct service calls between CLOUD_GATEWAY_CLIENT and LOCKING_SERVICE.
- **AC3:** NATS subjects use the pattern `vehicles.{vin}.{action}`.
- **IA2:** Containerized NATS server (nats-server) for local dev. CLOUD_GATEWAY_CLIENT uses the async-nats Rust crate.
- **IA6:** Self-created VINs, self-registration on startup.
- **I2:** Same-partition communication uses UDS; cross-partition uses network TCP.
- **I4:** Lock/unlock commands flow through DATA_BROKER using custom VSS signals.
- **U4:** Bearer tokens for the demo.

## Out-of-Scope

- Real authentication/authorization beyond basic bearer tokens
- Direct calls to LOCKING_SERVICE (all communication goes through DATA_BROKER)
- Production-grade TLS certificate management
- Multi-VIN handling in a single process instance
