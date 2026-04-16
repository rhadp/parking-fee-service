// Package config handles loading and providing defaults for the
// parking-fee-service configuration.
package config

import (
	"parking-fee-service/backend/parking-fee-service/model"
)

// LoadConfig reads configuration from the file at path. If the file does not
// exist, it returns DefaultConfig(). If the file contains invalid JSON, it
// returns an error.
func LoadConfig(path string) (*model.Config, error) {
	panic("not implemented")
}

// DefaultConfig returns the built-in Munich demo configuration used when no
// config file is present.
func DefaultConfig() *model.Config {
	panic("not implemented")
}
