# Requirements Document: COMPANION_APP

## Introduction

This specification defines the COMPANION_APP, a Flutter/Dart mobile
application that enables a user to remotely pair with a vehicle, send
lock/unlock commands, and monitor vehicle status via the CLOUD_GATEWAY
REST API.

## Glossary

| Term | Definition |
|------|-----------|
| CLOUD_GATEWAY | Backend REST API for vehicle commands and status (spec 03) |
| Command feedback | Polling for the result of a lock/unlock command after it is accepted |
| COMPANION_APP | The Flutter mobile application specified in this document |
| Pairing | Associating the app with a vehicle using VIN + PIN to obtain a bearer token |
| Token | Bearer authentication token obtained through pairing |
| VIN | Vehicle Identification Number, used to identify the target vehicle |

## Requirements

### Requirement 1: Vehicle Pairing

**User Story:** As a vehicle owner, I want to pair my phone with my vehicle
using a VIN and PIN, so that I can send commands and view its status remotely.

#### Acceptance Criteria

1. **07-REQ-1.1** THE COMPANION_APP SHALL provide a pairing screen with text
   input fields for VIN and PIN, and a "Pair" button.

2. **07-REQ-1.2** WHEN the user submits VIN and PIN, THE app SHALL call
   `POST /api/v1/pair` on CLOUD_GATEWAY with `{vin, pin}` and display a
   loading indicator during the request.

3. **07-REQ-1.3** WHEN pairing succeeds (HTTP 200 with `{token, vin}`), THE
   app SHALL persist the token and VIN locally and navigate to the dashboard.

#### Edge Cases

1. **07-REQ-1.E1** IF pairing fails with HTTP 403 (wrong PIN) or 404
   (unknown VIN), THEN the app SHALL display the corresponding error message.

2. **07-REQ-1.E2** IF CLOUD_GATEWAY is unreachable, THEN the app SHALL
   display a connection error message.

---

### Requirement 2: Vehicle Status Display

**User Story:** As a vehicle owner, I want to see my vehicle's current
status on the dashboard, so that I know its state at a glance.

#### Acceptance Criteria

1. **07-REQ-2.1** THE dashboard SHALL display vehicle status fields:
   locked state, door open/closed, speed, latitude, longitude, and parking
   session active.

2. **07-REQ-2.2** WHILE the dashboard is visible, THE app SHALL poll
   `GET /api/v1/vehicles/{vin}/status` every 5 seconds and update the
   displayed values.

3. **07-REQ-2.3** THE dashboard SHALL display the timestamp of the last
   successful status update.

#### Edge Cases

1. **07-REQ-2.E1** IF a status field is `null` or missing in the response,
   THEN the app SHALL display "Unknown" for that field.

2. **07-REQ-2.E2** IF a status request fails, THEN the app SHALL display a
   "Connection lost" indicator without clearing the last known data.

---

### Requirement 3: Lock/Unlock Commands with Feedback

**User Story:** As a vehicle owner, I want to remotely lock or unlock my
vehicle and see the result, so that I know whether the command succeeded.

#### Acceptance Criteria

1. **07-REQ-3.1** THE dashboard SHALL provide distinct "Lock" and "Unlock"
   buttons.

2. **07-REQ-3.2** WHEN the user taps Lock or Unlock, THE app SHALL call the
   corresponding `POST /api/v1/vehicles/{vin}/lock` or `unlock` endpoint
   with the bearer token and show a loading indicator.

3. **07-REQ-3.3** AFTER receiving a 202 response with `{command_id}`, THE
   app SHALL poll `GET /api/v1/vehicles/{vin}/status` every 1 second (up to
   10 seconds) until `last_command.command_id` matches the sent command and
   `last_command.status` is not `"accepted"`.

4. **07-REQ-3.4** WHEN the command result is received, THE app SHALL display
   a user-friendly message: "Locked successfully", "Unlocked successfully",
   "Rejected: vehicle speed too high", or "Rejected: door is open".

#### Edge Cases

1. **07-REQ-3.E1** IF the command result polling times out (10 seconds
   without a result), THEN the app SHALL display "Command timed out — check
   status manually."

2. **07-REQ-3.E2** IF the lock/unlock request fails (network error, HTTP
   error), THEN the app SHALL display an error message.

---

### Requirement 4: Token Persistence

**User Story:** As a vehicle owner, I want my pairing to persist across app
restarts, so that I don't need to re-pair every time.

#### Acceptance Criteria

1. **07-REQ-4.1** THE app SHALL persist the bearer token and VIN to local
   storage using `shared_preferences`.

2. **07-REQ-4.2** WHEN the app starts AND a persisted token and VIN exist,
   THE app SHALL navigate directly to the dashboard (skip pairing).

3. **07-REQ-4.3** THE dashboard SHALL provide an "Unpair" action that clears
   the persisted token and VIN and returns to the pairing screen.

---

### Requirement 5: Configuration

**User Story:** As a developer, I want the CLOUD_GATEWAY address to be
configurable, so that the app works in different environments.

#### Acceptance Criteria

1. **07-REQ-5.1** THE app SHALL allow the CLOUD_GATEWAY base URL to be
   configured via a settings screen or build-time constant.

2. **07-REQ-5.2** THE default CLOUD_GATEWAY address SHALL be
   `http://10.0.2.2:8081` (Android emulator alias for host localhost).

---

### Requirement 6: Build System

**User Story:** As a developer, I want the COMPANION_APP to build with
standard Flutter tooling and integrate with the monorepo Makefile.

#### Acceptance Criteria

1. **07-REQ-6.1** THE COMPANION_APP SHALL build using the Flutter SDK
   (`flutter build apk`).

2. **07-REQ-6.2** THE root Makefile SHALL include a `build-flutter` target
   that builds the COMPANION_APP APK.

3. **07-REQ-6.3** THE root Makefile SHALL include a `test-flutter` target
   that runs COMPANION_APP unit tests.

#### Edge Cases

1. **07-REQ-6.E1** IF the Flutter SDK is not installed, THEN the Makefile
   targets SHALL skip with a warning message rather than failing the build.

---

### Requirement 7: Unit Testing

**User Story:** As a developer, I want unit tests for the COMPANION_APP's
service client and state management logic.

#### Acceptance Criteria

1. **07-REQ-7.1** THE app SHALL have unit tests for the
   `CloudGatewayClient` service class (pairing, lock, unlock, status).

2. **07-REQ-7.2** THE app SHALL have unit tests for the state management
   logic (VehicleProvider) covering pairing flow, status polling, and
   command feedback.

3. **07-REQ-7.3** THE unit tests SHALL use mock HTTP responses (no real
   service dependencies).

#### Edge Cases

1. **07-REQ-7.E1** IF the Flutter SDK is not available in CI, THEN tests
   SHALL be skipped with a clear message.
