# Requirements Document

## Introduction

This document specifies requirements for a local development environment that enables developers to run the complete SDV Parking Demo System (excluding Android apps) on their local machine. The environment supports manual testing via CLI simulators (COMPANION_CLI and PARKING_CLI) and automated integration testing via TMT (Test Management Tool).

The local development environment orchestrates all backend services (Go), RHIVOS services (Rust), and infrastructure components (MQTT broker, Kuksa Databroker) through Makefile targets, providing a single command to bring up or tear down the complete system.

## Glossary

- **LOCAL_DEV_ENVIRONMENT**: The complete local development stack including all services except Android apps
- **COMPANION_CLI**: Go CLI simulator that mimics COMPANION_APP behavior for remote vehicle control via CLOUD_GATEWAY REST API
- **PARKING_CLI**: Go CLI simulator that mimics PARKING_APP behavior for parking session management via gRPC
- **LOCKING_SERVICE**: ASIL-B Rust service managing door lock state (safety partition)
- **DATA_BROKER**: Rust wrapper around Eclipse Kuksa Databroker for VSS signal pub/sub
- **CLOUD_GATEWAY_CLIENT**: Rust MQTT client for vehicle-to-cloud communication (safety partition)
- **PARKING_OPERATOR_ADAPTOR**: Rust dynamic adapter for parking operator integration (QM partition)
- **UPDATE_SERVICE**: Rust container lifecycle management service (QM partition)
- **CLOUD_GATEWAY**: Go REST API gateway and MQTT bridge for companion app communication
- **PARKING_FEE_SERVICE**: Go backend service for parking session management
- **KUKSA_DATABROKER**: Eclipse Kuksa VSS-compliant signal broker
- **MOSQUITTO**: Eclipse Mosquitto MQTT broker
- **MOCK_PARKING_OPERATOR**: Mock service simulating external parking operator API
- **TMT**: Test Management Tool for defining and executing integration tests
- **Health_Check**: HTTP or gRPC endpoint that reports service readiness

## Requirements

### Requirement 1: Start Local Development Environment

**User Story:** As a developer, I want a single make target to start all services except Android apps, so that I can quickly set up a complete local testing environment.

#### Acceptance Criteria

1. WHEN a developer runs `make dev-up` THEN the LOCAL_DEV_ENVIRONMENT SHALL start all infrastructure services (MOSQUITTO, KUKSA_DATABROKER, MOCK_PARKING_OPERATOR)
2. WHEN a developer runs `make dev-up` THEN the LOCAL_DEV_ENVIRONMENT SHALL start all backend services (CLOUD_GATEWAY, PARKING_FEE_SERVICE)
3. WHEN a developer runs `make dev-up` THEN the LOCAL_DEV_ENVIRONMENT SHALL start all RHIVOS services (LOCKING_SERVICE, DATA_BROKER, CLOUD_GATEWAY_CLIENT, PARKING_OPERATOR_ADAPTOR, UPDATE_SERVICE)
4. WHEN all services start THEN the LOCAL_DEV_ENVIRONMENT SHALL display status output showing each service name and its endpoint
5. WHEN a service fails to start THEN the LOCAL_DEV_ENVIRONMENT SHALL display an error message identifying the failed service
6. THE LOCAL_DEV_ENVIRONMENT SHALL NOT start any Android applications (PARKING_APP, COMPANION_APP)

### Requirement 2: Stop Local Development Environment

**User Story:** As a developer, I want a single make target to stop all running services, so that I can cleanly tear down the environment and free resources.

#### Acceptance Criteria

1. WHEN a developer runs `make dev-down` THEN the LOCAL_DEV_ENVIRONMENT SHALL stop all RHIVOS services
2. WHEN a developer runs `make dev-down` THEN the LOCAL_DEV_ENVIRONMENT SHALL stop all backend services
3. WHEN a developer runs `make dev-down` THEN the LOCAL_DEV_ENVIRONMENT SHALL stop all infrastructure services
4. WHEN services are stopped THEN the LOCAL_DEV_ENVIRONMENT SHALL release all bound ports
5. WHEN `make dev-down` completes THEN the LOCAL_DEV_ENVIRONMENT SHALL display confirmation that all services have stopped

### Requirement 3: Service Health Verification

**User Story:** As a developer, I want to verify that all services are healthy and ready, so that I can confirm the environment is properly running before testing.

#### Acceptance Criteria

1. WHEN a developer runs `make dev-status` THEN the LOCAL_DEV_ENVIRONMENT SHALL display the running state of each service
2. WHEN a service is healthy THEN the LOCAL_DEV_ENVIRONMENT SHALL display a success indicator for that service
3. WHEN a service is unhealthy or not running THEN the LOCAL_DEV_ENVIRONMENT SHALL display a failure indicator for that service
4. THE LOCAL_DEV_ENVIRONMENT SHALL check health endpoints for services that expose them (CLOUD_GATEWAY at `/health`, PARKING_FEE_SERVICE at `/health`, MOCK_PARKING_OPERATOR at `/health`)
5. THE LOCAL_DEV_ENVIRONMENT SHALL check port availability for services without HTTP health endpoints (KUKSA_DATABROKER at port 55556, MOSQUITTO at port 1883)

### Requirement 4: CLI Simulator Integration

**User Story:** As a developer, I want to use CLI simulators immediately after starting the environment, so that I can perform manual testing without additional configuration.

#### Acceptance Criteria

