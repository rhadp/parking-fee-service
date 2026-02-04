# Implementation Plan: COMPANION_APP

## Overview

This implementation plan covers the Flutter/Dart mobile application for the SDV Parking Demo System. The app uses BLoC pattern for state management, Dio for HTTP communication, flutter_secure_storage for credential storage, and go_router for navigation. Property-based tests use the glados package.

## Tasks

- [ ] 1. Set up project structure and dependencies
  - Create Flutter project structure under `android/companion-app/`
  - Configure `pubspec.yaml` with dependencies:
    - flutter_bloc ^8.1.0, dio ^5.0.0, flutter_secure_storage ^9.0.0
    - go_router ^12.0.0, shared_preferences ^2.2.0, equatable ^2.0.0, intl ^0.18.0
    - Dev: bloc_test ^9.1.0, mocktail ^1.0.0, glados ^1.1.0
  - Set up folder structure: lib/config, lib/data, lib/bloc, lib/ui, lib/util, test/unit, test/property
  - Configure environment variables for CLOUD_GATEWAY_URL and DEMO_MODE
  - _Requirements: 9.1, 9.3_

- [ ] 2. Implement data models and configuration
  - [ ] 2.1 Create authentication models
    - Implement `AuthResponse` with accessToken, refreshToken, expiresIn, userId, email
    - Implement `AuthCredentials` with email, password
    - _Requirements: 1.2, 1.3_
  
  - [ ] 2.2 Create vehicle and telemetry models
    - Implement `Vehicle` with vin, name, model
    - Implement `VehicleTelemetry` with vin, latitude, longitude, isLocked, isDoorOpen, parkingSessionActive, activeSession, timestamp
    - Implement `ParkingSession` with sessionId, zoneName, hourlyRate, currency, duration, currentCost
    - _Requirements: 2.2, 3.2, 3.3, 3.4, 3.5_
  
  - [ ] 2.3 Create command models
    - Implement `VehicleCommandType` enum (lock, unlock)
    - Implement `VehicleCommand` with type, doors
    - Implement `CommandResponse` with commandId, status, errorMessage
    - Implement `CommandStatus` with commandId, state, errorMessage, timestamp
    - Implement `CommandState` enum (pending, sent, delivered, executed, failed, timeout)
    - _Requirements: 4.1, 5.1_
  
  - [ ] 2.4 Create AppConfig
    - Implement `AppConfig` with cloudGatewayBaseUrl, timeouts, polling interval, retry config
    - Implement `AppConfig.fromEnvironment()` factory with defaults
    - Configure telemetryPollInterval as 10 seconds per requirements
    - _Requirements: 3.7, 11.3_
  
  - [ ] 2.5 Create exception types
    - Implement `AppException` sealed class with userMessage and isRetryable
    - Implement `NetworkException` - "Unable to connect. Please check your internet connection."
    - Implement `AuthenticationException` - "Invalid credentials. Please try again."
    - Implement `TokenExpiredException` - "Your session has expired. Please log in again."
    - Implement `CommandTimeoutException` - "Command timed out. Please try again."
    - Implement `VehicleOfflineException` - "Vehicle is offline. Please try again later."
    - Implement `UnknownException` - "An unexpected error occurred. Please try again."
    - _Requirements: 7.1, 7.2, 7.3, 7.4_
  
  - [ ] 2.6 Write property test for error message mapping
    - **Property 9: Error Message Mapping**
    - *For any* AppException, userMessage SHALL return a non-empty, user-friendly string without technical details
    - **Validates: Requirements 7.1, 7.2, 7.3, 7.4**

- [ ] 3. Implement REST client layer
  - [ ] 3.1 Implement CloudGatewayClient interface
    - Define abstract `CloudGatewayClient` class with all API methods
    - Define `authenticate(email, password)` returning AuthResponse
    - Define `refreshToken(refreshToken)` returning AuthResponse
    - Define `getVehicles()` returning List<Vehicle>
    - Define `getTelemetry(vin)` returning VehicleTelemetry
    - Define `getParkingSession(vin)` returning ParkingSession (for detailed session info)
    - Define `sendCommand(vin, command)` returning CommandResponse
    - Define `getCommandStatus(vin, commandId)` returning CommandStatus
    - _Requirements: 1.2, 2.1, 3.1, 3.5, 4.1, 5.1_
  
  - [ ] 3.2 Implement DioCloudGatewayClient
    - Create Dio-based implementation of CloudGatewayClient
    - Configure Dio with connectTimeout (10s), receiveTimeout (30s)
    - Implement auth token interceptor for Authorization header injection
    - Map HTTP status codes to AppException types per design
    - _Requirements: 1.2, 2.1, 3.1, 4.1, 5.1, 7.1, 7.2, 7.3, 7.4_
  
  - [ ] 3.3 Implement retry utility
    - Create `retryWithBackoff<T>()` function per design
    - Configure maxAttempts=3, baseDelay=1s, maxDelay=30s
    - Only retry on retryable exceptions
    - _Requirements: 7.5_
  
  - [ ] 3.4 Implement MockCloudGatewayClient for demo mode
    - Create mock implementation returning simulated responses
    - Simulate 2-second delay for commands per requirements
    - Return mock vehicles, telemetry, and command responses
    - _Requirements: 11.1, 11.2_

