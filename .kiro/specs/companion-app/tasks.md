# Implementation Plan: COMPANION_APP

## Overview

This implementation plan covers the Flutter/Dart mobile application for the SDV Parking Demo System. The app uses BLoC pattern for state management, Dio for HTTP communication, and flutter_secure_storage for credential storage.

## Tasks

- [ ] 1. Set up project structure and dependencies
  - Create Flutter project structure under `android/companion-app/`
  - Configure `pubspec.yaml` with flutter_bloc, dio, flutter_secure_storage, go_router, and glados dependencies
  - Set up project folder structure (lib/data, lib/bloc, lib/ui, test/)
  - Configure environment variables for CLOUD_GATEWAY_URL and DEMO_MODE
  - _Requirements: 9.1_

- [ ] 2. Implement data models and configuration
  - [ ] 2.1 Create data models
    - Implement `AuthResponse`, `AuthCredentials` classes
    - Implement `Vehicle`, `VehicleTelemetry`, `ParkingSession` classes
    - Implement `VehicleCommand`, `CommandResponse`, `CommandStatus`, `CommandState` classes
    - _Requirements: 1.2, 2.2, 3.2, 4.1, 5.1_
  
  - [ ] 2.2 Create AppConfig
    - Implement `AppConfig` with connection settings, timeouts, and demo mode flag
    - Implement `AppConfig.fromEnvironment()` factory
    - _Requirements: 11.3_
  
  - [ ] 2.3 Create exception types
    - Implement `AppException` sealed class hierarchy
    - Implement `NetworkException`, `AuthenticationException`, `TokenExpiredException`
    - Implement `CommandTimeoutException`, `VehicleOfflineException`, `UnknownException`
    - _Requirements: 7.1, 7.2, 7.3, 7.4_
  
  - [ ] 2.4 Write property test for error message mapping
    - **Property 9: Error Message Mapping**
    - **Validates: Requirements 7.1, 7.2, 7.3, 7.4**


- [ ] 3. Implement REST client
  - [ ] 3.1 Implement CloudGatewayClient
    - Create `CloudGatewayClient` abstract class and Dio implementation
    - Implement `authenticate()` and `refreshToken()` methods
    - Implement `getVehicles()` method
    - Implement `getTelemetry()` method
    - Implement `sendCommand()` and `getCommandStatus()` methods
    - Configure Dio interceptors for auth token injection
    - _Requirements: 1.2, 2.1, 3.1, 4.1, 5.1_
  
  - [ ] 3.2 Implement mock client for demo mode
    - Create `MockCloudGatewayClient` implementation
    - Return simulated responses with configurable delay
    - _Requirements: 11.1, 11.2_

- [ ] 4. Implement repository layer
  - [ ] 4.1 Implement AuthRepository
    - Create `AuthRepository` with secure storage integration
    - Implement `login()`, `logout()`, `isAuthenticated()` methods
    - Implement `getValidToken()` with automatic refresh
    - Implement `getUserEmail()` method
    - _Requirements: 1.2, 1.3, 1.5, 10.3_
  
  - [ ] 4.2 Write property test for token storage
    - **Property 1: Token Storage Security**
    - **Validates: Requirements 1.3**
  
  - [ ] 4.3 Write property test for logout clears credentials
    - **Property 12: Logout Clears Credentials**
    - **Validates: Requirements 10.3**
  
  - [ ] 4.4 Implement VehicleRepository
    - Create `VehicleRepository` for vehicle list retrieval
    - Implement `getVehicles()` method
    - _Requirements: 2.1_
  
  - [ ] 4.5 Write property test for vehicle list
    - **Property 3: Vehicle List Display**
    - **Validates: Requirements 2.1, 2.2**
  
  - [ ] 4.6 Implement TelemetryRepository
    - Create `TelemetryRepository` with caching support
    - Implement `getTelemetry()` with cache check
    - Implement `getCachedTelemetry()` and `clearCache()` methods
    - _Requirements: 3.1, 8.3_
  
  - [ ] 4.7 Write property test for telemetry completeness
    - **Property 4: Telemetry Data Completeness**
    - **Validates: Requirements 3.2, 3.3, 3.4**
  
  - [ ] 4.8 Implement CommandRepository
    - Create `CommandRepository` for command operations
    - Implement `sendLockCommand()` and `sendUnlockCommand()` methods
    - Implement `getCommandStatus()` method
    - _Requirements: 4.1, 5.1_

- [ ] 5. Checkpoint - Ensure all repository tests pass
  - Ensure all tests pass, ask the user if questions arise.


