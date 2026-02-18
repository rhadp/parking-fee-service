# Requirements Document: PARKING_APP

## Introduction

This specification defines the PARKING_APP, an Android Automotive OS
application providing a minimal functional UI for parking zone discovery,
adapter lifecycle management, and parking session monitoring. The app
communicates with backend services via gRPC and REST, reading vehicle state
from the Kuksa DATA_BROKER.

## Glossary

| Term | Definition |
|------|-----------|
| AAOS | Android Automotive OS — the vehicle's infotainment platform |
| Adapter | A PARKING_OPERATOR_ADAPTOR container managed by UPDATE_SERVICE |
| DATA_BROKER | Eclipse Kuksa Databroker providing vehicle signals via gRPC |
| Jetpack Compose | Android's modern declarative UI toolkit |
| MVVM | Model-View-ViewModel architecture pattern |
| PARKING_APP | The Android application specified in this document |
| PARKING_FEE_SERVICE | Backend REST API for zone lookup and adapter metadata |
| SessionActive | VSS signal `Vehicle.Parking.SessionActive` |
| StateFlow | Kotlin coroutines flow for observable state |
| UPDATE_SERVICE | gRPC service managing adapter container lifecycle |
| Zone | A parking area with an associated operator and adapter |

## Requirements

### Requirement 1: Zone Discovery

**User Story:** As a driver, I want to see available parking zones near my
vehicle, so that I can choose where to park and have the correct adapter
installed automatically.

#### Acceptance Criteria

1. **06-REQ-1.1** WHEN the Zone Discovery screen is displayed, THE
   PARKING_APP SHALL read `Vehicle.CurrentLocation.Latitude` and
   `Vehicle.CurrentLocation.Longitude` from DATA_BROKER via network gRPC.

2. **06-REQ-1.2** WHEN the location is obtained, THE PARKING_APP SHALL
   query `GET /api/v1/zones?lat={lat}&lon={lon}` on PARKING_FEE_SERVICE and
   display the matching zones in a list.

3. **06-REQ-1.3** EACH zone in the list SHALL display: zone name, operator
   name, rate information (`rate_type`, `rate_amount`, `currency`), and
   distance in meters.

4. **06-REQ-1.4** WHEN the user selects a zone, THE PARKING_APP SHALL
   retrieve adapter metadata via
   `GET /api/v1/zones/{zone_id}/adapter` on PARKING_FEE_SERVICE and call
   `UPDATE_SERVICE.InstallAdapter(image_ref, checksum)` to install the
   adapter.

#### Edge Cases

1. **06-REQ-1.E1** IF no zones are found near the location, THEN the app
   SHALL display a message: "No parking zones nearby."

2. **06-REQ-1.E2** IF DATA_BROKER is unreachable when reading location, THEN
   the app SHALL display an error message with a retry option.

3. **06-REQ-1.E3** IF PARKING_FEE_SERVICE is unreachable, THEN the app SHALL
   display an error message with a retry option.

---

### Requirement 2: Adapter Lifecycle UI

**User Story:** As a driver, I want to see the adapter installation progress,
so that I know when the parking service is ready.

#### Acceptance Criteria

1. **06-REQ-2.1** WHEN an adapter installation is requested, THE PARKING_APP
   SHALL call `UPDATE_SERVICE.WatchAdapterStates()` and display state
   transitions in real-time.

2. **06-REQ-2.2** THE PARKING_APP SHALL display the current adapter state
   using visual indicators (progress indicator for INSTALLING, status text
   for each state).

3. **06-REQ-2.3** WHEN the adapter state transitions to `RUNNING`, THE
   PARKING_APP SHALL automatically navigate to the Session Dashboard screen.

#### Edge Cases

1. **06-REQ-2.E1** IF the adapter state transitions to `ERROR`, THEN the app
   SHALL display the error message and offer a retry option (re-install).

2. **06-REQ-2.E2** IF UPDATE_SERVICE becomes unreachable during streaming,
   THEN the app SHALL display a connection error with a retry option.

---

### Requirement 3: Session Dashboard

**User Story:** As a driver, I want to see my parking session status and
current fee in real time, so that I know how much I'm paying.

#### Acceptance Criteria

1. **06-REQ-3.1** THE PARKING_APP SHALL subscribe to
   `Vehicle.Parking.SessionActive` on DATA_BROKER via gRPC streaming to
   detect session start and stop events.

