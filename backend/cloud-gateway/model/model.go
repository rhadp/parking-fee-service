// Package model defines core data types for the CLOUD_GATEWAY.
package model

// Command represents a lock/unlock command submitted by a COMPANION_APP.
type Command struct {
	CommandID string   `json:"command_id"`
	Type      string   `json:"type"`  // "lock" | "unlock"
	Doors     []string `json:"doors"`
}

// CommandResponse represents the result of a command execution.
type CommandResponse struct {
	CommandID string `json:"command_id"`
	Status    string `json:"status"`           // "success" | "failed" | "timeout"
	Reason    string `json:"reason,omitempty"`
}
