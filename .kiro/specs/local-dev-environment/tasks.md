# Implementation Plan: Local Development Environment

## Overview

This plan implements a local development environment for the SDV Parking Demo System. The implementation uses shell scripts for orchestration, with Makefile targets as the developer interface. RHIVOS services run as native Rust processes while infrastructure services run in containers via podman-compose.

## Tasks

- [ ] 1. Create orchestration script foundation
  - [ ] 1.1 Create `scripts/local-dev.sh` with command parsing (start, stop, status, test, logs)
    - Implement argument parsing for subcommands
    - Add help text and usage information
    - Set up color-coded output functions (log_info, log_success, log_error, log_warn)
    - Create `logs/` and `logs/pids/` directories if they don't exist
    - _Requirements: 1.4, 1.5, 2.5_

  - [ ] 1.2 Implement port and health check utility functions
    - `check_port()`: Use netcat to verify port is listening
    - `check_http_health()`: Use curl to check HTTP health endpoints
    - `wait_for_port()`: Poll port with timeout
    - `wait_for_health()`: Poll health endpoint with timeout
    - _Requirements: 3.4, 3.5, 8.7_

- [ ] 2. Implement infrastructure service management
  - [ ] 2.1 Implement `start_infrastructure()` function
    - Call `podman-compose up -d` in `infra/compose/`
    - Wait for MOSQUITTO health (port 1883)
    - Wait for KUKSA_DATABROKER health (port 55556)
    - Wait for MOCK_PARKING_OPERATOR health (HTTP /health on 8080)
    - Wait for PARKING_FEE_SERVICE health (HTTP /health on 8081)
    - Wait for CLOUD_GATEWAY health (HTTP /health on 8082)
    - Display status for each service
    - _Requirements: 1.1, 1.2, 8.1, 8.2, 8.3, 8.5_

  - [ ] 2.2 Implement `stop_infrastructure()` function
    - Call `podman-compose down` in `infra/compose/`
    - Verify ports are released
    - _Requirements: 2.2, 2.3, 2.4_

- [ ] 3. Implement RHIVOS service management
  - [ ] 3.1 Implement `build_rhivos()` function
    - Run `cargo build --workspace` in `rhivos/`
    - Handle build failures with clear error messages
    - _Requirements: 6.1_

  - [ ] 3.2 Implement `start_rhivos_service()` generic function
    - Accept service name, binary path, port, and environment variables
    - Start service in background with nohup
    - Redirect stdout/stderr to `logs/{service}.log`
    - Write PID to `logs/pids/{service}.pid`
    - Wait for service to be healthy (port listening)
    - _Requirements: 6.7_

  - [ ] 3.3 Implement `start_locking_service()` function
    - Set environment: `RUST_LOG=debug`
    - Binary: `rhivos/target/debug/locking-service`
    - Port: 50053
    - Wait for port to be listening
    - _Requirements: 1.3, 6.2, 7.8_

  - [ ] 3.4 Implement `start_update_service()` function
    - Set environment: `UPDATE_SERVICE_LISTEN_ADDR=0.0.0.0:50051`, `LOG_LEVEL=debug`
    - Binary: `rhivos/target/debug/update-service`
    - Port: 50051
    - Wait for port to be listening
    - _Requirements: 1.3, 6.3, 7.6_

  - [ ] 3.5 Implement `start_parking_operator_adaptor()` function
    - Set environment variables for local development
    - Binary: `rhivos/target/debug/parking-operator-adaptor`
    - Port: 50052
    - Wait for port to be listening
    - _Requirements: 1.3, 6.4, 7.7_

  - [ ] 3.6 Implement `start_cloud_gateway_client()` function
    - Set environment variables for local MQTT connection
    - Binary: `rhivos/target/debug/cloud-gateway-client`
    - No port (connects to MOSQUITTO)
    - Check PID file for running status
    - _Requirements: 1.3, 6.5_

  - [ ] 3.7 Implement `stop_rhivos_services()` function
    - Read PID files from `logs/pids/`
    - Send SIGTERM to each process
    - Wait up to 5 seconds for graceful shutdown
    - Send SIGKILL if still running
    - Remove PID files
    - _Requirements: 2.1_

