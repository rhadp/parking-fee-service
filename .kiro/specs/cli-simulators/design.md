# Design Document

## Overview

The CLI_SIMULATORS feature provides two Go command-line applications for local development testing:

1. **COMPANION_CLI** (`backend/companion-cli/`) - Simulates the Flutter COMPANION_APP for remote vehicle control via the CLOUD_GATEWAY REST API
2. **PARKING_CLI** (`backend/parking-cli/`) - Simulates the Kotlin PARKING_APP for parking session management via gRPC services

Both CLIs use a REPL (Read-Eval-Print Loop) interface similar to interactive shells, allowing developers to test the full vehicle-to-cloud flow without Android emulators.

### Key Design Decisions

- **Go language**: Matches existing backend services, shares generated protobuf code from `backend/gen/`
- **REPL interface**: Interactive command loop with readline support for history and editing
- **Non-interactive mode**: Support for scripting and automated testing via CLI arguments
- **Same interfaces**: Uses identical REST/gRPC endpoints as the real mobile apps
- **Minimal dependencies**: Standard library plus existing project dependencies (gRPC, protobuf)

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Developer Workstation                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────┐                      ┌──────────────────┐             │
│  │  COMPANION_CLI   │                      │   PARKING_CLI    │             │
│  │                  │                      │                  │             │
│  │  Commands:       │                      │  Commands:       │             │
│  │  - lock          │                      │  - location      │             │
│  │  - unlock        │                      │  - zone          │             │
│  │  - status        │                      │  - adapters      │             │
│  │  - help          │                      │  - install       │             │
│  │  - ping          │                      │  - uninstall     │             │
│  │  - quit          │                      │  - start         │             │
│  │                  │                      │  - stop          │             │
│  │                  │                      │  - session       │             │
│  │                  │                      │  - locks         │             │
│  │                  │                      │  - ping          │             │
│  │                  │                      │  - help          │             │
│  │                  │                      │  - quit          │             │
│  └────────┬─────────┘                      └────────┬─────────┘             │
│           │                                         │                        │
│           │ HTTP/REST                               │ gRPC                   │
│           ▼                                         ▼                        │
├───────────────────────────────────────────────────────────────────────────── │
│                              Local Infrastructure                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐       │
│  │  CLOUD_GATEWAY   │    │  DATA_BROKER     │    │ PARKING_FEE_SVC  │       │
│  │  :8080           │    │  :55556          │    │  :8081           │       │
│  │  (REST API)      │    │  (gRPC)          │    │  (REST API)      │       │
│  └──────────────────┘    └──────────────────┘    └──────────────────┘       │
│                                                                              │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐       │
│  │ LOCKING_SERVICE  │    │  UPDATE_SERVICE  │    │ PARKING_ADAPTOR  │       │
│  │  :50053          │    │  :50051          │    │  :50052          │       │
│  │  (gRPC)          │    │  (gRPC)          │    │  (gRPC)          │       │
│  └──────────────────┘    └──────────────────┘    └──────────────────┘       │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Communication Patterns

| CLI | Target Service | Protocol | Port |
|-----|----------------|----------|------|
| COMPANION_CLI | CLOUD_GATEWAY | HTTP/REST | 8080 |
| PARKING_CLI | DATA_BROKER | gRPC | 55556 |
| PARKING_CLI | PARKING_FEE_SERVICE | HTTP/REST | 8081 |
| PARKING_CLI | UPDATE_SERVICE | gRPC | 50051 |
| PARKING_CLI | PARKING_OPERATOR_ADAPTOR | gRPC | 50052 |
| PARKING_CLI | LOCKING_SERVICE | gRPC | 50053 |

## Components and Interfaces

### COMPANION_CLI Components

```
backend/companion-cli/
├── cmd/
│   └── companion-cli/
│       └── main.go           # Entry point, REPL loop
└── internal/
    ├── config/
    │   └── config.go         # Environment configuration
    ├── client/
    │   └── gateway.go        # CLOUD_GATEWAY HTTP client
    └── repl/
        └── repl.go           # REPL implementation
```

#### GatewayClient Interface

```go
// GatewayClient handles communication with CLOUD_GATEWAY REST API
type GatewayClient interface {
    // SendLockCommand sends a lock command for the configured VIN
    SendLockCommand(ctx context.Context) (*CommandResponse, error)
    
    // SendUnlockCommand sends an unlock command for the configured VIN
    SendUnlockCommand(ctx context.Context) (*CommandResponse, error)
    
    // GetCommandStatus retrieves the status of a command by ID
    GetCommandStatus(ctx context.Context, commandID string) (*CommandStatusResponse, error)
    
    // Ping tests connectivity to the CLOUD_GATEWAY
    Ping(ctx context.Context) error
}

// CommandResponse represents the response from submitting a command
type CommandResponse struct {
    CommandID string `json:"command_id"`
    Status    string `json:"status"`
}

// CommandStatusResponse represents the status of a command
type CommandStatusResponse struct {
    CommandID    string `json:"command_id"`
    Status       string `json:"status"`
    ErrorMessage string `json:"error_message,omitempty"`
}
```

