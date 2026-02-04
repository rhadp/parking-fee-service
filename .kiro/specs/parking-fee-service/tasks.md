# Implementation Plan: PARKING_FEE_SERVICE

## Overview

This plan implements the PARKING_FEE_SERVICE as a Go backend service providing REST APIs for parking zone lookup, adapter registry, and mock parking operations. The implementation uses SQLite for session persistence, gorilla/mux for routing, and gopter for property-based testing.

## Tasks

- [ ] 1. Set up project structure and core dependencies
  - Create directory structure under `backend/parking-fee-service/`
  - Initialize Go module with `go mod init`
  - Add dependencies: gorilla/mux (routing), gopter (property testing), modernc.org/sqlite (SQLite driver), slog (logging)
  - Create Makefile targets for build and test
  - _Requirements: 9.1_

- [ ] 2. Implement configuration and models
  - [ ] 2.1 Create configuration loader with environment variable support
    - Implement Config struct with env tags and defaults
    - Add LoadConfig function
    - Support: PORT, DATABASE_PATH, DEMO_ZONE_*, DEMO_ADAPTER_*, LOG_LEVEL
    - _Requirements: 9.1, 9.2, 9.3, 9.4_
  
  - [ ] 2.2 Create data models
    - Implement Zone, Bounds, Adapter, AdapterSummary structs
    - Implement Session, SessionStatus structs
    - Implement request/response models (ZoneResponse, StartSessionRequest, StopSessionRequest, etc.)
    - Implement ErrorResponse, HealthResponse, ReadyResponse structs
    - Define error code constants (ErrInvalidParameters, ErrZoneNotFound, etc.)
    - _Requirements: 1.2, 2.2, 3.2, 4.3, 5.3, 6.2, 11.1_

- [ ] 3. Implement middleware and utilities
  - [ ] 3.1 Implement request ID middleware
    - Generate UUID for each request
    - Store in context for downstream use via GetRequestID helper
    - _Requirements: 11.3_
  
  - [ ] 3.2 Implement logging middleware
    - Log request method, path, status, duration
    - Use structured JSON logging with slog
    - _Requirements: 10.1, 10.3, 10.4_
  
  - [ ] 3.3 Implement error response helpers
    - Create WriteError, WriteValidationError, WriteNotFound, WriteDatabaseError functions
    - Ensure consistent error format with error, message, and request_id fields
    - _Requirements: 11.1, 11.2, 11.3_
  
  - [ ] 3.4 Implement input validation helpers
    - Create ValidateCoordinates (lat: -90 to 90, lng: -180 to 180)
    - Create ValidateStartSessionRequest
    - Create ValidateStopSessionRequest
    - _Requirements: 1.5, 1.6, 4.6, 5.2_

- [ ] 4. Implement zone lookup functionality
  - [ ] 4.1 Implement ZoneStore
    - Create in-memory store initialized with demo zone from config
    - Implement FindByLocation with bounds checking
    - Implement ContainsPoint method for Bounds struct
    - _Requirements: 1.1, 1.3_
  
  - [ ] 4.2 Implement ZoneService
    - Create FindZoneByLocation business logic
    - Return nil if no zone contains the location
    - _Requirements: 1.1, 1.3, 1.4_
  
  - [ ] 4.3 Implement ZoneHandler
    - Handle GET /api/v1/zones?lat=X&lng=Y
    - Parse and validate lat/lng query parameters
    - Return zone response or appropriate error (400 for invalid/missing params, 404 for not found)
    - _Requirements: 1.1, 1.2, 1.4, 1.5, 1.6_
  
  - [ ] 4.4 Write property tests for zone lookup
    - **Property 1: Zone Containment** - coordinates within bounds return zone, outside return nil
    - **Property 2: Zone Response Completeness** - all required fields present and non-empty
    - **Property 3: Invalid Coordinate Validation** - out-of-range coordinates return 400
    - **Validates: Requirements 1.1, 1.2, 1.3, 1.4, 1.6**

