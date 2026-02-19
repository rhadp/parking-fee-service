# Implementation Plan: PARKING_APP

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
- Android SDK and Gradle must be available for all tasks
- The project setup (Gradle, proto) must be done before any UI or client work
- Service clients must be done before ViewModels that use them
-->

## Overview

This plan implements the PARKING_APP Android application in dependency order:

1. Android project setup (Gradle, proto integration).
2. Service client wrappers (gRPC + REST).
3. ViewModels and state management.
4. Compose UI screens and navigation.
5. Unit tests.
6. Makefile integration.

## Test Commands

- Android unit tests: `cd android/parking-app && ./gradlew test`
- Android lint: `cd android/parking-app && ./gradlew lint`
- Build APK: `cd android/parking-app && ./gradlew assembleDebug`
- All tests (from root): `make test` (skips Android if SDK unavailable)
- Build all (from root): `make build` (skips Android if SDK unavailable)

## Tasks

- [x] 1. Android Project Setup
  - [x] 1.1 Create Gradle project structure
    - Create `aaos/parking-app/settings.gradle.kts` with project name
    - Create `aaos/parking-app/build.gradle.kts` (project-level) with
      plugin declarations (Android, Kotlin, protobuf, serialization)
    - Create `aaos/parking-app/gradle.properties` with Android config
    - Create `aaos/parking-app/app/build.gradle.kts` with all dependencies
      (Compose, gRPC, OkHttp, testing) per design doc
    - Add Gradle wrapper (`gradlew`, `gradle/wrapper/`)
    - _Requirements: 06-REQ-6.1_

  - [x] 1.2 Configure proto compilation
    - Configure `protobuf-gradle-plugin` in `app/build.gradle.kts`
    - Set proto source directory to `../../proto` (repo root)
    - Configure `protoc`, `grpc-java`, and `grpckt` plugins with lite option
    - Include Kuksa vendor protos from `../../proto/vendor/kuksa/`
    - Verify proto compilation succeeds: `./gradlew generateDebugProto`
    - _Requirements: 06-REQ-4.3, 06-REQ-6.2_

  - [x] 1.3 Create application skeleton
    - Create `AndroidManifest.xml` with internet permission and main activity
    - Create `ParkingApp.kt` (Application class)
    - Create `MainActivity.kt` with empty Compose content
    - Create `res/values/strings.xml` and `res/values/themes.xml`
    - Verify build: `./gradlew assembleDebug`
    - _Requirements: 06-REQ-6.1_

  - [x] 1.V Verify task group 1
    - [x] `./gradlew assembleDebug` succeeds
    - [x] `./gradlew generateDebugProto` generates Kotlin gRPC stubs
    - [x] Proto classes for UpdateService, ParkingAdapter, and Kuksa
      val.v2 are available
    - [x] Requirements 06-REQ-6.1, 06-REQ-6.2 met