### PARKING_CLI Components

```
backend/parking-cli/
├── cmd/
│   └── parking-cli/
│       └── main.go           # Entry point, REPL loop
└── internal/
    ├── config/
    │   └── config.go         # Environment configuration
    ├── client/
    │   ├── databroker.go     # DATA_BROKER gRPC client
    │   ├── parking.go        # PARKING_FEE_SERVICE HTTP client
    │   ├── update.go         # UPDATE_SERVICE gRPC client
    │   ├── adaptor.go        # PARKING_OPERATOR_ADAPTOR gRPC client
    │   └── locking.go        # LOCKING_SERVICE gRPC client
    └── repl/
        └── repl.go           # REPL implementation
```

#### Client Interfaces

```go
// DataBrokerClient handles communication with DATA_BROKER gRPC service
type DataBrokerClient interface {
    // SetLocation sets the vehicle location signals
    SetLocation(ctx context.Context, lat, lng float64) error
    
    // GetLocation retrieves the current vehicle location
    GetLocation(ctx context.Context) (lat, lng float64, err error)
    
    // Close closes the gRPC connection
    Close() error
}

// ParkingFeeClient handles communication with PARKING_FEE_SERVICE REST API
type ParkingFeeClient interface {
    // GetZone looks up the parking zone for the given coordinates
    GetZone(ctx context.Context, lat, lng float64) (*ZoneInfo, error)
    
    // ListAdapters retrieves available adapters from the registry
    ListAdapters(ctx context.Context) ([]AdapterInfo, error)
}

// UpdateServiceClient handles communication with UPDATE_SERVICE gRPC service
type UpdateServiceClient interface {
    // ListAdapters lists installed adapters
    ListAdapters(ctx context.Context) ([]*update.AdapterInfo, error)
    
    // InstallAdapter requests adapter installation
    InstallAdapter(ctx context.Context, imageRef, checksum string) (*update.InstallAdapterResponse, error)
    
    // UninstallAdapter requests adapter removal
    UninstallAdapter(ctx context.Context, adapterID string) error
    
    // Close closes the gRPC connection
    Close() error
}

// ParkingAdaptorClient handles communication with PARKING_OPERATOR_ADAPTOR gRPC service
type ParkingAdaptorClient interface {
    // StartSession starts a new parking session
    StartSession(ctx context.Context, zoneID string) (*parking.StartSessionResponse, error)
    
    // StopSession stops the current parking session
    StopSession(ctx context.Context) (*parking.StopSessionResponse, error)
    
    // GetSessionStatus retrieves current session status
    GetSessionStatus(ctx context.Context) (*parking.GetSessionStatusResponse, error)
    
    // Close closes the gRPC connection
    Close() error
}

// LockingServiceClient handles communication with LOCKING_SERVICE gRPC service
type LockingServiceClient interface {
    // GetLockState retrieves the lock state for a door
    GetLockState(ctx context.Context, door locking.Door) (*locking.GetLockStateResponse, error)
    
    // GetAllLockStates retrieves lock states for all doors
    GetAllLockStates(ctx context.Context) ([]*locking.GetLockStateResponse, error)
    
    // Close closes the gRPC connection
    Close() error
}
```

### REPL Interface

```go
// REPL provides the interactive command interface
type REPL struct {
    prompt   string
    commands map[string]Command
    history  []string
    running  bool
}

// Command represents a CLI command
type Command struct {
    Name        string
    Description string
    Usage       string
    Handler     func(args []string) error
}

// Run starts the REPL loop
func (r *REPL) Run() error

// RegisterCommand adds a command to the REPL
func (r *REPL) RegisterCommand(cmd Command)
```

### CLI Flags and Non-Interactive Mode

Both CLIs support non-interactive execution for scripting and automated testing:

```go
// CLIFlags holds command-line flags for non-interactive mode
type CLIFlags struct {
    Command string // -c, --command: Execute single command and exit
    JSON    bool   // --json: Output in JSON format
    Quiet   bool   // -q, --quiet: Suppress informational messages
}
```

#### Usage Examples

