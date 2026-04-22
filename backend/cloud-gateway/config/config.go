package config

import (
	"encoding/json"
	"fmt"
	"os"
)

// Config holds the service configuration loaded from a JSON file.
type Config struct {
	Port                  int            `json:"port"`
	NatsURL               string         `json:"nats_url"`
	CommandTimeoutSeconds int            `json:"command_timeout_seconds"`
	Tokens                []TokenMapping `json:"tokens"`
}

// TokenMapping maps a bearer token to a VIN.
type TokenMapping struct {
	Token string `json:"token"`
	VIN   string `json:"vin"`
}

// LoadConfig reads and parses the JSON configuration file at the given path.
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

// GetVINForToken returns the VIN associated with the given bearer token.
func (c *Config) GetVINForToken(token string) (string, bool) {
	for _, tm := range c.Tokens {
		if tm.Token == token {
			return tm.VIN, true
		}
	}
	return "", false
}
