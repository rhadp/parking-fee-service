package config

import (
	"testing"
	"time"
)

func TestLoad_Defaults(t *testing.T) {
	// Clear env vars that could interfere
	for _, key := range []string{"PORT", "MQTT_BROKER_URL", "MQTT_CLIENT_ID", "COMMAND_TIMEOUT", "AUTH_TOKEN"} {
		t.Setenv(key, "")
	}

	cfg := Load()

	if cfg.Port != "8081" {
		t.Errorf("expected default Port '8081', got %q", cfg.Port)
	}
	if cfg.MQTTBrokerURL != "tcp://localhost:1883" {
		t.Errorf("expected default MQTTBrokerURL 'tcp://localhost:1883', got %q", cfg.MQTTBrokerURL)
	}
	if cfg.MQTTClientID != "cloud-gateway" {
		t.Errorf("expected default MQTTClientID 'cloud-gateway', got %q", cfg.MQTTClientID)
	}
	if cfg.CommandTimeout != 30*time.Second {
		t.Errorf("expected default CommandTimeout 30s, got %v", cfg.CommandTimeout)
	}
	if cfg.AuthToken != "demo-token" {
		t.Errorf("expected default AuthToken 'demo-token', got %q", cfg.AuthToken)
	}
}

func TestLoad_EnvOverrides(t *testing.T) {
	t.Setenv("PORT", "9090")
	t.Setenv("MQTT_BROKER_URL", "tcp://broker:1883")
	t.Setenv("MQTT_CLIENT_ID", "custom-client")
	t.Setenv("COMMAND_TIMEOUT", "5s")
	t.Setenv("AUTH_TOKEN", "custom-token")

	cfg := Load()

	if cfg.Port != "9090" {
		t.Errorf("expected Port '9090', got %q", cfg.Port)
	}
	if cfg.MQTTBrokerURL != "tcp://broker:1883" {
		t.Errorf("expected MQTTBrokerURL 'tcp://broker:1883', got %q", cfg.MQTTBrokerURL)
	}
	if cfg.MQTTClientID != "custom-client" {
		t.Errorf("expected MQTTClientID 'custom-client', got %q", cfg.MQTTClientID)
	}
	if cfg.CommandTimeout != 5*time.Second {
		t.Errorf("expected CommandTimeout 5s, got %v", cfg.CommandTimeout)
	}
	if cfg.AuthToken != "custom-token" {
		t.Errorf("expected AuthToken 'custom-token', got %q", cfg.AuthToken)
	}
}

func TestLoad_InvalidTimeout(t *testing.T) {
	t.Setenv("COMMAND_TIMEOUT", "not-a-duration")

	cfg := Load()

	// Should fall back to 30s default
	if cfg.CommandTimeout != 30*time.Second {
		t.Errorf("expected fallback CommandTimeout 30s for invalid input, got %v", cfg.CommandTimeout)
	}
}

func TestEnvOrDefault(t *testing.T) {
	t.Setenv("TEST_VAR", "custom")
	if v := envOrDefault("TEST_VAR", "default"); v != "custom" {
		t.Errorf("expected 'custom', got %q", v)
	}

	t.Setenv("TEST_VAR", "")
	if v := envOrDefault("TEST_VAR", "default"); v != "default" {
		t.Errorf("expected 'default' for empty env, got %q", v)
	}

	if v := envOrDefault("NONEXISTENT_VAR_12345", "fallback"); v != "fallback" {
		t.Errorf("expected 'fallback' for missing env, got %q", v)
	}
}
