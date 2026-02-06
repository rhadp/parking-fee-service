// Package config provides configuration loading for the parking-fee-service.
package config

import (
	"log/slog"
	"os"
	"strconv"
)

// Config holds all configuration for the parking-fee-service.
type Config struct {
	// Server configuration
	Port int

	// Database configuration
	DatabasePath string

	// Demo zone configuration
	DemoZoneID       string
	DemoOperatorName string
	DemoZoneMinLat   float64
	DemoZoneMaxLat   float64
	DemoZoneMinLng   float64
	DemoZoneMaxLng   float64
	DemoHourlyRate   float64
	DemoCurrency     string

	// Demo adapter configuration
	DemoAdapterID       string
	DemoAdapterVersion  string
	DemoAdapterImageRef string
	DemoAdapterChecksum string

	// Logging
	LogLevel string
}

// DefaultConfig returns a Config with sensible defaults.
func DefaultConfig() *Config {
	return &Config{
		Port:                8080,
		DatabasePath:        "./parking.db",
		DemoZoneID:          "demo-zone-001",
		DemoOperatorName:    "Demo Parking Operator",
		DemoZoneMinLat:      37.0,
		DemoZoneMaxLat:      38.0,
		DemoZoneMinLng:      -123.0,
		DemoZoneMaxLng:      -122.0,
		DemoHourlyRate:      2.50,
		DemoCurrency:        "USD",
		DemoAdapterID:       "demo-operator",
		DemoAdapterVersion:  "1.0.0",
		DemoAdapterImageRef: "us-docker.pkg.dev/sdv-demo/adapters/demo-operator:v1.0.0",
		DemoAdapterChecksum: "sha256:0000000000000000000000000000000000000000000000000000000000000000",
		LogLevel:            "info",
	}
}

// LoadConfig loads configuration from environment variables with defaults.
// Missing environment variables will use default values and log a warning.
func LoadConfig() *Config {
	cfg := DefaultConfig()

	// Server configuration
	if v := os.Getenv("PORT"); v != "" {
		if port, err := strconv.Atoi(v); err == nil {
			cfg.Port = port
		} else {
			slog.Warn("invalid PORT value, using default", "value", v, "default", cfg.Port)
		}
	}

	// Database configuration
	if v := os.Getenv("DATABASE_PATH"); v != "" {
		cfg.DatabasePath = v
	}

	// Demo zone configuration
	if v := os.Getenv("DEMO_ZONE_ID"); v != "" {
		cfg.DemoZoneID = v
	}
	if v := os.Getenv("DEMO_OPERATOR_NAME"); v != "" {
		cfg.DemoOperatorName = v
	}
	if v := os.Getenv("DEMO_ZONE_MIN_LAT"); v != "" {
		if f, err := strconv.ParseFloat(v, 64); err == nil {
			cfg.DemoZoneMinLat = f
		} else {
			slog.Warn("invalid DEMO_ZONE_MIN_LAT value, using default", "value", v, "default", cfg.DemoZoneMinLat)
		}
	}
	if v := os.Getenv("DEMO_ZONE_MAX_LAT"); v != "" {
		if f, err := strconv.ParseFloat(v, 64); err == nil {
			cfg.DemoZoneMaxLat = f
		} else {
			slog.Warn("invalid DEMO_ZONE_MAX_LAT value, using default", "value", v, "default", cfg.DemoZoneMaxLat)
		}
	}
	if v := os.Getenv("DEMO_ZONE_MIN_LNG"); v != "" {
		if f, err := strconv.ParseFloat(v, 64); err == nil {
			cfg.DemoZoneMinLng = f
		} else {
			slog.Warn("invalid DEMO_ZONE_MIN_LNG value, using default", "value", v, "default", cfg.DemoZoneMinLng)
		}
	}
	if v := os.Getenv("DEMO_ZONE_MAX_LNG"); v != "" {
		if f, err := strconv.ParseFloat(v, 64); err == nil {
			cfg.DemoZoneMaxLng = f
		} else {
			slog.Warn("invalid DEMO_ZONE_MAX_LNG value, using default", "value", v, "default", cfg.DemoZoneMaxLng)
		}
	}
	if v := os.Getenv("DEMO_HOURLY_RATE"); v != "" {
		if f, err := strconv.ParseFloat(v, 64); err == nil {
			cfg.DemoHourlyRate = f
		} else {
			slog.Warn("invalid DEMO_HOURLY_RATE value, using default", "value", v, "default", cfg.DemoHourlyRate)
		}
	}
	if v := os.Getenv("DEMO_CURRENCY"); v != "" {
		cfg.DemoCurrency = v
	}

	// Demo adapter configuration
	if v := os.Getenv("DEMO_ADAPTER_ID"); v != "" {
		cfg.DemoAdapterID = v
	}
	if v := os.Getenv("DEMO_ADAPTER_VERSION"); v != "" {
		cfg.DemoAdapterVersion = v
	}
	if v := os.Getenv("DEMO_ADAPTER_IMAGE_REF"); v != "" {
		cfg.DemoAdapterImageRef = v
	}
	if v := os.Getenv("DEMO_ADAPTER_CHECKSUM"); v != "" {
		cfg.DemoAdapterChecksum = v
	}

	// Logging
	if v := os.Getenv("LOG_LEVEL"); v != "" {
		cfg.LogLevel = v
	}

	return cfg
}

// LogLevel returns the slog.Level based on the configured log level string.
func (c *Config) SlogLevel() slog.Level {
	switch c.LogLevel {
	case "debug":
		return slog.LevelDebug
	case "info":
		return slog.LevelInfo
	case "warn":
		return slog.LevelWarn
	case "error":
		return slog.LevelError
	default:
		return slog.LevelInfo
	}
}