- [ ] 6. Implement BLoC layer
  - [ ] 6.1 Implement AuthBloc
    - Create `AuthBloc` with events and states
    - Handle `AuthCheckRequested`, `AuthLoginRequested`, `AuthLogoutRequested` events
    - Emit appropriate states based on repository results
    - _Requirements: 1.1, 1.2, 1.4, 10.3_
  
  - [ ] 6.2 Write property test for authentication state transitions
    - **Property 2: Authentication State Transitions**
    - **Validates: Requirements 1.2, 1.4**
  
  - [ ] 6.3 Implement VehicleBloc
    - Create `VehicleBloc` with events and states
    - Handle `VehicleListRequested`, `VehicleSelected` events
    - Store selected vehicle for subsequent operations
    - _Requirements: 2.1, 2.2, 2.3, 2.4_
  
  - [ ] 6.4 Implement TelemetryBloc
    - Create `TelemetryBloc` with events and states
    - Handle `TelemetryStartPolling`, `TelemetryStopPolling`, `TelemetryRefreshRequested` events
    - Implement 10-second polling interval
    - Handle offline state with cached telemetry
    - _Requirements: 3.1, 3.6, 8.1, 8.3, 8.4_
  
  - [ ] 6.5 Write property test for telemetry polling interval
    - **Property 5: Telemetry Polling Interval**
    - **Validates: Requirements 3.6**
  
  - [ ] 6.6 Write property test for offline state detection
    - **Property 10: Offline State Detection**
    - **Validates: Requirements 8.1, 8.3**
  
  - [ ] 6.7 Implement CommandBloc
    - Create `CommandBloc` with events and states
    - Handle `LockRequested`, `UnlockRequested`, `CommandConfirmed`, `CommandCancelled` events
    - Implement confirmation flow before sending commands
    - Trigger telemetry refresh on command success
    - _Requirements: 4.1, 4.2, 4.3, 5.1, 5.2, 5.3, 6.1, 6.2, 6.3_
  
  - [ ] 6.8 Write property test for command confirmation flow
    - **Property 6: Command Confirmation Flow**
    - **Validates: Requirements 6.1, 6.2, 6.3**
  
  - [ ] 6.9 Write property test for command success triggers refresh
    - **Property 7: Command Success Triggers Telemetry Refresh**
    - **Validates: Requirements 4.4, 5.4**
  
  - [ ] 6.10 Write property test for demo mode simulation
    - **Property 13: Demo Mode Command Simulation**
    - **Validates: Requirements 11.1, 11.2**

- [ ] 7. Checkpoint - Ensure all BLoC tests pass
  - Ensure all tests pass, ask the user if questions arise.


- [ ] 8. Implement UI screens
  - [ ] 8.1 Implement NavigationShell and routing
    - Create `NavigationShell` with bottom navigation bar
    - Configure go_router with routes for all screens
    - Implement navigation animations
    - _Requirements: 9.1, 9.2_
  
  - [ ] 8.2 Implement LoginScreen
    - Create login form with email and password fields
    - Display loading indicator during authentication
    - Display error messages for failed authentication
    - _Requirements: 1.1, 1.4, 9.4_
  
  - [ ] 8.3 Implement VehicleSelectScreen
    - Display list of vehicles with VIN and name
    - Handle vehicle selection
    - Display "No vehicles found" message when list is empty
    - _Requirements: 2.2, 2.4_
  
  - [ ] 8.4 Implement VehicleDetailScreen
    - Display vehicle telemetry (location, lock status, door status)
    - Display parking session info when active
    - Implement lock/unlock buttons with confirmation dialogs
    - Display loading indicator during commands
    - Disable buttons when offline or command in progress
    - _Requirements: 3.2, 3.3, 3.4, 3.5, 4.2, 4.6, 5.2, 5.6, 6.1, 8.2_
  
  - [ ] 8.5 Write property test for button disable during command
    - **Property 8: Button Disable During Command**
    - **Validates: Requirements 4.6, 5.6**
  
  - [ ] 8.6 Write property test for offline button state
    - **Property 11: Offline Button State**
    - **Validates: Requirements 8.2**
  
  - [ ] 8.7 Write property test for parking session display
    - **Property 14: Parking Session Display**
    - **Validates: Requirements 3.5**
  
  - [ ] 8.8 Implement SettingsScreen
    - Display logged-in user email
    - Display app version
    - Implement logout button
    - _Requirements: 10.1, 10.2, 10.3, 10.4_
  
  - [ ] 8.9 Implement shared UI widgets
    - Create `LoadingIndicator` widget
    - Create `ErrorMessage` widget with retry button
    - Create `TelemetryCard` widget for telemetry display
    - Create `CommandButtons` widget for lock/unlock
    - _Requirements: 7.5, 9.4_

- [ ] 9. Implement offline handling
  - [ ] 9.1 Implement connectivity monitoring
    - Monitor network connectivity state
    - Display offline indicator when disconnected
    - Auto-refresh telemetry when connectivity restored
    - _Requirements: 8.1, 8.4_

- [ ] 10. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- All tasks are required including property-based tests
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties using glados
- Unit tests validate specific examples and edge cases
- The app uses BLoC pattern for state management
- REST client communicates with CLOUD_GATEWAY over HTTPS
- Demo mode allows testing without real vehicle connection
