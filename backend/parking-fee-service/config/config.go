package config

import "github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"

// LoadConfig reads configuration from the specified JSON file path.
// If the file does not exist, it returns DefaultConfig and logs a warning.
// If the file contains invalid JSON, it returns an error.
func LoadConfig(_ string) (*model.Config, error) {
	return nil, nil
}

// DefaultConfig returns the built-in Munich demo configuration.
func DefaultConfig() *model.Config {
	return nil
}
