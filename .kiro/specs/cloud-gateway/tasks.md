# Implementation Plan: CLOUD_GATEWAY

## Overview

This plan implements the CLOUD_GATEWAY Go backend service with dual interfaces: REST API (Northbound) for COMPANION_APP and MQTT (Southbound) for vehicle communication. The service translates REST commands to MQTT messages, proxies parking session queries to PARKING_FEE_SERVICE, and exports telemetry to OpenTelemetry collector. Implementation uses paho.mqtt.golang for MQTT, gorilla/mux for routing, and gopter for property-based testing.

## Tasks

- [x] 1. Set up project structure and core dependencies
  - Create directory structure under `backend/cloud-gateway/`
  - Initialize Go module with `go mod init`
  - Add dependencies: gorilla/mux, paho.mqtt.golang, gopter, slog, go.opentelemetry.io/otel
  - Create main.go entry point skeleton
  - _Requirements: 9.1, 9.2, 15.1, 15.2_

- [x] 2. Implement configuration management
  - [x] 2.1 Create config struct with environment variable loading
    - Define Config struct with all fields (Port, MQTT settings, VIN, timeouts, OTLP endpoint, PARKING_FEE_SERVICE URL)
    - Implement LoadConfig() with environment variable parsing
    - Add validation for required fields (MQTT_BROKER_URL, CONFIGURED_VIN)
    - Implement sensible defaults for optional fields (OTLP endpoint optional)
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.6_

  - [x] 2.2 Write unit tests for configuration loading
    - Test required field validation
    - Test default value application
    - Test environment variable parsing
    - Test OTLP endpoint optional handling
    - _Requirements: 9.3, 9.4, 9.6_

- [x] 3. Implement data models
  - [x] 3.1 Create model package with all data structures
    - Define Command struct with all fields
    - Define Telemetry struct with all fields
    - Define ParkingSession struct for session data
    - Define request/response models (SubmitCommandRequest, CommandStatusResponse, ParkingSessionResponse, etc.)
    - Define MQTT message models (MQTTCommandMessage, MQTTCommandResponse, MQTTTelemetryMessage)
    - Define audit event models (CommandSubmissionEvent, CommandStatusChangeEvent, etc.)
    - Define error codes as constants
    - _Requirements: 2.2, 3.2, 6.2, 14.1, 14.2, 14.3, 14.4, 14.5, 14.6, 14.7, 16.2_

- [x] 4. Implement middleware
  - [x] 4.1 Create request ID middleware
    - Generate unique request ID for each request
    - Store request ID in context (used as correlation_id for audit logs)
    - Implement GetRequestID helper function
    - _Requirements: 11.3, 14.5_

  - [x] 4.2 Create logging middleware
    - Log timestamp, method, path for each request
    - Log response status and duration
    - Use structured JSON logging with slog
    - _Requirements: 10.1, 10.4, 10.5_

- [x] 5. Implement in-memory command store
  - [x] 5.1 Implement CommandStore
    - Create thread-safe map with mutex
    - Implement Save, Get, Update methods
    - Implement GetPendingCommands for timeout checking
    - Implement FIFO eviction when max size (100) exceeded
    - _Requirements: 12.1, 12.2_

  - [x] 5.2 Write property test for CommandStore FIFO eviction
    - **Property 12: Command Store FIFO Eviction**
    - **Validates: Requirements 12.1, 12.2**

- [x] 6. Implement audit logging
  - [x] 6.1 Create AuditLogger interface and implementation
    - Define AuditLogger interface with all event logging methods
    - Implement LogCommandSubmission, LogCommandStatusChange, LogAuthAttempt
    - Implement LogTelemetryUpdate, LogMQTTConnectionEvent, LogValidationFailure
    - Implement hashToken for sensitive data protection (first 8 chars of SHA256)
    - Ensure all audit logs have log_type="audit" and correlation_id
    - _Requirements: 14.1, 14.2, 14.3, 14.4, 14.5, 14.6, 14.7, 14.8, 14.9_

  - [x] 6.2 Write property test for audit log event completeness
    - **Property 14: Audit Log Event Completeness**
    - **Validates: Requirements 14.1, 14.2, 14.3, 14.4, 14.6, 14.7**

  - [x] 6.3 Write property test for audit log structure consistency
    - **Property 15: Audit Log Structure Consistency**
    - **Validates: Requirements 14.5, 14.8**

  - [x] 6.4 Write property test for sensitive data exclusion
    - **Property 16: Sensitive Data Exclusion**
    - **Validates: Requirements 14.9**

  - [x] 6.5 Write property test for command lifecycle traceability
    - **Property 17: Command Lifecycle Traceability**
    - **Validates: Requirements 14.10**

