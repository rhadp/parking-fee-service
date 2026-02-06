package config

import (
	"log/slog"
	"os"
	"testing"
	"time"
)

// Helper to set environment variables and restore them after the test.
func setEnv(t *testing.T, key, value string) {
	t.Helper()
	old := os.Getenv(key)
	if err := os.Setenv(key, value); err != nil {
		t.Fatalf("failed to set env %s: %v", key, err)
	}
	t.Cleanup(func() {
		if old == "" {
			os.Unsetenv(key)
		} else {
			os.Setenv(key, old)
		}
	})
}

// Helper to unset environment variables and restore them after the test.
func unsetEnv(t *testing.T, key string) {
	t.Helper()
	old := os.Getenv(key)
	os.Unsetenv(key)
	t.Cleanup(func() {
		if old != "" {
			os.Setenv(key, old)
		}
	})
}

func TestLoadConfig_RequiredFields(t *testing.T) {
	// Clear required env vars
	unsetEnv(t, "MQTT_BROKER_URL")
	unsetEnv(t, "CONFIGURED_VIN")

	// Should fail without required fields
	_, err := LoadConfig()
	if err == nil {
		t.Error("expected error when MQTT_BROKER_URL is missing")
	}

	// Set MQTT_BROKER_URL but not VIN
	setEnv(t, "MQTT_BROKER_URL", "tcp://localhost:1883")
	_, err = LoadConfig()
	if err == nil {
		t.Error("expected error when CONFIGURED_VIN is missing")
	}

	// Set both required fields
	setEnv(t, "CONFIGURED_VIN", "DEMO_VIN_123")
	cfg, err := LoadConfig()
	if err != nil {
		t.Fatalf("unexpected error with all required fields: %v", err)
	}
	if cfg.MQTTBrokerURL != "tcp://localhost:1883" {
		t.Errorf("expected MQTT_BROKER_URL=tcp://localhost:1883, got %s", cfg.MQTTBrokerURL)
	}
	if cfg.ConfiguredVIN != "DEMO_VIN_123" {
		t.Errorf("expected CONFIGURED_VIN=DEMO_VIN_123, got %s", cfg.ConfiguredVIN)
	}
}

func TestLoadConfig_DefaultValues(t *testing.T) {
	// Set only required fields
	setEnv(t, "MQTT_BROKER_URL", "tcp://localhost:1883")
	setEnv(t, "CONFIGURED_VIN", "TEST_VIN")

	// Clear optional fields to test defaults
	unsetEnv(t, "PORT")
	unsetEnv(t, "MQTT_USERNAME")
	unsetEnv(t, "MQTT_PASSWORD")
	unsetEnv(t, "MQTT_CLIENT_ID")
	unsetEnv(t, "COMMAND_TIMEOUT_SECONDS")
	unsetEnv(t, "MAX_COMMANDS")
	unsetEnv(t, "OTLP_ENDPOINT")
	unsetEnv(t, "PARKING_FEE_SERVICE_URL")
	unsetEnv(t, "LOG_LEVEL")

	cfg, err := LoadConfig()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	// Check defaults
	if cfg.Port != 8080 {
		t.Errorf("expected Port=8080, got %d", cfg.Port)
	}
	if cfg.MQTTClientID != "cloud-gateway" {
		t.Errorf("expected MQTTClientID=cloud-gateway, got %s", cfg.MQTTClientID)
	}
	if cfg.CommandTimeout != 30*time.Second {
		t.Errorf("expected CommandTimeout=30s, got %v", cfg.CommandTimeout)
	}
	if cfg.MaxCommands != 100 {
		t.Errorf("expected MaxCommands=100, got %d", cfg.MaxCommands)
	}
	if cfg.OTLPEndpoint != "" {
		t.Errorf("expected OTLPEndpoint='', got %s", cfg.OTLPEndpoint)
	}
	if cfg.ParkingFeeServiceURL != "http://localhost:8081" {
		t.Errorf("expected ParkingFeeServiceURL=http://localhost:8081, got %s", cfg.ParkingFeeServiceURL)
	}
	if cfg.LogLevel != "info" {
		t.Errorf("expected LogLevel=info, got %s", cfg.LogLevel)
	}
}

