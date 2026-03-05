# Requirements: DATA_BROKER (Spec 02)

## Introduction

This document specifies the requirements for deploying and configuring Eclipse Kuksa Databroker as the DATA_BROKER in the RHIVOS safety partition. The DATA_BROKER provides a VSS-compliant gRPC pub/sub interface for vehicle signals, serving both same-partition consumers (via UDS) and cross-partition consumers (via network TCP).

## Glossary

| Term | Definition |
|------|-----------|
| DATA_BROKER | Eclipse Kuksa Databroker instance deployed in the RHIVOS safety partition |
| VSS | COVESA Vehicle Signal Specification, version 5.1 |
| UDS | Unix Domain Socket, used for same-partition gRPC communication |
| Overlay file | A JSON file that extends the standard VSS tree with custom signal definitions |
| Custom signal | A VSS signal defined in an overlay file, not part of the standard VSS v5.1 tree |
| Consumer | Any component that reads from or writes to the DATA_BROKER via gRPC |
| Same-partition consumer | A service running in the same RHIVOS partition as DATA_BROKER (e.g., LOCKING_SERVICE, CLOUD_GATEWAY_CLIENT) |
| Cross-partition consumer | A service running outside the safety partition (e.g., PARKING_APP, PARKING_OPERATOR_ADAPTOR) |

## Requirements

### Requirement 1: Kuksa Databroker Binary Deployment

**User Story:** As a platform engineer, I want the DATA_BROKER deployed as the official Kuksa Databroker binary, so that we use a proven, standards-compliant vehicle signal broker without custom code.

#### Acceptance Criteria

1. **02-REQ-1.1** THE DATA_BROKER SHALL be deployed as the Eclipse Kuksa Databroker pre-built binary without any wrapper code or reimplementation.
2. **02-REQ-1.2** WHEN the DATA_BROKER binary is started, THE DATA_BROKER SHALL reach a healthy state (accepting gRPC connections) within 30 seconds.

#### Edge Cases

1. **02-REQ-1.E1** IF the DATA_BROKER binary fails to start (e.g., missing configuration, port conflict), THEN THE system SHALL report the failure via a non-zero exit code and log the error cause.

### Requirement 2: VSS Overlay for Custom Signals

**User Story:** As a system integrator, I want custom VSS signals defined in an overlay file, so that all vehicle services can publish and subscribe to parking-specific and command signals.

#### Acceptance Criteria

1. **02-REQ-2.1** THE DATA_BROKER SHALL load a VSS overlay file at startup that defines the following custom signals:

   | Signal Path | Data Type | Description |
   |-------------|-----------|-------------|
   | `Vehicle.Parking.SessionActive` | boolean | Parking session state |
   | `Vehicle.Command.Door.Lock` | string | Lock/unlock command request (JSON payload) |
   | `Vehicle.Command.Door.Response` | string | Command execution result (JSON payload) |

2. **02-REQ-2.2** WHEN the overlay is loaded, THE DATA_BROKER SHALL register the custom signals alongside the standard VSS v5.1 signals without conflicts or overwriting standard signal paths.

#### Edge Cases

1. **02-REQ-2.E1** IF the overlay file is missing or contains malformed JSON, THEN THE DATA_BROKER SHALL fail to start and report the error.

### Requirement 3: Standard VSS Signal Availability

**User Story:** As a vehicle service developer, I want standard VSS signals available in the DATA_BROKER, so that I can read and write vehicle state using industry-standard signal paths.

#### Acceptance Criteria

1. **02-REQ-3.1** THE DATA_BROKER SHALL register the following standard VSS v5.1 signals on startup:

   | Signal Path | Data Type |
   |-------------|-----------|
   | `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` | bool |
   | `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` | bool |
   | `Vehicle.CurrentLocation.Latitude` | double |
   | `Vehicle.CurrentLocation.Longitude` | double |
   | `Vehicle.Speed` | float |

