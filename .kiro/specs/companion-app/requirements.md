# Requirements Document

## Introduction

The COMPANION_APP is a Flutter/Dart mobile application (Android/iOS) that enables remote vehicle interaction for the SDV Parking Demo System. It allows users to remotely lock/unlock their vehicle and view vehicle telemetry (location, door status, parking state) by communicating with the CLOUD_GATEWAY via HTTPS/REST.

The application serves as the mobile companion to the in-vehicle PARKING_APP, enabling the demo scenario where users can lock their vehicle remotely to start a parking session and unlock to end it.

## Glossary

- **COMPANION_APP**: The Flutter mobile application for remote vehicle control
- **CLOUD_GATEWAY**: Cloud-based service routing commands between mobile apps and vehicles
- **VIN**: Vehicle Identification Number used to identify the target vehicle
- **Telemetry**: Vehicle state data including location, door status, and parking state
- **Command**: A remote action request (lock/unlock) sent to the vehicle
- **Session**: An active parking period managed by the vehicle's parking system

## Requirements

### Requirement 1: User Authentication

**User Story:** As a user, I want to authenticate with my credentials, so that I can securely access my vehicle's remote features.

#### Acceptance Criteria

1. WHEN the COMPANION_APP starts, THE COMPANION_APP SHALL display a login screen if no valid authentication token exists
2. WHEN the user enters valid credentials, THE COMPANION_APP SHALL authenticate with the CLOUD_GATEWAY via HTTPS/REST
3. WHEN authentication succeeds, THE COMPANION_APP SHALL store the authentication token securely
4. WHEN authentication fails, THE COMPANION_APP SHALL display an error message indicating invalid credentials
5. THE COMPANION_APP SHALL support token refresh to maintain session validity

### Requirement 2: Vehicle Selection

**User Story:** As a user, I want to select my vehicle from a list, so that I can control the correct vehicle.

#### Acceptance Criteria

1. WHEN the user is authenticated, THE COMPANION_APP SHALL query the CLOUD_GATEWAY for the list of vehicles associated with the user
2. WHEN vehicles are retrieved, THE COMPANION_APP SHALL display a vehicle selection screen with VIN and vehicle name
3. WHEN the user selects a vehicle, THE COMPANION_APP SHALL store the selected VIN for subsequent operations
4. IF no vehicles are associated with the user, THEN THE COMPANION_APP SHALL display a message indicating no vehicles found

### Requirement 3: Vehicle Telemetry Display

**User Story:** As a user, I want to see my vehicle's current status, so that I know its location, door state, and parking status.

#### Acceptance Criteria

1. WHEN a vehicle is selected, THE COMPANION_APP SHALL query the CLOUD_GATEWAY for current vehicle telemetry
2. WHEN telemetry is received, THE COMPANION_APP SHALL display the vehicle's current location (latitude, longitude)
3. WHEN telemetry is received, THE COMPANION_APP SHALL display the door lock status (locked/unlocked)
4. WHEN telemetry is received, THE COMPANION_APP SHALL display the parking session status (active/inactive)
5. IF a parking session is active, THEN THE COMPANION_APP SHALL display the session duration and current cost
6. THE COMPANION_APP SHALL refresh telemetry automatically every 10 seconds while on the vehicle status screen

### Requirement 4: Remote Lock Command

**User Story:** As a user, I want to remotely lock my vehicle, so that I can start a parking session without being at the vehicle.

#### Acceptance Criteria

1. WHEN the user taps the "Lock" button, THE COMPANION_APP SHALL send a lock command to the CLOUD_GATEWAY via HTTPS/REST
2. WHEN the lock command is sent, THE COMPANION_APP SHALL display a loading indicator
3. WHEN the CLOUD_GATEWAY acknowledges the command, THE COMPANION_APP SHALL display a success message
4. WHEN the lock command succeeds, THE COMPANION_APP SHALL refresh the vehicle telemetry to show updated door status
5. IF the lock command fails, THEN THE COMPANION_APP SHALL display an error message with the failure reason
6. THE COMPANION_APP SHALL disable the lock button while a command is in progress

### Requirement 5: Remote Unlock Command

