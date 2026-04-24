package config

// TokenMapping maps a bearer token to a VIN.
type TokenMapping struct {
	Token string `json:"token"`
	VIN   string `json:"vin"`
}

// Config holds the service configuration.
type Config struct {
	Port                  int            `json:"port"`
	NatsURL               string         `json:"nats_url"`
	CommandTimeoutSeconds int            `json:"command_timeout_seconds"`
	Tokens                []TokenMapping `json:"tokens"`
}

// LoadConfig reads and parses configuration from the given file path.
func LoadConfig(path string) (*Config, error) {
	return &Config{}, nil
}

// GetVINForToken returns the VIN mapped to the given token.
func (c *Config) GetVINForToken(token string) (string, bool) {
	return "", false
}
