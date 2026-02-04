# Requirements Document

## Introduction

This document defines the requirements for the PARKING_FEE_SERVICE component of the SDV Parking Demo System. The PARKING_FEE_SERVICE is a Go backend service deployed on OpenShift that provides REST APIs for parking operations, manages parking zone lookups, and serves adapter registry information.

The service acts as the backend for the parking system, providing zone information based on vehicle location, adapter metadata for the UPDATE_SERVICE to download, and mock parking operator functionality for demo purposes.

## Glossary

- **PARKING_FEE_SERVICE**: Go backend service providing REST APIs for parking operations and adapter registry
- **PARKING_OPERATOR_ADAPTOR**: Containerized parking session management service that calls PARKING_FEE_SERVICE APIs
- **UPDATE_SERVICE**: RHIVOS service that downloads adapter containers based on registry information
- **Zone**: A geographic parking area with associated operator, rates, and adapter
- **Zone_ID**: Unique identifier for a parking zone
- **Adapter**: OCI container image implementing parking operator integration
- **Adapter_ID**: Unique identifier for an adapter in the registry
- **Image_Ref**: OCI image reference (registry/repository:tag format)
- **Checksum**: SHA256 digest of the adapter container image for integrity verification
- **Session**: A parking session with start time, location, and billing information
- **Session_ID**: Unique identifier for a parking session
- **OpenShift**: Red Hat's Kubernetes platform where the service is deployed
- **OCI**: Open Container Initiative - standard for container images
- **Google Artifact Registry**: Cloud container registry storing adapter images

## Requirements

### Requirement 1: Zone Lookup by Location

**User Story:** As a vehicle system, I want to find the parking zone for my current location, so that I can download the appropriate parking operator adapter.

#### Acceptance Criteria

1. WHEN the PARKING_FEE_SERVICE receives a GET /api/v1/zones request with lat and lng query parameters THEN it SHALL return the parking zone containing that location
2. THE zone response SHALL include zone_id, operator_name, hourly_rate, currency, adapter_image_ref, and adapter_checksum
3. IF the location is within the demo zone bounds THEN the PARKING_FEE_SERVICE SHALL return the demo zone information
4. IF the location is outside all configured zone bounds THEN the PARKING_FEE_SERVICE SHALL return HTTP 404 with an error message indicating no zone found
5. IF lat or lng parameters are missing THEN the PARKING_FEE_SERVICE SHALL return HTTP 400 with an error message indicating required parameters
6. IF lat or lng parameters are invalid (non-numeric or out of range) THEN the PARKING_FEE_SERVICE SHALL return HTTP 400 with a validation error message

### Requirement 2: Adapter Registry Listing

**User Story:** As an UPDATE_SERVICE, I want to list all available parking operator adapters, so that I can manage adapter lifecycle.

#### Acceptance Criteria

1. WHEN the PARKING_FEE_SERVICE receives a GET /api/v1/adapters request THEN it SHALL return a list of all registered adapters
2. THE adapter list response SHALL include adapter_id, operator_name, version, and image_ref for each adapter
3. THE PARKING_FEE_SERVICE SHALL return an empty list if no adapters are registered
4. THE adapter list SHALL be sorted by operator_name alphabetically

### Requirement 3: Adapter Details Retrieval

**User Story:** As an UPDATE_SERVICE, I want to get detailed information about a specific adapter, so that I can verify and download the correct container image.

#### Acceptance Criteria

1. WHEN the PARKING_FEE_SERVICE receives a GET /api/v1/adapters/{adapter_id} request THEN it SHALL return the adapter details
2. THE adapter details response SHALL include adapter_id, operator_name, version, image_ref, checksum, and created_at
3. IF the adapter_id does not exist THEN the PARKING_FEE_SERVICE SHALL return HTTP 404 with an error message
4. THE checksum SHALL be a SHA256 digest of the container image

### Requirement 4: Mock Parking Session Start

**User Story:** As a PARKING_OPERATOR_ADAPTOR, I want to start a parking session, so that the vehicle's parking time is tracked.

#### Acceptance Criteria

1. WHEN the PARKING_FEE_SERVICE receives a POST /api/v1/parking/start request THEN it SHALL create a new parking session
2. THE start request SHALL accept vehicle_id, latitude, longitude, zone_id, and timestamp in the request body
3. WHEN a session is created successfully THEN the PARKING_FEE_SERVICE SHALL return session_id, zone_id, hourly_rate, and start_time
4. THE PARKING_FEE_SERVICE SHALL generate a unique session_id for each new session
5. THE PARKING_FEE_SERVICE SHALL store the session in memory for the demo
6. IF required fields are missing THEN the PARKING_FEE_SERVICE SHALL return HTTP 400 with a validation error

