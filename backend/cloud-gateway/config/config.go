package config

// Config holds the service configuration loaded from a JSON file.
type Config struct {
	Port                  int            `json:"port"`
	NatsURL               string         `json:"nats_url"`
	CommandTimeoutSeconds int            `json:"command_timeout_seconds"`
	Tokens                []TokenMapping `json:"tokens"`
}

// TokenMapping maps a bearer token to a VIN.
type TokenMapping struct {
	Token string `json:"token"`
	VIN   string `json:"vin"`
}

// LoadConfig reads and parses the JSON configuration file at the given path.
func LoadConfig(path string) (*Config, error) {
	return nil, nil // stub
}

// GetVINForToken returns the VIN associated with the given bearer token.
func (c *Config) GetVINForToken(token string) (string, bool) {
	return "", false // stub
}