- [ ] 5. Checkpoint - Ensure zone lookup tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 6. Implement adapter registry functionality
  - [ ] 6.1 Implement AdapterStore
    - Create in-memory store initialized with demo adapter from config
    - Implement List method returning all adapters
    - Implement Get method returning adapter by ID
    - _Requirements: 2.1, 3.1_
  
  - [ ] 6.2 Implement AdapterService
    - Implement ListAdapters with alphabetical sorting by operator_name
    - Implement GetAdapter by ID, return nil if not found
    - _Requirements: 2.1, 2.4, 3.1_
  
  - [ ] 6.3 Implement AdapterHandler
    - Handle GET /api/v1/adapters (list all adapters)
    - Handle GET /api/v1/adapters/{adapter_id} (get adapter details)
    - Return empty list if no adapters registered
    - Return 404 for unknown adapter_id
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 3.3, 3.4_
  
  - [ ] 6.4 Write property tests for adapter registry
    - **Property 4: Adapter List Completeness and Sorting** - all fields present, sorted by operator_name
    - **Property 5: Adapter Details Retrieval** - valid adapter_id returns complete details
    - **Property 6: Adapter Not Found** - invalid adapter_id returns 404
    - **Property 7: Checksum Format Validation** - checksum is sha256: followed by 64 hex chars
    - **Validates: Requirements 2.2, 2.4, 3.1, 3.2, 3.3, 3.4**

- [ ] 7. Checkpoint - Ensure adapter registry tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 8. Implement SQLite session store
  - [ ] 8.1 Implement SessionStore with SQLite backend
    - Create NewSessionStore accepting *sql.DB
    - Implement InitSchema to create sessions table
    - Implement Save to insert new session
    - Implement Update to modify existing session
    - Implement Get to retrieve session by ID
    - Implement GetActiveByVehicle to find active session for vehicle_id
    - Implement IsInitialized and Ping for health checks
    - _Requirements: 4.5, 8.3_
  
  - [ ] 8.2 Write unit tests for SessionStore
    - Test schema initialization
    - Test Save/Get round-trip
    - Test Update modifies existing session
    - Test GetActiveByVehicle returns correct session
    - Test Ping verifies database connection
    - _Requirements: 4.5, 8.3_

- [ ] 9. Implement mock parking operations
  - [ ] 9.1 Implement ParkingService
    - Implement StartSession with unique session_id generation (UUID)
    - Implement idempotent start: return existing active session for same vehicle_id
    - Implement StopSession with cost calculation
    - Implement idempotent stop: return previous result if already stopped
    - Implement GetSessionStatus with current cost calculation
    - Implement GetActiveSessionByVehicle
    - Implement CalculateCost: (duration_seconds / 3600) * hourly_rate, rounded to 2 decimals
    - _Requirements: 4.1, 4.4, 4.7, 5.1, 5.4, 5.5, 5.7, 6.1, 6.3, 6.4_
  
  - [ ] 9.2 Implement ParkingHandler
    - Handle POST /api/v1/parking/start
      - Validate required fields (vehicle_id, zone_id, timestamp, lat, lng)
      - Return existing session if active for vehicle_id (idempotent)
      - Return session_id, zone_id, hourly_rate, start_time
    - Handle POST /api/v1/parking/stop
      - Validate required fields (session_id, timestamp)
      - Return previous result if already stopped (idempotent)
      - Return session_id, start_time, end_time, duration_seconds, total_cost, payment_status
    - Handle GET /api/v1/parking/status/{session_id}
      - Return session_id, state, start_time, duration_seconds, current_cost, zone_id
    - Return 400 for validation errors, 404 for session not found
    - _Requirements: 4.1, 4.2, 4.3, 4.6, 4.7, 5.1, 5.2, 5.3, 5.6, 5.7, 6.1, 6.2, 6.5_
  
  - [ ] 9.3 Write property tests for parking operations
    - **Property 8: Session Creation Round-Trip** - created session retrievable via status endpoint
    - **Property 9: Session Start Idempotency** - duplicate start returns existing session
    - **Property 10: Session Stop Response Completeness** - all fields present, end_time > start_time
    - **Property 11: Session Stop Idempotency** - duplicate stop returns same result
    - **Property 12: Cost Calculation Correctness** - cost = (duration/3600) * rate, rounded
    - **Property 13: Mock Payment Always Succeeds** - payment_status always "success"
    - **Property 14: Session Not Found** - invalid session_id returns 404 for stop and status
    - **Property 15: Session Status Consistency** - state is "active" or "stopped" appropriately
    - **Validates: Requirements 4.1, 4.3, 4.4, 4.5, 4.7, 5.1, 5.3, 5.4, 5.5, 5.6, 5.7, 6.1, 6.2, 6.4, 6.5**

