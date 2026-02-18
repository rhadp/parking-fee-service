# Implementation Plan: COMPANION_APP

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
- Flutter SDK must be available for all tasks
- Models and services must be done before providers and screens
-->

## Overview

This plan implements the COMPANION_APP in dependency order:

1. Flutter project setup.
2. Data models and REST client.
3. State management (VehicleProvider).
4. UI screens (pairing + dashboard).
5. Makefile integration and final verification.

The app is a straightforward REST client with two screens. No gRPC, no
proto compilation, no complex dependencies.

## Test Commands

- Flutter unit tests: `cd android/companion-app && flutter test`
- Flutter analyze: `cd android/companion-app && flutter analyze`
- Build APK: `cd android/companion-app && flutter build apk --debug`
- All tests (from root): `make test` (skips Flutter if SDK unavailable)
- Build all (from root): `make build` (skips Flutter if SDK unavailable)

## Tasks

- [ ] 1. Flutter Project Setup
  - [ ] 1.1 Create Flutter project
    - Run `flutter create` in `android/companion-app/` or create manually
    - Configure `pubspec.yaml` with dependencies: `http`, `provider`,
      `shared_preferences`
    - Configure `analysis_options.yaml` with recommended lints
    - Set app name, package ID (`com.rhadp.companion`)
    - _Requirements: 07-REQ-6.1_

  - [ ] 1.2 Configure Android settings
    - Set `minSdkVersion` to 21 in `android/app/build.gradle`
    - Add internet permission to `AndroidManifest.xml`
    - Set app label and theme
    - Verify build: `flutter build apk --debug`
    - _Requirements: 07-REQ-6.1_

  - [ ] 1.V Verify task group 1
    - [ ] `flutter build apk --debug` succeeds
    - [ ] `flutter analyze` clean
    - [ ] App launches in emulator (blank screen)

- [ ] 2. Models and REST Client
  - [ ] 2.1 Create data models
    - Create `lib/models/models.dart`
    - Define: `PairResponse`, `VehicleStatus`, `CommandInfo`,
      `CommandResponse`, `GatewayException`
    - JSON serialization: `fromJson` factory constructors, `toJson` methods
    - Handle nullable fields in `VehicleStatus`
    - _Requirements: 07-REQ-2.1 (prerequisite)_

  - [ ] 2.2 Create CloudGatewayClient
    - Create `lib/services/cloud_gateway_client.dart`
    - `pair(vin, pin)` → POST /api/v1/pair → PairResponse
    - `lock(vin, token)` → POST /api/v1/vehicles/{vin}/lock → CommandResponse
    - `unlock(vin, token)` → POST /api/v1/vehicles/{vin}/unlock →
      CommandResponse
    - `getStatus(vin, token)` → GET /api/v1/vehicles/{vin}/status →
      VehicleStatus
    - Accept `http.Client` in constructor for testability
    - Bearer token in Authorization header for protected endpoints
    - Throw `GatewayException` for non-success HTTP responses
    - _Requirements: 07-REQ-1.2, 07-REQ-2.2, 07-REQ-3.2_

  - [ ] 2.3 Write client unit tests
    - Create `test/cloud_gateway_client_test.dart`
    - Use `http.MockClient` for all tests
    - Pair: 200 → parsed response, 403 → exception, 404 → exception
    - Lock/Unlock: 202 → parsed command_id, 401 → exception
    - Status: 200 → all fields parsed, null fields → preserved as null
    - Network error → GatewayException
    - Verify Authorization header present on protected calls
    - **Property 1: Token-Request Consistency**
    - **Validates: 07-REQ-1.2, 07-REQ-1.E1, 07-REQ-1.E2, 07-REQ-2.2,
      07-REQ-3.2, 07-REQ-3.E2**

  - [ ] 2.V Verify task group 2
    - [ ] `flutter test test/cloud_gateway_client_test.dart` passes
    - [ ] `flutter analyze` clean
    - [ ] All HTTP methods tested with mock responses
    - [ ] Error cases covered