- [ ] 4. Implement repository layer
  - [ ] 4.1 Implement AuthRepository
    - Create `AuthRepository` with CloudGatewayClient and SecureStorage dependencies
    - Implement `login(email, password)` returning AuthResult (AuthSuccess/AuthFailure)
    - Implement `getValidToken()` with automatic token refresh when expired
    - Implement `logout()` clearing all stored credentials
    - Implement `isAuthenticated()` checking for valid stored token
    - Implement `getUserEmail()` returning stored email
    - _Requirements: 1.2, 1.3, 1.5, 10.2, 10.3_
  
  - [ ] 4.2 Write property test for token storage security
    - **Property 1: Token Storage Security**
    - *For any* successful authentication, tokens SHALL be stored and retrievable exactly as stored
    - **Validates: Requirements 1.3**
  
  - [ ] 4.3 Write property test for logout clears credentials
    - **Property 12: Logout Clears Credentials**
    - *For any* logout action, getValidToken() SHALL return null and isAuthenticated() SHALL return false
    - **Validates: Requirements 10.3**
  
  - [ ] 4.4 Implement VehicleRepository
    - Create `VehicleRepository` with CloudGatewayClient dependency
    - Implement `getVehicles()` returning List<Vehicle>
    - _Requirements: 2.1_
  
  - [ ] 4.5 Write property test for vehicle list display
    - **Property 3: Vehicle List Display**
    - *For any* vehicle list from API, VehicleState SHALL contain all vehicles with VIN and name
    - **Validates: Requirements 2.1, 2.2**
  
  - [ ] 4.6 Implement TelemetryRepository
    - Create `TelemetryRepository` with CloudGatewayClient dependency
    - Implement caching with _cachedTelemetry and _lastFetchTime
    - Implement `getTelemetry(vin)` returning TelemetryResult (TelemetrySuccess/TelemetryFailure)
    - Fetch parking session details when parkingSessionActive is true
    - Implement `getCachedTelemetry(vin)` for offline access
    - Implement `clearCache()` method
    - _Requirements: 3.1, 3.5, 8.3_
  
  - [ ] 4.7 Write property test for telemetry data completeness
    - **Property 4: Telemetry Data Completeness**
    - *For any* telemetry response, TelemetryState SHALL contain latitude, longitude, isLocked, isDoorOpen, parkingSession (if active), timestamp
    - **Validates: Requirements 3.2, 3.3, 3.4**
  
  - [ ] 4.8 Implement CommandRepository
    - Create `CommandRepository` with CloudGatewayClient dependency
    - Implement `sendLockCommand(vin)` returning CommandResult
    - Implement `sendUnlockCommand(vin)` returning CommandResult
    - Implement `getCommandStatus(vin, commandId)` returning CommandStatus
    - _Requirements: 4.1, 5.1_

