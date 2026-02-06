# Requirements Document

## Introduction

The CLI_SIMULATORS are Go command-line applications that enable local development testing of the SDV Parking Demo System without requiring Android emulators or physical devices. They provide REPL-style interfaces that simulate the COMPANION_APP and PARKING_APP functionality, using the same protobuf/gRPC interfaces as the real applications.

These tools allow developers to test the full vehicle-to-cloud flow locally by:
- Sending lock/unlock commands and viewing vehicle status (COMPANION_CLI)
- Setting vehicle location, managing parking operators, and controlling parking sessions (PARKING_CLI)

## Glossary

- **CLI_SIMULATORS**: Collective term for the COMPANION_CLI and PARKING_CLI tools
- **COMPANION_CLI**: CLI simulator replicating COMPANION_APP functionality for remote vehicle control
- **PARKING_CLI**: CLI simulator replicating PARKING_APP functionality for parking session management
- **CLOUD_GATEWAY**: Cloud-based service routing commands between apps and vehicles via REST API
- **LOCKING_SERVICE**: ASIL-B door locking service on RHIVOS accessed via gRPC
- **DATA_BROKER**: Eclipse Kuksa VSS-compliant signal broker accessed via gRPC
- **PARKING_FEE_SERVICE**: Backend service providing zone lookup and adapter registry via REST
- **PARKING_OPERATOR_ADAPTOR**: Dynamic adapter handling parking sessions via gRPC
- **UPDATE_SERVICE**: Container lifecycle management service via gRPC
- **REPL**: Read-Eval-Print Loop interactive command interface
- **VIN**: Vehicle Identification Number used to identify the target vehicle
- **VSS**: Vehicle Signal Specification (COVESA standard)

## Requirements

### Requirement 1: COMPANION_CLI Lock Command

**User Story:** As a developer, I want to send lock commands from the CLI, so that I can test the remote locking flow without the Flutter app.

#### Acceptance Criteria

1. WHEN the user enters the "lock" command, THE COMPANION_CLI SHALL send a POST request to the CLOUD_GATEWAY `/api/v1/vehicles/{vin}/commands` endpoint with command_type "lock"
2. WHEN the lock command is sent, THE COMPANION_CLI SHALL display the command_id returned by the CLOUD_GATEWAY
3. WHEN the lock command succeeds, THE COMPANION_CLI SHALL display "Lock command sent successfully" with the command_id
4. IF the lock command fails, THEN THE COMPANION_CLI SHALL display the error message from the CLOUD_GATEWAY response

### Requirement 2: COMPANION_CLI Unlock Command

**User Story:** As a developer, I want to send unlock commands from the CLI, so that I can test the remote unlocking flow without the Flutter app.

#### Acceptance Criteria

1. WHEN the user enters the "unlock" command, THE COMPANION_CLI SHALL send a POST request to the CLOUD_GATEWAY `/api/v1/vehicles/{vin}/commands` endpoint with command_type "unlock"
2. WHEN the unlock command is sent, THE COMPANION_CLI SHALL display the command_id returned by the CLOUD_GATEWAY
3. WHEN the unlock command succeeds, THE COMPANION_CLI SHALL display "Unlock command sent successfully" with the command_id
4. IF the unlock command fails, THEN THE COMPANION_CLI SHALL display the error message from the CLOUD_GATEWAY response

### Requirement 3: COMPANION_CLI Command Status

**User Story:** As a developer, I want to check the status of a command, so that I can verify command execution completed.

#### Acceptance Criteria

1. WHEN the user enters the "status <command_id>" command, THE COMPANION_CLI SHALL send a GET request to the CLOUD_GATEWAY `/api/v1/vehicles/{vin}/commands/{command_id}` endpoint
2. WHEN the status response is received, THE COMPANION_CLI SHALL display the command state (pending, completed, failed, timed_out)
3. IF the command completed successfully, THEN THE COMPANION_CLI SHALL display "Command completed successfully"
4. IF the command failed, THEN THE COMPANION_CLI SHALL display the error message from the response

### Requirement 4: PARKING_CLI Location Setting

