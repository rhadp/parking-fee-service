package model

// Command represents a lock/unlock command sent by a COMPANION_APP.
type Command struct {
	CommandID string   `json:"command_id"`
	Type      string   `json:"type"`
	Doors     []string `json:"doors"`
}

// CommandResponse represents the result of a command execution.
type CommandResponse struct {
	CommandID string `json:"command_id"`
	Status    string `json:"status"`
	Reason    string `json:"reason,omitempty"`
}
