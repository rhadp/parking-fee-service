# PRD: PARKING_APP (Phase 2.4 — Android AAOS)

> Extracted from the [main PRD](../prd.md). This spec covers Phase 2.4
> (Android portion): the PARKING_APP running on Android Automotive OS,
> providing a minimal UI for parking zone discovery, adapter management,
> and session monitoring.

## Scope

From the main PRD, Phase 2.4 (AAOS app):

- Implement PARKING_APP (Kotlin): Android Automotive OS app with minimal
  functional UI.
- Integrate with all backend services via gRPC and REST.
- Build system integration with the monorepo Makefile.

### Components in scope

| Component | Work | Language |
|-----------|------|----------|
| PARKING_APP | Full implementation | Kotlin (Jetpack Compose) |

### Out of scope for this spec

- **COMPANION_APP:** Separate spec (07).
- **CI/CD pipelines:** Deferred (operational concern).
- **Virtual validation:** Deferred (infrastructure concern).
- **Car API integration:** Standard Android app, no android.car.* APIs.

### App behavior

The PARKING_APP is a standard Android application running on AAOS. It
communicates with backend services entirely through network gRPC and REST —
no AAOS Car APIs are used.

**Discovery flow:**
1. Read vehicle location from DATA_BROKER (Vehicle.CurrentLocation.*).
2. Query PARKING_FEE_SERVICE for available zones at that location.
3. Display zones to the driver. Driver selects a zone.
4. Retrieve adapter metadata from PARKING_FEE_SERVICE.
5. Request adapter installation via UPDATE_SERVICE.

**Session monitoring flow:**
1. Subscribe to Vehicle.Parking.SessionActive on DATA_BROKER.
2. When session becomes active, periodically poll
   PARKING_OPERATOR_ADAPTOR.GetStatus() for fee/duration details.
3. When session ends, display summary.

### Screens

1. **Zone Discovery** — Shows available parking zones from
   PARKING_FEE_SERVICE based on vehicle location. User selects a zone to
   install the adapter.
2. **Adapter Status** — Shows adapter installation progress via
   UPDATE_SERVICE.WatchAdapterStates() streaming. Transitions to session
   dashboard when adapter is RUNNING.
3. **Session Dashboard** — Shows parking session status, current fee,
   duration. Displays "lock to start" when no session is active.

### Service communication

| Target | Protocol | Library | Purpose |
|--------|----------|---------|---------|
| DATA_BROKER | gRPC (Kuksa val.v2) | grpc-kotlin | Read location, subscribe SessionActive |
| UPDATE_SERVICE | gRPC | grpc-kotlin | InstallAdapter, WatchAdapterStates |
| PARKING_OPERATOR_ADAPTOR | gRPC | grpc-kotlin | GetStatus, GetRate |
| PARKING_FEE_SERVICE | REST/HTTP | OkHttp | Zone lookup, adapter metadata |

## Dependencies

| Spec | Relationship | Notes |
|------|-------------|-------|
| 01_repo_setup | Depends on | Proto definitions, Makefile, placeholder directory `aaos/` |
| 02_locking_service | Reads from | Kuksa val.v2 proto for DATA_BROKER subscription |
| 04_qm_partition | Integrates with | UPDATE_SERVICE gRPC, PARKING_OPERATOR_ADAPTOR gRPC |
| 05_parking_fee_service | Integrates with | PARKING_FEE_SERVICE REST API |

## Clarifications

### Architecture

- **A1 (Session status source):** Both. PARKING_APP subscribes to
  DATA_BROKER for the SessionActive boolean (reactive notification), and
  polls PARKING_OPERATOR_ADAPTOR.GetStatus() for detailed session info
  (fee, duration).

- **A2 (Location source):** DATA_BROKER via network gRPC. The PARKING_APP
  reads Vehicle.CurrentLocation.Latitude and Longitude from DATA_BROKER
  (values set by mock-sensors).

- **A3 (AAOS APIs):** Standard Android app. No android.car.* APIs. All
  vehicle data comes through DATA_BROKER gRPC.

### Implementation

- **UI framework:** Jetpack Compose (modern, minimal boilerplate).
- **Architecture:** MVVM with ViewModels + StateFlow.
- **gRPC transport:** grpc-okhttp (standard Android gRPC transport).
- **Proto generation:** protobuf-gradle-plugin referencing repo root proto/.
- **Session poll interval:** 5 seconds while session is active.
- **Auto-install:** User confirms zone selection, then adapter installs
  automatically.
