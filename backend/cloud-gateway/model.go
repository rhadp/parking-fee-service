package main

// CommandRequest represents a lock/unlock command from the COMPANION_APP.
type CommandRequest struct {
	CommandID string   `json:"command_id"`
	Type      string   `json:"type"`
	Doors     []string `json:"doors"`
}

// CommandStatus represents the current status of a command.
type CommandStatus struct {
	CommandID string `json:"command_id"`
	Status    string `json:"status"`
	Reason    string `json:"reason,omitempty"`
}

// NATSCommand represents a command message published to NATS.
type NATSCommand struct {
	CommandID string   `json:"command_id"`
	Action    string   `json:"action"`
	Doors     []string `json:"doors"`
	Source    string   `json:"source"`
}

// NATSCommandResponse represents a command response received from NATS.
type NATSCommandResponse struct {
	CommandID string `json:"command_id"`
	Status    string `json:"status"`
	Reason    string `json:"reason,omitempty"`
}

// TelemetryData represents vehicle telemetry received from NATS.
type TelemetryData struct {
	VIN           string  `json:"vin"`
	DoorLocked    bool    `json:"door_locked"`
	Latitude      float64 `json:"latitude"`
	Longitude     float64 `json:"longitude"`
	ParkingActive bool    `json:"parking_active"`
	Timestamp     int64   `json:"timestamp"`
}

// ErrorResponse represents a JSON error response.
type ErrorResponse struct {
	Error string `json:"error"`
}
