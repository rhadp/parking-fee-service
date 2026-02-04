# Implementation Plan: CLOUD_GATEWAY

## Overview

This plan implements the CLOUD_GATEWAY Go backend service that acts as an MQTT broker/router for vehicle-to-cloud communication. The implementation follows a layered architecture with handlers, services, and stores, using the paho.mqtt.golang library for MQTT connectivity.

## Tasks

- [ ] 1. Set up project structure and core dependencies
  - Create directory structure under `backend/cloud-gateway/`
  - Initialize Go module with `go mod init`
  - Add dependencies: gorilla/mux, paho.mqtt.golang, gopter, slog
  - Create main.go entry point skeleton
  - _Requirements: 9.1, 9.2_

- [ ] 2. Implement configuration management
  - [ ] 2.1 Create config struct with environment variable loading
    - Define Config struct with all fields (Port, MQTT settings, VIN, timeouts)
    - Implement LoadConfig() with environment variable parsing
    - Add validation for required fields (MQTT_BROKER_URL, CONFIGURED_VIN)
    - Implement sensible defaults for optional fields
    - _Requirements: 9.1, 9.2, 9.3, 9.4_
  
  - [ ] 2.2 Write unit tests for configuration loading
    - Test required field validation
    - Test default value application
    - Test environment variable parsing
    - _Requirements: 9.3, 9.4_

- [ ] 3. Implement data models
  - [ ] 3.1 Create model package with all data structures
    - Define Command struct with all fields
    - Define Telemetry struct with all fields
    - Define request/response models (SubmitCommandRequest, CommandStatusResponse, etc.)
    - Define MQTT message models (MQTTCommandMessage, MQTTCommandResponse, MQTTTelemetryMessage)
    - Define error codes as constants
    - _Requirements: 2.2, 3.2, 6.2_

- [ ] 4. Implement middleware
  - [ ] 4.1 Create request ID middleware
    - Generate unique request ID for each request
    - Store request ID in context
    - Implement GetRequestID helper function
    - _Requirements: 11.3_
  
  - [ ] 4.2 Create logging middleware
    - Log timestamp, method, path for each request
    - Log response status and duration
    - Use structured JSON logging with slog
    - _Requirements: 10.1, 10.4, 10.5_

- [ ] 5. Implement in-memory stores
  - [ ] 5.1 Implement CommandStore
    - Create thread-safe map with mutex
    - Implement Save, Get, Update methods
    - Implement GetPendingCommands for timeout checking
    - Implement FIFO eviction when max size exceeded
    - _Requirements: 12.1, 12.2_
  
  - [ ] 5.2 Write property test for CommandStore FIFO eviction
    - **Property 12: Command Store FIFO Eviction**
    - **Validates: Requirements 12.1, 12.2**
  
  - [ ] 5.3 Implement TelemetryStore
    - Create thread-safe map keyed by VIN
    - Implement Save (overwrites previous) and Get methods
    - _Requirements: 12.3_
  
  - [ ] 5.4 Write property test for TelemetryStore overwrites
    - **Property 10: Telemetry Overwrites Previous**
    - **Validates: Requirements 12.3**

- [ ] 6. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 7. Implement MQTT client
  - [ ] 7.1 Create MQTTClient interface and implementation
    - Define MQTTClient interface (Connect, Disconnect, IsConnected, Subscribe, Publish)
    - Implement using paho.mqtt.golang
    - Configure TLS connection to broker
    - Implement reconnection with exponential backoff
    - _Requirements: 1.1, 1.4, 1.5_
  
  - [ ] 7.2 Write property test for exponential backoff calculation
    - **Property 13: Exponential Backoff Calculation**
    - **Validates: Requirements 1.4**
  
  - [ ] 7.3 Implement MQTT message handlers
    - Create handler for command_responses topic
    - Create handler for telemetry topic
    - Parse JSON payloads and delegate to services
    - _Requirements: 1.2, 1.3, 4.1, 6.1_

- [ ] 8. Implement command service
  - [ ] 8.1 Create CommandService with core logic
    - Implement SubmitCommand (generate ID, store, publish to MQTT)
    - Implement GetCommandStatus
    - Implement HandleCommandResponse (update stored command)
    - _Requirements: 2.1, 2.3, 2.4, 2.5, 3.1, 4.2, 4.3, 4.4_
  
  - [ ] 8.2 Write property test for unique command ID generation
    - **Property 2: Command Creation with Unique ID**
    - **Validates: Requirements 2.1, 2.3, 2.4**
  
  - [ ] 8.3 Write property test for command response processing
    - **Property 7: Command Response Processing**
    - **Validates: Requirements 4.2, 4.3, 4.4**
  
  - [ ] 8.4 Implement command timeout checker
    - Start background goroutine to check pending commands
    - Set status to "timeout" for expired commands
    - Set error_code and error_message for timeouts
    - _Requirements: 5.1, 5.2, 5.3_
  
  - [ ] 8.5 Write property test for command timeout handling
    - **Property 8: Command Timeout Status**
    - **Validates: Requirements 5.2, 5.3**

