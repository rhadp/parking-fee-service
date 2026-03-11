package main

import "sync"

// CommandStore provides thread-safe storage for command statuses.
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
	// Stub - to be implemented
}

// UpdateCommandStatus updates the status of an existing command.
// Respects terminal states: does not update if already success/failed.
func (cs *CommandStore) UpdateCommandStatus(cmdID, status, reason string) {
	// Stub - to be implemented
}

// GetCommandStatus retrieves the status of a command by ID.
func (cs *CommandStore) GetCommandStatus(cmdID string) (*CommandStatus, bool) {
	// Stub - to be implemented
	return nil, false
}

// TelemetryStore provides thread-safe storage for latest telemetry per VIN.
type TelemetryStore struct {
	mu        sync.RWMutex
	telemetry map[string]*TelemetryData
}

// NewTelemetryStore creates a new empty TelemetryStore.
func NewTelemetryStore() *TelemetryStore {
	return &TelemetryStore{telemetry: make(map[string]*TelemetryData)}
}

// StoreTelemetry stores telemetry data for the given VIN.
func (ts *TelemetryStore) StoreTelemetry(vin string, data TelemetryData) {
	// Stub - to be implemented
}

// GetTelemetry retrieves the latest telemetry for the given VIN.
func (ts *TelemetryStore) GetTelemetry(vin string) (*TelemetryData, bool) {
	// Stub - to be implemented
	return nil, false
}