- [x] 2. Service Clients
  - [x] 2.1 Create DataBrokerClient
    - Create `data/DataBrokerClient.kt`
    - `getLocation(): Location?` — reads CurrentLocation.Lat/Lon from Kuksa
    - `subscribeSessionActive(): Flow<Boolean>` — streams SessionActive
    - Use `VALGrpcKt.VALCoroutineStub` from generated Kuksa protos
    - _Requirements: 06-REQ-1.1, 06-REQ-3.1, 06-REQ-4.1_

  - [x] 2.2 Create ParkingFeeServiceClient
    - Create `data/ParkingFeeServiceClient.kt`
    - `lookupZones(lat, lon): List<ZoneMatch>` — GET /api/v1/zones
    - `getZoneAdapter(zoneId): AdapterMetadata` — GET /zones/{id}/adapter
    - Use OkHttp + kotlinx.serialization for JSON parsing
    - _Requirements: 06-REQ-1.2, 06-REQ-1.4, 06-REQ-4.2_

  - [x] 2.3 Create UpdateServiceClient and ParkingAdapterClient
    - Create `data/UpdateServiceClient.kt`
    - `installAdapter(imageRef, checksum): InstallAdapterResponse`
    - `watchAdapterStates(): Flow<AdapterStateEvent>`
    - Create `data/ParkingAdapterClient.kt`
    - `getStatus(sessionId): GetStatusResponse`
    - Use generated gRPC Kotlin stubs
    - _Requirements: 06-REQ-1.4, 06-REQ-2.1, 06-REQ-3.2, 06-REQ-4.1_

  - [x] 2.4 Create data model classes
    - Create `model/Models.kt`
    - `Location`, `ZoneMatch`, `AdapterMetadata`, `SessionInfo`
    - JSON serialization annotations for REST response models
    - _Requirements: 06-REQ-1.3 (prerequisite)_

  - [x] 2.5 Write service client unit tests
    - DataBrokerClient: grpc-testing InProcessServer, verify Get and
      Subscribe calls
    - ParkingFeeServiceClient: MockWebServer (OkHttp), verify URL
      construction and JSON parsing
    - UpdateServiceClient: grpc-testing InProcessServer, verify
      InstallAdapter params
    - ParkingAdapterClient: grpc-testing InProcessServer, verify GetStatus
    - **Property 1: Location-to-Zone Pipeline** (client correctness)
    - **Property 2: Adapter Install Trigger** (client correctness)
    - **Validates: 06-REQ-4.1, 06-REQ-4.2, 06-REQ-4.3, 06-REQ-4.E1**

  - [x] 2.V Verify task group 2
    - [x] `./gradlew test` passes all client tests
    - [x] All 4 service clients compile and are usable
    - [x] Proto-generated stubs are used correctly
    - [x] Requirements 06-REQ-4.1–4.3 met

- [ ] 3. Checkpoint — Service Clients Complete
  - All service clients working with unit tests
  - Commit and verify clean state

- [ ] 4. ViewModels
  - [ ] 4.1 Create ZoneDiscoveryViewModel
    - Create `ui/zone/ZoneDiscoveryViewModel.kt`
    - States: Loading, ZonesFound, NoZones, Error, Installing
    - `loadZones()`: read location → query PFS → update state
    - `selectZone(zone)`: get adapter metadata → install adapter
    - Error handling: catch exceptions → Error state with message
    - _Requirements: 06-REQ-1.1, 06-REQ-1.2, 06-REQ-1.3, 06-REQ-1.4,
      06-REQ-1.E1, 06-REQ-1.E2, 06-REQ-1.E3_

  - [ ] 4.2 Create AdapterStatusViewModel
    - Create `ui/adapter/AdapterStatusViewModel.kt`
    - States: InProgress, Ready, Error
    - `watchAdapter(adapterId)`: stream WatchAdapterStates → update state
    - On RUNNING → transition to Ready
    - On ERROR → Error state with message and retry
    - _Requirements: 06-REQ-2.1, 06-REQ-2.2, 06-REQ-2.3, 06-REQ-2.E1,
      06-REQ-2.E2_

  - [ ] 4.3 Create SessionDashboardViewModel
    - Create `ui/session/SessionDashboardViewModel.kt`
    - States: WaitingForSession, SessionActive, SessionCompleted, Error
    - `startMonitoring()`: subscribe SessionActive → on true, start polling
      GetStatus every 5s → on false, show summary
    - Handle adaptor unreachable: show last known + indicator
    - _Requirements: 06-REQ-3.1, 06-REQ-3.2, 06-REQ-3.3, 06-REQ-3.4,
      06-REQ-3.E1_

  - [ ] 4.4 Write ViewModel unit tests
    - ZoneDiscoveryViewModel: test Loading → ZonesFound, Loading → NoZones,
      Loading → Error (DB unreachable), Loading → Error (PFS unreachable),
      selectZone triggers install
    - AdapterStatusViewModel: test InProgress → Ready (navigation trigger),
      InProgress → Error, retry behavior
    - SessionDashboardViewModel: test WaitingForSession → SessionActive
      (polling), SessionActive → SessionCompleted, connection lost handling
    - Use MockK for service client mocking
    - Use kotlinx-coroutines-test for Flow testing
    - **Property 3: Session State Consistency**
    - **Property 4: Error Visibility**
    - **Property 5: Navigation Integrity**
    - **Validates: 06-REQ-1.1–1.4, 06-REQ-1.E1–1.E3, 06-REQ-2.1–2.3,
      06-REQ-2.E1–2.E2, 06-REQ-3.1–3.4, 06-REQ-3.E1**

  - [ ] 4.V Verify task group 4
    - [ ] `./gradlew test` passes all ViewModel tests
    - [ ] All state transitions covered
    - [ ] Error states tested for all service failures
    - [ ] Requirements 06-REQ-1.*, 06-REQ-2.*, 06-REQ-3.* met

