// Package config handles loading and parsing of the cloud-gateway configuration.
package config

import (
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// LoadConfig reads configuration from the given file path.
// If the file does not exist, it returns default configuration.
// If the file contains invalid JSON, it returns an error.
func LoadConfig(path string) (*model.Config, error) {
	return nil, nil
}

// DefaultConfig returns the built-in default configuration.
func DefaultConfig() *model.Config {
	return nil
}
