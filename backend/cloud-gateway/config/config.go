// Package config handles loading and querying service configuration.
package config

import (
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// LoadConfig reads a JSON configuration file from the given path and returns
// the parsed Config. Returns an error if the file cannot be read or contains
// invalid JSON.
func LoadConfig(path string) (*model.Config, error) {
	// TODO: implement
	return nil, nil
}

// GetVINForToken looks up the VIN associated with the given bearer token.
// Returns the VIN and true if found, or empty string and false if not.
func GetVINForToken(cfg *model.Config, token string) (string, bool) {
	// TODO: implement
	return "", false
}