- [ ] 5. Checkpoint - Ensure all repository tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 6. Implement BLoC layer
  - [ ] 6.1 Implement AuthBloc events and states
    - Create `AuthEvent` sealed class: AuthCheckRequested, AuthLoginRequested, AuthLogoutRequested
    - Create `AuthState` sealed class: AuthInitial, AuthLoading, AuthAuthenticated, AuthUnauthenticated, AuthError
    - _Requirements: 1.1, 1.2, 1.4, 10.3_
  
  - [ ] 6.2 Implement AuthBloc logic
    - Handle AuthCheckRequested: check isAuthenticated, emit AuthAuthenticated or AuthUnauthenticated
    - Handle AuthLoginRequested: call login, emit AuthAuthenticated on success, AuthError on failure
    - Handle AuthLogoutRequested: call logout, emit AuthUnauthenticated
    - _Requirements: 1.1, 1.2, 1.4, 10.3_
  
  - [ ] 6.3 Write property test for authentication state transitions
    - **Property 2: Authentication State Transitions**
    - *For any* auth attempt, valid credentials → AuthAuthenticated, invalid → AuthError
    - **Validates: Requirements 1.2, 1.4**
  
  - [ ] 6.4 Implement VehicleBloc events and states
    - Create `VehicleEvent` sealed class: VehicleListRequested, VehicleSelected
    - Create `VehicleState` sealed class: VehicleInitial, VehicleLoading, VehicleListLoaded, VehicleSelectedState, VehicleError
    - _Requirements: 2.1, 2.2, 2.3, 2.4_
  
  - [ ] 6.5 Implement VehicleBloc logic
    - Handle VehicleListRequested: fetch vehicles, emit VehicleListLoaded or VehicleError
    - Handle VehicleSelected: store selected VIN, emit VehicleSelectedState
    - Handle empty vehicle list case with appropriate message
    - _Requirements: 2.1, 2.2, 2.3, 2.4_
  
  - [ ] 6.6 Implement TelemetryBloc events and states
    - Create `TelemetryEvent` sealed class: TelemetryStartPolling, TelemetryStopPolling, TelemetryRefreshRequested
    - Create `TelemetryState` sealed class: TelemetryInitial, TelemetryLoading, TelemetryLoaded, TelemetryOffline, TelemetryError
    - _Requirements: 3.1, 3.7, 8.1, 8.3_
  
  - [ ] 6.7 Implement TelemetryBloc logic
    - Handle TelemetryStartPolling: start 10-second polling timer
    - Handle TelemetryStopPolling: cancel polling timer
    - Handle TelemetryRefreshRequested: fetch telemetry immediately
    - On NetworkException: emit TelemetryOffline with cached data and lastUpdated timestamp
    - On connectivity restored: auto-refresh telemetry
    - _Requirements: 3.1, 3.7, 8.1, 8.3, 8.4_
  
  - [ ] 6.8 Write property test for telemetry polling interval
    - **Property 5: Telemetry Polling Interval**
    - *For any* active polling session, time between requests SHALL be 10 seconds ± 1 second
    - **Validates: Requirements 3.7**
  
  - [ ] 6.9 Write property test for offline state detection
    - **Property 10: Offline State Detection**
    - *For any* NetworkException during fetch, TelemetryState SHALL be TelemetryOffline with cached data
    - **Validates: Requirements 8.1, 8.3**
  
  - [ ] 6.10 Implement CommandBloc events and states
    - Create `CommandEvent` sealed class: LockRequested, UnlockRequested, CommandConfirmed, CommandCancelled
    - Create `CommandState` sealed class: CommandIdle, CommandConfirming, CommandSending, CommandSuccess, CommandFailure
    - _Requirements: 4.1, 5.1, 6.1, 6.2, 6.3_
  
  - [ ] 6.11 Implement CommandBloc logic
    - Handle LockRequested/UnlockRequested: emit CommandConfirming with command type
    - Handle CommandConfirmed: emit CommandSending, send command, emit CommandSuccess/CommandFailure
    - Handle CommandCancelled: emit CommandIdle without API call
    - On success: trigger TelemetryBloc refresh
    - Support demo mode with 2-second simulated delay
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3, 5.4, 6.1, 6.2, 6.3, 11.1, 11.2_
  
  - [ ] 6.12 Write property test for command confirmation flow
    - **Property 6: Command Confirmation Flow**
    - *For any* command: first CommandConfirming, then CommandSending (if confirmed) or CommandIdle (if cancelled)
    - **Validates: Requirements 6.1, 6.2, 6.3**
  
  - [ ] 6.13 Write property test for command success triggers refresh
    - **Property 7: Command Success Triggers Telemetry Refresh**
    - *For any* successful command, TelemetryBloc SHALL receive refresh event within 1 second
    - **Validates: Requirements 4.4, 5.4**
  
  - [ ] 6.14 Write property test for demo mode simulation
    - **Property 13: Demo Mode Command Simulation**
    - *For any* command in demo mode: no API call, success after 2 seconds ± 100ms
    - **Validates: Requirements 11.1, 11.2**

- [ ] 7. Checkpoint - Ensure all BLoC tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 8. Implement UI layer - Navigation and shared widgets
  - [ ] 8.1 Implement NavigationShell with go_router
    - Create `NavigationShell` widget with bottom navigation bar
    - Configure tabs: "Status" and "Settings" per requirements
    - Configure go_router with routes: /login, /vehicles, /vehicle/:vin, /settings
    - Implement navigation animations for screen transitions
    - _Requirements: 9.1, 9.2_
  
  - [ ] 8.2 Implement shared UI widgets
    - Create `LoadingIndicator` widget for async operations
    - Create `ErrorMessage` widget with message and optional retry button
    - Create `OfflineIndicator` widget for connectivity status
    - Create `ConfirmationDialog` widget for lock/unlock confirmation
    - _Requirements: 7.5, 9.4_

