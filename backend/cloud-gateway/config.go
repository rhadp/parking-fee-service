package main

// Config holds the service configuration.
type Config struct {
	HTTPPort  string
	NATSURL   string
	KnownVINs []string
}

// LoadConfig reads configuration from environment variables with defaults.
func LoadConfig() Config {
	// Stub: not yet implemented
	return Config{
		HTTPPort:  "8081",
		NATSURL:   "nats://localhost:4222",
		KnownVINs: []string{"VIN12345", "VIN67890"},
	}
}