1. WHEN the LOCAL_DEV_ENVIRONMENT is running THEN COMPANION_CLI SHALL connect to CLOUD_GATEWAY at `http://localhost:8082`
2. WHEN the LOCAL_DEV_ENVIRONMENT is running THEN PARKING_CLI SHALL connect to KUKSA_DATABROKER at `localhost:55556`
3. WHEN the LOCAL_DEV_ENVIRONMENT is running THEN PARKING_CLI SHALL connect to PARKING_FEE_SERVICE at `http://localhost:8081`
4. WHEN the LOCAL_DEV_ENVIRONMENT is running THEN PARKING_CLI SHALL connect to LOCKING_SERVICE at `localhost:50053`
5. WHEN the LOCAL_DEV_ENVIRONMENT is running THEN PARKING_CLI SHALL connect to UPDATE_SERVICE at `localhost:50051`
6. WHEN the LOCAL_DEV_ENVIRONMENT is running THEN PARKING_CLI SHALL connect to PARKING_OPERATOR_ADAPTOR at `localhost:50052`
7. THE LOCAL_DEV_ENVIRONMENT SHALL document required environment variables for CLI simulators

### Requirement 5: TMT Integration Test Support

**User Story:** As a QA engineer, I want to run TMT integration tests against the local environment, so that I can validate system behavior through automated tests.

#### Acceptance Criteria

1. WHEN a developer runs `make dev-test` THEN the LOCAL_DEV_ENVIRONMENT SHALL execute TMT tests against running services
2. WHEN TMT tests execute THEN the LOCAL_DEV_ENVIRONMENT SHALL verify all services are healthy before running tests
3. WHEN TMT tests complete THEN the LOCAL_DEV_ENVIRONMENT SHALL display test results summary
4. IF any service is unhealthy before tests THEN the LOCAL_DEV_ENVIRONMENT SHALL abort test execution with an error message
5. THE LOCAL_DEV_ENVIRONMENT SHALL provide a TMT test plan file in FMF format at `tests/integration/main.fmf`

### Requirement 6: RHIVOS Service Execution

**User Story:** As a developer, I want RHIVOS services to run as native processes during local development, so that I can iterate quickly without container rebuilds.

#### Acceptance Criteria

1. WHEN `make dev-up` runs THEN the LOCAL_DEV_ENVIRONMENT SHALL build RHIVOS services using `cargo build --workspace`
2. WHEN `make dev-up` runs THEN the LOCAL_DEV_ENVIRONMENT SHALL start LOCKING_SERVICE listening on port 50053
3. WHEN `make dev-up` runs THEN the LOCAL_DEV_ENVIRONMENT SHALL start UPDATE_SERVICE listening on port 50051
4. WHEN `make dev-up` runs THEN the LOCAL_DEV_ENVIRONMENT SHALL start PARKING_OPERATOR_ADAPTOR listening on port 50052
5. WHEN `make dev-up` runs THEN the LOCAL_DEV_ENVIRONMENT SHALL start CLOUD_GATEWAY_CLIENT connecting to MOSQUITTO at port 1883
6. WHEN `make dev-up` runs THEN the LOCAL_DEV_ENVIRONMENT SHALL start DATA_BROKER connecting to KUKSA_DATABROKER at port 55556
7. THE LOCAL_DEV_ENVIRONMENT SHALL run RHIVOS services in the background with logs redirected to files in `logs/` directory

### Requirement 7: Port Assignment

**User Story:** As a developer, I want predictable port assignments for all services, so that I can configure clients and tests reliably.

#### Acceptance Criteria

1. THE LOCAL_DEV_ENVIRONMENT SHALL expose MOSQUITTO on ports 1883 (MQTT) and 8883 (MQTT/TLS)
2. THE LOCAL_DEV_ENVIRONMENT SHALL expose KUKSA_DATABROKER on port 55556 (gRPC)
3. THE LOCAL_DEV_ENVIRONMENT SHALL expose MOCK_PARKING_OPERATOR on port 8080 (HTTP)
4. THE LOCAL_DEV_ENVIRONMENT SHALL expose PARKING_FEE_SERVICE on port 8081 (HTTP)
5. THE LOCAL_DEV_ENVIRONMENT SHALL expose CLOUD_GATEWAY on port 8082 (HTTP)
6. THE LOCAL_DEV_ENVIRONMENT SHALL expose UPDATE_SERVICE on port 50051 (gRPC)
7. THE LOCAL_DEV_ENVIRONMENT SHALL expose PARKING_OPERATOR_ADAPTOR on port 50052 (gRPC)
8. THE LOCAL_DEV_ENVIRONMENT SHALL expose LOCKING_SERVICE on port 50053 (gRPC)

### Requirement 8: Dependency Management

**User Story:** As a developer, I want services to start in the correct order based on dependencies, so that the environment initializes reliably.

#### Acceptance Criteria

1. WHEN `make dev-up` runs THEN the LOCAL_DEV_ENVIRONMENT SHALL start MOSQUITTO before CLOUD_GATEWAY_CLIENT
2. WHEN `make dev-up` runs THEN the LOCAL_DEV_ENVIRONMENT SHALL start MOSQUITTO before CLOUD_GATEWAY
3. WHEN `make dev-up` runs THEN the LOCAL_DEV_ENVIRONMENT SHALL start KUKSA_DATABROKER before DATA_BROKER
4. WHEN `make dev-up` runs THEN the LOCAL_DEV_ENVIRONMENT SHALL start KUKSA_DATABROKER before PARKING_CLI operations
5. WHEN `make dev-up` runs THEN the LOCAL_DEV_ENVIRONMENT SHALL start PARKING_FEE_SERVICE before CLOUD_GATEWAY
6. WHEN a dependency service fails to start THEN the LOCAL_DEV_ENVIRONMENT SHALL not attempt to start dependent services
7. THE LOCAL_DEV_ENVIRONMENT SHALL wait for each service to be healthy before starting dependent services
