package config

import "github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"

// LoadConfig loads configuration from the specified file path.
// If the file does not exist, it returns the default configuration.
// If the file contains invalid JSON, it returns an error.
func LoadConfig(path string) (*model.Config, error) {
	return &model.Config{}, nil
}

// DefaultConfig returns the built-in default configuration with Munich demo data.
func DefaultConfig() *model.Config {
	return &model.Config{}
}