func TestLoadConfig_EnvironmentVariableParsing(t *testing.T) {
	// Set all environment variables
	setEnv(t, "PORT", "9090")
	setEnv(t, "MQTT_BROKER_URL", "ssl://broker.example.com:8883")
	setEnv(t, "MQTT_USERNAME", "test-user")
	setEnv(t, "MQTT_PASSWORD", "test-pass")
	setEnv(t, "MQTT_CLIENT_ID", "custom-client")
	setEnv(t, "CONFIGURED_VIN", "VIN_12345")
	setEnv(t, "COMMAND_TIMEOUT_SECONDS", "60")
	setEnv(t, "MAX_COMMANDS", "200")
	setEnv(t, "OTLP_ENDPOINT", "http://otel:4317")
	setEnv(t, "PARKING_FEE_SERVICE_URL", "http://parking:8080")
	setEnv(t, "LOG_LEVEL", "debug")

	cfg, err := LoadConfig()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	// Verify all values
	if cfg.Port != 9090 {
		t.Errorf("expected Port=9090, got %d", cfg.Port)
	}
	if cfg.MQTTBrokerURL != "ssl://broker.example.com:8883" {
		t.Errorf("expected MQTTBrokerURL=ssl://broker.example.com:8883, got %s", cfg.MQTTBrokerURL)
	}
	if cfg.MQTTUsername != "test-user" {
		t.Errorf("expected MQTTUsername=test-user, got %s", cfg.MQTTUsername)
	}
	if cfg.MQTTPassword != "test-pass" {
		t.Errorf("expected MQTTPassword=test-pass, got %s", cfg.MQTTPassword)
	}
	if cfg.MQTTClientID != "custom-client" {
		t.Errorf("expected MQTTClientID=custom-client, got %s", cfg.MQTTClientID)
	}
	if cfg.ConfiguredVIN != "VIN_12345" {
		t.Errorf("expected ConfiguredVIN=VIN_12345, got %s", cfg.ConfiguredVIN)
	}
	if cfg.CommandTimeout != 60*time.Second {
		t.Errorf("expected CommandTimeout=60s, got %v", cfg.CommandTimeout)
	}
	if cfg.MaxCommands != 200 {
		t.Errorf("expected MaxCommands=200, got %d", cfg.MaxCommands)
	}
	if cfg.OTLPEndpoint != "http://otel:4317" {
		t.Errorf("expected OTLPEndpoint=http://otel:4317, got %s", cfg.OTLPEndpoint)
	}
	if cfg.ParkingFeeServiceURL != "http://parking:8080" {
		t.Errorf("expected ParkingFeeServiceURL=http://parking:8080, got %s", cfg.ParkingFeeServiceURL)
	}
	if cfg.LogLevel != "debug" {
		t.Errorf("expected LogLevel=debug, got %s", cfg.LogLevel)
	}
}

func TestLoadConfig_OTLPEndpointOptional(t *testing.T) {
	setEnv(t, "MQTT_BROKER_URL", "tcp://localhost:1883")
	setEnv(t, "CONFIGURED_VIN", "TEST_VIN")
	unsetEnv(t, "OTLP_ENDPOINT")

	cfg, err := LoadConfig()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	// OTel should be disabled when endpoint is empty
	otelCfg := cfg.GetOTelConfig()
	if otelCfg.Enabled {
		t.Error("expected OTel to be disabled when OTLP_ENDPOINT is empty")
	}
	if otelCfg.Endpoint != "" {
		t.Errorf("expected empty endpoint, got %s", otelCfg.Endpoint)
	}

	// Now set the endpoint
	setEnv(t, "OTLP_ENDPOINT", "http://collector:4317")
	cfg, err = LoadConfig()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	otelCfg = cfg.GetOTelConfig()
	if !otelCfg.Enabled {
		t.Error("expected OTel to be enabled when OTLP_ENDPOINT is set")
	}
	if otelCfg.Endpoint != "http://collector:4317" {
		t.Errorf("expected endpoint=http://collector:4317, got %s", otelCfg.Endpoint)
	}
}