```bash
# Direct command execution (positional arguments)
companion-cli lock
companion-cli status abc123
parking-cli location 37.7749 -122.4194
parking-cli adapters

# Command flag execution
companion-cli -c "lock"
parking-cli --command "start zone-123"

# JSON output for machine parsing
companion-cli --json lock
parking-cli --json session

# Quiet mode (result only)
companion-cli -q status abc123

# Piped input
echo "lock" | companion-cli
echo -e "location 37.7749 -122.4194\nzone" | parking-cli

# Scripted end-to-end test example
#!/bin/bash
set -e
parking-cli -q location 37.7749 -122.4194
ZONE=$(parking-cli --json zone | jq -r '.zone_id')
parking-cli start "$ZONE"
companion-cli lock
sleep 5
companion-cli unlock
parking-cli stop
```

#### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Command executed successfully |
| 1 | Command execution failed (service error) |
| 2 | Invalid arguments or usage error |
| 3 | Connection error (service unavailable) |

#### JSON Output Format

```go
// JSONOutput wraps command results for JSON output mode
type JSONOutput struct {
    Success bool        `json:"success"`
    Command string      `json:"command"`
    Result  interface{} `json:"result,omitempty"`
    Error   *JSONError  `json:"error,omitempty"`
}

// JSONError represents an error in JSON output
type JSONError struct {
    Code    string `json:"code"`
    Message string `json:"message"`
    Details string `json:"details,omitempty"`
}
```

## Data Models

### Configuration Models

```go
// CompanionConfig holds COMPANION_CLI configuration
type CompanionConfig struct {
    CloudGatewayURL string        // CLOUD_GATEWAY_URL env var
    VIN             string        // VIN env var
    Timeout         time.Duration // Request timeout
}

// ParkingConfig holds PARKING_CLI configuration
type ParkingConfig struct {
    DataBrokerAddr      string        // DATA_BROKER_ADDR env var
    ParkingFeeServiceURL string       // PARKING_FEE_SERVICE_URL env var
    UpdateServiceAddr   string        // UPDATE_SERVICE_ADDR env var
    ParkingAdaptorAddr  string        // PARKING_ADAPTOR_ADDR env var
    LockingServiceAddr  string        // LOCKING_SERVICE_ADDR env var
    Timeout             time.Duration // Request timeout
}
```

### Response Models

```go
// ZoneInfo represents parking zone information from PARKING_FEE_SERVICE
type ZoneInfo struct {
    ZoneID          string  `json:"zone_id"`
    OperatorName    string  `json:"operator_name"`
    HourlyRate      float64 `json:"hourly_rate"`
    Currency        string  `json:"currency"`
    AdapterImageRef string  `json:"adapter_image_ref"`
    AdapterChecksum string  `json:"adapter_checksum"`
}

// AdapterInfo represents adapter information (mirrors proto definition)
type AdapterInfo struct {
    AdapterID    string
    ImageRef     string
    Version      string
    State        string
    ErrorMessage string
}

// SessionInfo represents parking session information
type SessionInfo struct {
    SessionID       string
    State           string
    ZoneID          string
    DurationSeconds int64
    CurrentCost     float64
    StartTime       time.Time
}

// LockStateInfo represents door lock state
type LockStateInfo struct {
    Door     string
    IsLocked bool
    IsOpen   bool
}
```

### Ping Result Model

```go
// PingResult represents connectivity test result for a service
type PingResult struct {
    Service   string
    Address   string
    Connected bool
    Latency   time.Duration
    Error     string
}
```



## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

Based on the prework analysis, the following properties consolidate related acceptance criteria into testable universal statements:

### Property 1: Response Field Propagation

*For any* successful service response containing structured data (command responses, zone info, adapter info, session status, lock states), the CLI output SHALL contain all fields specified in the requirements for that response type.

**Validates: Requirements 1.2, 2.2, 3.2, 5.2, 6.2, 6.4, 7.2, 7.4, 7.6, 11.2**

### Property 2: Error Message Propagation

*For any* error response from a service (HTTP error, gRPC error, connection failure), the CLI output SHALL contain the error message and relevant context information (target address for gRPC, URL and status code for HTTP).

**Validates: Requirements 1.4, 2.4, 3.4, 4.3, 5.4, 6.6, 7.7, 10.1, 10.2, 11.3**

### Property 3: Configuration with Defaults

*For any* configuration field, loading configuration SHALL return the environment variable value if set, otherwise the documented default value.

**Validates: Requirements 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7**

### Property 4: Command Argument Parsing

*For any* command that accepts arguments (location, status, install, uninstall, start), the parsed argument values SHALL be correctly passed to the corresponding service call.

**Validates: Requirements 3.1, 4.1, 6.3, 6.5, 7.1**

### Property 5: Unknown Command Handling

*For any* input string that does not match a registered command name, the REPL SHALL display the standard unknown command message.

**Validates: Requirements 8.6**

### Property 6: Help Command Completeness

*For any* registered command in the REPL, the help output SHALL include that command's name and description.

**Validates: Requirements 8.4**

### Property 7: Timeout Message Format

*For any* operation that times out, the CLI output SHALL display a message containing the timeout duration in seconds.

