package config

import (
	"errors"
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
	return nil, errors.New("not implemented")
}

// GetVINForToken looks up the VIN for a given bearer token.
func (c *Config) GetVINForToken(token string) (string, bool) {
	return "", false
}
