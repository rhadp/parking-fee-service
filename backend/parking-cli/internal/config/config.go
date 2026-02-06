// Package config provides configuration management for PARKING_CLI.
package config

import (
	"os"
	"time"
)

// Default configuration values.
const (
	DefaultDataBrokerAddr       = "localhost:55556"
	DefaultParkingFeeServiceURL = "http://localhost:8081"
	DefaultUpdateServiceAddr    = "localhost:50051"
	DefaultParkingAdaptorAddr   = "localhost:50052"
	DefaultLockingServiceAddr   = "localhost:50053"
	DefaultTimeout              = 10 * time.Second
)

// Config holds PARKING_CLI configuration.
type Config struct {
	DataBrokerAddr       string
	ParkingFeeServiceURL string
	UpdateServiceAddr    string
	ParkingAdaptorAddr   string
	LockingServiceAddr   string
	Timeout              time.Duration
}

// Load loads configuration from environment variables with defaults.
func Load() *Config {
	return &Config{
		DataBrokerAddr:       getEnvOrDefault("DATA_BROKER_ADDR", DefaultDataBrokerAddr),
		ParkingFeeServiceURL: getEnvOrDefault("PARKING_FEE_SERVICE_URL", DefaultParkingFeeServiceURL),
		UpdateServiceAddr:    getEnvOrDefault("UPDATE_SERVICE_ADDR", DefaultUpdateServiceAddr),
		ParkingAdaptorAddr:   getEnvOrDefault("PARKING_ADAPTOR_ADDR", DefaultParkingAdaptorAddr),
		LockingServiceAddr:   getEnvOrDefault("LOCKING_SERVICE_ADDR", DefaultLockingServiceAddr),
		Timeout:              DefaultTimeout,
	}
}

// getEnvOrDefault returns the environment variable value or default if not set.
func getEnvOrDefault(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}