- [ ] 5. Compose UI and Navigation
  - [ ] 5.1 Create navigation graph
    - Create `navigation/NavGraph.kt`
    - Routes: ZoneDiscovery → AdapterStatus(adapterId) → SessionDashboard
    - Back navigation from SessionDashboard to ZoneDiscovery
    - _Requirements: 06-REQ-2.3 (navigation on RUNNING)_

  - [ ] 5.2 Create ZoneDiscoveryScreen
    - Create `ui/zone/ZoneDiscoveryScreen.kt`
    - Compose UI: loading indicator, zone list (name, operator, rate,
      distance), error state with retry button, "no zones" message
    - Collect uiState from ViewModel
    - _Requirements: 06-REQ-1.2, 06-REQ-1.3, 06-REQ-1.E1_

  - [ ] 5.3 Create AdapterStatusScreen
    - Create `ui/adapter/AdapterStatusScreen.kt`
    - Compose UI: progress indicator for INSTALLING, status text,
      error state with retry button
    - Navigate to SessionDashboard when Ready
    - _Requirements: 06-REQ-2.2, 06-REQ-2.E1_

  - [ ] 5.4 Create SessionDashboardScreen
    - Create `ui/session/SessionDashboardScreen.kt`
    - Compose UI: "Lock to start" message, active session card (fee,
      duration, zone), completed session summary, connection-lost indicator
    - _Requirements: 06-REQ-3.2, 06-REQ-3.3, 06-REQ-3.4, 06-REQ-3.E1_

  - [ ] 5.5 Wire up MainActivity
    - Update `MainActivity.kt` with Compose theme and NavHost
    - Initialize service clients with configured addresses
    - Create ViewModels with client dependencies
    - _Requirements: 06-REQ-5.1, 06-REQ-5.2_

  - [ ] 5.V Verify task group 5
    - [ ] `./gradlew assembleDebug` succeeds
    - [ ] App launches in emulator (manual verification)
    - [ ] Navigation between screens works
    - [ ] Requirements 06-REQ-5.1, 06-REQ-5.2 met

- [ ] 6. Checkpoint — PARKING_APP Feature Complete
  - All screens, ViewModels, and service clients working
  - Commit and verify clean state