- [ ] 10. Checkpoint - Ensure parking operation tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 11. Implement health endpoints
  - [ ] 11.1 Implement HealthHandler
    - Handle GET /health
      - Return status "healthy", service name "parking-fee-service", timestamp
      - Should respond within 100ms
    - Handle GET /ready
      - Verify SQLite database connection via Ping
      - Return status "ready" if database operational
      - Return 503 with status "not ready" and reason if database unavailable
    - _Requirements: 7.1, 7.2, 7.3, 8.1, 8.2, 8.3_
  
  - [ ] 11.2 Write property test for readiness check
    - **Property 16: Readiness Check Database Verification** - ready only when DB connection operational
    - **Validates: Requirements 8.3**
  
  - [ ] 11.3 Write unit tests for health endpoints
    - Test health returns 200 with required fields
    - Test ready returns 200 when database initialized
    - Test ready returns 503 when database unavailable
    - _Requirements: 7.1, 7.2, 8.1, 8.2_

- [ ] 12. Implement main server and routing
  - [ ] 12.1 Create HTTP server with router
    - Set up gorilla/mux router
    - Register all API routes:
      - GET /api/v1/zones
      - GET /api/v1/adapters
      - GET /api/v1/adapters/{adapter_id}
      - POST /api/v1/parking/start
      - POST /api/v1/parking/stop
      - GET /api/v1/parking/status/{session_id}
      - GET /health
      - GET /ready
    - Apply middleware chain (request ID, logging)
    - _Requirements: 1.1, 2.1, 3.1, 4.1, 5.1, 6.1, 7.1, 8.1_
  
  - [ ] 12.2 Create main.go entry point
    - Load configuration from environment
    - Initialize SQLite database connection
    - Initialize stores with demo data from config
    - Initialize services and handlers
    - Start HTTP server on configured port
    - Log startup information
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 10.2_
  
  - [ ] 12.3 Write property test for error response format
    - **Property 17: Error Response Format Consistency** - all errors have error, message, request_id fields
    - **Validates: Requirements 11.1, 11.3**

- [ ] 13. Create Containerfile and deployment artifacts
  - [ ] 13.1 Create Containerfile for the service
    - Multi-stage build: builder stage with Go, runtime stage with minimal image
    - Copy binary and set as entrypoint
    - Set appropriate labels and environment defaults
    - Expose port 8080
    - _Requirements: 9.1_
  
  - [ ] 13.2 Update root Makefile with build targets
    - Add build-parking-fee-service target
    - Add test-parking-fee-service target for Go tests
    - Integrate with existing make build and make test targets
    - _Requirements: 9.1_

- [ ] 14. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- All tasks including property and unit tests are required
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties (17 total)
- Unit tests validate specific examples and edge cases
- The service uses:
  - Go with gorilla/mux for routing
  - gopter for property-based testing (minimum 100 iterations per property)
  - SQLite (modernc.org/sqlite) for session persistence
  - slog for structured JSON logging