**Validates: Requirements 10.3**

### Property 8: Ping Command Coverage

*For any* configured service endpoint, the ping command output SHALL include a connectivity status entry for that service.

**Validates: Requirements 10.5**

### Property 9: Non-Interactive Exit Codes

*For any* command executed in non-interactive mode, the CLI SHALL exit with code 0 on success and a non-zero code on failure, with the exit code correctly reflecting the error category.

**Validates: Requirements 12.3, 12.4**

### Property 10: JSON Output Completeness

*For any* command executed with the `--json` flag, the output SHALL be valid JSON containing the success status, command name, and either result data or error information.

**Validates: Requirements 12.7**

## Error Handling

### Error Categories

| Category | Source | Handling |
|----------|--------|----------|
| Connection Error | gRPC dial failure | Display address and error, suggest checking service |
| HTTP Error | Non-2xx response | Display URL, status code, and response body |
| Timeout | Context deadline | Display "Connection timed out after Xs" |
| Parse Error | Invalid arguments | Display usage hint for the command |
| Unknown Command | REPL input | Display "Unknown command. Type 'help' for available commands." |

### Error Message Format

```
Error: <brief description>
  Service: <service name>
  Address: <host:port or URL>
  Details: <error message from service>
```

### gRPC Error Mapping

| gRPC Status | User Message |
|-------------|--------------|
| UNAVAILABLE | "Service unavailable at <address>" |
| DEADLINE_EXCEEDED | "Connection timed out after <seconds>s" |
| INVALID_ARGUMENT | "Invalid request: <details>" |
| NOT_FOUND | "Resource not found: <details>" |
| INTERNAL | "Internal service error: <details>" |

### HTTP Error Mapping

| HTTP Status | User Message |
|-------------|--------------|
| 400 | "Bad request: <body>" |
| 404 | "Not found: <resource>" |
| 500 | "Server error: <body>" |
| Connection refused | "Cannot connect to <url>" |

## Testing Strategy

### Dual Testing Approach

The CLI simulators will use both unit tests and property-based tests:

- **Unit tests**: Verify specific examples, edge cases, command parsing, and output formatting
- **Property tests**: Verify universal properties across generated inputs using `gopter`

### Property-Based Testing Configuration

- **Library**: `github.com/leanovate/gopter` (already in project dependencies)
- **Iterations**: Minimum 100 iterations per property test
- **Tag format**: `// Feature: cli-simulators, Property N: <property_text>`

### Test Structure

```
backend/companion-cli/
└── tests/
    ├── unit/
    │   ├── config_test.go      # Configuration loading tests
    │   ├── client_test.go      # HTTP client tests with mock server
    │   └── repl_test.go        # REPL command parsing tests
    └── property/
        ├── setup_test.go       # Test helpers and generators
        ├── config_test.go      # Property 3: Config with defaults
        ├── output_test.go      # Property 1, 2: Response/error propagation
        └── repl_test.go        # Property 5, 6: Unknown cmd, help completeness

backend/parking-cli/
└── tests/
    ├── unit/
    │   ├── config_test.go      # Configuration loading tests
    │   ├── client_test.go      # gRPC client tests with mock servers
    │   └── repl_test.go        # REPL command parsing tests
    └── property/
        ├── setup_test.go       # Test helpers and generators
        ├── config_test.go      # Property 3: Config with defaults
        ├── output_test.go      # Property 1, 2, 7: Response/error/timeout
        ├── repl_test.go        # Property 5, 6: Unknown cmd, help completeness
        └── ping_test.go        # Property 8: Ping coverage
```

### Test Generators

```go
// CommandIDGen generates random command IDs (UUIDs)
func CommandIDGen() gopter.Gen

// ZoneInfoGen generates random ZoneInfo structs
func ZoneInfoGen() gopter.Gen

// AdapterInfoGen generates random AdapterInfo structs
func AdapterInfoGen() gopter.Gen

// SessionInfoGen generates random SessionInfo structs
func SessionInfoGen() gopter.Gen

// ErrorMessageGen generates random error messages
func ErrorMessageGen() gopter.Gen

// ConfigEnvGen generates random environment variable configurations
func ConfigEnvGen() gopter.Gen
```

### Unit Test Focus Areas

1. **Command parsing**: Verify argument extraction for commands with parameters
2. **Output formatting**: Verify specific message formats match requirements
3. **Edge cases**: Empty responses, zero values, special characters
4. **Error conditions**: Network errors, invalid responses, timeouts

### Integration Test Approach

Integration tests should be run manually against the local infrastructure:

```bash
# Start infrastructure
make infra-up

# Run CLI and test commands manually
./bin/companion-cli
> lock
> status <command_id>
> quit

./bin/parking-cli
> location 37.7749 -122.4194
> zone
> quit
```