- [x] 7. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 8. Implement MQTT client (Southbound interface)
  - [x] 8.1 Create MQTTClient interface and implementation
    - Define MQTTClient interface (Connect, Disconnect, IsConnected, Subscribe, Publish)
    - Implement using paho.mqtt.golang
    - Configure TLS connection to broker
    - Implement reconnection with exponential backoff (1s start, 30s cap)
    - Integrate audit logging for connection events
    - _Requirements: 1.1, 1.4, 1.5, 14.6, 15.2_

  - [x] 8.2 Write property test for exponential backoff calculation
    - **Property 13: Exponential Backoff Calculation**
    - **Validates: Requirements 1.4**

  - [x] 8.3 Implement MQTT message handlers
    - Create handler for command_responses topic (from CLOUD_GATEWAY_CLIENT)
    - Create handler for telemetry topic (from CLOUD_GATEWAY_CLIENT)
    - Parse JSON payloads and delegate to services
    - _Requirements: 1.2, 1.3, 4.1, 6.1, 15.4_

- [ ] 9. Implement OpenTelemetry exporter
  - [ ] 9.1 Create OTelExporter for telemetry metrics
    - Initialize OpenTelemetry meter provider with OTLP exporter
    - Implement ExportTelemetry method to export vehicle metrics
    - Include VIN as attribute on all exported metrics
    - Export latitude, longitude, door_locked, door_open, parking_session_active
    - Handle collector unavailability gracefully (log warning, continue)
    - Implement Shutdown for graceful cleanup
    - _Requirements: 6.3, 6.4, 6.7, 15.6_
  
  - [ ] 9.2 Write property test for OpenTelemetry export resilience
    - **Property 20: OpenTelemetry Export Resilience**
    - **Validates: Requirements 6.7**

- [ ] 10. Implement command service
  - [ ] 10.1 Create CommandService with core logic
    - Implement SubmitCommand (generate ID, store, publish to MQTT)
    - Implement GetCommandStatus
    - Implement HandleCommandResponse (update stored command)
    - Integrate audit logging for command operations
    - _Requirements: 2.1, 2.3, 2.4, 2.5, 3.1, 4.2, 4.3, 4.4, 14.1, 14.2, 15.5_
  
  - [ ] 10.2 Write property test for unique command ID generation
    - **Property 2: Command Creation with Unique ID**
    - **Validates: Requirements 2.1, 2.3, 2.4**
  
  - [ ] 10.3 Write property test for command response processing
    - **Property 7: Command Response Processing**
    - **Validates: Requirements 4.2, 4.3, 4.4**
  
  - [ ] 10.4 Implement command timeout checker
    - Start background goroutine to check pending commands
    - Set status to "timeout" for expired commands (30s default)
    - Set error_code "TIMEOUT" and error_message for timeouts
    - Log audit event for status changes
    - _Requirements: 5.1, 5.2, 5.3, 14.2_
  
  - [ ] 10.5 Write property test for command timeout handling
    - **Property 8: Command Timeout Status**
    - **Validates: Requirements 5.2, 5.3**
  
  - [ ] 10.6 Write property test for protocol translation correctness
    - **Property 19: Protocol Translation Correctness**
    - **Validates: Requirements 15.5**

- [ ] 11. Implement telemetry service
  - [ ] 11.1 Create TelemetryService with OpenTelemetry export
    - Implement HandleTelemetryMessage (parse MQTT message, export to OTel)
    - Export telemetry as OpenTelemetry metrics (NOT stored for REST API)
    - Integrate audit logging for telemetry updates
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 14.4, 15.6_
  
  - [ ] 11.2 Write property test for telemetry OpenTelemetry export
    - **Property 9: Telemetry OpenTelemetry Export**
    - **Validates: Requirements 6.1, 6.2, 6.3, 6.4**
  
  - [ ] 11.3 Write property test for telemetry not exposed via REST
    - **Property 10: Telemetry Not Exposed via REST**
    - **Validates: Requirements 6.5, 15.6**

- [ ] 12. Implement parking session service
  - [ ] 12.1 Create ParkingSessionService with proxy logic
    - Implement GetParkingSession (proxy to PARKING_FEE_SERVICE)
    - Implement response caching with 5-second TTL
    - Handle PARKING_FEE_SERVICE unavailability gracefully
    - _Requirements: 16.1, 16.2, 16.3, 16.5_
  
  - [ ] 12.2 Write unit tests for parking session proxy
    - Test successful session retrieval
    - Test no active session (404 response)
    - Test VIN validation
    - Test cache behavior (5-second TTL)
    - _Requirements: 16.1, 16.2, 16.3, 16.4, 16.5_

