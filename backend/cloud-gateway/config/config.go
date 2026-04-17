// Package config handles loading configuration for the CLOUD_GATEWAY.
package config

import (
	"encoding/json"
	"fmt"
	"os"
)

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
func LoadConfig(path string) (*Config, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read config file %q: %w", path, err)
	}
	var cfg Config
	if err := json.Unmarshal(data, &cfg); err != nil {
		return nil, fmt.Errorf("parse config file %q: %w", path, err)
	}
	return &cfg, nil
}

// GetVINForToken returns the VIN associated with a bearer token.
// Returns ("", false) if the token is not in the configuration.
func (c *Config) GetVINForToken(token string) (string, bool) {
	for _, tm := range c.Tokens {
		if tm.Token == token {
			return tm.VIN, true
		}
	}
	return "", false
}
