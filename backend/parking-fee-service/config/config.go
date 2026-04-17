// Package config handles loading configuration for the PARKING_FEE_SERVICE.
package config

import (
	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/model"
)

// LoadConfig reads a JSON configuration file at path and returns the parsed
// Config. If the file does not exist, it returns DefaultConfig() and a nil
// error (with a logged warning). If the file exists but contains invalid JSON,
// it returns a non-nil error.
//
// This is a stub — full implementation is in task group 2.
func LoadConfig(path string) (*model.Config, error) {
	// Stub: always returns empty config — tests will fail.
	return &model.Config{}, nil
}

// DefaultConfig returns the built-in Munich demo configuration used when no
// config file is present.
//
// This is a stub — full implementation is in task group 2.
func DefaultConfig() *model.Config {
	return &model.Config{}
}
