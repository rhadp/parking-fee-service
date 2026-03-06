package main

import (
	"os"
	"strings"
)

// Config holds the service configuration.
type Config struct {
	HTTPPort  string
	NATSURL   string
	KnownVINs []string
}

// LoadConfig reads configuration from environment variables with defaults.
// Supported environment variables:
//   - HTTP_PORT: HTTP server port (default: "8081")
//   - NATS_URL: NATS server URL (default: "nats://localhost:4222")
//   - KNOWN_VINS: Comma-separated list of known VINs (default: "VIN12345,VIN67890")
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

	if vins := os.Getenv("KNOWN_VINS"); vins != "" {
		cfg.KnownVINs = strings.Split(vins, ",")
	}

	return cfg
}
