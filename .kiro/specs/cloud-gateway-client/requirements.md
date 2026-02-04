# Requirements Document

## Introduction

This document defines the requirements for the CLOUD_GATEWAY_CLIENT component of the SDV Parking Demo System. The CLOUD_GATEWAY_CLIENT is an ASIL-B service running in the RHIVOS safety partition that maintains a secure MQTT connection to the CLOUD_GATEWAY, receives authenticated lock/unlock commands from the COMPANION_APP via cloud, and forwards them to the LOCKING_SERVICE.

The service also subscribes to vehicle state changes from the DATA_BROKER and publishes telemetry data (location, door status, parking state) to the cloud for consumption by the COMPANION_APP and backend services.

## Glossary

- **CLOUD_GATEWAY_CLIENT**: ASIL-B service running in the RHIVOS safety partition that bridges cloud commands to local services
- **CLOUD_GATEWAY**: Cloud-side MQTT broker/router that handles vehicle-to-cloud communication
- **LOCKING_SERVICE**: ASIL-B door locking service that executes lock/unlock commands
- **DATA_BROKER**: Eclipse Kuksa Databroker providing VSS-compliant gRPC pub/sub interface for vehicle signals
- **COMPANION_APP**: Mobile application for remote vehicle control (lock/unlock)
- **MQTT**: Message Queuing Telemetry Transport protocol for lightweight pub/sub messaging
- **TLS**: Transport Layer Security for encrypted communication
- **VIN**: Vehicle Identification Number used as unique vehicle identifier
- **UDS**: Unix Domain Sockets for local inter-process communication
- **gRPC**: High-performance RPC framework using Protocol Buffers
- **Command_ID**: Unique identifier for tracking command execution and responses
- **Auth_Token**: Authentication token for command authorization (demo-grade, not production)
- **VSS**: Vehicle Signal Specification - standardized taxonomy for vehicle data from COVESA
- **Telemetry**: Vehicle state data published to cloud (location, door status, parking state)

## Requirements

### Requirement 1: MQTT Connection Management

**User Story:** As a system operator, I want the CLOUD_GATEWAY_CLIENT to maintain a persistent MQTT connection to the cloud, so that remote commands can be received reliably.

#### Acceptance Criteria

1. WHEN the CLOUD_GATEWAY_CLIENT starts THEN the CLOUD_GATEWAY_CLIENT SHALL establish an MQTT connection to the CLOUD_GATEWAY using TLS
2. THE CLOUD_GATEWAY_CLIENT SHALL authenticate to the MQTT broker using configured credentials
3. WHEN the MQTT connection is lost THEN the CLOUD_GATEWAY_CLIENT SHALL attempt reconnection with exponential backoff starting at 1 second and capping at 60 seconds
4. WHEN reconnection succeeds THEN the CLOUD_GATEWAY_CLIENT SHALL resubscribe to all required topics
5. THE CLOUD_GATEWAY_CLIENT SHALL maintain a heartbeat/keepalive with the MQTT broker at the configured interval
6. WHEN the TLS certificate is updated on disk THEN the CLOUD_GATEWAY_CLIENT SHALL detect the change via file system notification and reload the certificate without service restart
7. WHEN certificate reload fails (invalid certificate, permission error) THEN the CLOUD_GATEWAY_CLIENT SHALL continue using the existing certificate and log an error
8. THE CLOUD_GATEWAY_CLIENT SHALL log certificate reload events including success/failure status and certificate expiry date

### Requirement 2: Command Reception

**User Story:** As a vehicle owner using the COMPANION_APP, I want my lock/unlock commands to be received by the vehicle, so that I can remotely control my vehicle's doors.

#### Acceptance Criteria

1. WHEN the MQTT connection is established THEN the CLOUD_GATEWAY_CLIENT SHALL subscribe to the topic `vehicles/{VIN}/commands`
2. WHEN a message is received on the commands topic THEN the CLOUD_GATEWAY_CLIENT SHALL parse the JSON payload into a command structure
3. WHEN the message payload is malformed JSON THEN the CLOUD_GATEWAY_CLIENT SHALL log the error and publish a failure response
4. WHEN the message is missing required fields (command_id, type, auth_token) THEN the CLOUD_GATEWAY_CLIENT SHALL publish a failure response with appropriate error code