- [ ] 13. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 14. Implement REST API handlers (Northbound interface)
  - [ ] 14.1 Implement CommandHandler
    - HandleSubmitCommand: validate request, call service, return response
    - HandleGetCommandStatus: validate VIN, call service, return response
    - Implement input validation (command_type, auth_token, doors)
    - Integrate audit logging for submissions and auth attempts
    - _Requirements: 2.1, 2.2, 2.6, 2.7, 2.8, 3.1, 3.5, 3.6, 14.1, 14.3, 14.7, 15.1, 15.3_
  
  - [ ] 14.2 Write property tests for command validation
    - **Property 3: Command Type Validation**
    - **Property 4: Auth Token Validation**
    - **Validates: Requirements 2.6, 2.7**
  
  - [ ] 14.3 Write property test for command status response completeness
    - **Property 5: Command Status Response Completeness**
    - **Validates: Requirements 3.2, 3.3, 3.4**
  
  - [ ] 14.4 Write property test for command not found
    - **Property 6: Command Not Found**
    - **Validates: Requirements 3.5**
  
  - [ ] 14.5 Implement HealthHandler
    - HandleHealth: return healthy status with service name and timestamp
    - HandleReady: check MQTT connection (Southbound), return ready/not ready with mqtt_connected field
    - _Requirements: 7.1, 7.2, 7.3, 8.1, 8.2, 8.3, 8.4, 15.8_
  
  - [ ] 14.6 Implement ParkingSessionHandler
    - HandleGetParkingSession: validate VIN, call service, return response
    - Return 404 with NO_ACTIVE_SESSION when no session exists
    - Return 404 with VEHICLE_NOT_FOUND for invalid VIN
    - _Requirements: 16.1, 16.2, 16.3, 16.4_

- [ ] 15. Implement error handling
  - [ ] 15.1 Create error response helpers
    - Implement WriteError, WriteValidationError, WriteNotFound
    - Ensure consistent JSON format with error_code, message, request_id
    - Integrate audit logging for validation failures
    - _Requirements: 11.1, 11.2, 11.3, 11.4, 14.7_
  
  - [ ] 15.2 Write property test for VIN validation across endpoints
    - **Property 1: VIN Validation Across Endpoints**
    - **Validates: Requirements 2.8, 3.6, 16.4**
  
  - [ ] 15.3 Write property test for error response format
    - **Property 11: Error Response Format Consistency**
    - **Validates: Requirements 11.1, 11.2, 11.3**
  
  - [ ] 15.4 Write property test for interface independence
    - **Property 18: Interface Independence**
    - **Validates: Requirements 15.7, 15.8**

- [ ] 16. Wire components together
  - [ ] 16.1 Create router and register routes
    - Set up gorilla/mux router
    - Register command endpoints: POST/GET /api/v1/vehicles/{vin}/commands
    - Register command status endpoint: GET /api/v1/vehicles/{vin}/commands/{command_id}
    - Register parking session endpoint: GET /api/v1/vehicles/{vin}/parking-session
    - Register health endpoints: GET /health, GET /ready
    - Apply middleware chain (RequestID, Logging)
    - Note: NO telemetry REST endpoint (telemetry exported to OTel only)
    - _Requirements: 2.1, 3.1, 7.1, 8.1, 15.1, 15.3, 16.1_
  
  - [ ] 16.2 Implement main.go with full initialization
    - Load configuration
    - Initialize stores, services, handlers, audit logger
    - Initialize OpenTelemetry exporter (if OTLP endpoint configured)
    - Connect MQTT client and subscribe to topics
    - Start HTTP server
    - Start command timeout checker
    - Log configuration values (except secrets) on startup
    - _Requirements: 1.1, 1.2, 1.3, 9.5, 9.6, 15.1, 15.2_
  
  - [ ] 16.3 Implement graceful shutdown
    - Handle SIGTERM signal
    - Stop accepting new HTTP requests
    - Complete in-flight requests (10s timeout)
    - Disconnect MQTT client cleanly
    - Shutdown OpenTelemetry exporter
    - Complete shutdown within 15 seconds
    - _Requirements: 13.1, 13.2, 13.3, 13.4, 13.5_

- [ ] 17. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 18. Create Containerfile and deployment config
  - [ ] 18.1 Create Containerfile for cloud-gateway
    - Multi-stage build for minimal image size
    - Copy binary and set entrypoint
    - Place in `containers/backend/cloud-gateway/Containerfile`
  
  - [ ] 18.2 Add cloud-gateway to infra compose
    - Add service definition to compose file
    - Configure environment variables (including OTLP_ENDPOINT, PARKING_FEE_SERVICE_URL)
    - Set up network connectivity with Mosquitto, OTel collector, and PARKING_FEE_SERVICE

## Notes

- All tasks are required by default per workspace rules
- Each task references specific requirements for traceability
- Property tests validate universal correctness properties (20 total)
- Unit tests validate specific examples and edge cases
- The service uses paho.mqtt.golang for MQTT connectivity (Southbound)
- gopter is used for property-based testing in Go
- Telemetry is exported to OpenTelemetry collector, NOT exposed via REST API
- Parking session queries are proxied to PARKING_FEE_SERVICE with 5-second caching
- Audit logging covers all security-relevant operations per Requirement 14
- Dual interface architecture separates Northbound (REST) and Southbound (MQTT) concerns
