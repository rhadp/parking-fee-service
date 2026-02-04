# Requirements Document

## Introduction

This document defines the requirements for the CLOUD_GATEWAY component of the SDV Parking Demo System. The CLOUD_GATEWAY is a Go backend service deployed on OpenShift that acts as an MQTT broker/router for vehicle-to-cloud communication. It bridges commands from the COMPANION_APP to vehicles and routes telemetry from vehicles to interested clients.

The service accepts MQTT connections from vehicles (via CLOUD_GATEWAY_CLIENT), provides REST APIs for the COMPANION_APP to send commands and query vehicle state, and manages the routing of messages between these endpoints.

## Glossary

- **CLOUD_GATEWAY**: Go backend service providing MQTT broker/router functionality for vehicle-to-cloud communication
- **CLOUD_GATEWAY_CLIENT**: ASIL-B service in the vehicle that maintains MQTT connection to CLOUD_GATEWAY
- **COMPANION_APP**: Mobile application for remote vehicle control (lock/unlock) and status viewing
- **MQTT**: Message Queuing Telemetry Transport protocol for lightweight pub/sub messaging
- **TLS**: Transport Layer Security for encrypted communication
- **VIN**: Vehicle Identification Number used as unique vehicle identifier
- **Command**: A lock or unlock request sent from COMPANION_APP to a vehicle
- **Command_ID**: Unique identifier for tracking command execution and responses
- **Command_Status**: Current state of a command (pending, success, failed, timeout)
- **Telemetry**: Vehicle state data received from vehicles (location, door status, parking state)
- **Auth_Token**: Authentication token for command authorization (demo-grade)
- **Eclipse_Mosquitto**: Open-source MQTT broker used as underlying message broker
- **OpenShift**: Red Hat's Kubernetes platform where the service is deployed

## Requirements

### Requirement 1: MQTT Broker Integration

**User Story:** As a system integrator, I want the CLOUD_GATEWAY to connect to an MQTT broker, so that it can route messages between vehicles and clients.

#### Acceptance Criteria

1. WHEN the CLOUD_GATEWAY starts THEN it SHALL connect to the Eclipse_Mosquitto broker using configured connection parameters
2. THE CLOUD_GATEWAY SHALL subscribe to `vehicles/{VIN}/command_responses` topic to receive command responses from vehicles
3. THE CLOUD_GATEWAY SHALL subscribe to `vehicles/{VIN}/telemetry` topic to receive telemetry from vehicles
4. WHEN the MQTT connection is lost THEN the CLOUD_GATEWAY SHALL attempt reconnection with exponential backoff starting at 1 second and capping at 30 seconds
5. WHEN reconnection succeeds THEN the CLOUD_GATEWAY SHALL resubscribe to all required topics

### Requirement 2: Command Submission API

**User Story:** As a COMPANION_APP user, I want to send lock/unlock commands to my vehicle via REST API, so that I can remotely control my vehicle's doors.

#### Acceptance Criteria

1. WHEN the CLOUD_GATEWAY receives a POST /api/v1/vehicles/{vin}/commands request THEN it SHALL create a new command and publish it to the vehicle
2. THE request body SHALL accept command_type ("lock" or "unlock"), doors (array of "driver" or "all"), and auth_token fields
3. WHEN a command is created THEN the CLOUD_GATEWAY SHALL generate a unique command_id and return it in the response
4. THE response SHALL include command_id and status fields where status is initially "pending"
5. WHEN the command is published to MQTT THEN the CLOUD_GATEWAY SHALL publish to `vehicles/{VIN}/commands` topic
6. IF command_type is not "lock" or "unlock" THEN the CLOUD_GATEWAY SHALL return HTTP 400 with validation error
7. IF auth_token is missing or empty THEN the CLOUD_GATEWAY SHALL return HTTP 400 with validation error
8. IF the VIN does not match the configured vehicle THEN the CLOUD_GATEWAY SHALL return HTTP 404 with error message

