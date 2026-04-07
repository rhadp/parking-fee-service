package config

import (
	"parking-fee-service/backend/cloud-gateway/model"
)

// LoadConfig reads and parses the JSON configuration file at the given path.
func LoadConfig(path string) (*model.Config, error) {
	panic("not implemented")
}

// GetVINForToken returns the VIN associated with the given bearer token.
// Returns ("", false) if the token is not found.
func GetVINForToken(cfg *model.Config, token string) (string, bool) {
	panic("not implemented")
}
