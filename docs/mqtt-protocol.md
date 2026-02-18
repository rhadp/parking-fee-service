# MQTT Protocol Reference

This document describes the MQTT topic structure and message schemas used for
vehicle-to-cloud communication in the SDV Parking Demo System.

## Overview

The system uses Eclipse Mosquitto as the MQTT broker to relay messages between
**CLOUD_GATEWAY** (backend) and **CLOUD_GATEWAY_CLIENT** (vehicle-side). All
topics use the vehicle's VIN as a routing key. Commands and responses use QoS 2
(exactly-once delivery), while telemetry uses QoS 0 (fire-and-forget).

## Broker Configuration

| Setting | Default | Environment Variable |
|---------|---------|---------------------|
| Host | `localhost` | `MQTT_ADDR` |
| Port | `1883` | `MQTT_ADDR` |
| Protocol | MQTT v3.1.1 | — |
| Authentication | None (local dev) | — |
| TLS | Disabled (local dev) | — |

Both CLOUD_GATEWAY and CLOUD_GATEWAY_CLIENT accept the broker address via
`--mqtt-addr` CLI flag or `MQTT_ADDR` environment variable.

## Topic Structure

All topics follow the pattern `vehicles/{vin}/...` where `{vin}` is the 17-character
Vehicle Identification Number (e.g., `DEMOE1MKQT64H3RBV`).

### Topic Summary

| Topic Pattern | Direction | QoS | Publisher | Subscriber |
|---------------|-----------|-----|-----------|------------|
| `vehicles/{vin}/commands` | Cloud → Vehicle | 2 | CLOUD_GATEWAY | CLOUD_GATEWAY_CLIENT |
| `vehicles/{vin}/command_responses` | Vehicle → Cloud | 2 | CLOUD_GATEWAY_CLIENT | CLOUD_GATEWAY |
| `vehicles/{vin}/status_request` | Cloud → Vehicle | 2 | CLOUD_GATEWAY | CLOUD_GATEWAY_CLIENT |
| `vehicles/{vin}/status_response` | Vehicle → Cloud | 2 | CLOUD_GATEWAY_CLIENT | CLOUD_GATEWAY |
| `vehicles/{vin}/telemetry` | Vehicle → Cloud | 0 | CLOUD_GATEWAY_CLIENT | CLOUD_GATEWAY |
| `vehicles/{vin}/registration` | Vehicle → Cloud | 2 | CLOUD_GATEWAY_CLIENT | CLOUD_GATEWAY |

### Subscription Patterns

**CLOUD_GATEWAY** subscribes to (using `+` wildcard for all VINs):

- `vehicles/+/command_responses` (QoS 2)
- `vehicles/+/status_response` (QoS 2)
- `vehicles/+/telemetry` (QoS 0)
- `vehicles/+/registration` (QoS 2)

**CLOUD_GATEWAY_CLIENT** subscribes to (using its specific VIN):

- `vehicles/{vin}/commands` (QoS 2)
- `vehicles/{vin}/status_request` (QoS 2)

## Message Schemas

All messages use JSON encoding. Timestamps are Unix epoch seconds (integer).

### CommandMessage

Published by CLOUD_GATEWAY when a lock or unlock REST request is received.

**Topic:** `vehicles/{vin}/commands`
**QoS:** 2

