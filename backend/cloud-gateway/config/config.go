// Package config handles loading and querying service configuration.
package config

import (
	"encoding/json"
	"fmt"
	"os"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// LoadConfig reads a JSON configuration file from the given path and returns
// the parsed Config. Returns an error if the file cannot be read or contains
// invalid JSON.
func LoadConfig(path string) (*model.Config, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("reading config file: %w", err)
	}

	var cfg model.Config
	if err := json.Unmarshal(data, &cfg); err != nil {
		return nil, fmt.Errorf("parsing config JSON: %w", err)
	}

	return &cfg, nil
}

// GetVINForToken looks up the VIN associated with the given bearer token.
// Returns the VIN and true if found, or empty string and false if not.
func GetVINForToken(cfg *model.Config, token string) (string, bool) {
	for _, tm := range cfg.Tokens {
		if tm.Token == token {
			return tm.VIN, true
		}
	}
	return "", false
}