**User Story:** As a developer, I want to set the vehicle location from the CLI, so that I can simulate the vehicle being in different parking zones.

#### Acceptance Criteria

1. WHEN the user enters the "location <lat> <lng>" command, THE PARKING_CLI SHALL send a SetSignal request to the DATA_BROKER via gRPC for Vehicle.CurrentLocation.Latitude and Vehicle.CurrentLocation.Longitude
2. WHEN the location is set successfully, THE PARKING_CLI SHALL display "Location set to (<lat>, <lng>)"
3. IF the DATA_BROKER connection fails, THEN THE PARKING_CLI SHALL display an error message with connection details
4. WHEN the user enters the "location" command without arguments, THE PARKING_CLI SHALL query and display the current location from the DATA_BROKER

### Requirement 5: PARKING_CLI Zone Lookup

**User Story:** As a developer, I want to look up parking zones, so that I can verify zone detection works for different locations.

#### Acceptance Criteria

1. WHEN the user enters the "zone" command, THE PARKING_CLI SHALL query the PARKING_FEE_SERVICE `/api/v1/zones` endpoint with the current location
2. WHEN a zone is found, THE PARKING_CLI SHALL display the zone_id, operator_name, hourly_rate, and adapter_image_ref
3. IF no zone is found, THEN THE PARKING_CLI SHALL display "No parking zone detected at current location"
4. IF the zone lookup fails, THEN THE PARKING_CLI SHALL display the error message from the response

### Requirement 6: PARKING_CLI Adapter Management

**User Story:** As a developer, I want to list and manage parking adapters, so that I can test the adapter lifecycle flow.

#### Acceptance Criteria

1. WHEN the user enters the "adapters" command, THE PARKING_CLI SHALL send a ListAdapters request to the UPDATE_SERVICE via gRPC
2. WHEN adapters are listed, THE PARKING_CLI SHALL display each adapter's adapter_id, image_ref, version, and state
3. WHEN the user enters the "install <image_ref>" command, THE PARKING_CLI SHALL send an InstallAdapter request to the UPDATE_SERVICE via gRPC
4. WHEN an adapter installation is requested, THE PARKING_CLI SHALL display the job_id and initial state
5. WHEN the user enters the "uninstall <adapter_id>" command, THE PARKING_CLI SHALL send an UninstallAdapter request to the UPDATE_SERVICE via gRPC
6. IF an adapter operation fails, THEN THE PARKING_CLI SHALL display the error message from the response

### Requirement 7: PARKING_CLI Session Management

**User Story:** As a developer, I want to start and stop parking sessions, so that I can test the full parking flow.

#### Acceptance Criteria

1. WHEN the user enters the "start <zone_id>" command, THE PARKING_CLI SHALL send a StartSession request to the PARKING_OPERATOR_ADAPTOR via gRPC
2. WHEN a session starts successfully, THE PARKING_CLI SHALL display the session_id and initial state
3. WHEN the user enters the "stop" command, THE PARKING_CLI SHALL send a StopSession request to the PARKING_OPERATOR_ADAPTOR via gRPC
4. WHEN a session stops successfully, THE PARKING_CLI SHALL display the session_id, final_cost, and duration_seconds
5. WHEN the user enters the "session" command, THE PARKING_CLI SHALL send a GetSessionStatus request to the PARKING_OPERATOR_ADAPTOR via gRPC
6. WHEN session status is received, THE PARKING_CLI SHALL display the session_id, state, zone_id, duration_seconds, and current_cost
7. IF a session operation fails, THEN THE PARKING_CLI SHALL display the error message from the response

### Requirement 8: REPL Interface

**User Story:** As a developer, I want an interactive command interface, so that I can efficiently test multiple operations in sequence.

#### Acceptance Criteria