### Requirement 3: Command Validation

**User Story:** As a security engineer, I want commands to be validated before execution, so that only authorized commands are processed.

#### Acceptance Criteria

1. WHEN a command is received THEN the CLOUD_GATEWAY_CLIENT SHALL validate the auth_token field
2. WHEN the auth_token is invalid or missing THEN the CLOUD_GATEWAY_CLIENT SHALL reject the command and publish a failure response with error code AUTH_FAILED
3. WHEN the command type is not "lock" or "unlock" THEN the CLOUD_GATEWAY_CLIENT SHALL reject the command and publish a failure response with error code INVALID_COMMAND_TYPE
4. WHEN the doors field contains invalid door identifiers THEN the CLOUD_GATEWAY_CLIENT SHALL reject the command and publish a failure response with error code INVALID_DOOR

### Requirement 4: Command Forwarding to LOCKING_SERVICE

**User Story:** As a system integrator, I want validated commands to be forwarded to the LOCKING_SERVICE, so that door operations are executed by the appropriate safety-critical component.

#### Acceptance Criteria

1. WHEN a valid lock command is received THEN the CLOUD_GATEWAY_CLIENT SHALL forward it to the LOCKING_SERVICE via gRPC over UDS
2. WHEN a valid unlock command is received THEN the CLOUD_GATEWAY_CLIENT SHALL forward it to the LOCKING_SERVICE via gRPC over UDS
3. WHEN the LOCKING_SERVICE returns a success response THEN the CLOUD_GATEWAY_CLIENT SHALL publish a success response to the cloud
4. WHEN the LOCKING_SERVICE returns an error THEN the CLOUD_GATEWAY_CLIENT SHALL publish a failure response with the error details
5. IF the LOCKING_SERVICE is unavailable THEN the CLOUD_GATEWAY_CLIENT SHALL publish a failure response with error code SERVICE_UNAVAILABLE

### Requirement 5: Command Response Publishing

**User Story:** As a vehicle owner, I want to receive feedback on my commands, so that I know whether my lock/unlock request succeeded.

#### Acceptance Criteria

1. WHEN a command completes (success or failure) THEN the CLOUD_GATEWAY_CLIENT SHALL publish a response to `vehicles/{VIN}/command_responses`
2. THE response message SHALL include the original command_id for correlation
3. THE response message SHALL include a status field with value "success" or "failed"
4. WHEN the status is "failed" THEN the response SHALL include error_code and error_message fields
5. THE CLOUD_GATEWAY_CLIENT SHALL publish the response within 5 seconds of receiving the command

### Requirement 6: Vehicle Telemetry Subscription

**User Story:** As a system integrator, I want the CLOUD_GATEWAY_CLIENT to subscribe to vehicle state changes, so that telemetry can be published to the cloud.

#### Acceptance Criteria

1. WHEN the CLOUD_GATEWAY_CLIENT starts THEN the CLOUD_GATEWAY_CLIENT SHALL connect to the DATA_BROKER via gRPC over UDS
2. THE CLOUD_GATEWAY_CLIENT SHALL subscribe to Vehicle.CurrentLocation.Latitude signal from DATA_BROKER
3. THE CLOUD_GATEWAY_CLIENT SHALL subscribe to Vehicle.CurrentLocation.Longitude signal from DATA_BROKER
4. THE CLOUD_GATEWAY_CLIENT SHALL subscribe to Vehicle.Cabin.Door.Row1.DriverSide.IsLocked signal from DATA_BROKER
5. THE CLOUD_GATEWAY_CLIENT SHALL subscribe to Vehicle.Cabin.Door.Row1.DriverSide.IsOpen signal from DATA_BROKER
6. THE CLOUD_GATEWAY_CLIENT SHALL subscribe to Vehicle.Parking.SessionActive signal from DATA_BROKER

