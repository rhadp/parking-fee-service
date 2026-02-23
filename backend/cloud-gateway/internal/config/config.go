// Package config provides configuration loading for the CLOUD_GATEWAY service.
// Configuration values are read from environment variables with sensible defaults.
package config

import (
	"os"
	"time"
)

// Config holds all configuration values for the CLOUD_GATEWAY service.
type Config struct {
	// Port is the HTTP listen port for the REST API.
	Port string

	// MQTTBrokerURL is the MQTT broker connection URL.
	MQTTBrokerURL string

	// MQTTClientID is the MQTT client identifier.
	MQTTClientID string

	// CommandTimeout is the maximum time to wait for a command response
	// from MQTT before returning a 504 Gateway Timeout.
	CommandTimeout time.Duration

	// AuthToken is the valid bearer token for authentication (demo only).
	AuthToken string
}

// Load reads configuration from environment variables and applies defaults
// for any unset values.
func Load() Config {
	cfg := Config{
		Port:           envOrDefault("PORT", "8081"),
		MQTTBrokerURL:  envOrDefault("MQTT_BROKER_URL", "tcp://localhost:1883"),
		MQTTClientID:   envOrDefault("MQTT_CLIENT_ID", "cloud-gateway"),
		AuthToken:      envOrDefault("AUTH_TOKEN", "demo-token"),
	}

	timeoutStr := envOrDefault("COMMAND_TIMEOUT", "30s")
	d, err := time.ParseDuration(timeoutStr)
	if err != nil {
		d = 30 * time.Second
	}
	cfg.CommandTimeout = d

	return cfg
}

// envOrDefault returns the value of the environment variable named by key,
// or defaultVal if the variable is not set or is empty.
func envOrDefault(key, defaultVal string) string {
	v := os.Getenv(key)
	if v == "" {
		return defaultVal
	}
	return v
}
