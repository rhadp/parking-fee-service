# Implementation Plan: PARKING_FEE_SERVICE

## Overview

This plan implements the PARKING_FEE_SERVICE as a Go backend service providing REST APIs for parking zone lookup, adapter registry, and mock parking operations. The implementation follows a layered architecture with handlers, services, and stores.

## Tasks

- [ ] 1. Set up project structure and core dependencies
  - Create directory structure under `backend/parking-fee-service/`
  - Initialize Go module with `go mod init`
  - Add dependencies: gorilla/mux (routing), gopter (property testing), slog (logging)
  - Create Makefile targets for build and test
  - _Requirements: 9.1_

- [ ] 2. Implement configuration and models
  - [ ] 2.1 Create configuration loader with environment variable support
    - Implement Config struct with env tags and defaults
    - Add LoadConfig function using envconfig or similar
    - _Requirements: 9.1, 9.2, 9.3, 9.4_
  
  - [ ] 2.2 Create data models
    - Implement Zone, Bounds, Adapter, Session structs
    - Implement request/response models (ZoneResponse, StartSessionRequest, etc.)
    - Implement ErrorResponse struct
    - _Requirements: 1.2, 2.2, 3.2, 4.3, 5.3, 6.2, 11.1_

- [ ] 3. Implement middleware and utilities
  - [ ] 3.1 Implement request ID middleware
    - Generate UUID for each request
    - Store in context for downstream use
    - _Requirements: 11.3_
  
  - [ ] 3.2 Implement logging middleware
    - Log request method, path, status, duration
    - Use structured JSON logging with slog
    - _Requirements: 10.1, 10.3, 10.4_
  
  - [ ] 3.3 Implement error response helpers
    - Create WriteError, WriteValidationError, WriteNotFound, WriteConflict functions
    - Ensure consistent error format with request_id
    - _Requirements: 11.1, 11.2, 11.3_

- [ ] 4. Implement zone lookup functionality
  - [ ] 4.1 Implement ZoneStore
    - Create in-memory store with demo zone
    - Implement FindByLocation with bounds checking
    - Implement ContainsPoint for Bounds
    - _Requirements: 1.1, 1.3_
  
  - [ ] 4.2 Implement ZoneService
    - Create FindZoneByLocation business logic
    - _Requirements: 1.1, 1.3, 1.4_
  
  - [ ] 4.3 Implement ZoneHandler
    - Parse and validate lat/lng query parameters
    - Return zone or appropriate error (400, 404)
    - _Requirements: 1.1, 1.2, 1.4, 1.5, 1.6_
  
  - [ ] 4.4 Write property tests for zone lookup
    - **Property 1: Zone Containment**
    - **Property 2: Zone Response Completeness**
    - **Property 3: Invalid Coordinate Validation**
    - **Validates: Requirements 1.1, 1.2, 1.3, 1.4, 1.6**

- [ ] 5. Checkpoint - Ensure zone lookup tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 6. Implement adapter registry functionality
  - [ ] 6.1 Implement AdapterStore
    - Create in-memory store with demo adapter
    - Implement List and Get methods
    - _Requirements: 2.1, 3.1_
  
  - [ ] 6.2 Implement AdapterService
    - Implement ListAdapters with alphabetical sorting
    - Implement GetAdapter by ID
    - _Requirements: 2.1, 2.4, 3.1_
  
  - [ ] 6.3 Implement AdapterHandler
    - Handle GET /api/v1/adapters (list)
    - Handle GET /api/v1/adapters/{adapter_id} (details)
    - Return 404 for unknown adapter
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 3.3, 3.4_
  
  - [ ] 6.4 Write property tests for adapter registry
    - **Property 4: Adapter List Completeness and Sorting**
    - **Property 5: Adapter Details Retrieval**
    - **Property 6: Adapter Not Found**
    - **Property 7: Checksum Format Validation**
    - **Validates: Requirements 2.2, 2.4, 3.1, 3.2, 3.3, 3.4**

- [ ] 7. Implement mock parking operations
  - [ ] 7.1 Implement SessionStore
    - Create in-memory map for sessions
    - Implement Save, Get, IsInitialized methods
    - _Requirements: 4.5, 8.3_
  
  - [ ] 7.2 Implement ParkingService
    - Implement StartSession with unique ID generation
    - Implement StopSession with cost calculation
    - Implement GetSessionStatus with current cost
    - Implement CalculateCost formula
    - _Requirements: 4.1, 4.4, 5.1, 5.4, 5.5, 6.1, 6.3, 6.4_
  
  - [ ] 7.3 Implement ParkingHandler
    - Handle POST /api/v1/parking/start
    - Handle POST /api/v1/parking/stop
    - Handle GET /api/v1/parking/status/{session_id}
    - Validate requests and return appropriate errors
    - _Requirements: 4.1, 4.2, 4.3, 4.6, 5.1, 5.2, 5.3, 5.6, 5.7, 6.1, 6.2, 6.5_
  
  - [ ] 7.4 Write property tests for parking operations
    - **Property 8: Session Creation Round-Trip**
    - **Property 9: Session Stop Response Completeness**
    - **Property 10: Cost Calculation Correctness**
    - **Property 11: Mock Payment Always Succeeds**
    - **Property 12: Session Not Found**
    - **Property 13: Session Already Stopped**
    - **Property 14: Session Status Consistency**
    - **Validates: Requirements 4.1, 4.3, 4.4, 4.5, 5.1, 5.3, 5.4, 5.5, 5.6, 5.7, 6.1, 6.2, 6.4, 6.5**

- [ ] 8. Checkpoint - Ensure parking operation tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 9. Implement health endpoints
  - [ ] 9.1 Implement HealthHandler
    - Handle GET /health returning status, service name, timestamp
    - Handle GET /ready checking session store initialization
    - Return 503 if not ready
    - _Requirements: 7.1, 7.2, 8.1, 8.2, 8.3_
  
  - [ ] 9.2 Write unit tests for health endpoints
    - Test health returns 200 with required fields
    - Test ready returns 200 when initialized
    - Test ready returns 503 when not initialized
    - _Requirements: 7.1, 7.2, 8.1, 8.2_

- [ ] 10. Implement main server and routing
  - [ ] 10.1 Create HTTP server with router
    - Set up gorilla/mux router
    - Register all routes with handlers
    - Apply middleware chain (request ID, logging)
    - _Requirements: 1.1, 2.1, 3.1, 4.1, 5.1, 6.1, 7.1, 8.1_
  
  - [ ] 10.2 Create main.go entry point
    - Load configuration
    - Initialize stores with demo data
    - Initialize services and handlers
    - Start HTTP server
    - _Requirements: 9.1, 9.2, 9.3_
  
  - [ ] 10.3 Write property test for error response format
    - **Property 15: Error Response Format Consistency**
    - **Validates: Requirements 11.1, 11.3**

- [ ] 11. Create Containerfile and deployment artifacts
  - [ ] 11.1 Create Containerfile for the service
    - Multi-stage build for minimal image
    - Set appropriate labels and environment defaults
    - _Requirements: 9.1_
  
  - [ ] 11.2 Update root Makefile with build targets
    - Add build-parking-fee-service target
    - Add test target for Go tests
    - _Requirements: 9.1_

- [ ] 12. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- All tasks are required including property and unit tests
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties
- Unit tests validate specific examples and edge cases
- The service uses Go with gorilla/mux for routing and gopter for property-based testing
