package main

import "os"

// Config holds the configuration for the CLOUD_GATEWAY service.
type Config struct {
	HTTPPort  string
	NATSURL   string
	KnownVINs []string
}

// LoadConfig reads configuration from environment variables with sensible defaults.
func LoadConfig() Config {
	cfg := Config{
		HTTPPort:  "8081",
		NATSURL:   "nats://localhost:4222",
		KnownVINs: []string{"VIN12345", "VIN67890"},
	}

	if port := os.Getenv("HTTP_PORT"); port != "" {
		cfg.HTTPPort = port
	}
	if natsURL := os.Getenv("NATS_URL"); natsURL != "" {
		cfg.NATSURL = natsURL
	}

	return cfg
}