### Requirement 5: Mock Parking Session Stop

**User Story:** As a PARKING_OPERATOR_ADAPTOR, I want to stop a parking session, so that the final cost can be calculated and payment processed.

#### Acceptance Criteria

1. WHEN the PARKING_FEE_SERVICE receives a POST /api/v1/parking/stop request THEN it SHALL end the specified parking session
2. THE stop request SHALL accept session_id and timestamp in the request body
3. WHEN a session is stopped successfully THEN the PARKING_FEE_SERVICE SHALL return session_id, start_time, end_time, duration_seconds, total_cost, and payment_status
4. THE total_cost SHALL be calculated as (duration_seconds / 3600) * hourly_rate
5. THE payment_status SHALL always be "success" for the demo (mock payment)
6. IF the session_id does not exist THEN the PARKING_FEE_SERVICE SHALL return HTTP 404 with an error message
7. IF the session is already stopped THEN the PARKING_FEE_SERVICE SHALL return HTTP 409 with an error message

### Requirement 6: Mock Parking Session Status

**User Story:** As a PARKING_OPERATOR_ADAPTOR, I want to query the status of a parking session, so that I can display current cost to the user.

#### Acceptance Criteria

1. WHEN the PARKING_FEE_SERVICE receives a GET /api/v1/parking/status/{session_id} request THEN it SHALL return the session status
2. THE status response SHALL include session_id, state, start_time, duration_seconds, current_cost, and zone_id
3. THE current_cost SHALL be calculated based on elapsed time since start_time
4. THE state SHALL be "active" for ongoing sessions and "stopped" for ended sessions
5. IF the session_id does not exist THEN the PARKING_FEE_SERVICE SHALL return HTTP 404 with an error message

### Requirement 7: Health Check Endpoint

**User Story:** As an OpenShift operator, I want to check if the service is running, so that I can monitor service health.

#### Acceptance Criteria

1. WHEN the PARKING_FEE_SERVICE receives a GET /health request THEN it SHALL return HTTP 200 with status "healthy"
2. THE health response SHALL include service name and current timestamp
3. THE health endpoint SHALL respond within 100ms under normal conditions

### Requirement 8: Readiness Check Endpoint

**User Story:** As an OpenShift operator, I want to check if the service is ready to accept traffic, so that I can manage load balancing.

#### Acceptance Criteria

1. WHEN the PARKING_FEE_SERVICE receives a GET /ready request THEN it SHALL return HTTP 200 if ready to serve requests
2. IF the service is not ready THEN the PARKING_FEE_SERVICE SHALL return HTTP 503 with status "not ready"
3. THE readiness check SHALL verify that the in-memory session store is initialized

### Requirement 9: Configuration Management

**User Story:** As a system operator, I want to configure the service via environment variables, so that I can deploy it in different environments.

#### Acceptance Criteria

1. THE PARKING_FEE_SERVICE SHALL support configuration via environment variables
2. THE PARKING_FEE_SERVICE SHALL be configurable for: HTTP listen port, demo zone bounds (min/max lat/lng), demo adapter image reference, demo adapter checksum, and demo hourly rate
3. THE PARKING_FEE_SERVICE SHALL provide sensible defaults for all configuration options
4. WHEN required configuration is missing THEN the PARKING_FEE_SERVICE SHALL use default values and log a warning

### Requirement 10: Request Logging

**User Story:** As a system operator, I want all API requests to be logged, so that I can debug issues and monitor usage.

#### Acceptance Criteria

1. THE PARKING_FEE_SERVICE SHALL log every incoming HTTP request with timestamp, method, path, and response status
2. THE PARKING_FEE_SERVICE SHALL log all parking session operations with session_id and operation type
3. THE PARKING_FEE_SERVICE SHALL use structured JSON logging format
4. THE PARKING_FEE_SERVICE SHALL include request duration in log entries

### Requirement 11: Error Response Format

**User Story:** As an API consumer, I want consistent error responses, so that I can handle errors programmatically.

#### Acceptance Criteria

1. THE PARKING_FEE_SERVICE SHALL return errors in a consistent JSON format with error code and message fields
2. THE PARKING_FEE_SERVICE SHALL use appropriate HTTP status codes (400 for client errors, 404 for not found, 500 for server errors)
3. THE PARKING_FEE_SERVICE SHALL include a request_id in error responses for tracing
4. THE PARKING_FEE_SERVICE SHALL NOT expose internal error details in production responses

