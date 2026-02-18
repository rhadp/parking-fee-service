# PRD: COMPANION_APP (Phase 2.5)

> Extracted from the [main PRD](../prd.md). This spec covers Phase 2.5:
> the COMPANION_APP, a Flutter/Dart mobile application for remote vehicle
> control via CLOUD_GATEWAY.

## Scope

From the main PRD, Phase 2.5:

- Implement COMPANION_APP (Flutter/Dart): mobile app for vehicle pairing,
  remote lock/unlock, and vehicle status monitoring.
- Build system integration with the monorepo Makefile.

### Components in scope

| Component | Work | Language |
|-----------|------|----------|
| COMPANION_APP | Full implementation | Flutter/Dart |

### Out of scope

- **iOS build:** Flutter supports iOS but only Android APK is required for
  the demo.
- **Push notifications:** No real-time push. Status is polled.
- **Multi-vehicle support:** Single vehicle pairing at a time.

### App behavior

The COMPANION_APP is a standard Flutter mobile app. It communicates
exclusively with CLOUD_GATEWAY via REST/HTTP. Two screens:

1. **Pairing Screen** — User enters VIN and PIN. App calls
   `POST /api/v1/pair`. On success, token is persisted and the app navigates
   to the dashboard.

2. **Dashboard** — Shows vehicle status (polled every 5s). Provides Lock
   and Unlock buttons. After sending a command, polls for the result
   (up to 10s) and displays feedback (SUCCESS, REJECTED_SPEED, etc.).

### Command feedback flow

```
User taps "Lock"
    │
    ▼
POST /api/v1/vehicles/{vin}/lock → 202 {command_id, status: "accepted"}
    │
    ▼ (poll every 1s, up to 10s)
GET /api/v1/vehicles/{vin}/status
    → Check last_command.command_id matches
    → Check last_command.status != "accepted"
    │
    ▼
Display result: "Locked successfully" or "Rejected: speed too high"
```

## Dependencies

| Spec | Relationship | Notes |
|------|-------------|-------|
| 01_repo_setup | Depends on | Makefile, placeholder directory `android/` |
| 03_cloud_connectivity | Integrates with | CLOUD_GATEWAY REST API (pairing, lock/unlock, status) |

## Clarifications

### Architecture

- **A1 (Protocol):** REST only. COMPANION_APP talks to CLOUD_GATEWAY via
  HTTP/JSON. No gRPC, no MQTT.

- **A2 (Command feedback):** Poll-based. After sending lock/unlock, poll
  GET /status every 1s for up to 10s to get the command result.

### Implementation

- **U1 (State management):** Provider with ChangeNotifier. Simple, sufficient
  for 2 screens.

- **U2 (HTTP client):** Dart `http` package (standard library wrapper).

- **U3 (Token persistence):** `shared_preferences` plugin.

- **U4 (Status refresh):** Poll every 5 seconds while dashboard is active.

- **U5 (Gateway address):** Configurable, default `http://10.0.2.2:8081`
  (Android emulator alias for host localhost).