**User Story:** As a user, I want to remotely unlock my vehicle, so that I can end a parking session without being at the vehicle.

#### Acceptance Criteria

1. WHEN the user taps the "Unlock" button, THE COMPANION_APP SHALL send an unlock command to the CLOUD_GATEWAY via HTTPS/REST
2. WHEN the unlock command is sent, THE COMPANION_APP SHALL display a loading indicator
3. WHEN the CLOUD_GATEWAY acknowledges the command, THE COMPANION_APP SHALL display a success message
4. WHEN the unlock command succeeds, THE COMPANION_APP SHALL refresh the vehicle telemetry to show updated door status
5. IF the unlock command fails, THEN THE COMPANION_APP SHALL display an error message with the failure reason
6. THE COMPANION_APP SHALL disable the unlock button while a command is in progress

### Requirement 6: Command Confirmation

**User Story:** As a user, I want to confirm before sending lock/unlock commands, so that I don't accidentally trigger vehicle actions.

#### Acceptance Criteria

1. WHEN the user taps "Lock" or "Unlock", THE COMPANION_APP SHALL display a confirmation dialog
2. WHEN the user confirms the action, THE COMPANION_APP SHALL proceed with sending the command
3. WHEN the user cancels the action, THE COMPANION_APP SHALL dismiss the dialog without sending a command

### Requirement 7: Error Handling

**User Story:** As a user, I want to see clear error messages when something goes wrong, so that I understand the issue and can take action.

#### Acceptance Criteria

1. WHEN any API call fails due to network error, THE COMPANION_APP SHALL display "Unable to connect. Please check your internet connection."
2. WHEN authentication expires, THE COMPANION_APP SHALL redirect to the login screen with a message
3. WHEN a command times out, THE COMPANION_APP SHALL display "Command timed out. Please try again."
4. WHEN the vehicle is unreachable, THE COMPANION_APP SHALL display "Vehicle is offline. Please try again later."
5. THE COMPANION_APP SHALL provide a "Retry" option for retryable errors

### Requirement 8: Offline Handling

**User Story:** As a user, I want the app to handle offline scenarios gracefully, so that I understand when features are unavailable.

#### Acceptance Criteria

1. WHEN the device loses network connectivity, THE COMPANION_APP SHALL display an offline indicator
2. WHEN offline, THE COMPANION_APP SHALL disable lock/unlock buttons
3. WHEN offline, THE COMPANION_APP SHALL display cached telemetry with a "Last updated" timestamp
4. WHEN connectivity is restored, THE COMPANION_APP SHALL automatically refresh telemetry

### Requirement 9: UI Navigation

**User Story:** As a user, I want smooth navigation between screens, so that I can easily access vehicle controls.

#### Acceptance Criteria

1. THE COMPANION_APP SHALL implement a bottom navigation bar with "Status" and "Settings" tabs
2. WHEN transitioning between screens, THE COMPANION_APP SHALL use appropriate navigation animations
3. THE COMPANION_APP SHALL maintain UI state across configuration changes (rotation, etc.)
4. THE COMPANION_APP SHALL display a loading indicator during asynchronous operations

### Requirement 10: Settings and Logout

**User Story:** As a user, I want to manage my settings and log out, so that I can control my account and preferences.

#### Acceptance Criteria

1. THE COMPANION_APP SHALL provide a Settings screen accessible from the bottom navigation
2. THE COMPANION_APP SHALL display the currently logged-in user's email on the Settings screen
3. THE COMPANION_APP SHALL provide a "Logout" button that clears stored credentials and returns to login
4. THE COMPANION_APP SHALL display the app version on the Settings screen

### Requirement 11: Demo Mode Support

**User Story:** As a demo user, I want to use the app with simulated responses, so that I can demonstrate the flow without a real vehicle connection.

#### Acceptance Criteria

1. WHERE demo mode is enabled, THE COMPANION_APP SHALL use mock API responses instead of real CLOUD_GATEWAY calls
2. WHERE demo mode is enabled, THE COMPANION_APP SHALL simulate command success after a 2-second delay
3. THE COMPANION_APP SHALL read demo mode configuration from build settings