- [ ] 9. Implement telemetry service
  - [ ] 9.1 Create TelemetryService with core logic
    - Implement HandleTelemetryMessage (parse and store)
    - Implement GetLatestTelemetry
    - Add received_at timestamp on storage
    - _Requirements: 6.1, 6.2, 6.3, 6.4_
  
  - [ ] 9.2 Write property test for telemetry round-trip
    - **Property 9: Telemetry Round-Trip**
    - **Validates: Requirements 6.1, 6.2, 6.3, 6.4**

- [ ] 10. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 11. Implement REST API handlers
  - [ ] 11.1 Implement CommandHandler
    - HandleSubmitCommand: validate request, call service, return response
    - HandleGetCommandStatus: validate VIN, call service, return response
    - Implement input validation (command_type, auth_token, doors)
    - _Requirements: 2.1, 2.2, 2.6, 2.7, 2.8, 3.1, 3.5, 3.6_
  
  - [ ] 11.2 Write property tests for command validation
    - **Property 3: Command Type Validation**
    - **Property 4: Auth Token Validation**
    - **Validates: Requirements 2.6, 2.7**
  
  - [ ] 11.3 Write property test for command status response completeness
    - **Property 5: Command Status Response Completeness**
    - **Validates: Requirements 3.2, 3.3, 3.4**
  
  - [ ] 11.4 Write property test for command not found
    - **Property 6: Command Not Found**
    - **Validates: Requirements 3.5**
  
  - [ ] 11.5 Implement TelemetryHandler
    - HandleGetTelemetry: validate VIN, call service, return response
    - Return 404 if no telemetry received
    - _Requirements: 6.3, 6.5, 6.6_
  
  - [ ] 11.6 Implement HealthHandler
    - HandleHealth: return healthy status with service name and timestamp
    - HandleReady: check MQTT connection, return ready/not ready
    - _Requirements: 7.1, 7.2, 8.1, 8.2, 8.3, 8.4_

- [ ] 12. Implement error handling
  - [ ] 12.1 Create error response helpers
    - Implement WriteError, WriteValidationError, WriteNotFound
    - Ensure consistent JSON format with error_code, message, request_id
    - _Requirements: 11.1, 11.2, 11.3_
  
  - [ ] 12.2 Write property test for VIN validation across endpoints
    - **Property 1: VIN Validation Across Endpoints**
    - **Validates: Requirements 2.8, 3.6, 6.6**
  
  - [ ] 12.3 Write property test for error response format
    - **Property 11: Error Response Format Consistency**
    - **Validates: Requirements 11.1, 11.2, 11.3**

- [ ] 13. Wire components together
  - [ ] 13.1 Create router and register routes
    - Set up gorilla/mux router
    - Register all API endpoints with handlers
    - Apply middleware chain (RequestID, Logging)
    - _Requirements: 2.1, 3.1, 6.3, 7.1, 8.1_
  
  - [ ] 13.2 Implement main.go with full initialization
    - Load configuration
    - Initialize stores, services, handlers
    - Connect MQTT client and subscribe to topics
    - Start HTTP server
    - Start command timeout checker
    - _Requirements: 1.1, 1.2, 1.3, 9.5_
  
  - [ ] 13.3 Implement graceful shutdown
    - Handle SIGTERM signal
    - Stop accepting new HTTP requests
    - Complete in-flight requests (10s timeout)
    - Disconnect MQTT client cleanly
    - Complete shutdown within 15 seconds
    - _Requirements: 13.1, 13.2, 13.3, 13.4, 13.5_

- [ ] 14. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 15. Create Containerfile and deployment config
  - [ ] 15.1 Create Containerfile for cloud-gateway
    - Multi-stage build for minimal image size
    - Copy binary and set entrypoint
    - Place in `containers/backend/cloud-gateway/Containerfile`
  
  - [ ] 15.2 Add cloud-gateway to infra compose
    - Add service definition to compose file
    - Configure environment variables
    - Set up network connectivity with Mosquitto

## Notes

- All tasks are required including property-based tests
- Each task references specific requirements for traceability
- Property tests validate universal correctness properties
- Unit tests validate specific examples and edge cases
- The service uses paho.mqtt.golang for MQTT connectivity
- gopter is used for property-based testing in Go
