// Package config provides environment variable parsing for companion-app-cli.
package config

import "os"

const (
	DefaultCloudGatewayURL = "http://localhost:8081"
)

// GetEnv returns the value of the environment variable named by the key,
// or the fallback value if the variable is not set.
func GetEnv(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

// CloudGatewayURL returns the configured CLOUD_GATEWAY URL.
func CloudGatewayURL() string {
	return GetEnv("CLOUD_GATEWAY_URL", DefaultCloudGatewayURL)
}

// BearerToken returns the configured bearer token for authentication.
// Reads from CLOUD_GATEWAY_TOKEN environment variable.
func BearerToken() string {
	return os.Getenv("CLOUD_GATEWAY_TOKEN")
}