- [ ] 3. State Management (VehicleProvider)
  - [ ] 3.1 Create VehicleProvider
    - Create `lib/providers/vehicle_provider.dart`
    - Extend `ChangeNotifier`
    - Pairing: `pair(vin, pin)`, `unpair()`, `loadPersistedPairing()`
    - Status: `startStatusPolling()`, `stopStatusPolling()`, poll every 5s
    - Commands: `sendCommand(type)` with 1s poll for result (up to 10s)
    - Result formatting: SUCCESS → "Locked/Unlocked successfully",
      REJECTED_SPEED → "Rejected: vehicle speed too high",
      REJECTED_DOOR_OPEN → "Rejected: door is open"
    - Timeout handling: 10 polls → "Command timed out"
    - Error state: preserve last known status on poll failure
    - _Requirements: 07-REQ-1.3, 07-REQ-2.2, 07-REQ-2.3, 07-REQ-3.3,
      07-REQ-3.4, 07-REQ-4.1, 07-REQ-4.2, 07-REQ-4.3_

  - [ ] 3.2 Write provider unit tests
    - Create `test/vehicle_provider_test.dart`
    - Use mock CloudGatewayClient and mock SharedPreferences
    - Pair flow: pair() → isPaired, token persisted
    - Auto-login: load with stored token → isPaired
    - Unpair: unpair() → not paired, prefs cleared
    - Status polling: verify periodic calls, state updates
    - Command feedback: send lock → poll → result displayed
    - Command timeout: no result after 10 polls → timeout message
    - Connection lost: poll failure → error indicator, last status preserved
    - **Property 2: Command-Result Correlation**
    - **Property 3: Token Persistence Round-Trip**
    - **Property 4: Error Visibility**
    - **Property 5: Status Data Preservation**
    - **Validates: 07-REQ-1.3, 07-REQ-2.E1, 07-REQ-2.E2, 07-REQ-3.3,
      07-REQ-3.4, 07-REQ-3.E1, 07-REQ-4.1, 07-REQ-4.2, 07-REQ-4.3**

  - [ ] 3.V Verify task group 3
    - [ ] `flutter test` passes all tests
    - [ ] `flutter analyze` clean
    - [ ] Pairing, polling, command feedback, and error states tested
    - [ ] Requirements 07-REQ-1.3, 07-REQ-3.3–3.4, 07-REQ-4.1–4.3 met

- [ ] 4. Checkpoint — Logic Complete
  - Models, REST client, and state management all working with tests
  - Commit and verify clean state

- [ ] 5. UI Screens
  - [ ] 5.1 Create pairing screen
    - Create `lib/screens/pairing_screen.dart`
    - VIN text field, PIN text field (obscured), Pair button
    - Loading indicator during pairing
    - Error messages for failed pairing (wrong PIN, unknown VIN, connection)
    - On success: navigate to dashboard
    - Optional: gateway address text field for configuration
    - _Requirements: 07-REQ-1.1, 07-REQ-1.E1, 07-REQ-1.E2, 07-REQ-5.1_

  - [ ] 5.2 Create dashboard screen
    - Create `lib/screens/dashboard_screen.dart`
    - Vehicle status card: locked, door, speed, location, parking, timestamp
    - "Unknown" for null fields
    - Lock and Unlock buttons (disabled while command pending)
    - Command result banner (success/rejection/timeout)
    - "Connection lost" indicator on poll failure
    - Unpair button (navigates to pairing screen)
    - _Requirements: 07-REQ-2.1, 07-REQ-2.3, 07-REQ-2.E1, 07-REQ-2.E2,
      07-REQ-3.1, 07-REQ-3.4, 07-REQ-3.E1, 07-REQ-4.3_

  - [ ] 5.3 Wire up main.dart and navigation
    - Create `lib/main.dart`
    - Initialize: SharedPreferences → VehicleProvider → load persisted pairing
    - Provider at root: `ChangeNotifierProvider<VehicleProvider>`
    - Navigation: if paired → Dashboard, else → Pairing
    - Set Material theme, app title
    - Default gateway address: `http://10.0.2.2:8081`
    - _Requirements: 07-REQ-4.2, 07-REQ-5.2_

  - [ ] 5.V Verify task group 5
    - [ ] `flutter build apk --debug` succeeds
    - [ ] App launches in emulator (manual verification)
    - [ ] Pairing screen displays and accepts input
    - [ ] Dashboard displays status and controls
    - [ ] Navigation works (pair → dashboard, unpair → pairing)