2. **06-REQ-3.2** WHILE a session is active (`SessionActive = true`), THE
   PARKING_APP SHALL poll `PARKING_OPERATOR_ADAPTOR.GetStatus()` every 5
   seconds and display: session ID, start time, current fee, and zone info.

3. **06-REQ-3.3** WHEN `SessionActive` transitions from `true` to `false`,
   THE PARKING_APP SHALL call `PARKING_OPERATOR_ADAPTOR.GetStatus()` once
   and display the completed session summary: total fee, duration, and zone
   info.

4. **06-REQ-3.4** WHEN no session is active AND an adapter is `RUNNING`, THE
   PARKING_APP SHALL display: "Parking available — lock vehicle to start
   session."

#### Edge Cases

1. **06-REQ-3.E1** IF PARKING_OPERATOR_ADAPTOR is unreachable during
   polling, THEN the app SHALL show the last known session info with a
   "Connection lost" indicator.

---

### Requirement 4: Service Communication

**User Story:** As the PARKING_APP, I need reliable clients for all backend
services so that the app functions correctly.

#### Acceptance Criteria

1. **06-REQ-4.1** THE PARKING_APP SHALL use gRPC with `grpc-kotlin` and
   `grpc-okhttp` transport for communication with UPDATE_SERVICE,
   PARKING_OPERATOR_ADAPTOR, and DATA_BROKER.

2. **06-REQ-4.2** THE PARKING_APP SHALL use OkHttp for HTTP communication
   with PARKING_FEE_SERVICE.

3. **06-REQ-4.3** THE PARKING_APP SHALL use proto definitions from the
   repository's `proto/` directory for all gRPC communication, compiled via
   the protobuf-gradle-plugin.

#### Edge Cases

1. **06-REQ-4.E1** IF a gRPC call fails with a non-OK status, THEN the
   PARKING_APP SHALL display a user-friendly error message (not the raw gRPC
   status code or message).

---

### Requirement 5: Configuration

**User Story:** As a developer, I want configurable service addresses so
that the app works in different environments (local dev, cloud deployment).

#### Acceptance Criteria

1. **06-REQ-5.1** THE PARKING_APP SHALL allow service addresses to be
   configured via build-time constants or a settings screen.

2. **06-REQ-5.2** THE default service addresses SHALL be:
   `DATA_BROKER=10.0.2.2:55555`, `UPDATE_SERVICE=10.0.2.2:50053`,
   `PARKING_OPERATOR_ADAPTOR=10.0.2.2:50054`,
   `PARKING_FEE_SERVICE=http://10.0.2.2:8080`.

---

### Requirement 6: Build System

**User Story:** As a developer, I want the PARKING_APP to build with standard
Android tooling and integrate with the monorepo's Makefile.

#### Acceptance Criteria

1. **06-REQ-6.1** THE PARKING_APP SHALL build using Gradle with the Android
   Gradle Plugin.

2. **06-REQ-6.2** THE proto definitions SHALL be compiled using the
   `protobuf-gradle-plugin`, referencing proto files from the repository root
   `proto/` directory.

3. **06-REQ-6.3** THE root Makefile SHALL include a `build-android` target
   that builds the PARKING_APP APK.

4. **06-REQ-6.4** THE root Makefile SHALL include a `test-android` target
   that runs PARKING_APP unit tests.

#### Edge Cases

1. **06-REQ-6.E1** IF the Android SDK is not installed, THEN the Makefile
   targets SHALL skip with a warning message rather than failing the entire
   build.

---

### Requirement 7: Unit Testing

**User Story:** As a developer, I want unit tests for the PARKING_APP's
non-UI logic so that service communication and state management are verified.

#### Acceptance Criteria

1. **06-REQ-7.1** THE PARKING_APP SHALL have unit tests for all ViewModel
   classes, verifying state transitions and service call sequences.

2. **06-REQ-7.2** THE PARKING_APP SHALL have unit tests for service client
   wrappers (ParkingFeeServiceClient, UpdateServiceClient,
   ParkingAdapterClient, DataBrokerClient).

3. **06-REQ-7.3** THE unit tests SHALL use mock service responses (no real
   service dependencies). gRPC tests SHALL use `grpc-testing` with
   in-process servers.

#### Edge Cases

1. **06-REQ-7.E1** IF the Android SDK is not available in CI, THEN tests
   SHALL be skipped with a clear message.
