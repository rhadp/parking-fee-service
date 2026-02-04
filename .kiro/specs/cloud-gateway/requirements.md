# Requirements Document

## Introduction

This document defines the requirements for the CLOUD_GATEWAY component of the SDV Parking Demo System. The CLOUD_GATEWAY is a Go backend service deployed on OpenShift that acts as a bridge for vehicle-to-cloud communication. It provides two distinct interfaces:

1. **REST API Interface (Northbound)**: Serves the COMPANION_APP for remote vehicle control and command status queries via HTTPS/REST
2. **MQTT Interface (Southbound)**: Communicates with vehicles via the CLOUD_GATEWAY_CLIENT through an Eclipse Mosquitto MQTT broker

The service translates REST API requests from the COMPANION_APP into MQTT messages for vehicles, and routes MQTT command responses from vehicles back to REST API consumers. Vehicle telemetry received via MQTT is exported to an OpenTelemetry collector for observability.

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
- **Telemetry**: Vehicle state data received from vehicles (location, door status, parking state), exported via OpenTelemetry
- **Auth_Token**: Authentication token for command authorization (demo-grade)
- **Eclipse_Mosquitto**: Open-source MQTT broker used as underlying message broker
- **OpenShift**: Red Hat's Kubernetes platform where the service is deployed
- **OpenTelemetry**: Observability framework for collecting and exporting telemetry data (metrics, traces, logs)
- **OTLP**: OpenTelemetry Protocol for exporting telemetry to collectors

## Requirements

### Requirement 15: Dual Interface Architecture

**User Story:** As a system architect, I want the CLOUD_GATEWAY to expose two distinct interfaces (REST API and MQTT), so that it can bridge communication between mobile clients and vehicles.

#### Acceptance Criteria

1. THE CLOUD_GATEWAY SHALL expose a REST API interface (Northbound) for COMPANION_APP communication over HTTPS
2. THE CLOUD_GATEWAY SHALL maintain an MQTT client connection (Southbound) to communicate with vehicles via Eclipse_Mosquitto broker
3. THE REST API interface SHALL support command submission, command status queries, and parking session queries
4. THE MQTT interface SHALL support publishing commands to vehicles and subscribing to command responses and telemetry
5. THE CLOUD_GATEWAY SHALL translate REST API command requests into MQTT messages for vehicle delivery
6. THE CLOUD_GATEWAY SHALL export MQTT telemetry messages to OpenTelemetry collector (telemetry is NOT exposed via REST API)
7. THE two interfaces SHALL operate independently such that REST API availability is not affected by temporary MQTT disconnections (cached command data remains queryable)
8. THE CLOUD_GATEWAY SHALL report interface health separately in readiness checks (mqtt_connected status)

### Requirement 16: Parking Session Query API (Northbound Interface)

**User Story:** As a COMPANION_APP user, I want to get detailed parking session information, so that I can see the zone, duration, and cost of my active parking session.

#### Acceptance Criteria

1. WHEN the CLOUD_GATEWAY receives a GET /api/v1/vehicles/{vin}/parking-session request from COMPANION_APP THEN it SHALL proxy the request to PARKING_FEE_SERVICE
2. WHEN an active parking session exists THEN the response SHALL include session_id, zone_name, hourly_rate, currency, duration_seconds, current_cost, and timestamp
3. WHEN no active parking session exists THEN the CLOUD_GATEWAY SHALL return HTTP 404 with error code "NO_ACTIVE_SESSION"
4. IF the VIN does not match the configured vehicle THEN the CLOUD_GATEWAY SHALL return HTTP 404 with error message
5. THE CLOUD_GATEWAY SHALL cache parking session data for up to 5 seconds to reduce load on PARKING_FEE_SERVICE

### Requirement 1: MQTT Broker Integration (Southbound Interface)

**User Story:** As a system integrator, I want the CLOUD_GATEWAY to connect to an MQTT broker, so that it can route messages between vehicles and clients.

