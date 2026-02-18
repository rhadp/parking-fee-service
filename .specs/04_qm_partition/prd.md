# PRD: PARKING_OPERATOR_ADAPTOR + UPDATE_SERVICE (Phase 2.3)

> Extracted from the [main PRD](../prd.md). This spec covers Phase 2.3: the
> RHIVOS QM partition — PARKING_OPERATOR_ADAPTOR, UPDATE_SERVICE, mock
> PARKING_OPERATOR, and mock PARKING_APP CLI.

## Scope

From the main PRD, Phase 2.3:

- Implement generic PARKING_OPERATOR_ADAPTOR (Rust): event-driven parking
  session management based on lock events, gRPC interface for PARKING_APP,
  REST client for PARKING_OPERATOR.
- Implement UPDATE_SERVICE (Rust): container lifecycle management using podman,
  gRPC interface for PARKING_APP, adapter state persistence, timer-based
  offloading.
- Implement mock PARKING_OPERATOR (Go): REST API simulating a parking operator.
- Implement mock PARKING_APP CLI (Go): real gRPC client replacing spec 01
  skeleton.
- Integration test of DATA_BROKER → PARKING_OPERATOR_ADAPTOR flow.

### Components in scope

| Component | Work | Language |
|-----------|------|----------|
| PARKING_OPERATOR_ADAPTOR | Full implementation | Rust |
| UPDATE_SERVICE | Full implementation (local images, no OCI pull) | Rust |
| Mock PARKING_OPERATOR | New implementation | Go |
| Mock PARKING_APP CLI | Real implementation (replace spec 01 skeleton) | Go |

### Event-driven session flow (PRD Flow 1)

```
LOCKING_SERVICE writes IsLocked = true to DATA_BROKER
        │
        ▼ (subscription stream)
PARKING_OPERATOR_ADAPTOR
   ├─ IsLocked changed to true AND no active session
   │   ├─ POST /parking/start → PARKING_OPERATOR
   │   ├─ Receive {session_id, rate}
   │   └─ Write Vehicle.Parking.SessionActive = true → DATA_BROKER
   │
   └─ IsLocked changed to false AND active session
       ├─ POST /parking/stop → PARKING_OPERATOR
       ├─ Receive {total_fee, duration}
       └─ Write Vehicle.Parking.SessionActive = false → DATA_BROKER
```

### Container lifecycle (UPDATE_SERVICE)

For the demo, adapter container images are **pre-built locally** (no OCI
registry pull). UPDATE_SERVICE manages adapters via **podman CLI**:

```
InstallAdapter(image_ref)
    │
    ▼
INSTALLING ──► podman create + start
    │
    ▼
RUNNING ──► adapter container is active
    │
    ├── RemoveAdapter() → STOPPED → podman rm
    │
    └── offloading timer (5 min after session ends) → OFFLOADING → podman rm → UNKNOWN
```

Full state machine: `UNKNOWN → INSTALLING → RUNNING → STOPPED | OFFLOADING | ERROR`

### Offloading

Offloading is triggered by a configurable timer after the parking session
ends. Default: 5 minutes for the demo. When triggered, the adapter container
is stopped and removed.

### Adapter configuration

When UPDATE_SERVICE starts an adapter container, it passes configuration via
environment variables:

| Env Var | Purpose |
|---------|---------|
| `DATABROKER_ADDR` | Kuksa DATA_BROKER address |
| `PARKING_OPERATOR_URL` | PARKING_OPERATOR REST API base URL |
| `ZONE_ID` | Parking zone identifier |
| `VEHICLE_VIN` | Vehicle identification number |
| `LISTEN_ADDR` | gRPC listen address for the adapter |

### Rate model

The mock PARKING_OPERATOR supports two rate types:
- **Per-minute:** `rate_amount` × duration in minutes.
- **Flat fee:** Fixed `rate_amount` per parking session.

Rates are configured in the mock PARKING_OPERATOR.

## Dependencies

| Spec | Relationship | Notes |
|------|-------------|-------|
| 01_repo_setup | Depends on | Rust workspace, Go modules, proto definitions, Makefile |
| 02_locking_service | Depends on | Kuksa client library, VSS overlay (SessionActive signal), LOCKING_SERVICE writes IsLocked |

## Clarifications

### Architecture

- **A1 (Session trigger):** Event-driven is primary. Adaptor subscribes to
  `IsLocked` and auto-starts/stops sessions. gRPC `StartSession`/`StopSession`
  RPCs exist for explicit control by PARKING_APP (manual override). `GetStatus`
  and `GetRate` are query endpoints.

- **A2 (Container management):** Real podman for start/stop with pre-built
  local images (no OCI registry pull). Proper OTA/update with container
  validation is deferred.

- **A3 (Zone ID source):** The PARKING_APP performs a location-based lookup
  against PARKING_FEE_SERVICE (spec 05) and the user selects a zone. The
  `zone_id` is passed to the adaptor as an environment variable by
  UPDATE_SERVICE when starting the container.

### Implementation

- **U1 (Mock PARKING_OPERATOR API):** `POST /parking/start`,
  `POST /parking/stop`, `GET /parking/sessions/{id}`, `GET /parking/rate`.

- **U2 (Session data):** `session_id`, `vehicle_id`, `zone_id`, `start_time`,
  `end_time`, `rate_per_hour` (or `rate_amount` + `rate_type`), `currency`,
  `total_fee`, `status`.

- **U3 (Rate model):** Per-minute and flat-fee rate types. Configured in the
  mock PARKING_OPERATOR. Currency: EUR for demo.

- **U4 (Mock PARKING_OPERATOR language):** Go.

- **U5 (Offloading):** Simple configurable timer. Demo default: 5 minutes
  after parking session ends.

- **U6 (State persistence):** UPDATE_SERVICE persists adapter states across
  restarts (file-based JSON).
