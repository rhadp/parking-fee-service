package main

import "sync"

// CommandStore provides thread-safe in-memory storage for command statuses.
type CommandStore struct {
	mu       sync.RWMutex
	commands map[string]*CommandStatus
}

// NewCommandStore creates a new empty CommandStore.
func NewCommandStore() *CommandStore {
	return &CommandStore{commands: make(map[string]*CommandStatus)}
}

// StoreCommand stores a new command with the given status.
func (cs *CommandStore) StoreCommand(cmdID, status string) {
	// Stub: not yet implemented
}

// UpdateCommandStatus updates the status of an existing command.
// Respects terminal states: no update if already "success" or "failed".
func (cs *CommandStore) UpdateCommandStatus(cmdID, status, reason string) {
	// Stub: not yet implemented
}

// GetCommandStatus returns the status of a command by ID.
func (cs *CommandStore) GetCommandStatus(cmdID string) (*CommandStatus, bool) {
	// Stub: not yet implemented
	return nil, false
}

// TelemetryStore provides thread-safe in-memory storage for latest telemetry per VIN.
type TelemetryStore struct {
	mu        sync.RWMutex
	telemetry map[string]*TelemetryData
}

// NewTelemetryStore creates a new empty TelemetryStore.
func NewTelemetryStore() *TelemetryStore {
	return &TelemetryStore{telemetry: make(map[string]*TelemetryData)}
}

// StoreTelemetry stores the latest telemetry data for a VIN.
func (ts *TelemetryStore) StoreTelemetry(vin string, data TelemetryData) {
	// Stub: not yet implemented
}

// GetTelemetry returns the latest telemetry for a VIN.
func (ts *TelemetryStore) GetTelemetry(vin string) (*TelemetryData, bool) {
	// Stub: not yet implemented
	return nil, false
}
