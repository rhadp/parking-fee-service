// Package config handles environment-based configuration for the parking fee service.
package config

// Config holds the service configuration.
type Config struct {
	Port               string
	OperatorsConfigPath string
	FuzzinessMeters    float64
	AuthTokens         []string
}

// LoadConfig reads configuration from environment variables and applies
// defaults. Returns zero-value config until implemented.
func LoadConfig() Config {
	// TODO: implement env var loading (task group 4)
	return Config{}
}
