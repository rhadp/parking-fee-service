# Vehicle Pairing Flow

This document describes the vehicle pairing process that associates a companion
app (or CLI tool) with a specific vehicle, enabling remote lock/unlock commands
and status queries.

## Overview

Vehicle pairing is a two-step process:

1. **Registration:** CLOUD_GATEWAY_CLIENT publishes a registration message
   containing its VIN and a 6-digit pairing PIN to CLOUD_GATEWAY via MQTT.
2. **Pairing:** The companion app sends the VIN and PIN to CLOUD_GATEWAY via
   REST. If valid, CLOUD_GATEWAY returns a bearer token for subsequent
   authenticated requests.

## VIN and PIN Generation

When CLOUD_GATEWAY_CLIENT starts for the first time, it generates:

- **VIN:** A 17-character vehicle identification number in the format
  `DEMO` + 13 random alphanumeric characters (e.g., `DEMOE1MKQT64H3RBV`).
- **Pairing PIN:** A 6-digit numeric code (e.g., `723041`), zero-padded.

Both are persisted to `{data_dir}/vin.json` (default: `./data/vin.json`):

```json
{
  "vin": "DEMOE1MKQT64H3RBV",
  "pairing_pin": "723041"
}
```

On subsequent starts, the existing VIN and PIN are reused from the file. The
data directory is configurable via `--data-dir` flag or `DATA_DIR` environment
variable.

## Sequence Diagram

```mermaid
sequenceDiagram
    participant CGC as CLOUD_GATEWAY_CLIENT
    participant MQ as Mosquitto
    participant GW as CLOUD_GATEWAY
    participant App as companion-app-cli

    Note over CGC: First start: generate VIN + PIN
    Note over CGC: Subsequent starts: load from vin.json
    CGC->>CGC: Load/generate VIN and PIN

    CGC->>MQ: PUBLISH vehicles/{vin}/registration<br/>{vin, pairing_pin, timestamp}
    MQ->>GW: Forward registration message
    GW->>GW: Store VehicleEntry(VIN, PIN)

    Note over App,GW: User reads VIN and PIN<br/>from CLOUD_GATEWAY_CLIENT logs<br/>or vin.json file

    App->>GW: POST /api/v1/pair<br/>{vin: "DEMO...", pin: "723041"}
    GW->>GW: Validate VIN exists and PIN matches
    GW->>GW: Generate bearer token
    GW->>GW: Associate token with VIN
    GW-->>App: 200 OK<br/>{token: "y1YK2F...", vin: "DEMO..."}

    Note over App,GW: Token is used for all<br/>subsequent requests

    App->>GW: POST /api/v1/vehicles/{vin}/lock<br/>Authorization: Bearer y1YK2F...
    GW->>GW: Validate token is associated with VIN
    GW-->>App: 202 Accepted<br/>{command_id: "...", status: "accepted"}
```

## REST API

### POST /api/v1/pair

Pairs a companion app with a registered vehicle.

**Request:**

```http
POST /api/v1/pair HTTP/1.1
Content-Type: application/json

{
  "vin": "DEMOE1MKQT64H3RBV",
  "pin": "723041"
}
```

**Success Response (200 OK):**

```json
{
  "token": "y1YK2FpU54S5fg0F1S_NlKZ7gNxq3Y5FGhBJvxJb1zA=",
  "vin": "DEMOE1MKQT64H3RBV"
}
```

**Error Responses:**

| Status | Condition | Response Body |
|--------|-----------|--------------|
| 400 Bad Request | Invalid JSON body or missing fields | `{"error": "...", "code": "BAD_REQUEST"}` |
| 404 Not Found | VIN not registered | `{"error": "vehicle not found", "code": "NOT_FOUND"}` |
| 403 Forbidden | PIN does not match | `{"error": "invalid pairing PIN", "code": "FORBIDDEN"}` |

### Authentication

After pairing, the returned token must be included in all subsequent requests
as a Bearer token:

```http
POST /api/v1/vehicles/DEMOE1MKQT64H3RBV/lock HTTP/1.1
Authorization: Bearer y1YK2FpU54S5fg0F1S_NlKZ7gNxq3Y5FGhBJvxJb1zA=
```

