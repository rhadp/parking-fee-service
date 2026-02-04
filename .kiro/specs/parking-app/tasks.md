# Implementation Plan: PARKING_APP

## Overview

This implementation plan covers the Kotlin Android Automotive OS application for the SDV Parking Demo System. The app uses Jetpack Compose for UI, Kotlin Coroutines for async operations, gRPC-kotlin for service communication, and Ktor for REST API calls.

## Tasks

- [ ] 1. Set up project structure and dependencies
  - Create Android project structure under `android/parking-app/`
  - Configure `build.gradle.kts` with Jetpack Compose, Hilt, gRPC-kotlin, Ktor, and Kotest dependencies
  - Set up Hilt application class and dependency injection modules
  - Configure proto generation for gRPC stubs
  - _Requirements: 8.1_

- [ ] 2. Implement data models and configuration
  - [ ] 2.1 Create domain models
    - Implement `LocationUpdate`, `ZoneInfo`, `SessionStatus`, `SessionState`, `FinalSessionInfo` data classes
    - Implement `ParkingUiState`, `ParkingScreen`, `ParkingError`, `NavigationEvent` UI state classes
    - Implement `AppConfig` with connection settings and retry configuration
    - _Requirements: 1.2, 3.2, 5.2, 6.2_
  
  - [ ] 2.2 Create exception types and error mapping
    - Implement `ParkingException` sealed class hierarchy
    - Implement `ErrorMessages.getUserMessage()` mapping function
    - _Requirements: 7.1, 7.4_
  
  - [ ] 2.3 Write property test for error message mapping
    - **Property 10: Error Message Mapping**
    - **Validates: Requirements 7.1, 7.4**

- [ ] 3. Implement gRPC clients
  - [ ] 3.1 Implement DataBrokerClient
    - Create `DataBrokerClient` interface and implementation
    - Implement `subscribeToLocation()` returning Flow of LocationUpdate
    - Implement `subscribeToSessionActive()` returning Flow of Boolean
    - Implement `isConnected()` and `reconnect()` with exponential backoff
    - _Requirements: 1.1, 1.3, 2.1_
  
  - [ ] 3.2 Implement UpdateServiceClient
    - Create `UpdateServiceClient` interface and implementation
    - Implement `isAdapterInstalled()`, `installAdapter()`, `getAdapterStatus()`
    - Return Flow of `InstallationProgress` for installation tracking
    - _Requirements: 4.1, 4.3_
  
  - [ ] 3.3 Implement ParkingAdaptorClient
    - Create `ParkingAdaptorClient` interface and implementation
    - Implement `getSessionStatus()` and `hasActiveSession()`
    - Map proto responses to domain models
    - _Requirements: 5.1, 6.1_

- [ ] 4. Implement REST client
  - [ ] 4.1 Implement ParkingFeeServiceClient
    - Create `ParkingFeeServiceClient` interface and Ktor implementation
    - Implement `lookupZone()` with latitude/longitude parameters
    - Handle HTTP 404 as null result, other errors as exceptions
    - _Requirements: 3.1, 3.3_
  
  - [ ] 4.2 Write property test for zone data storage
    - **Property 5: Zone Data Storage Completeness**
    - **Validates: Requirements 3.2**

- [ ] 5. Implement repository layer
  - [ ] 5.1 Implement SignalRepository
    - Create `SignalRepository` with location and session active flows
    - Implement `startSubscriptions()` with reconnection logic
    - Implement exponential backoff for DATA_BROKER reconnection (max 5 attempts)
    - Expose `connectionState` flow for UI feedback
    - Set `isOffline=true` in UI state when gRPC connection lost
    - Auto-restore `isOffline=false` when connection re-established
    - _Requirements: 1.2, 1.3, 1.4, 2.2, 2.3_
  
  - [ ] 5.2 Write property test for location storage
    - **Property 1: Location Signal Storage**
    - **Validates: Requirements 1.2**
  
  - [ ] 5.3 Write property test for reconnection backoff
    - **Property 2: Reconnection with Exponential Backoff**
    - **Validates: Requirements 1.3, 1.4**
  
  - [ ] 5.4 Implement ZoneRepository
    - Create `ZoneRepository` with retry logic for zone lookup
    - Implement `lookupZone()` with exponential backoff (max 3 retries)
    - _Requirements: 3.1, 3.4, 3.5_
  
  - [ ] 5.5 Write property test for zone lookup retry
    - **Property 6: Zone Lookup Retry with Backoff**
    - **Validates: Requirements 3.4**
  
  - [ ] 5.6 Implement AdapterRepository
    - Create `AdapterRepository` for adapter lifecycle management
    - Implement `isAdapterInstalled()` and `installAdapter()` with progress flow
    - _Requirements: 4.1, 4.2, 4.4, 4.5_
  
  - [ ] 5.7 Implement SessionRepository
    - Create `SessionRepository` for session status queries
    - Implement `getSessionStatus()` and `pollSessionStatus()` with 100ms interval (10 updates/sec)
    - _Requirements: 5.1, 5.3, 6.1_

