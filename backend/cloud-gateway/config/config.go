package config

import (
	"encoding/json"
	"fmt"
	"os"

	"parking-fee-service/backend/cloud-gateway/model"
)

// LoadConfig reads and parses the JSON configuration file at the given path.
func LoadConfig(path string) (*model.Config, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read config file: %w", err)
	}

	var cfg model.Config
	if err := json.Unmarshal(data, &cfg); err != nil {
		return nil, fmt.Errorf("failed to parse config file: %w", err)
	}

	return &cfg, nil
}

// GetVINForToken returns the VIN associated with the given bearer token.
// Returns ("", false) if the token is not found.
func GetVINForToken(cfg *model.Config, token string) (string, bool) {
	for _, tm := range cfg.Tokens {
		if tm.Token == token {
			return tm.VIN, true
		}
	}
	return "", false
}
