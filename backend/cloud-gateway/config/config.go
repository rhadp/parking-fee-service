// Package config handles loading and parsing of the cloud-gateway configuration.
package config

import (
	"encoding/json"
	"errors"
	"fmt"
	"log/slog"
	"os"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// LoadConfig reads configuration from the given file path.
// If the file does not exist, it returns default configuration and logs a warning.
// If the file contains invalid JSON, it returns an error.
func LoadConfig(path string) (*model.Config, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			slog.Warn("config file not found, using defaults", "path", path)
			return DefaultConfig(), nil
		}
		return nil, fmt.Errorf("reading config file %q: %w", path, err)
	}

	cfg := DefaultConfig()
	if err := json.Unmarshal(data, cfg); err != nil {
		return nil, fmt.Errorf("parsing config file %q: %w", path, err)
	}

	// Re-apply defaults for zero-value fields that were not set in the file.
	if cfg.Port == 0 {
		cfg.Port = 8081
	}
	if cfg.NatsURL == "" {
		cfg.NatsURL = "nats://localhost:4222"
	}
	if cfg.CommandTimeout == 0 {
		cfg.CommandTimeout = 30
	}

	return cfg, nil
}

// DefaultConfig returns the built-in default configuration.
func DefaultConfig() *model.Config {
	return &model.Config{
		Port:           8081,
		NatsURL:        "nats://localhost:4222",
		CommandTimeout: 30,
		Tokens:         []model.TokenMapping{},
	}
}