The token is validated against the VIN in the request path. A token obtained
for one vehicle cannot be used to control a different vehicle.

**Auth Error Responses:**

| Status | Condition |
|--------|-----------|
| 401 Unauthorized | Missing `Authorization` header, invalid scheme (not `Bearer`), or token not recognized |
| 401 Unauthorized | Token is valid but associated with a different VIN |

## Using the companion-app-cli

### 1. Start the Services

```bash
# Start infrastructure (Kuksa + Mosquitto)
make infra-up

# Start LOCKING_SERVICE (in terminal 1)
./rhivos/target/debug/locking-service

# Start CLOUD_GATEWAY (in terminal 2)
./backend/cloud-gateway/cloud-gateway

# Start CLOUD_GATEWAY_CLIENT (in terminal 3)
./rhivos/target/debug/cloud-gateway-client
```

### 2. Get VIN and PIN

The VIN and PIN are logged by CLOUD_GATEWAY_CLIENT on startup:

```
INFO Vehicle VIN: DEMOE1MKQT64H3RBV
INFO Pairing PIN: 723041
```

Or read them from the data file:

```bash
cat data/vin.json
```

### 3. Pair

```bash
./mock/companion-app-cli/companion-app-cli \
  --vin DEMOE1MKQT64H3RBV \
  --pin 723041 \
  pair
```

Output:

```
Pairing successful!
  VIN:   DEMOE1MKQT64H3RBV
  Token: y1YK2FpU54S5fg0F1S_NlKZ7gNxq3Y5FGhBJvxJb1zA=
```

### 4. Lock/Unlock/Status

```bash
# Lock the vehicle
./mock/companion-app-cli/companion-app-cli \
  --vin DEMOE1MKQT64H3RBV \
  --token "y1YK2FpU54S5fg0F1S_NlKZ7gNxq3Y5FGhBJvxJb1zA=" \
  lock

# Check vehicle status
./mock/companion-app-cli/companion-app-cli \
  --vin DEMOE1MKQT64H3RBV \
  --token "y1YK2FpU54S5fg0F1S_NlKZ7gNxq3Y5FGhBJvxJb1zA=" \
  status

# Unlock the vehicle
./mock/companion-app-cli/companion-app-cli \
  --vin DEMOE1MKQT64H3RBV \
  --token "y1YK2FpU54S5fg0F1S_NlKZ7gNxq3Y5FGhBJvxJb1zA=" \
  unlock
```

## Security Notes (Demo Limitations)

- **Plaintext MQTT:** The demo uses unencrypted MQTT. Production deployments
  should use MQTT over TLS.
- **No token expiry:** Bearer tokens do not expire. Production systems should
  implement token expiry and refresh.
- **In-memory state:** CLOUD_GATEWAY stores pairings in memory. Restarting the
  gateway clears all pairings. CLOUD_GATEWAY_CLIENT re-registers automatically,
  but the companion app must re-pair.
- **Simple token generation:** Tokens are generated using `crypto/rand` (Go)
  and base64-encoded. Production systems may use JWT or other structured tokens.
- **Single-vehicle demo:** Only one CLOUD_GATEWAY_CLIENT instance is expected.
  Multiple vehicles would each register with unique VINs.

## Troubleshooting

### "vehicle not found" (404) when pairing

The vehicle has not registered with CLOUD_GATEWAY. Possible causes:

- CLOUD_GATEWAY_CLIENT has not started yet or failed to connect to MQTT.
- CLOUD_GATEWAY was not running when CLOUD_GATEWAY_CLIENT published its
  registration (the registration message is not retained).
- **Fix:** Restart CLOUD_GATEWAY_CLIENT after CLOUD_GATEWAY is running.

### "invalid pairing PIN" (403)

The PIN provided does not match the registered vehicle's PIN. Check the correct
PIN from CLOUD_GATEWAY_CLIENT logs or the `vin.json` file.

### "unauthorized" (401) on lock/unlock/status

- Missing or malformed `Authorization: Bearer` header.
- Token was issued for a different VIN.
- CLOUD_GATEWAY was restarted (pairings are cleared). Re-pair to obtain a new
  token.