- [ ] 9. Implement UI screens
  - [ ] 9.1 Implement LoginScreen
    - Create login form with email and password TextFields
    - Connect to AuthBloc for authentication
    - Display loading indicator during authentication (AuthLoading state)
    - Display error message on AuthError state
    - Navigate to VehicleSelectScreen on AuthAuthenticated
    - _Requirements: 1.1, 1.4, 9.4_
  
  - [ ] 9.2 Implement VehicleSelectScreen
    - Display list of vehicles with VIN and name from VehicleBloc
    - Handle vehicle selection, store VIN, navigate to VehicleDetailScreen
    - Display "No vehicles found" message when VehicleListLoaded with empty list
    - Display loading indicator during VehicleLoading state
    - _Requirements: 2.2, 2.3, 2.4_
  
  - [ ] 9.3 Implement VehicleDetailScreen - Telemetry display
    - Display vehicle location (latitude, longitude) from TelemetryLoaded state
    - Display door lock status (locked/unlocked)
    - Display parking session status (active/inactive)
    - Display parking session details when active: zone name, duration (HH:MM:SS), current cost with currency
    - Display "Last updated" timestamp when showing cached data
    - Start telemetry polling on screen enter, stop on exit
    - _Requirements: 3.2, 3.3, 3.4, 3.5, 3.6, 8.3_
  
  - [ ] 9.4 Implement VehicleDetailScreen - Command controls
    - Create Lock and Unlock buttons
    - Show confirmation dialog on button tap (CommandConfirming state)
    - Display loading indicator during command (CommandSending state)
    - Display success message on CommandSuccess
    - Display error message with failure reason on CommandFailure
    - Disable buttons when CommandSending or TelemetryOffline
    - _Requirements: 4.1, 4.2, 4.3, 4.5, 4.6, 5.1, 5.2, 5.3, 5.5, 5.6, 6.1, 6.2, 6.3, 8.2_
  
  - [ ] 9.5 Write property test for button disable during command
    - **Property 8: Button Disable During Command**
    - *For any* CommandSending state, lock/unlock buttons SHALL be disabled
    - **Validates: Requirements 4.6, 5.6**
  
  - [ ] 9.6 Write property test for offline button state
    - **Property 11: Offline Button State**
    - *For any* TelemetryOffline state, lock/unlock buttons SHALL be disabled
    - **Validates: Requirements 8.2**
  
  - [ ] 9.7 Write property test for parking session display
    - **Property 14: Parking Session Display**
    - *For any* active parking session, UI SHALL display duration (HH:MM:SS), cost with currency, zone name
    - **Validates: Requirements 3.5, 3.6**
  
  - [ ] 9.8 Implement SettingsScreen
    - Display currently logged-in user email from AuthRepository
    - Display app version from package info
    - Implement Logout button triggering AuthLogoutRequested
    - Navigate to LoginScreen on logout
    - _Requirements: 10.1, 10.2, 10.3, 10.4_

- [ ] 10. Implement offline handling
  - [ ] 10.1 Implement connectivity monitoring
    - Monitor network connectivity state changes
    - Display offline indicator when device loses connectivity
    - Disable lock/unlock buttons when offline
    - _Requirements: 8.1, 8.2_
  
  - [ ] 10.2 Implement auto-refresh on connectivity restore
    - Detect when connectivity is restored
    - Automatically trigger telemetry refresh
    - Re-enable lock/unlock buttons
    - _Requirements: 8.4_

- [ ] 11. Implement state persistence
  - [ ] 11.1 Implement UI state preservation
    - Maintain UI state across configuration changes (rotation)
    - Use BLoC state restoration where applicable
    - _Requirements: 9.3_

- [ ] 12. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- All tasks including property-based tests are required for comprehensive testing
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties using glados (minimum 100 iterations)
- Unit tests validate specific examples and edge cases using bloc_test and mocktail
- The app uses BLoC pattern for state management with flutter_bloc
- REST client communicates with CLOUD_GATEWAY over HTTPS using Dio
- Demo mode allows testing without real vehicle connection (2-second simulated delay)
- Parking session details require separate API call when parkingSessionActive is true