- [ ] 6. Checkpoint - Ensure all repository tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 7. Implement ViewModel
  - [ ] 7.1 Implement ParkingViewModel
    - Create `ParkingViewModel` with Hilt injection
    - Implement `initialize()` to check existing session and start subscriptions
    - Implement `onLocationUpdate()` to trigger zone lookup
    - Implement `onSessionActiveChanged()` for screen transitions
    - Implement `retry()` and `acknowledgeSessionEnded()` actions
    - Implement `setMockLocation()` for debug mode
    - _Requirements: 2.2, 2.3, 2.4, 3.1, 4.1, 5.1, 6.1, 7.2, 7.3, 9.1, 10.1, 10.2, 10.3, 10.4_
  
  - [ ] 7.2 Write property test for session state transitions
    - **Property 3: Session State Transitions**
    - **Validates: Requirements 2.2, 2.3**
  
  - [ ] 7.3 Write property test for adapter installation workflow
    - **Property 7: Adapter Installation Workflow**
    - **Validates: Requirements 4.1, 4.2, 4.3, 4.4**
  
  - [ ] 7.4 Write property test for session status polling
    - **Property 8: Session Status Polling and Display**
    - **Validates: Requirements 5.1, 5.2, 5.3, 5.4**
  
  - [ ] 7.5 Write property test for session end handling
    - **Property 9: Session End Handling**
    - **Validates: Requirements 6.1, 6.2**
  
  - [ ] 7.6 Write property test for UI state preservation
    - **Property 11: UI State Preservation**
    - **Validates: Requirements 8.3**
  
  - [ ] 7.7 Write property test for loading indicator
    - **Property 12: Loading Indicator During Async Operations**
    - **Validates: Requirements 8.4**
  
  - [ ] 7.8 Write property test for mock location mode
    - **Property 13: Mock Location Mode**
    - **Validates: Requirements 9.1**
  
  - [ ] 7.9 Write property test for offline state display
    - **Property 17: Offline State Display**
    - **Validates: Requirements 1.3, 1.4**
  
  - [ ] 7.10 Write property test for UI update rate compliance
    - **Property 18: UI Update Rate Compliance**
    - **Validates: Requirements 5.3**

- [ ] 8. Checkpoint - Ensure all ViewModel tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 9. Implement UI screens
  - [ ] 9.1 Implement MainActivity and NavHost
    - Create single `MainActivity` with Compose content
    - Implement `ParkingNavHost` with fragment-based navigation
    - Handle navigation events from ViewModel
    - _Requirements: 8.1, 8.2_
  
  - [ ] 9.2 Implement MainScreen
    - Display "No active zone" when no zone detected
    - Display zone info when zone is detected but no session
    - Show loading indicator during async operations
    - _Requirements: 2.4, 3.3, 8.4_
  
  - [ ] 9.3 Implement ZoneDetectedScreen
    - Display zone name and operator info
    - Show adapter download progress (0-100%)
    - Handle installation completion transition
    - _Requirements: 4.2, 4.3_
  
  - [ ] 9.4 Implement SessionActiveScreen
    - Display zone name, hourly rate, current duration, current cost
    - Auto-refresh display at 10 updates/second (100ms polling) for responsive UI
    - _Requirements: 5.2, 5.3_
  
  - [ ] 9.5 Implement SessionEndedScreen
    - Display final duration and final cost
    - Provide "Done" button to acknowledge and return to Main
    - _Requirements: 6.2, 6.3_
  
  - [ ] 9.6 Implement ErrorScreen
    - Display user-friendly error message
    - Provide "Retry" button for retryable errors
    - _Requirements: 7.1, 7.2, 7.3_
  
  - [ ] 9.7 Implement shared UI components
    - Create `ProgressIndicator` composable for loading states
    - Create `ErrorMessage` composable for error display
    - _Requirements: 8.4_
  
  - [ ] 9.8 Implement offline overlay
    - Create `OfflineOverlay` composable showing "Parking Service Not Available"
    - Display semi-transparent overlay when gRPC connection lost
    - Auto-dismiss when connection restored
    - _Requirements: 1.3, 1.4_

- [ ] 10. Implement lifecycle handling
  - [ ] 10.1 Implement background/foreground handling
    - Maintain signal subscriptions when backgrounded
    - Refresh session status when foregrounded
    - _Requirements: 10.1, 10.2_
  
  - [ ] 10.2 Write property test for background subscription persistence
    - **Property 14: Background Subscription Persistence**
    - **Validates: Requirements 10.1**
  
  - [ ] 10.3 Write property test for foreground refresh
    - **Property 15: Foreground Status Refresh**
    - **Validates: Requirements 10.2**
  
  - [ ] 10.4 Implement restart session restoration
    - Check for existing active session on cold start
    - Restore UI state from PARKING_OPERATOR_ADAPTOR
    - _Requirements: 10.3, 10.4_
  
  - [ ] 10.5 Write property test for restart restoration
    - **Property 16: Restart Session Restoration**
    - **Validates: Requirements 10.3, 10.4**

- [ ] 11. Implement mock location support
  - [ ] 11.1 Implement mock location mode
    - Add build config flag for mock location mode
    - Implement debug UI for setting mock coordinates
    - Override DATA_BROKER signals when mock mode enabled
    - _Requirements: 9.1, 9.2, 9.3_

- [ ] 12. Implement dependency injection
  - [ ] 12.1 Create Hilt modules
    - Create `NetworkModule` for gRPC channel and Ktor client configuration
    - Create `RepositoryModule` for repository bindings
    - Create `ConfigModule` for AppConfig
    - Wire all dependencies together
    - _Requirements: 8.1_

- [ ] 13. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- All tasks are required including property-based tests
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties using Kotest
- Unit tests validate specific examples and edge cases
- The app uses Jetpack Compose for UI and follows MVVM architecture
- gRPC clients communicate with RHIVOS services over TLS
- REST client communicates with PARKING_FEE_SERVICE over HTTPS
