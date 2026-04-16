package config

import (
	"errors"
	"parking-fee-service/backend/cloud-gateway/model"
)

var errNotImplemented = errors.New("not implemented")

// LoadConfig reads a JSON config file from the given path.
// Returns an error if the file does not exist or contains invalid JSON.
func LoadConfig(path string) (*model.Config, error) {
	return nil, errNotImplemented
}

// GetVINForToken returns the VIN associated with the given bearer token.
// Returns ("", false) if the token is not found.
func GetVINForToken(cfg *model.Config, token string) (string, bool) {
	return "", false
}