### Requirement 7: Telemetry Publishing

**User Story:** As a backend service, I want to receive vehicle telemetry, so that I can track vehicle state and location for parking operations.

#### Acceptance Criteria

1. WHEN a subscribed VSS signal changes THEN the CLOUD_GATEWAY_CLIENT SHALL publish updated telemetry to `vehicles/{VIN}/telemetry`
2. THE telemetry message SHALL include timestamp, location (latitude, longitude), door_locked status, door_open status, and parking_session_active status
3. THE CLOUD_GATEWAY_CLIENT SHALL batch telemetry updates and publish at most once per second to avoid flooding
4. IF the DATA_BROKER connection is lost THEN the CLOUD_GATEWAY_CLIENT SHALL stop publishing telemetry until reconnected
5. WHEN the DATA_BROKER connection is restored THEN the CLOUD_GATEWAY_CLIENT SHALL resume telemetry publishing with current state
6. WHEN the MQTT connection is offline THEN the CLOUD_GATEWAY_CLIENT SHALL buffer telemetry messages up to a maximum of 100 messages or 60 seconds of data, whichever is reached first
7. WHEN the offline buffer is full THEN the CLOUD_GATEWAY_CLIENT SHALL drop the oldest messages to make room for new ones (FIFO eviction)
8. WHEN the MQTT connection is restored THEN the CLOUD_GATEWAY_CLIENT SHALL publish buffered messages in chronological order before resuming real-time publishing

### Requirement 8: Configuration Management

**User Story:** As a system operator, I want the service to be configurable, so that I can deploy it in different environments.

#### Acceptance Criteria

1. THE CLOUD_GATEWAY_CLIENT SHALL load configuration from environment variables
2. THE configuration SHALL include MQTT broker URL, VIN, TLS certificate paths, and socket paths
3. THE CLOUD_GATEWAY_CLIENT SHALL validate configuration on startup and fail fast with clear error messages if invalid
4. THE CLOUD_GATEWAY_CLIENT SHALL support configurable reconnection parameters (initial delay, max delay, max attempts)

### Requirement 9: Graceful Shutdown

**User Story:** As a system operator, I want the service to shut down gracefully, so that in-flight operations complete cleanly.

#### Acceptance Criteria

1. WHEN the CLOUD_GATEWAY_CLIENT receives SIGTERM THEN the CLOUD_GATEWAY_CLIENT SHALL initiate graceful shutdown
2. DURING graceful shutdown THE CLOUD_GATEWAY_CLIENT SHALL complete any in-flight command processing
3. DURING graceful shutdown THE CLOUD_GATEWAY_CLIENT SHALL disconnect from MQTT broker cleanly
4. THE CLOUD_GATEWAY_CLIENT SHALL complete shutdown within 10 seconds of receiving SIGTERM

### Requirement 10: Operation Logging

**User Story:** As a system auditor, I want all operations to be logged, so that I can trace command execution for debugging and compliance.

#### Acceptance Criteria

1. THE CLOUD_GATEWAY_CLIENT SHALL log every received command with timestamp, command_id, command type, and source topic
2. THE CLOUD_GATEWAY_CLIENT SHALL log the result of every command forwarding including success/failure status and any error codes
3. THE CLOUD_GATEWAY_CLIENT SHALL log all MQTT connection state changes (connected, disconnected, reconnecting)
4. THE CLOUD_GATEWAY_CLIENT SHALL log all DATA_BROKER subscription state changes
5. THE CLOUD_GATEWAY_CLIENT SHALL include correlation identifiers in logs to enable end-to-end tracing

## Out of Scope

The following items are explicitly out of scope for this component:

- **UDS Credential Protection**: How Unix Domain Socket credentials are protected is managed at the RHIVOS platform level and is not the responsibility of the CLOUD_GATEWAY_CLIENT. The service assumes the UDS endpoints are secured by the underlying operating system's file permissions and SELinux policies.
