package config

import "os"

const (
	DefaultCloudGatewayURL = "http://localhost:8081"
)

// CloudGatewayURL returns the configured URL for CLOUD_GATEWAY.
func CloudGatewayURL() string {
	if v := os.Getenv("CLOUD_GATEWAY_URL"); v != "" {
		return v
	}
	return DefaultCloudGatewayURL
}

// BearerToken returns the configured bearer token.
// Returns empty string if not set.
func BearerToken() string {
	return os.Getenv("BEARER_TOKEN")
}
