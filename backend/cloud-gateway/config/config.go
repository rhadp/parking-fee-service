package config

import (
	"encoding/json"
	"fmt"
	"os"
)

// TokenMapping maps a bearer token to a VIN.
type TokenMapping struct {
	Token string `json:"token"`
	VIN   string `json:"vin"`
}

// Config holds the service configuration.
type Config struct {
	Port                  int            `json:"port"`
	NatsURL               string         `json:"nats_url"`
	CommandTimeoutSeconds int            `json:"command_timeout_seconds"`
	Tokens                []TokenMapping `json:"tokens"`
}

// LoadConfig reads and parses configuration from the given file path.
func LoadConfig(path string) (*Config, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("reading config file: %w", err)
	}

	var cfg Config
	if err := json.Unmarshal(data, &cfg); err != nil {
		return nil, fmt.Errorf("parsing config JSON: %w", err)
	}

	return &cfg, nil
}

// GetVINForToken returns the VIN mapped to the given token.
func (c *Config) GetVINForToken(token string) (string, bool) {
	for _, tm := range c.Tokens {
		if tm.Token == token {
			return tm.VIN, true
		}
	}
	return "", false
}
