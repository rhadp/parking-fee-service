package model

// Config holds service configuration.
type Config struct {
	Port                  int            `json:"port"`
	NatsURL               string         `json:"nats_url"`
	CommandTimeoutSeconds int            `json:"command_timeout_seconds"`
	Tokens                []TokenMapping `json:"tokens"`
}

// TokenMapping associates a bearer token with a VIN.
type TokenMapping struct {
	Token string `json:"token"`
	VIN   string `json:"vin"`
}

// Command represents a lock/unlock command from a COMPANION_APP.
type Command struct {
	CommandID string   `json:"command_id"`
	Type      string   `json:"type"`  // "lock" | "unlock"
	Doors     []string `json:"doors"`
}

// CommandResponse represents the outcome of a command.
type CommandResponse struct {
	CommandID string `json:"command_id"`
	Status    string `json:"status"` // "success" | "failed" | "timeout"
	Reason    string `json:"reason,omitempty"`
}
