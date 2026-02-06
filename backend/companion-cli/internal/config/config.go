// Package config provides configuration management for COMPANION_CLI.
package config

import (
	"os"
	"time"
)

// Default configuration values.
const (
	DefaultCloudGatewayURL = "http://localhost:8080"
	DefaultVIN             = "DEMO_VIN_001"
	DefaultTimeout         = 10 * time.Second
)

// Config holds COMPANION_CLI configuration.
type Config struct {
	CloudGatewayURL string
	VIN             string
	Timeout         time.Duration
}

// Load loads configuration from environment variables with defaults.
func Load() *Config {
	return &Config{
		CloudGatewayURL: getEnvOrDefault("CLOUD_GATEWAY_URL", DefaultCloudGatewayURL),
		VIN:             getEnvOrDefault("VIN", DefaultVIN),
		Timeout:         DefaultTimeout,
	}
}

// getEnvOrDefault returns the environment variable value or default if not set.
func getEnvOrDefault(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}