- [ ] 6. Makefile Integration and Final Verification
  - [ ] 6.1 Add Makefile targets
    - Add `build-flutter` target to root Makefile
    - Add `test-flutter` target to root Makefile
    - Both skip with warning if `flutter` is not in PATH
    - Update `build` and `test` targets to conditionally include Flutter
    - _Requirements: 07-REQ-6.2, 07-REQ-6.3, 07-REQ-6.E1_

  - [ ] 6.2 Run full test suite
    - `flutter test` — all unit tests pass
    - `flutter analyze` — no warnings
    - `make build && make test` — no regressions in specs 01–06
    - _Requirements: 07-REQ-7.1, 07-REQ-7.2, 07-REQ-7.3_

  - [ ] 6.3 Update documentation
    - Document Flutter build setup in `docs/flutter-setup.md`
    - Document COMPANION_APP screens and gateway configuration
    - Update README with Flutter build instructions

  - [ ] 6.V Verify task group 6
    - [ ] `make build` succeeds (skips Flutter if no SDK)
    - [ ] `make test` passes (skips Flutter if no SDK)
    - [ ] `flutter test` passes all tests
    - [ ] `flutter analyze` clean
    - [ ] No regressions from specs 01–06
    - [ ] All 07-REQ requirements verified

### Checkbox States

| Syntax   | Meaning                |
|----------|------------------------|
| `- [ ]`  | Not started (required) |
| `- [ ]*` | Not started (optional) |
| `- [x]`  | Completed              |
| `- [-]`  | In progress            |
| `- [~]`  | Queued                 |

## Traceability

| Requirement | Implemented By Task | Verified By Test |
|-------------|---------------------|------------------|
| 07-REQ-1.1 | 5.1 | Manual verification |
| 07-REQ-1.2 | 2.2 | Client test (2.3) |
| 07-REQ-1.3 | 3.1 | Provider test (3.2) |
| 07-REQ-1.E1 | 2.2, 5.1 | Client test (2.3) |
| 07-REQ-1.E2 | 2.2, 5.1 | Client test (2.3) |
| 07-REQ-2.1 | 2.1, 5.2 | Client test (2.3) — status parsing |
| 07-REQ-2.2 | 3.1 | Provider test (3.2) — polling |
| 07-REQ-2.3 | 5.2 | Manual verification |
| 07-REQ-2.E1 | 2.1, 5.2 | Client test (2.3) — null fields |
| 07-REQ-2.E2 | 3.1, 5.2 | Provider test (3.2) — poll failure |
| 07-REQ-3.1 | 5.2 | Manual verification |
| 07-REQ-3.2 | 2.2 | Client test (2.3) — lock/unlock |
| 07-REQ-3.3 | 3.1 | Provider test (3.2) — command polling |
| 07-REQ-3.4 | 3.1, 5.2 | Provider test (3.2) — result formatting |
| 07-REQ-3.E1 | 3.1 | Provider test (3.2) — timeout |
| 07-REQ-3.E2 | 2.2 | Client test (2.3) — network error |
| 07-REQ-4.1 | 3.1 | Provider test (3.2) — persist |
| 07-REQ-4.2 | 3.1, 5.3 | Provider test (3.2) — auto-login |
| 07-REQ-4.3 | 3.1, 5.2 | Provider test (3.2) — unpair |
| 07-REQ-5.1 | 5.1, 5.3 | Manual verification |
| 07-REQ-5.2 | 5.3 | Build config verification |
| 07-REQ-6.1 | 1.1 | Build verification (1.V) |
| 07-REQ-6.2 | 6.1 | Makefile test (6.2) |
| 07-REQ-6.3 | 6.1 | Makefile test (6.2) |
| 07-REQ-6.E1 | 6.1 | Makefile skip behavior (6.2) |
| 07-REQ-7.1 | 2.3 | `flutter test` — client tests |
| 07-REQ-7.2 | 3.2 | `flutter test` — provider tests |
| 07-REQ-7.3 | 2.3, 3.2 | All tests use mocks |
| 07-REQ-7.E1 | 6.1 | Makefile skip behavior |

## Notes

- **Simplest spec in the series:** The COMPANION_APP is a pure REST client
  with no gRPC, no proto compilation, and no complex state. Two screens,
  one HTTP client, one state provider.
- **Emulator addresses:** Default `10.0.2.2` is the Android emulator alias
  for host localhost. Must be reconfigured for physical devices.
- **No widget tests:** Only JVM-level unit tests for client and provider.
  Compose/widget testing is deferred.
- **http.MockClient:** Dart's `http` package provides `MockClient` for
  injecting mock HTTP responses — no additional mocking library needed.
- **shared_preferences mocking:** The `shared_preferences` package provides
  `SharedPreferences.setMockInitialValues({})` for testing.
- **Timer handling in tests:** Use `fake_async` package to control timers
  in VehicleProvider tests (status polling, command result polling).
- **No changes to CLOUD_GATEWAY or mock companion-app-cli:** These
  components from spec 03 are used as-is.
