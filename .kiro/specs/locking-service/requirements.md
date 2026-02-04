# Requirements Document

## Introduction

This document defines the requirements for the LOCKING_SERVICE component of the SDV Parking Demo System. The LOCKING_SERVICE is an ASIL-B safety-critical service running in the RHIVOS safety partition that handles door lock/unlock commands and publishes lock state changes to the DATA_BROKER.

The service receives commands from the CLOUD_GATEWAY_CLIENT (originating from the COMPANION_APP via cloud), validates safety constraints before execution, and publishes state changes to enable the parking session workflow.

## Glossary

- **LOCKING_SERVICE**: ASIL-B door locking service running in the RHIVOS safety partition
- **CLOUD_GATEWAY_CLIENT**: Service that receives authenticated commands from the cloud and forwards them to LOCKING_SERVICE
- **DATA_BROKER**: Eclipse Kuksa Databroker providing VSS-compliant gRPC pub/sub interface for vehicle signals
- **VSS**: Vehicle Signal Specification - standardized taxonomy for vehicle data from COVESA
- **ASIL-B**: Automotive Safety Integrity Level B - safety classification requiring systematic capability
- **RHIVOS**: Red Hat In-Vehicle Operating System with safety and QM partitions
- **UDS**: Unix Domain Sockets for local inter-process communication
- **gRPC**: High-performance RPC framework using Protocol Buffers
- **Command_ID**: Unique identifier for tracking command execution and responses
- **Auth_Token**: Authentication token for basic command authorization (demo-grade, not production)

## Requirements

### Requirement 1: Lock Command Handling

**User Story:** As a vehicle owner using the COMPANION_APP, I want to remotely lock my vehicle doors, so that I can secure my vehicle and start a parking session.

#### Acceptance Criteria

1. WHEN the LOCKING_SERVICE receives a Lock command with a valid Auth_Token THEN the LOCKING_SERVICE SHALL validate safety constraints before execution
2. WHEN all safety constraints pass for a Lock command THEN the LOCKING_SERVICE SHALL execute the lock operation and return a success response with the Command_ID
3. WHEN the door is ajar (IsOpen = true) during a Lock command THEN the LOCKING_SERVICE SHALL reject the command and return an error indicating the door must be closed
4. WHEN the Auth_Token is invalid or missing THEN the LOCKING_SERVICE SHALL reject the command and return an authentication error
5. WHEN a Lock command succeeds THEN the LOCKING_SERVICE SHALL publish the updated IsLocked state (true) to the DATA_BROKER

### Requirement 2: Unlock Command Handling

**User Story:** As a vehicle owner using the COMPANION_APP, I want to remotely unlock my vehicle doors, so that I can access my vehicle and end a parking session.

#### Acceptance Criteria

1. WHEN the LOCKING_SERVICE receives an Unlock command with a valid Auth_Token THEN the LOCKING_SERVICE SHALL validate safety constraints before execution
2. WHEN all safety constraints pass for an Unlock command THEN the LOCKING_SERVICE SHALL execute the unlock operation and return a success response with the Command_ID
3. WHEN the vehicle speed is greater than 0 during an Unlock command THEN the LOCKING_SERVICE SHALL reject the command and return an error indicating the vehicle must be stationary
4. WHEN the Auth_Token is invalid or missing THEN the LOCKING_SERVICE SHALL reject the command and return an authentication error
5. WHEN an Unlock command succeeds THEN the LOCKING_SERVICE SHALL publish the updated IsLocked state (false) to the DATA_BROKER

### Requirement 3: Lock State Query

**User Story:** As a system component, I want to query the current lock state, so that I can determine the vehicle's security status.

#### Acceptance Criteria

1. WHEN the LOCKING_SERVICE receives a GetLockState request THEN the LOCKING_SERVICE SHALL return the current lock state for the specified door
2. THE GetLockState response SHALL include both the IsLocked and IsOpen status for the requested door
3. WHEN an invalid door identifier is provided THEN the LOCKING_SERVICE SHALL return an error indicating the door is not recognized

### Requirement 4: Safety Constraint Validation

**User Story:** As a safety engineer, I want the LOCKING_SERVICE to enforce safety constraints, so that door operations do not create hazardous conditions.

#### Acceptance Criteria

1. THE LOCKING_SERVICE SHALL read the Vehicle.Speed signal from the DATA_BROKER before executing any Unlock command
2. THE LOCKING_SERVICE SHALL read the Vehicle.Cabin.Door.Row1.DriverSide.IsOpen signal from the DATA_BROKER before executing any Lock command
3. IF the DATA_BROKER is unavailable THEN the LOCKING_SERVICE SHALL reject all commands and return a service unavailable error
4. THE LOCKING_SERVICE SHALL complete safety constraint validation within 100ms of receiving a command

### Requirement 5: DATA_BROKER Integration

**User Story:** As a system integrator, I want the LOCKING_SERVICE to publish state changes to the DATA_BROKER, so that other components can react to lock events.

#### Acceptance Criteria

1. WHEN a lock state change occurs THEN the LOCKING_SERVICE SHALL publish the new state to Vehicle.Cabin.Door.Row1.DriverSide.IsLocked via gRPC
2. THE LOCKING_SERVICE SHALL connect to the DATA_BROKER via Unix Domain Socket at the configured path
3. IF publishing to the DATA_BROKER fails THEN the LOCKING_SERVICE SHALL retry up to 3 times with exponential backoff
4. IF all publish retries fail THEN the LOCKING_SERVICE SHALL log the failure and return a partial success response indicating the lock operation succeeded but state publication failed

### Requirement 6: Command Timeout Handling

**User Story:** As a system operator, I want commands to have bounded execution time, so that the system remains responsive and predictable.

#### Acceptance Criteria

1. THE LOCKING_SERVICE SHALL complete command execution within 500ms of receiving the request
2. IF command execution exceeds the timeout THEN the LOCKING_SERVICE SHALL abort the operation and return a timeout error
3. WHEN a timeout occurs THEN the LOCKING_SERVICE SHALL ensure the lock mechanism is left in a safe state (no partial operations)

### Requirement 7: Operation Logging

**User Story:** As a system auditor, I want all lock operations to be logged, so that I can trace command execution for debugging and compliance.

#### Acceptance Criteria

1. THE LOCKING_SERVICE SHALL log every received command with timestamp, Command_ID, command type, and door identifier
2. THE LOCKING_SERVICE SHALL log the result of every command execution including success/failure status and any error codes
3. THE LOCKING_SERVICE SHALL log all safety constraint validation results
4. THE LOCKING_SERVICE SHALL include correlation identifiers in logs to enable end-to-end tracing

### Requirement 8: gRPC Service Interface

**User Story:** As a developer, I want a well-defined gRPC interface, so that I can integrate with the LOCKING_SERVICE from other components.

#### Acceptance Criteria

1. THE LOCKING_SERVICE SHALL expose a Lock RPC that accepts LockRequest and returns LockResponse
2. THE LOCKING_SERVICE SHALL expose an Unlock RPC that accepts UnlockRequest and returns UnlockResponse
3. THE LOCKING_SERVICE SHALL expose a GetLockState RPC that accepts GetLockStateRequest and returns GetLockStateResponse
4. THE LOCKING_SERVICE SHALL listen on a Unix Domain Socket for local gRPC connections from CLOUD_GATEWAY_CLIENT
5. THE LOCKING_SERVICE SHALL use standard gRPC status codes with custom ErrorDetails for domain-specific errors