1. WHEN the CLI starts, THE CLI_SIMULATORS SHALL display a welcome message with available commands
2. THE CLI_SIMULATORS SHALL display a prompt ("> ") and wait for user input
3. WHEN the user enters a command, THE CLI_SIMULATORS SHALL execute the command and display the result
4. WHEN the user enters "help", THE CLI_SIMULATORS SHALL display a list of available commands with descriptions
5. WHEN the user enters "quit" or "exit", THE CLI_SIMULATORS SHALL terminate gracefully
6. WHEN the user enters an unknown command, THE CLI_SIMULATORS SHALL display "Unknown command. Type 'help' for available commands."
7. THE CLI_SIMULATORS SHALL support command history navigation using up/down arrow keys

### Requirement 9: Configuration

**User Story:** As a developer, I want to configure service endpoints, so that I can connect to different environments.

#### Acceptance Criteria

1. THE CLI_SIMULATORS SHALL read configuration from environment variables
2. THE COMPANION_CLI SHALL read CLOUD_GATEWAY_URL from environment (default: http://localhost:8080)
3. THE COMPANION_CLI SHALL read VIN from environment (default: DEMO_VIN_001)
4. THE PARKING_CLI SHALL read DATA_BROKER_ADDR from environment (default: localhost:55556)
5. THE PARKING_CLI SHALL read PARKING_FEE_SERVICE_URL from environment (default: http://localhost:8081)
6. THE PARKING_CLI SHALL read UPDATE_SERVICE_ADDR from environment (default: localhost:50051)
7. THE PARKING_CLI SHALL read PARKING_ADAPTOR_ADDR from environment (default: localhost:50052)
8. WHEN the CLI starts, THE CLI_SIMULATORS SHALL display the configured endpoints

### Requirement 10: Connection Handling

**User Story:** As a developer, I want clear feedback on connection status, so that I can diagnose connectivity issues.

#### Acceptance Criteria

1. WHEN a gRPC connection fails, THE CLI_SIMULATORS SHALL display the target address and error details
2. WHEN an HTTP request fails, THE CLI_SIMULATORS SHALL display the URL and HTTP status code
3. WHEN a connection times out, THE CLI_SIMULATORS SHALL display "Connection timed out after <seconds>s"
4. THE CLI_SIMULATORS SHALL use a default timeout of 10 seconds for all operations
5. WHEN the user enters "ping", THE CLI_SIMULATORS SHALL test connectivity to all configured services and display status

### Requirement 11: Lock State Display

**User Story:** As a developer, I want to view the current door lock state directly, so that I can verify locking operations.

#### Acceptance Criteria

1. WHEN the user enters the "locks" command in PARKING_CLI, THE PARKING_CLI SHALL send a GetLockState request to the LOCKING_SERVICE via gRPC for all doors
2. WHEN lock state is received, THE PARKING_CLI SHALL display each door's lock status (locked/unlocked) and open status (open/closed)
3. IF the LOCKING_SERVICE connection fails, THEN THE PARKING_CLI SHALL display an error message with connection details

### Requirement 12: Non-Interactive Command Mode

**User Story:** As a developer, I want to run CLI commands from shell scripts or test tools, so that I can automate end-to-end testing without interactive REPL sessions.

#### Acceptance Criteria

1. WHEN the CLI is invoked with command arguments (e.g., `companion-cli lock`), THE CLI_SIMULATORS SHALL execute the command, print the result, and exit
2. WHEN the CLI is invoked with the `-c` or `--command` flag (e.g., `companion-cli -c "status abc123"`), THE CLI_SIMULATORS SHALL execute the command string, print the result, and exit
3. WHEN a non-interactive command succeeds, THE CLI_SIMULATORS SHALL exit with code 0
4. WHEN a non-interactive command fails, THE CLI_SIMULATORS SHALL exit with a non-zero exit code and print the error to stderr
5. WHEN the CLI is invoked without arguments, THE CLI_SIMULATORS SHALL start in interactive REPL mode (default behavior)
6. THE CLI_SIMULATORS SHALL support piping input from stdin (e.g., `echo "lock" | companion-cli`)
7. WHEN the `--json` flag is provided, THE CLI_SIMULATORS SHALL output results in JSON format for machine parsing
8. WHEN the `--quiet` or `-q` flag is provided, THE CLI_SIMULATORS SHALL suppress informational messages and only output the command result
