// Package config handles loading and defaulting service configuration.
package config

import (
	"parking-fee-service/backend/parking-fee-service/model"
)

// LoadConfig reads configuration from the specified file path.
// If the file does not exist, it returns DefaultConfig and logs a warning.
// If the file contains invalid JSON, it returns an error.
func LoadConfig(path string) (*model.Config, error) {
	return nil, nil
}

// DefaultConfig returns built-in Munich demo configuration.
func DefaultConfig() *model.Config {
	return nil
}
