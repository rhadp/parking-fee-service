package config

import (
	"encoding/json"
	"os"

	"parking-fee-service/backend/cloud-gateway/model"
)

// LoadConfig reads a JSON config file from the given path.
// Returns an error if the file does not exist or contains invalid JSON.
func LoadConfig(path string) (*model.Config, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var cfg model.Config
	if err := json.Unmarshal(data, &cfg); err != nil {
		return nil, err
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
