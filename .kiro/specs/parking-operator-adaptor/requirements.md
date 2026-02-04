# Requirements Document

## Introduction

This document defines the requirements for the PARKING_OPERATOR_ADAPTOR component of the SDV Parking Demo System. The PARKING_OPERATOR_ADAPTOR is a containerized Rust service running in the RHIVOS QM partition that implements parking session management.

The service subscribes to lock/unlock events from the DATA_BROKER, automatically starts and stops parking sessions based on vehicle lock state, communicates with external parking operators via REST API, and provides session status to the PARKING_APP via gRPC.

## Glossary

- **PARKING_OPERATOR_ADAPTOR**: Containerized parking session management service running in the RHIVOS QM partition
- **PARKING_APP**: Android Automotive OS IVI application that displays parking session status
- **PARKING_OPERATOR**: External parking operator REST API for session management and payment
- **DATA_BROKER**: Eclipse Kuksa Databroker providing VSS-compliant gRPC pub/sub interface for vehicle signals
- **VSS**: Vehicle Signal Specification - standardized taxonomy for vehicle data from COVESA
- **Session**: A parking session with start time, location, and billing information
- **Session_ID**: Unique identifier for a parking session returned by the PARKING_OPERATOR
- **Zone_ID**: Identifier for a parking zone determined by vehicle location
- **gRPC**: High-performance RPC framework using Protocol Buffers
- **TLS**: Transport Layer Security for encrypted cross-domain communication
- **UDS**: Unix Domain Sockets for local inter-process communication
- **REST**: Representational State Transfer - HTTP-based API protocol

## Requirements

### Requirement 1: Lock Event Subscription

**User Story:** As a vehicle owner, I want parking sessions to start automatically when I lock my vehicle, so that I don't have to manually initiate parking.

#### Acceptance Criteria

1. WHEN the PARKING_OPERATOR_ADAPTOR starts THEN it SHALL subscribe to Vehicle.Cabin.Door.Row1.DriverSide.IsLocked signal from the DATA_BROKER via gRPC over UDS
2. WHEN the IsLocked signal changes from false to true THEN the PARKING_OPERATOR_ADAPTOR SHALL initiate a parking session start
3. WHEN the IsLocked signal changes from true to false THEN the PARKING_OPERATOR_ADAPTOR SHALL initiate a parking session stop
4. IF the DATA_BROKER connection is lost THEN the PARKING_OPERATOR_ADAPTOR SHALL attempt to reconnect with exponential backoff
5. IF reconnection fails after 5 attempts THEN the PARKING_OPERATOR_ADAPTOR SHALL log the failure and enter a degraded state

### Requirement 2: Location Reading

**User Story:** As a parking system, I want to know the vehicle's location when a session starts, so that I can determine the correct parking zone and rates.

#### Acceptance Criteria

1. WHEN initiating a parking session THEN the PARKING_OPERATOR_ADAPTOR SHALL read Vehicle.CurrentLocation.Latitude from the DATA_BROKER
2. WHEN initiating a parking session THEN the PARKING_OPERATOR_ADAPTOR SHALL read Vehicle.CurrentLocation.Longitude from the DATA_BROKER
3. IF location signals are unavailable THEN the PARKING_OPERATOR_ADAPTOR SHALL reject the session start and return an error indicating location is required
4. THE PARKING_OPERATOR_ADAPTOR SHALL receive the Zone_ID from the PARKING_APP via the StartSession request (the PARKING_APP obtains the Zone_ID from the PARKING_FEE_SERVICE based on location)

### Requirement 3: Session Start

**User Story:** As a vehicle owner, I want the system to communicate with the parking operator when I lock my vehicle, so that my parking session is officially registered.

#### Acceptance Criteria

1. WHEN a parking session start is initiated THEN the PARKING_OPERATOR_ADAPTOR SHALL call POST /parking/start on the PARKING_OPERATOR API
2. THE start request SHALL include vehicle_id, latitude, longitude, zone_id, and timestamp
3. WHEN the PARKING_OPERATOR returns a successful response THEN the PARKING_OPERATOR_ADAPTOR SHALL store the Session_ID
4. WHEN a session starts successfully THEN the PARKING_OPERATOR_ADAPTOR SHALL publish Vehicle.Parking.SessionActive = true to the DATA_BROKER
5. IF the PARKING_OPERATOR API call fails THEN the PARKING_OPERATOR_ADAPTOR SHALL retry up to 3 times with exponential backoff
6. IF all retries fail THEN the PARKING_OPERATOR_ADAPTOR SHALL set session state to ERROR and include the failure reason

### Requirement 4: Session Stop

**User Story:** As a vehicle owner, I want the system to end my parking session when I unlock my vehicle, so that I am only charged for the time I was parked.

#### Acceptance Criteria

1. WHEN a parking session stop is initiated THEN the PARKING_OPERATOR_ADAPTOR SHALL call POST /parking/stop on the PARKING_OPERATOR API
2. THE stop request SHALL include the Session_ID and timestamp
3. WHEN the PARKING_OPERATOR returns a successful response THEN the PARKING_OPERATOR_ADAPTOR SHALL update the session with final cost and duration
4. WHEN a session stops successfully THEN the PARKING_OPERATOR_ADAPTOR SHALL publish Vehicle.Parking.SessionActive = false to the DATA_BROKER
5. IF the PARKING_OPERATOR API call fails THEN the PARKING_OPERATOR_ADAPTOR SHALL retry up to 3 times with exponential backoff
6. IF all retries fail THEN the PARKING_OPERATOR_ADAPTOR SHALL set session state to ERROR and preserve the session for manual resolution

### Requirement 5: Session Status Query