#### Acceptance Criteria

1. WHEN the CLOUD_GATEWAY starts THEN it SHALL connect to the Eclipse_Mosquitto broker using configured connection parameters
2. THE CLOUD_GATEWAY SHALL subscribe to `vehicles/{VIN}/command_responses` topic to receive command responses from CLOUD_GATEWAY_CLIENT
3. THE CLOUD_GATEWAY SHALL subscribe to `vehicles/{VIN}/telemetry` topic to receive telemetry from CLOUD_GATEWAY_CLIENT
4. WHEN the MQTT connection is lost THEN the CLOUD_GATEWAY SHALL attempt reconnection with exponential backoff starting at 1 second and capping at 30 seconds
5. WHEN reconnection succeeds THEN the CLOUD_GATEWAY SHALL resubscribe to all required topics

### Requirement 2: Command Submission API (Northbound Interface)

**User Story:** As a COMPANION_APP user, I want to send lock/unlock commands to my vehicle via REST API, so that I can remotely control my vehicle's doors.

#### Acceptance Criteria

1. WHEN the CLOUD_GATEWAY receives a POST /api/v1/vehicles/{vin}/commands request from COMPANION_APP THEN it SHALL create a new command and publish it to the vehicle via MQTT
2. THE request body SHALL accept command_type ("lock" or "unlock"), doors (array of "driver" or "all"), and auth_token fields
3. WHEN a command is created THEN the CLOUD_GATEWAY SHALL generate a unique command_id and return it in the response
4. THE response SHALL include command_id and status fields where status is initially "pending"
5. WHEN the command is published to MQTT THEN the CLOUD_GATEWAY SHALL publish to `vehicles/{VIN}/commands` topic for CLOUD_GATEWAY_CLIENT consumption
6. IF command_type is not "lock" or "unlock" THEN the CLOUD_GATEWAY SHALL return HTTP 400 with validation error
7. IF auth_token is missing or empty THEN the CLOUD_GATEWAY SHALL return HTTP 400 with validation error
8. IF the VIN does not match the configured vehicle THEN the CLOUD_GATEWAY SHALL return HTTP 404 with error message

### Requirement 3: Command Status Query API (Northbound Interface)

**User Story:** As a COMPANION_APP user, I want to check the status of my command, so that I know whether my lock/unlock request succeeded.

#### Acceptance Criteria

1. WHEN the CLOUD_GATEWAY receives a GET /api/v1/vehicles/{vin}/commands/{command_id} request from COMPANION_APP THEN it SHALL return the current command status
2. THE response SHALL include command_id, command_type, status, and created_at fields
3. WHEN the command has completed THEN the response SHALL include completed_at field
4. WHEN the command has failed THEN the response SHALL include error_code and error_message fields
5. IF the command_id does not exist THEN the CLOUD_GATEWAY SHALL return HTTP 404 with error message
6. IF the VIN does not match the configured vehicle THEN the CLOUD_GATEWAY SHALL return HTTP 404 with error message

### Requirement 4: Command Response Processing (Southbound Interface)

**User Story:** As a system integrator, I want command responses from CLOUD_GATEWAY_CLIENT to update command status, so that COMPANION_APP clients can track command completion.

#### Acceptance Criteria

1. WHEN a message is received on `vehicles/{VIN}/command_responses` topic from CLOUD_GATEWAY_CLIENT THEN the CLOUD_GATEWAY SHALL parse the JSON payload
2. WHEN the response contains a valid command_id THEN the CLOUD_GATEWAY SHALL update the stored command status
3. WHEN the response status is "success" THEN the CLOUD_GATEWAY SHALL set command status to "success"
4. WHEN the response status is "failed" THEN the CLOUD_GATEWAY SHALL set command status to "failed" and store error details
5. IF the command_id in the response does not match any stored command THEN the CLOUD_GATEWAY SHALL log a warning and discard the message
6. IF the response payload is malformed THEN the CLOUD_GATEWAY SHALL log an error and discard the message