2. **02-REQ-3.2** WHEN a consumer reads a signal that has not yet been written to, THE DATA_BROKER SHALL return the signal metadata with no current value (not-yet-set state).

### Requirement 4: UDS Listener for Same-Partition Consumers

**User Story:** As a safety-partition service developer, I want to connect to the DATA_BROKER via Unix Domain Sockets, so that same-partition communication avoids the network stack.

#### Acceptance Criteria

1. **02-REQ-4.1** THE DATA_BROKER SHALL expose a gRPC endpoint over a Unix Domain Socket at a configurable path so that same-partition consumers can connect.
2. **02-REQ-4.2** WHEN a same-partition consumer connects via the UDS endpoint, THE consumer SHALL be able to perform get, set, and subscribe operations on all registered signals.

#### Edge Cases

1. **02-REQ-4.E1** IF the UDS path is not accessible (e.g., permission denied, path does not exist), THEN THE consumer SHALL receive a connection error.

### Requirement 5: Network TCP Listener for Cross-Partition Consumers

**User Story:** As a QM-partition service developer, I want to connect to the DATA_BROKER via network TCP, so that cross-partition services can access vehicle signals over gRPC.

#### Acceptance Criteria

1. **02-REQ-5.1** THE DATA_BROKER SHALL expose a gRPC endpoint over network TCP on port 55556 so that cross-partition consumers can connect.
2. **02-REQ-5.2** WHEN a cross-partition consumer connects to the TCP endpoint, THE consumer SHALL be able to perform get, set, and subscribe operations on all registered signals.

#### Edge Cases

1. **02-REQ-5.E1** IF the DATA_BROKER is unreachable on port 55556, THEN THE consumer SHALL receive a gRPC UNAVAILABLE error on connection attempt.

### Requirement 6: Signal Read/Write Operations

**User Story:** As a vehicle service developer, I want to read and write signal values via gRPC, so that services can share vehicle state through the DATA_BROKER.

#### Acceptance Criteria

1. **02-REQ-6.1** WHEN a consumer writes a value to a registered signal via gRPC SetRequest, THE DATA_BROKER SHALL store the value and make it available for subsequent reads.
2. **02-REQ-6.2** WHEN a consumer reads a signal via gRPC GetRequest, THE DATA_BROKER SHALL return the most recently written value for that signal.

#### Edge Cases

1. **02-REQ-6.E1** IF a consumer attempts to get or set a signal path that does not exist in the registered VSS tree, THEN THE DATA_BROKER SHALL return a gRPC NOT_FOUND error.
2. **02-REQ-6.E2** IF a consumer writes a value with a type that does not match the signal's registered data type, THEN THE DATA_BROKER SHALL return a gRPC INVALID_ARGUMENT error.

### Requirement 7: gRPC Streaming Subscriptions

**User Story:** As a vehicle service developer, I want to subscribe to signal changes via gRPC streaming, so that my service receives real-time updates when signal values change.

#### Acceptance Criteria

1. **02-REQ-7.1** WHEN a consumer subscribes to a signal and another consumer writes a new value to that signal, THE DATA_BROKER SHALL deliver the updated value to the subscriber on the subscription stream.
2. **02-REQ-7.2** WHEN multiple consumers subscribe to the same signal, THE DATA_BROKER SHALL deliver the update to all active subscribers.

#### Edge Cases

1. **02-REQ-7.E1** IF a subscriber's connection drops during an active subscription, THEN THE gRPC stream SHALL terminate and updates for that subscriber SHALL be discarded (no durable queues).

### Requirement 8: Health and Readiness Check

**User Story:** As an operations engineer, I want to verify that the DATA_BROKER is healthy and ready, so that dependent services can wait for availability before connecting.

#### Acceptance Criteria

1. **02-REQ-8.1** THE DATA_BROKER SHALL support a health check mechanism that indicates whether it is ready to accept gRPC connections.
2. **02-REQ-8.2** WHEN the DATA_BROKER is not yet ready (still starting up), THE health check SHALL report an unhealthy or not-ready state.
