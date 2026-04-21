// Package config handles configuration loading for the parking-fee-service.
package config

import (
	"fmt"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

// LoadConfig loads configuration from the given file path.
// If the file does not exist, DefaultConfig is returned with a warning.
// If the file contains invalid JSON, an error is returned.
func LoadConfig(path string) (*model.Config, error) {
	// stub: not implemented
	return nil, fmt.Errorf("LoadConfig not implemented")
}

// DefaultConfig returns the built-in Munich demo configuration with 2 zones and 2 operators.
func DefaultConfig() *model.Config {
	// stub: not implemented
	return &model.Config{}
}
