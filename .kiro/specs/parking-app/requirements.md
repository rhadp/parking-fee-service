# Requirements Document

## Introduction

The PARKING_APP is a Kotlin Android Automotive OS (AAOS) application running on the vehicle's In-Vehicle Infotainment (IVI) system. It provides the user interface for parking sessions, displays parking status, orchestrates adapter downloads, and communicates with backend services to enable automatic parking fee payment.

The application serves as the primary driver interface for the SDV Parking Demo System, subscribing to vehicle signals, querying parking zones, managing adapter lifecycle, and displaying session information in a minimal, demo-focused UI.

## Glossary

- **PARKING_APP**: The Android Automotive OS application providing the parking session user interface
- **DATA_BROKER**: Eclipse Kuksa VSS-compliant signal broker providing vehicle signals
- **UPDATE_SERVICE**: RHIVOS service managing container lifecycle for parking adapters
- **PARKING_OPERATOR_ADAPTOR**: Dynamic containerized adapter handling parking sessions
- **PARKING_FEE_SERVICE**: Cloud backend service providing zone lookup and adapter registry
- **Session**: An active parking period with associated rate, duration, and cost
- **Zone**: A geographic parking area with associated operator and pricing
- **Adapter**: A containerized service specific to a parking operator
- **VSS**: Vehicle Signal Specification (COVESA standard)
- **VIN**: Vehicle Identification Number

## Requirements

### Requirement 1: Location Signal Subscription

**User Story:** As a driver, I want the app to automatically detect my parking location, so that the appropriate parking zone can be identified without manual input.

#### Acceptance Criteria

1. WHEN the PARKING_APP starts, THE PARKING_APP SHALL subscribe to Vehicle.CurrentLocation.Latitude and Vehicle.CurrentLocation.Longitude signals from the DATA_BROKER via gRPC over TLS
2. WHEN location signals are received, THE PARKING_APP SHALL store the current latitude and longitude for zone lookup
3. WHEN the DATA_BROKER connection is lost, THE PARKING_APP SHALL attempt to reconnect with exponential backoff up to 5 attempts
4. IF the DATA_BROKER connection cannot be established after 5 attempts, THEN THE PARKING_APP SHALL display an error message indicating signal unavailability

### Requirement 2: Parking State Signal Subscription

**User Story:** As a driver, I want the app to automatically know when a parking session is active, so that I can see real-time parking status.

#### Acceptance Criteria

1. WHEN the PARKING_APP starts, THE PARKING_APP SHALL subscribe to Vehicle.Parking.SessionActive signal from the DATA_BROKER via gRPC over TLS
2. WHEN Vehicle.Parking.SessionActive changes to true, THE PARKING_APP SHALL transition to the Session Active screen
3. WHEN Vehicle.Parking.SessionActive changes to false while a session was active, THE PARKING_APP SHALL transition to the Session Ended screen
4. WHEN Vehicle.Parking.SessionActive is false and no previous session exists, THE PARKING_APP SHALL display the Main Screen with "No active zone" status

### Requirement 3: Zone Lookup

**User Story:** As a driver, I want the app to identify the parking zone based on my location, so that the correct parking operator adapter can be loaded.

#### Acceptance Criteria

1. WHEN valid location coordinates are available, THE PARKING_APP SHALL query the PARKING_FEE_SERVICE zone lookup endpoint via HTTPS/REST
2. WHEN a zone is found, THE PARKING_APP SHALL store the zone_id, operator_name, hourly_rate, and adapter_image_ref
3. WHEN no zone is found (HTTP 404), THE PARKING_APP SHALL display "No parking zone detected" on the Main Screen
4. IF the zone lookup request fails due to network error, THEN THE PARKING_APP SHALL retry up to 3 times with exponential backoff
5. IF all zone lookup retries fail, THEN THE PARKING_APP SHALL display an error message with a retry option

### Requirement 4: Adapter Installation Request

**User Story:** As a driver, I want the app to automatically download the required parking adapter, so that parking sessions can be managed for the current zone.

#### Acceptance Criteria