- [ ] 4. Checkpoint - Verify start/stop functionality
  - Ensure `scripts/local-dev.sh start` and `scripts/local-dev.sh stop` work correctly
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 5. Implement status and test commands
  - [ ] 5.1 Implement `show_status()` function
    - Check each service health/port
    - Display colored status (green=healthy, red=unhealthy)
    - Show service name, port, and status
    - _Requirements: 3.1, 3.2, 3.3_

  - [ ] 5.2 Implement `run_tests()` function
    - First call `show_status()` and abort if any service unhealthy
    - Execute TMT with test plan from `tests/integration/`
    - Display test results summary
    - _Requirements: 5.1, 5.2, 5.3, 5.4_

  - [ ] 5.3 Implement `show_logs()` function
    - Tail all log files in `logs/` directory
    - Use `tail -f` with service name prefixes
    - _Requirements: 6.7_

- [ ] 6. Add Makefile targets
  - [ ] 6.1 Add `dev-up` target
    - Call `scripts/local-dev.sh start`
    - Add to help text
    - _Requirements: 1.1, 1.2, 1.3, 1.4_

  - [ ] 6.2 Add `dev-down` target
    - Call `scripts/local-dev.sh stop`
    - Add to help text
    - _Requirements: 2.1, 2.2, 2.3, 2.5_

  - [ ] 6.3 Add `dev-status` target
    - Call `scripts/local-dev.sh status`
    - Add to help text
    - _Requirements: 3.1, 3.2, 3.3_

  - [ ] 6.4 Add `dev-test` target
    - Call `scripts/local-dev.sh test`
    - Add to help text
    - _Requirements: 5.1, 5.2, 5.3_

  - [ ] 6.5 Add `dev-logs` target
    - Call `scripts/local-dev.sh logs`
    - Add to help text

- [ ] 7. Checkpoint - Verify Makefile integration
  - Ensure all `make dev-*` targets work correctly
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 8. Create TMT test infrastructure
  - [ ] 8.1 Create `tests/integration/main.fmf` TMT test plan
    - Define test metadata (summary, description)
    - Configure discover, execute, and prepare phases
    - Set environment variables for CLI simulators
    - _Requirements: 5.5_

  - [ ] 8.2 Create basic integration test scripts
    - `tests/integration/test-service-health.sh`: Verify all services respond
    - `tests/integration/test-cli-connectivity.sh`: Test CLI simulator connections
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6_

- [ ] 9. Update documentation
  - [ ] 9.1 Update `docs/local-infrastructure.md` with new targets
    - Document `make dev-up`, `dev-down`, `dev-status`, `dev-test`, `dev-logs`
    - Document environment variables for CLI simulators
    - Add troubleshooting section for common issues
    - _Requirements: 4.7_

  - [ ] 9.2 Update root `README.md` with quick start
    - Add "Local Development" section
    - Show basic workflow: `make dev-up`, run CLI, `make dev-down`

- [ ] 10. Final checkpoint
  - Ensure all tests pass, ask the user if questions arise.
  - Verify complete workflow: `make dev-up` → CLI testing → `make dev-down`

- [ ]* 11. Write property tests
  - [ ]* 11.1 Create `tests/integration/local-dev-environment.bats` test file
    - Set up bats-core test structure
    - Add setup/teardown functions
    - **Property 1: Service Startup Completeness**
    - **Validates: Requirements 1.1, 1.2, 1.3, 6.2-6.6, 7.1-7.8**

  - [ ]* 11.2 Add shutdown completeness test
    - **Property 2: Service Shutdown Completeness**
    - **Validates: Requirements 2.1, 2.2, 2.3, 2.4**

  - [ ]* 11.3 Add CLI connectivity test
    - **Property 3: CLI Connectivity**
    - **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 4.6**

  - [ ]* 11.4 Add dependency ordering test
    - **Property 4: Dependency Ordering**
    - **Validates: Requirements 8.1, 8.2, 8.3, 8.4, 8.5, 8.7**

  - [ ]* 11.5 Add log file creation test
    - **Property 5: Log File Creation**
    - **Validates: Requirements 6.7**

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- RHIVOS services run as native processes (not containers) for faster iteration during development
- The DATA_BROKER service is a placeholder (directory exists but no implementation) - skip for now
- CLOUD_GATEWAY_CLIENT may fail to start if MQTT TLS is required - use insecure mode for local dev
- Property tests require `bats-core` to be installed (`brew install bats-core` on macOS)
