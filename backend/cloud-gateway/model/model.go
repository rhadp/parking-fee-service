// Package model defines core data types for the cloud-gateway service.
package model

import (
	"encoding/json"
	"fmt"
	"time"
)

// Command represents a lock/unlock command sent by a COMPANION_APP.
type Command struct {
	CommandID string   `json:"command_id"`
	Type      string   `json:"type"`
	Doors     []string `json:"doors"`
}

// CommandStatus represents the stored status of a command.
type CommandStatus struct {
	CommandID string    `json:"command_id"`
	Status    string    `json:"status"`
	Reason    string    `json:"reason,omitempty"`
	VIN       string    `json:"-"`
	CreatedAt time.Time `json:"-"`
}

// CommandResponse represents a response received from a vehicle via NATS.
type CommandResponse struct {
	CommandID string `json:"command_id"`
	Status    string `json:"status"`
	Reason    string `json:"reason,omitempty"`
}

// TokenMapping maps a bearer token to a VIN.
type TokenMapping struct {
	Token string `json:"token"`
	VIN   string `json:"vin"`
}

// Config holds the service configuration.
type Config struct {
	Port           int            `json:"port"`
	NatsURL        string         `json:"nats_url"`
	CommandTimeout int            `json:"command_timeout_seconds"`
	Tokens         []TokenMapping `json:"tokens"`
}

// ParseCommand parses and validates a JSON command payload.
// Returns an error if the payload is invalid, missing required fields,
// or has an invalid command type.
func ParseCommand(data []byte) (*Command, error) {
	var cmd Command
	if err := json.Unmarshal(data, &cmd); err != nil {
		return nil, fmt.Errorf("invalid JSON: %w", err)
	}
	if cmd.CommandID == "" {
		return nil, fmt.Errorf("missing required field: command_id")
	}
	if cmd.Type == "" {
		return nil, fmt.Errorf("missing required field: type")
	}
	if cmd.Type != "lock" && cmd.Type != "unlock" {
		return nil, fmt.Errorf("invalid command type %q: must be \"lock\" or \"unlock\"", cmd.Type)
	}
	if len(cmd.Doors) == 0 {
		return nil, fmt.Errorf("missing required field: doors (must be non-empty)")
	}
	return &cmd, nil
}

// ParseResponse parses a JSON command response payload.
func ParseResponse(data []byte) (*CommandResponse, error) {
	var resp CommandResponse
	if err := json.Unmarshal(data, &resp); err != nil {
		return nil, fmt.Errorf("invalid JSON: %w", err)
	}
	return &resp, nil
}
