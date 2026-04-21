package config

import (
	"encoding/json"
	"fmt"
	"os"
)

type TokenMapping struct {
	Token string `json:"token"`
	VIN   string `json:"vin"`
}

type Config struct {
	Port                  int            `json:"port"`
	NatsURL               string         `json:"nats_url"`
	CommandTimeoutSeconds int            `json:"command_timeout_seconds"`
	Tokens                []TokenMapping `json:"tokens"`
}

// LoadConfig reads and parses the JSON config file at path.
func LoadConfig(path string) (*Config, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("config: read file %q: %w", path, err)
	}
	var cfg Config
	if err := json.Unmarshal(data, &cfg); err != nil {
		return nil, fmt.Errorf("config: parse %q: %w", path, err)
	}
	return &cfg, nil
}

// GetVINForToken looks up the VIN for a given bearer token.
func (c *Config) GetVINForToken(token string) (string, bool) {
	for _, tm := range c.Tokens {
		if tm.Token == token {
			return tm.VIN, true
		}
	}
	return "", false
}