**User Story:** As a vehicle owner using the PARKING_APP, I want to see the current status of my parking session, so that I know how long I've been parked and the current cost.

#### Acceptance Criteria

1. WHEN the PARKING_OPERATOR_ADAPTOR receives a GetSessionStatus request THEN it SHALL return the current session information
2. THE GetSessionStatusResponse SHALL include session_id, state, start_time, duration, current_cost, zone_id, and error_message (if applicable)
3. WHEN no active session exists THEN the PARKING_OPERATOR_ADAPTOR SHALL return a response indicating no active session
4. THE PARKING_OPERATOR_ADAPTOR SHALL periodically poll GET /parking/status/{session_id} to update session cost during an active session

### Requirement 6: Manual Session Control

**User Story:** As a vehicle owner, I want to manually start or stop a parking session through the PARKING_APP, so that I have control over my parking in edge cases.

#### Acceptance Criteria

1. WHEN the PARKING_OPERATOR_ADAPTOR receives a StartSession request THEN it SHALL initiate a parking session start regardless of lock state
2. WHEN the PARKING_OPERATOR_ADAPTOR receives a StopSession request THEN it SHALL initiate a parking session stop regardless of lock state
3. WHEN a manual StartSession is requested while a session is already active THEN the PARKING_OPERATOR_ADAPTOR SHALL return an error indicating a session is already in progress
4. WHEN a manual StopSession is requested while no session is active THEN the PARKING_OPERATOR_ADAPTOR SHALL return an error indicating no active session exists

### Requirement 7: Session State Management

**User Story:** As a system operator, I want the adaptor to track session state reliably, so that sessions are not lost or duplicated.

#### Acceptance Criteria

1. THE PARKING_OPERATOR_ADAPTOR SHALL track session state using the following states: NONE, STARTING, ACTIVE, STOPPING, STOPPED, ERROR
2. WHEN a session state changes THEN the PARKING_OPERATOR_ADAPTOR SHALL record the timestamp of the state change
3. THE PARKING_OPERATOR_ADAPTOR SHALL persist session state to survive container restarts
4. WHEN the PARKING_OPERATOR_ADAPTOR restarts with an ACTIVE session THEN it SHALL resume tracking and polling for that session
5. THE PARKING_OPERATOR_ADAPTOR SHALL prevent concurrent session operations (no start while stopping, no stop while starting)

### Requirement 8: gRPC Service Interface

**User Story:** As a developer, I want a well-defined gRPC interface, so that I can integrate with the PARKING_OPERATOR_ADAPTOR from the PARKING_APP.

#### Acceptance Criteria

1. THE PARKING_OPERATOR_ADAPTOR SHALL expose a StartSession RPC that accepts StartSessionRequest and returns StartSessionResponse
2. THE PARKING_OPERATOR_ADAPTOR SHALL expose a StopSession RPC that accepts StopSessionRequest and returns StopSessionResponse
3. THE PARKING_OPERATOR_ADAPTOR SHALL expose a GetSessionStatus RPC that accepts GetSessionStatusRequest and returns GetSessionStatusResponse
4. THE PARKING_OPERATOR_ADAPTOR SHALL listen on a TCP socket with TLS for gRPC connections from PARKING_APP
5. THE PARKING_OPERATOR_ADAPTOR SHALL use standard gRPC status codes with custom error details for domain-specific errors

### Requirement 9: PARKING_OPERATOR API Integration

**User Story:** As a system integrator, I want the adaptor to communicate with external parking operators via REST, so that parking sessions are registered with the operator.

#### Acceptance Criteria

1. THE PARKING_OPERATOR_ADAPTOR SHALL communicate with the PARKING_OPERATOR via HTTPS/REST
2. THE PARKING_OPERATOR_ADAPTOR SHALL support configurable PARKING_OPERATOR base URL
3. THE PARKING_OPERATOR_ADAPTOR SHALL include appropriate headers (Content-Type, Authorization) in API requests
4. THE PARKING_OPERATOR_ADAPTOR SHALL handle HTTP error responses (4xx, 5xx) and map them to appropriate session errors
5. THE PARKING_OPERATOR_ADAPTOR SHALL implement request timeout of 10 seconds for all PARKING_OPERATOR API calls

### Requirement 10: Configuration

**User Story:** As a system operator, I want the adaptor to be configurable, so that it can be deployed in different environments.

#### Acceptance Criteria

1. THE PARKING_OPERATOR_ADAPTOR SHALL support configuration via environment variables
2. THE PARKING_OPERATOR_ADAPTOR SHALL be configurable for: gRPC listen address, TLS certificate paths, DATA_BROKER socket path, PARKING_OPERATOR base URL, vehicle_id, and hourly rate
3. THE PARKING_OPERATOR_ADAPTOR SHALL provide sensible defaults for all configuration options
4. WHEN required configuration is missing THEN the PARKING_OPERATOR_ADAPTOR SHALL fail to start with a clear error message

### Requirement 11: Operation Logging

**User Story:** As a system auditor, I want all parking operations to be logged, so that I can trace operations for debugging and compliance.

#### Acceptance Criteria

1. THE PARKING_OPERATOR_ADAPTOR SHALL log every received gRPC request with timestamp, request type, and relevant parameters
2. THE PARKING_OPERATOR_ADAPTOR SHALL log all session state transitions with previous state, new state, and reason
3. THE PARKING_OPERATOR_ADAPTOR SHALL log all PARKING_OPERATOR API calls with request details and response status
4. THE PARKING_OPERATOR_ADAPTOR SHALL log all DATA_BROKER signal subscriptions and publications
5. THE PARKING_OPERATOR_ADAPTOR SHALL include correlation identifiers in logs to enable end-to-end tracing
