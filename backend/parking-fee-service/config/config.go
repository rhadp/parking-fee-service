// Package config handles loading and defaulting of service configuration.
package config

import (
	"fmt"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

// LoadConfig reads and parses the configuration file at the given path.
// If the file does not exist, it returns DefaultConfig and logs a warning.
// If the file contains invalid JSON, it returns an error.
func LoadConfig(path string) (*model.Config, error) {
	return nil, fmt.Errorf("not implemented")
}

// DefaultConfig returns the built-in Munich demo configuration.
func DefaultConfig() *model.Config {
	return nil
}