func TestConfig_GetMQTTConfig(t *testing.T) {
	cfg := &Config{
		MQTTBrokerURL: "tcp://broker:1883",
		MQTTUsername:  "user",
		MQTTPassword:  "pass",
		MQTTClientID:  "client-1",
	}

	mqttCfg := cfg.GetMQTTConfig()
	if mqttCfg.BrokerURL != "tcp://broker:1883" {
		t.Errorf("expected BrokerURL=tcp://broker:1883, got %s", mqttCfg.BrokerURL)
	}
	if mqttCfg.Username != "user" {
		t.Errorf("expected Username=user, got %s", mqttCfg.Username)
	}
	if mqttCfg.Password != "pass" {
		t.Errorf("expected Password=pass, got %s", mqttCfg.Password)
	}
	if mqttCfg.ClientID != "client-1" {
		t.Errorf("expected ClientID=client-1, got %s", mqttCfg.ClientID)
	}
}

func TestConfig_SlogLevel(t *testing.T) {
	tests := []struct {
		level    string
		expected slog.Level
	}{
		{"debug", slog.LevelDebug},
		{"info", slog.LevelInfo},
		{"warn", slog.LevelWarn},
		{"error", slog.LevelError},
		{"unknown", slog.LevelInfo}, // default to info
		{"", slog.LevelInfo},        // default to info
	}

	for _, tt := range tests {
		t.Run(tt.level, func(t *testing.T) {
			cfg := &Config{LogLevel: tt.level}
			if got := cfg.SlogLevel(); got != tt.expected {
				t.Errorf("SlogLevel(%q) = %v, want %v", tt.level, got, tt.expected)
			}
		})
	}
}

func TestConfig_Validate(t *testing.T) {
	// Missing MQTT_BROKER_URL
	cfg := &Config{ConfiguredVIN: "VIN"}
	if err := cfg.Validate(); err == nil {
		t.Error("expected error for missing MQTT_BROKER_URL")
	}

	// Missing CONFIGURED_VIN
	cfg = &Config{MQTTBrokerURL: "tcp://localhost:1883"}
	if err := cfg.Validate(); err == nil {
		t.Error("expected error for missing CONFIGURED_VIN")
	}

	// Both present
	cfg = &Config{
		MQTTBrokerURL: "tcp://localhost:1883",
		ConfiguredVIN: "VIN",
	}
	if err := cfg.Validate(); err != nil {
		t.Errorf("unexpected error: %v", err)
	}
}

func TestLoadConfig_InvalidNumericValues(t *testing.T) {
	setEnv(t, "MQTT_BROKER_URL", "tcp://localhost:1883")
	setEnv(t, "CONFIGURED_VIN", "TEST_VIN")

	// Invalid PORT should use default
	setEnv(t, "PORT", "not-a-number")
	cfg, err := LoadConfig()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if cfg.Port != 8080 {
		t.Errorf("expected Port=8080 (default) for invalid value, got %d", cfg.Port)
	}

	// Invalid COMMAND_TIMEOUT_SECONDS should use default
	setEnv(t, "PORT", "8080")
	setEnv(t, "COMMAND_TIMEOUT_SECONDS", "invalid")
	cfg, err = LoadConfig()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if cfg.CommandTimeout != 30*time.Second {
		t.Errorf("expected CommandTimeout=30s (default) for invalid value, got %v", cfg.CommandTimeout)
	}

	// Invalid MAX_COMMANDS should use default
	setEnv(t, "COMMAND_TIMEOUT_SECONDS", "30")
	setEnv(t, "MAX_COMMANDS", "abc")
	cfg, err = LoadConfig()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if cfg.MaxCommands != 100 {
		t.Errorf("expected MaxCommands=100 (default) for invalid value, got %d", cfg.MaxCommands)
	}
}