1. WHEN a zone is detected and the required adapter is not installed, THE PARKING_APP SHALL request adapter installation from the UPDATE_SERVICE via gRPC over TLS
2. WHEN an adapter installation is requested, THE PARKING_APP SHALL display the Zone Detected screen with download progress
3. WHEN the UPDATE_SERVICE reports installation progress, THE PARKING_APP SHALL update the progress indicator (0-100%)
4. WHEN the adapter installation completes successfully, THE PARKING_APP SHALL transition to monitoring for session activation
5. IF the adapter installation fails, THEN THE PARKING_APP SHALL display an error message with a retry option

### Requirement 5: Session Status Display

**User Story:** As a driver, I want to see my current parking session details, so that I know the rate, duration, and cost of my parking.

#### Acceptance Criteria

1. WHEN a parking session is active, THE PARKING_APP SHALL query the PARKING_OPERATOR_ADAPTOR for session status via gRPC over TLS
2. WHEN session status is received, THE PARKING_APP SHALL display the zone name, hourly rate, current duration, and current cost on the Session Active screen
3. WHILE a session is active, THE PARKING_APP SHALL poll for updated session status every 30 seconds
4. WHEN the session status indicates an error state, THE PARKING_APP SHALL display the error message on the Error Screen

### Requirement 6: Session Ended Display

**User Story:** As a driver, I want to see a summary when my parking session ends, so that I know the final cost and duration.

#### Acceptance Criteria

1. WHEN a parking session ends, THE PARKING_APP SHALL query the PARKING_OPERATOR_ADAPTOR for final session details
2. WHEN final session details are received, THE PARKING_APP SHALL display the final duration and final cost on the Session Ended screen
3. WHEN the user acknowledges the session summary, THE PARKING_APP SHALL return to the Main Screen

### Requirement 7: Error Handling and Display

**User Story:** As a driver, I want to see clear error messages when something goes wrong, so that I understand the issue and can take action.

#### Acceptance Criteria

1. WHEN any service communication fails, THE PARKING_APP SHALL display a user-friendly error message on the Error Screen
2. WHEN an error is displayed, THE PARKING_APP SHALL provide a "Retry" button to attempt the failed operation again
3. WHEN the user taps "Retry", THE PARKING_APP SHALL re-attempt the failed operation
4. THE PARKING_APP SHALL map technical error codes to user-friendly messages

### Requirement 8: UI Navigation

**User Story:** As a driver, I want smooth navigation between screens, so that I can easily understand the parking status.

#### Acceptance Criteria

1. THE PARKING_APP SHALL implement a single Activity with Fragment-based navigation
2. WHEN transitioning between screens, THE PARKING_APP SHALL use appropriate navigation animations
3. THE PARKING_APP SHALL maintain UI state across configuration changes (rotation, etc.)
4. THE PARKING_APP SHALL display a loading indicator during asynchronous operations

### Requirement 9: Mock Location Support

**User Story:** As a demo user, I want to simulate different parking locations, so that I can demonstrate the parking flow without physically moving the vehicle.

#### Acceptance Criteria

1. WHERE mock location mode is enabled, THE PARKING_APP SHALL accept simulated location coordinates instead of real DATA_BROKER signals
2. WHERE mock location mode is enabled, THE PARKING_APP SHALL provide a debug UI to set latitude and longitude values
3. THE PARKING_APP SHALL read mock location mode configuration from build settings

### Requirement 10: Application Lifecycle

**User Story:** As a driver, I want the app to handle lifecycle events properly, so that parking status is preserved when the app is backgrounded or restarted.

#### Acceptance Criteria

1. WHEN the PARKING_APP is backgrounded, THE PARKING_APP SHALL maintain signal subscriptions
2. WHEN the PARKING_APP is foregrounded, THE PARKING_APP SHALL refresh the current parking status
3. WHEN the PARKING_APP is terminated and restarted, THE PARKING_APP SHALL restore the current session state from the PARKING_OPERATOR_ADAPTOR
4. WHEN the PARKING_APP starts, THE PARKING_APP SHALL check for an existing active session before subscribing to new signals