```json
{
  "command_id": "550e8400-e29b-41d4-a716-446655440000",
  "type": "lock",
  "timestamp": 1708300800
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `command_id` | string (UUID) | Yes | Unique identifier for command correlation |
| `type` | string | Yes | `"lock"` or `"unlock"` |
| `timestamp` | integer | Yes | Unix epoch seconds when command was created |

### CommandResponse

Published by CLOUD_GATEWAY_CLIENT after the LOCKING_SERVICE processes a command
and writes a result to DATA_BROKER.

**Topic:** `vehicles/{vin}/command_responses`
**QoS:** 2

```json
{
  "command_id": "550e8400-e29b-41d4-a716-446655440000",
  "type": "lock",
  "result": "SUCCESS",
  "timestamp": 1708300801
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `command_id` | string (UUID) | Yes | Matches the original `CommandMessage.command_id` |
| `type` | string | Yes | `"lock"` or `"unlock"` |
| `result` | string | Yes | `"SUCCESS"`, `"REJECTED_SPEED"`, or `"REJECTED_DOOR_OPEN"` |
| `timestamp` | integer | Yes | Unix epoch seconds when result was produced |

#### Result Values

| Result | Meaning | Condition |
|--------|---------|-----------|
| `SUCCESS` | Command executed successfully | Safety validation passed |
| `REJECTED_SPEED` | Command rejected | Vehicle speed >= 1.0 km/h |
| `REJECTED_DOOR_OPEN` | Command rejected | Door is open during lock command |

### StatusRequest

Published by CLOUD_GATEWAY to request the current vehicle state on demand.

**Topic:** `vehicles/{vin}/status_request`
**QoS:** 2

```json
{
  "request_id": "660e8400-e29b-41d4-a716-446655440000",
  "timestamp": 1708300802
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `request_id` | string (UUID) | Yes | Unique identifier for request correlation |
| `timestamp` | integer | Yes | Unix epoch seconds |

### StatusResponse

Published by CLOUD_GATEWAY_CLIENT in response to a StatusRequest. Contains
current vehicle state read from DATA_BROKER.

**Topic:** `vehicles/{vin}/status_response`
**QoS:** 2

```json
{
  "request_id": "660e8400-e29b-41d4-a716-446655440000",
  "vin": "DEMOE1MKQT64H3RBV",
  "is_locked": true,
  "is_door_open": false,
  "speed": 0.0,
  "latitude": 48.1351,
  "longitude": 11.5820,
  "parking_session_active": false,
  "timestamp": 1708300802
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `request_id` | string (UUID) | Yes | Matches the original `StatusRequest.request_id` |
| `vin` | string | Yes | Vehicle VIN |
| `is_locked` | boolean or null | No | Door lock state |
| `is_door_open` | boolean or null | No | Door open state |
| `speed` | float or null | No | Vehicle speed in km/h |
| `latitude` | float or null | No | GPS latitude |
| `longitude` | float or null | No | GPS longitude |
| `parking_session_active` | boolean or null | No | Parking session state |
| `timestamp` | integer | Yes | Unix epoch seconds |

### TelemetryMessage

Published periodically by CLOUD_GATEWAY_CLIENT with current vehicle state.
Uses the same schema as StatusResponse but without `request_id`.

**Topic:** `vehicles/{vin}/telemetry`
**QoS:** 0

```json
{
  "vin": "DEMOE1MKQT64H3RBV",
  "is_locked": true,
  "is_door_open": false,
  "speed": 0.0,
  "latitude": 48.1351,
  "longitude": 11.5820,
  "parking_session_active": false,
  "timestamp": 1708300810
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `vin` | string | Yes | Vehicle VIN |
| `is_locked` | boolean or null | No | Door lock state |
| `is_door_open` | boolean or null | No | Door open state |
| `speed` | float or null | No | Vehicle speed in km/h |
| `latitude` | float or null | No | GPS latitude |
| `longitude` | float or null | No | GPS longitude |
| `parking_session_active` | boolean or null | No | Parking session state |
| `timestamp` | integer | Yes | Unix epoch seconds |

The telemetry interval is configurable via `--telemetry-interval` flag or
`TELEMETRY_INTERVAL` environment variable on CLOUD_GATEWAY_CLIENT (default: 5
seconds).

**Note:** Fields with unknown values (signal unavailable in DATA_BROKER) are
set to `null` in the JSON payload.

### RegistrationMessage

Published by CLOUD_GATEWAY_CLIENT on every startup to register the vehicle with
CLOUD_GATEWAY. The registration message is **not retained** — CLOUD_GATEWAY
must be subscribed before the message is published.

**Topic:** `vehicles/{vin}/registration`
**QoS:** 2

```json
{
  "vin": "DEMOE1MKQT64H3RBV",
  "pairing_pin": "723041",
  "timestamp": 1708300800
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `vin` | string | Yes | 17-character VIN (starts with "DEMO" in the demo) |
| `pairing_pin` | string | Yes | 6-digit numeric PIN for pairing |
| `timestamp` | integer | Yes | Unix epoch seconds |

## QoS Policy

| Message Type | QoS | Rationale |
|-------------|-----|-----------|
| Commands | 2 (exactly-once) | Lock/unlock commands must not be duplicated or lost |
| Command responses | 2 (exactly-once) | Results must be delivered reliably |
| Status requests | 2 (exactly-once) | On-demand queries must be delivered |
| Status responses | 2 (exactly-once) | Query responses must be delivered |
| Registration | 2 (exactly-once) | Vehicle registration must not be lost |
| Telemetry | 0 (fire-and-forget) | Periodic updates; missing one is acceptable since the next arrives shortly |

## Connection Behavior

### Auto-Reconnect

Both services implement automatic reconnection with exponential backoff when
the MQTT broker connection is lost:

- **CLOUD_GATEWAY** (Go): Uses `paho.mqtt.golang` with `AutoReconnect: true`
  and configurable `MaxReconnectInterval`.
- **CLOUD_GATEWAY_CLIENT** (Rust): Uses `rumqttc` with built-in reconnection.
  Re-subscribes to topics after reconnection.

### Startup Ordering

1. CLOUD_GATEWAY starts and subscribes to wildcard topics (`vehicles/+/...`).
2. CLOUD_GATEWAY_CLIENT starts, generates or loads VIN and PIN, connects to
   MQTT, subscribes to its vehicle-specific topics, and publishes a
   registration message.
3. CLOUD_GATEWAY receives the registration and stores the vehicle entry.

**Important:** Since the registration message is not retained, CLOUD_GATEWAY
must be started and subscribed **before** CLOUD_GATEWAY_CLIENT publishes its
registration. In integration tests, a short delay and retry loop mitigate
timing issues.

## DATA_BROKER Signal Mapping

The following Kuksa DATA_BROKER (VSS) signals are read by CLOUD_GATEWAY_CLIENT
for telemetry and status responses:

| MQTT Field | VSS Signal Path | Type |
|-----------|----------------|------|
| `is_locked` | `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` | boolean |
| `is_door_open` | `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` | boolean |
| `speed` | `Vehicle.Speed` | float |
| `latitude` | `Vehicle.CurrentLocation.Latitude` | double |
| `longitude` | `Vehicle.CurrentLocation.Longitude` | double |
| `parking_session_active` | `Vehicle.Parking.SessionActive` | boolean |

For commands, CLOUD_GATEWAY_CLIENT writes to:

| Command | VSS Signal Path | Value |
|---------|----------------|-------|
| Lock | `Vehicle.Command.Door.Lock` | `true` |
| Unlock | `Vehicle.Command.Door.Lock` | `false` |

Command results are received from DATA_BROKER via subscription to
`Vehicle.Command.Door.LockResult`.

See [vss-signals.md](vss-signals.md) for full VSS signal documentation.