### Requirement 5: Command Timeout Handling

**User Story:** As a COMPANION_APP user, I want commands to timeout if the vehicle doesn't respond, so that I'm not left waiting indefinitely.

#### Acceptance Criteria

1. WHEN a command is created THEN the CLOUD_GATEWAY SHALL start a timeout timer of 30 seconds
2. IF no response is received from CLOUD_GATEWAY_CLIENT within the timeout period THEN the CLOUD_GATEWAY SHALL set command status to "timeout"
3. WHEN a command times out THEN the CLOUD_GATEWAY SHALL set error_code to "TIMEOUT" and error_message to "Vehicle did not respond within timeout period"
4. IF a response arrives from CLOUD_GATEWAY_CLIENT after timeout THEN the CLOUD_GATEWAY SHALL log a warning and discard the late response

### Requirement 6: Telemetry Processing and OpenTelemetry Export

**User Story:** As a system operator, I want vehicle telemetry to be collected and exported to OpenTelemetry, so that I can monitor vehicle state through observability tools.

#### Acceptance Criteria

1. WHEN a message is received on `vehicles/{VIN}/telemetry` topic from CLOUD_GATEWAY_CLIENT THEN the CLOUD_GATEWAY SHALL parse the telemetry data (Southbound)
2. THE telemetry data SHALL include timestamp, latitude, longitude, door_locked, door_open, and parking_session_active fields
3. THE CLOUD_GATEWAY SHALL export telemetry data as OpenTelemetry metrics to a configured OTLP endpoint
4. THE exported metrics SHALL include vehicle VIN as an attribute for filtering
5. THE CLOUD_GATEWAY SHALL NOT expose telemetry data via REST API
6. IF the telemetry payload from CLOUD_GATEWAY_CLIENT is malformed THEN the CLOUD_GATEWAY SHALL log an error and discard the message
7. IF the OpenTelemetry collector is unavailable THEN the CLOUD_GATEWAY SHALL log a warning and continue processing (best-effort export)

### Requirement 7: Health Check Endpoint (Northbound Interface)

**User Story:** As an OpenShift operator, I want to check if the service is running, so that I can monitor service health.

#### Acceptance Criteria

1. WHEN the CLOUD_GATEWAY receives a GET /health request THEN it SHALL return HTTP 200 with status "healthy"
2. THE health response SHALL include service name and current timestamp
3. THE health endpoint SHALL respond within 100ms under normal conditions

### Requirement 8: Readiness Check Endpoint (Northbound Interface)

**User Story:** As an OpenShift operator, I want to check if the service is ready to accept traffic, so that I can manage load balancing.

#### Acceptance Criteria

1. WHEN the CLOUD_GATEWAY receives a GET /ready request THEN it SHALL return HTTP 200 if ready to serve requests
2. THE readiness check SHALL verify that the MQTT broker connection (Southbound interface) is established
3. IF the MQTT connection is not established THEN the CLOUD_GATEWAY SHALL return HTTP 503 with status "not ready"
4. THE readiness response SHALL include mqtt_connected boolean field indicating Southbound interface status

### Requirement 9: Configuration Management

**User Story:** As a system operator, I want to configure the service via environment variables, so that I can deploy it in different environments.

#### Acceptance Criteria

1. THE CLOUD_GATEWAY SHALL support configuration via environment variables
2. THE configuration SHALL include: HTTP listen port, MQTT broker URL, MQTT username and password, configured VIN, command timeout duration, and OTLP endpoint URL
3. THE CLOUD_GATEWAY SHALL provide sensible defaults for optional configuration options
4. WHEN required configuration (MQTT broker URL, VIN) is missing THEN the CLOUD_GATEWAY SHALL fail to start with a clear error message
5. THE CLOUD_GATEWAY SHALL log all configuration values (except secrets) on startup
6. THE OTLP endpoint configuration SHALL be optional; if not configured, telemetry export SHALL be disabled