### Requirement 3: Command Status Query API

**User Story:** As a COMPANION_APP user, I want to check the status of my command, so that I know whether my lock/unlock request succeeded.

#### Acceptance Criteria

1. WHEN the CLOUD_GATEWAY receives a GET /api/v1/vehicles/{vin}/commands/{command_id} request THEN it SHALL return the current command status
2. THE response SHALL include command_id, command_type, status, and created_at fields
3. WHEN the command has completed THEN the response SHALL include completed_at field
4. WHEN the command has failed THEN the response SHALL include error_code and error_message fields
5. IF the command_id does not exist THEN the CLOUD_GATEWAY SHALL return HTTP 404 with error message
6. IF the VIN does not match the configured vehicle THEN the CLOUD_GATEWAY SHALL return HTTP 404 with error message

### Requirement 4: Command Response Processing

**User Story:** As a system integrator, I want command responses from vehicles to update command status, so that clients can track command completion.

#### Acceptance Criteria

1. WHEN a message is received on `vehicles/{VIN}/command_responses` topic THEN the CLOUD_GATEWAY SHALL parse the JSON payload
2. WHEN the response contains a valid command_id THEN the CLOUD_GATEWAY SHALL update the stored command status
3. WHEN the response status is "success" THEN the CLOUD_GATEWAY SHALL set command status to "success"
4. WHEN the response status is "failed" THEN the CLOUD_GATEWAY SHALL set command status to "failed" and store error details
5. IF the command_id in the response does not match any stored command THEN the CLOUD_GATEWAY SHALL log a warning and discard the message
6. IF the response payload is malformed THEN the CLOUD_GATEWAY SHALL log an error and discard the message

### Requirement 5: Command Timeout Handling

**User Story:** As a COMPANION_APP user, I want commands to timeout if the vehicle doesn't respond, so that I'm not left waiting indefinitely.

#### Acceptance Criteria

1. WHEN a command is created THEN the CLOUD_GATEWAY SHALL start a timeout timer of 30 seconds
2. IF no response is received within the timeout period THEN the CLOUD_GATEWAY SHALL set command status to "timeout"
3. WHEN a command times out THEN the CLOUD_GATEWAY SHALL set error_code to "TIMEOUT" and error_message to "Vehicle did not respond within timeout period"
4. IF a response arrives after timeout THEN the CLOUD_GATEWAY SHALL log a warning and discard the late response

### Requirement 6: Telemetry Storage and Query API

**User Story:** As a COMPANION_APP user, I want to view my vehicle's current state, so that I can see its location and door status.

#### Acceptance Criteria

1. WHEN a message is received on `vehicles/{VIN}/telemetry` topic THEN the CLOUD_GATEWAY SHALL parse and store the telemetry data
2. THE stored telemetry SHALL include timestamp, latitude, longitude, door_locked, door_open, and parking_session_active fields
3. WHEN the CLOUD_GATEWAY receives a GET /api/v1/vehicles/{vin}/telemetry request THEN it SHALL return the latest stored telemetry
4. THE telemetry response SHALL include all stored telemetry fields plus a received_at timestamp
5. IF no telemetry has been received for the vehicle THEN the CLOUD_GATEWAY SHALL return HTTP 404 with error message
6. IF the VIN does not match the configured vehicle THEN the CLOUD_GATEWAY SHALL return HTTP 404 with error message
7. IF the telemetry payload is malformed THEN the CLOUD_GATEWAY SHALL log an error and discard the message

### Requirement 7: Health Check Endpoint

**User Story:** As an OpenShift operator, I want to check if the service is running, so that I can monitor service health.

#### Acceptance Criteria

1. WHEN the CLOUD_GATEWAY receives a GET /health request THEN it SHALL return HTTP 200 with status "healthy"
2. THE health response SHALL include service name and current timestamp
3. THE health endpoint SHALL respond within 100ms under normal conditions