- [ ] 7. Makefile Integration and Final Verification
  - [ ] 7.1 Add Makefile targets
    - Add `build-android` target to root Makefile
    - Add `test-android` target to root Makefile
    - Both targets skip with a warning if `ANDROID_HOME` is not set
    - Update `build` and `test` targets to conditionally include Android
    - _Requirements: 06-REQ-6.3, 06-REQ-6.4, 06-REQ-6.E1_

  - [ ] 7.2 Run full test suite
    - `./gradlew test` — all unit tests pass
    - `./gradlew lint` — no warnings
    - `make build && make test` — no regressions in specs 01–05
    - _Requirements: 06-REQ-7.1, 06-REQ-7.2, 06-REQ-7.3_

  - [ ] 7.3 Update documentation
    - Document Android build setup in `docs/android-setup.md`
    - Document PARKING_APP screens and service addresses
    - Update README with Android build instructions

  - [ ] 7.V Verify task group 7
    - [ ] `make build` succeeds (skips Android if no SDK)
    - [ ] `make test` passes (skips Android if no SDK)
    - [ ] `./gradlew test` passes all tests
    - [ ] `./gradlew lint` clean
    - [ ] No regressions from specs 01–05
    - [ ] All 06-REQ requirements verified

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
| 06-REQ-1.1 | 2.1, 4.1 | DataBrokerClient test (2.5), ViewModel test (4.4) |
| 06-REQ-1.2 | 2.2, 4.1, 5.2 | PFSClient test (2.5), ViewModel test (4.4) |
| 06-REQ-1.3 | 2.4, 5.2 | ViewModel test (4.4) |
| 06-REQ-1.4 | 2.2, 2.3, 4.1 | Client tests (2.5), ViewModel test (4.4) |
| 06-REQ-1.E1 | 4.1, 5.2 | ViewModel test (4.4) |
| 06-REQ-1.E2 | 4.1, 5.2 | ViewModel test (4.4) |
| 06-REQ-1.E3 | 4.1, 5.2 | ViewModel test (4.4) |
| 06-REQ-2.1 | 2.3, 4.2 | UpdateServiceClient test (2.5), ViewModel test (4.4) |
| 06-REQ-2.2 | 4.2, 5.3 | ViewModel test (4.4) |
| 06-REQ-2.3 | 4.2, 5.1, 5.3 | ViewModel test (4.4) |
| 06-REQ-2.E1 | 4.2, 5.3 | ViewModel test (4.4) |
| 06-REQ-2.E2 | 4.2, 5.3 | ViewModel test (4.4) |
| 06-REQ-3.1 | 2.1, 4.3 | DataBrokerClient test (2.5), ViewModel test (4.4) |
| 06-REQ-3.2 | 2.3, 4.3, 5.4 | ViewModel test (4.4) |
| 06-REQ-3.3 | 4.3, 5.4 | ViewModel test (4.4) |
| 06-REQ-3.4 | 4.3, 5.4 | ViewModel test (4.4) |
| 06-REQ-3.E1 | 4.3, 5.4 | ViewModel test (4.4) |
| 06-REQ-4.1 | 2.1, 2.3 | Client tests (2.5) |
| 06-REQ-4.2 | 2.2 | PFSClient test (2.5) |
| 06-REQ-4.3 | 1.2 | Proto generation verification (1.V) |
| 06-REQ-4.E1 | 4.1, 4.2, 4.3 | ViewModel tests (4.4) |
| 06-REQ-5.1 | 5.5 | Manual verification |
| 06-REQ-5.2 | 5.5 | Build config verification |
| 06-REQ-6.1 | 1.1 | Build verification (1.V) |
| 06-REQ-6.2 | 1.2 | Proto generation verification (1.V) |
| 06-REQ-6.3 | 7.1 | Makefile test (7.2) |
| 06-REQ-6.4 | 7.1 | Makefile test (7.2) |
| 06-REQ-6.E1 | 7.1 | Makefile test (7.2) — skip behavior |
| 06-REQ-7.1 | 4.4 | `./gradlew test` — ViewModel tests |
| 06-REQ-7.2 | 2.5 | `./gradlew test` — client tests |
| 06-REQ-7.3 | 2.5, 4.4 | All tests use mocks, no real services |
| 06-REQ-7.E1 | 7.1 | Makefile skip behavior |

## Notes

- **Android emulator addresses:** The default `10.0.2.2` is the Android
  emulator's alias for the host machine's localhost. For Cuttlefish or
  physical devices, these addresses must be reconfigured.
- **No instrumented tests:** This spec only includes JVM unit tests
  (ViewModel + client tests). Compose UI tests (Espresso, Compose testing)
  are deferred to keep scope minimal.
- **Proto source sharing:** The protobuf-gradle-plugin compiles protos
  from the repo root `proto/` directory, ensuring the Android app uses the
  same proto definitions as Rust and Go services.
- **Kuksa proto compatibility:** The Android app uses Kuksa's `val.v2` proto
  directly via grpc-kotlin. The vendored protos at `proto/vendor/kuksa/`
  are compiled alongside the service protos.
- **No dependency injection:** ViewModels are manually constructed in
  MainActivity. For a demo app, this avoids Hilt/Dagger complexity.
- **Session polling interval:** 5 seconds balances responsiveness with
  network overhead. The per_minute rate model means fee changes every 60s,
  so 5s polling is sufficient for smooth display updates.
