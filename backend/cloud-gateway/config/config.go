// Package config handles loading configuration for the CLOUD_GATEWAY.
package config

import "fmt"

// Config holds the CLOUD_GATEWAY service configuration.
type Config struct {
	Port                  int            `json:"port"`
	NatsURL               string         `json:"nats_url"`
	CommandTimeoutSeconds int            `json:"command_timeout_seconds"`
	Tokens                []TokenMapping `json:"tokens"`
}

// TokenMapping maps a bearer token to a vehicle VIN.
type TokenMapping struct {
	Token string `json:"token"`
	VIN   string `json:"vin"`
}

// LoadConfig reads a JSON configuration file at path and returns the parsed Config.
// Returns a non-nil error if the file is missing or contains invalid JSON.
// STUB: always returns an error (not implemented).
func LoadConfig(path string) (*Config, error) {
	return nil, fmt.Errorf("not implemented: LoadConfig(%q)", path)
}

// GetVINForToken returns the VIN associated with a bearer token.
// Returns ("", false) if the token is not in the configuration.
// STUB: always returns ("", false).
func (c *Config) GetVINForToken(token string) (string, bool) {
	return "", false
}
