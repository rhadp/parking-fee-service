// Package config provides configuration loading for the cloud-gateway service.
package config

import (
	"errors"
	"log/slog"
	"os"
	"strconv"
	"time"
)

// Config holds all configuration for the cloud-gateway service.
type Config struct {
	// Server configuration
	Port int

	// MQTT configuration (Southbound interface)
	MQTTBrokerURL string
	MQTTUsername  string
	MQTTPassword  string
	MQTTClientID  string

	// Vehicle configuration
	ConfiguredVIN string

	// Command configuration
	CommandTimeout time.Duration
	MaxCommands    int

	// OpenTelemetry configuration
	OTLPEndpoint string

	// Parking fee service URL (for proxying session queries)
	ParkingFeeServiceURL string

	// Logging
	LogLevel string
}

// MQTTConfig returns the MQTT-specific configuration.
type MQTTConfig struct {
	BrokerURL string
	Username  string
	Password  string
	ClientID  string
}

// OTelConfig returns the OpenTelemetry configuration.
type OTelConfig struct {
	Endpoint string
	Enabled  bool
}

// ErrMissingRequired is returned when a required configuration field is missing.
var ErrMissingRequired = errors.New("missing required configuration")

// DefaultConfig returns a Config with sensible defaults.
func DefaultConfig() *Config {
	return &Config{
		Port:                 8080,
		MQTTBrokerURL:        "",
		MQTTUsername:         "",
		MQTTPassword:         "",
		MQTTClientID:         "cloud-gateway",
		ConfiguredVIN:        "",
		CommandTimeout:       30 * time.Second,
		MaxCommands:          100,
		OTLPEndpoint:         "",
		ParkingFeeServiceURL: "http://localhost:8081",
		LogLevel:             "info",
	}
}

// LoadConfig loads configuration from environment variables with defaults.
// Returns an error if required fields (MQTT_BROKER_URL, CONFIGURED_VIN) are missing.
func LoadConfig() (*Config, error) {
	cfg := DefaultConfig()

	// Server configuration
	if v := os.Getenv("PORT"); v != "" {
		if port, err := strconv.Atoi(v); err == nil {
			cfg.Port = port
		} else {
			slog.Warn("invalid PORT value, using default", "value", v, "default", cfg.Port)
		}
	}

	// MQTT configuration (required)
	if v := os.Getenv("MQTT_BROKER_URL"); v != "" {
		cfg.MQTTBrokerURL = v
	}
	if v := os.Getenv("MQTT_USERNAME"); v != "" {
		cfg.MQTTUsername = v
	}
	if v := os.Getenv("MQTT_PASSWORD"); v != "" {
		cfg.MQTTPassword = v
	}
	if v := os.Getenv("MQTT_CLIENT_ID"); v != "" {
		cfg.MQTTClientID = v
	}

	// Vehicle configuration (required)
	if v := os.Getenv("CONFIGURED_VIN"); v != "" {
		cfg.ConfiguredVIN = v
	}

	// Command configuration
	if v := os.Getenv("COMMAND_TIMEOUT_SECONDS"); v != "" {
		if seconds, err := strconv.Atoi(v); err == nil {
			cfg.CommandTimeout = time.Duration(seconds) * time.Second
		} else {
			slog.Warn("invalid COMMAND_TIMEOUT_SECONDS value, using default", "value", v, "default", cfg.CommandTimeout)
		}
	}
	if v := os.Getenv("MAX_COMMANDS"); v != "" {
		if maxCmd, err := strconv.Atoi(v); err == nil {
			cfg.MaxCommands = maxCmd
		} else {
			slog.Warn("invalid MAX_COMMANDS value, using default", "value", v, "default", cfg.MaxCommands)
		}
	}

	// OpenTelemetry configuration (optional)
	if v := os.Getenv("OTLP_ENDPOINT"); v != "" {
		cfg.OTLPEndpoint = v
	}

	// Parking fee service URL
	if v := os.Getenv("PARKING_FEE_SERVICE_URL"); v != "" {
		cfg.ParkingFeeServiceURL = v
	}

	// Logging
	if v := os.Getenv("LOG_LEVEL"); v != "" {
		cfg.LogLevel = v
	}

	// Validate required fields
	if err := cfg.Validate(); err != nil {
		return nil, err
	}

	return cfg, nil
}

// Validate checks that all required configuration fields are present.
func (c *Config) Validate() error {
	if c.MQTTBrokerURL == "" {
		return errors.New("MQTT_BROKER_URL is required")
	}
	if c.ConfiguredVIN == "" {
		return errors.New("CONFIGURED_VIN is required")
	}
	return nil
}

// GetMQTTConfig returns the MQTT configuration.
func (c *Config) GetMQTTConfig() *MQTTConfig {
	return &MQTTConfig{
		BrokerURL: c.MQTTBrokerURL,
		Username:  c.MQTTUsername,
		Password:  c.MQTTPassword,
		ClientID:  c.MQTTClientID,
	}
}

// GetOTelConfig returns the OpenTelemetry configuration.
func (c *Config) GetOTelConfig() *OTelConfig {
	return &OTelConfig{
		Endpoint: c.OTLPEndpoint,
		Enabled:  c.OTLPEndpoint != "",
	}
}

// SlogLevel returns the slog.Level based on the configured log level string.
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

// LogConfigValues logs all configuration values except secrets.
func (c *Config) LogConfigValues(logger *slog.Logger) {
	logger.Info("configuration loaded",
		slog.Int("port", c.Port),
		slog.String("mqtt_broker_url", c.MQTTBrokerURL),
		slog.String("mqtt_client_id", c.MQTTClientID),
		slog.String("mqtt_username", c.MQTTUsername),
		slog.Bool("mqtt_password_set", c.MQTTPassword != ""),
		slog.String("configured_vin", c.ConfiguredVIN),
		slog.Duration("command_timeout", c.CommandTimeout),
		slog.Int("max_commands", c.MaxCommands),
		slog.String("otlp_endpoint", c.OTLPEndpoint),
		slog.Bool("otlp_enabled", c.OTLPEndpoint != ""),
		slog.String("parking_fee_service_url", c.ParkingFeeServiceURL),
		slog.String("log_level", c.LogLevel),
	)
}
