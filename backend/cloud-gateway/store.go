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
	cs.mu.Lock()
	defer cs.mu.Unlock()
	cs.commands[cmdID] = &CommandStatus{
		CommandID: cmdID,
		Status:    status,
	}
}

// UpdateCommandStatus updates the status of an existing command.
// Respects terminal states: no update if already "success" or "failed".
func (cs *CommandStore) UpdateCommandStatus(cmdID, status, reason string) {
	cs.mu.Lock()
	defer cs.mu.Unlock()
	cmd, ok := cs.commands[cmdID]
	if !ok {
		return
	}
	// Do not update terminal states
	if cmd.Status == "success" || cmd.Status == "failed" {
		return
	}
	cmd.Status = status
	cmd.Reason = reason
}

// GetCommandStatus returns the status of a command by ID.
func (cs *CommandStore) GetCommandStatus(cmdID string) (*CommandStatus, bool) {
	cs.mu.RLock()
	defer cs.mu.RUnlock()
	cmd, ok := cs.commands[cmdID]
	if !ok {
		return nil, false
	}
	// Return a copy to avoid data races
	copy := *cmd
	return &copy, true
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
	ts.mu.Lock()
	defer ts.mu.Unlock()
	ts.telemetry[vin] = &data
}

// GetTelemetry returns the latest telemetry for a VIN.
func (ts *TelemetryStore) GetTelemetry(vin string) (*TelemetryData, bool) {
	ts.mu.RLock()
	defer ts.mu.RUnlock()
	data, ok := ts.telemetry[vin]
	if !ok {
		return nil, false
	}
	copy := *data
	return &copy, true
}