### Requirement 10: Request Logging

**User Story:** As a system operator, I want all API requests and MQTT messages to be logged, so that I can debug issues and monitor usage.

#### Acceptance Criteria

1. THE CLOUD_GATEWAY SHALL log every incoming HTTP request with timestamp, method, path, and response status
2. THE CLOUD_GATEWAY SHALL log all command operations with command_id and operation type
3. THE CLOUD_GATEWAY SHALL log all MQTT messages received with topic and message summary
4. THE CLOUD_GATEWAY SHALL use structured JSON logging format
5. THE CLOUD_GATEWAY SHALL include request duration in HTTP log entries

### Requirement 14: Audit Logging

**User Story:** As a security auditor, I want comprehensive audit logs of all security-relevant operations, so that I can investigate incidents and verify compliance.

#### Acceptance Criteria

1. THE CLOUD_GATEWAY SHALL log all command submissions with VIN, command_type, doors, timestamp, source_ip, and request_id
2. THE CLOUD_GATEWAY SHALL log all command status changes with command_id, previous_status, new_status, and timestamp
3. THE CLOUD_GATEWAY SHALL log all authentication attempts with VIN, auth_token_hash (first 8 chars), success/failure, and source_ip
4. THE CLOUD_GATEWAY SHALL log all telemetry updates with VIN, timestamp, and data summary (location present, door state changed)
5. THE CLOUD_GATEWAY SHALL include a correlation_id in all audit log entries to enable request tracing across services
6. THE CLOUD_GATEWAY SHALL log MQTT connection events (connect, disconnect, reconnect) with broker address and timestamp
7. THE CLOUD_GATEWAY SHALL log all failed validation attempts with VIN, endpoint, validation_error, and source_ip
8. AUDIT log entries SHALL be distinguishable from operational logs via a log_type field set to "audit"
9. THE CLOUD_GATEWAY SHALL NOT log sensitive data (full auth_token, user credentials) in audit logs
10. AUDIT logs SHALL include sufficient detail to reconstruct the sequence of events for any command lifecycle

### Requirement 11: Error Response Format

**User Story:** As an API consumer, I want consistent error responses, so that I can handle errors programmatically.

#### Acceptance Criteria

1. THE CLOUD_GATEWAY SHALL return errors in a consistent JSON format with error_code and message fields
2. THE CLOUD_GATEWAY SHALL use appropriate HTTP status codes (400 for client errors, 404 for not found, 500 for server errors)
3. THE CLOUD_GATEWAY SHALL include a request_id in all responses for tracing
4. THE CLOUD_GATEWAY SHALL NOT expose internal error details in production responses

### Requirement 12: In-Memory Storage

**User Story:** As a demo system, I want commands stored in memory, so that the system is simple and stateless for demonstration purposes.

#### Acceptance Criteria

1. THE CLOUD_GATEWAY SHALL store commands in memory with a maximum retention of 100 commands
2. WHEN the command limit is reached THEN the CLOUD_GATEWAY SHALL remove the oldest commands first
3. WHEN the CLOUD_GATEWAY restarts THEN all stored command data SHALL be cleared (acceptable for demo)

### Requirement 13: Graceful Shutdown

**User Story:** As a system operator, I want the service to shut down gracefully, so that in-flight requests complete cleanly.

#### Acceptance Criteria

1. WHEN the CLOUD_GATEWAY receives SIGTERM THEN it SHALL initiate graceful shutdown
2. DURING graceful shutdown THE CLOUD_GATEWAY SHALL stop accepting new HTTP requests
3. DURING graceful shutdown THE CLOUD_GATEWAY SHALL complete in-flight HTTP requests within 10 seconds
4. DURING graceful shutdown THE CLOUD_GATEWAY SHALL disconnect from MQTT broker cleanly
5. THE CLOUD_GATEWAY SHALL complete shutdown within 15 seconds of receiving SIGTERM