### Requirement 8: Readiness Check Endpoint

**User Story:** As an OpenShift operator, I want to check if the service is ready to accept traffic, so that I can manage load balancing.

#### Acceptance Criteria

1. WHEN the CLOUD_GATEWAY receives a GET /ready request THEN it SHALL return HTTP 200 if ready to serve requests
2. THE readiness check SHALL verify that the MQTT broker connection is established
3. IF the MQTT connection is not established THEN the CLOUD_GATEWAY SHALL return HTTP 503 with status "not ready"
4. THE readiness response SHALL include mqtt_connected boolean field

### Requirement 9: Configuration Management

**User Story:** As a system operator, I want to configure the service via environment variables, so that I can deploy it in different environments.

#### Acceptance Criteria

1. THE CLOUD_GATEWAY SHALL support configuration via environment variables
2. THE configuration SHALL include: HTTP listen port, MQTT broker URL, MQTT username and password, configured VIN, and command timeout duration
3. THE CLOUD_GATEWAY SHALL provide sensible defaults for optional configuration options
4. WHEN required configuration (MQTT broker URL, VIN) is missing THEN the CLOUD_GATEWAY SHALL fail to start with a clear error message
5. THE CLOUD_GATEWAY SHALL log all configuration values (except secrets) on startup

### Requirement 10: Request Logging

**User Story:** As a system operator, I want all API requests and MQTT messages to be logged, so that I can debug issues and monitor usage.

#### Acceptance Criteria

1. THE CLOUD_GATEWAY SHALL log every incoming HTTP request with timestamp, method, path, and response status
2. THE CLOUD_GATEWAY SHALL log all command operations with command_id and operation type
3. THE CLOUD_GATEWAY SHALL log all MQTT messages received with topic and message summary
4. THE CLOUD_GATEWAY SHALL use structured JSON logging format
5. THE CLOUD_GATEWAY SHALL include request duration in HTTP log entries

### Requirement 11: Error Response Format

**User Story:** As an API consumer, I want consistent error responses, so that I can handle errors programmatically.

#### Acceptance Criteria

1. THE CLOUD_GATEWAY SHALL return errors in a consistent JSON format with error_code and message fields
2. THE CLOUD_GATEWAY SHALL use appropriate HTTP status codes (400 for client errors, 404 for not found, 500 for server errors)
3. THE CLOUD_GATEWAY SHALL include a request_id in all responses for tracing
4. THE CLOUD_GATEWAY SHALL NOT expose internal error details in production responses

### Requirement 12: In-Memory Storage

**User Story:** As a demo system, I want commands and telemetry stored in memory, so that the system is simple and stateless for demonstration purposes.

#### Acceptance Criteria

1. THE CLOUD_GATEWAY SHALL store commands in memory with a maximum retention of 100 commands
2. WHEN the command limit is reached THEN the CLOUD_GATEWAY SHALL remove the oldest commands first
3. THE CLOUD_GATEWAY SHALL store only the latest telemetry per vehicle
4. WHEN the CLOUD_GATEWAY restarts THEN all stored data SHALL be cleared (acceptable for demo)

### Requirement 13: Graceful Shutdown

**User Story:** As a system operator, I want the service to shut down gracefully, so that in-flight requests complete cleanly.

#### Acceptance Criteria

1. WHEN the CLOUD_GATEWAY receives SIGTERM THEN it SHALL initiate graceful shutdown
2. DURING graceful shutdown THE CLOUD_GATEWAY SHALL stop accepting new HTTP requests
3. DURING graceful shutdown THE CLOUD_GATEWAY SHALL complete in-flight HTTP requests within 10 seconds
4. DURING graceful shutdown THE CLOUD_GATEWAY SHALL disconnect from MQTT broker cleanly
5. THE CLOUD_GATEWAY SHALL complete shutdown within 15 seconds of receiving SIGTERM
