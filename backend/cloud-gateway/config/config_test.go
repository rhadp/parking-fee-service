package config_test

import (
	"os"
	"testing"

	"parking-fee-service/backend/cloud-gateway/config"
	"parking-fee-service/backend/cloud-gateway/model"
)

// TestLoadConfigFromFile verifies that LoadConfig reads all fields from a JSON file.
// Test Spec: TS-06-11
// Requirements: 06-REQ-6.1, 06-REQ-6.2
func TestLoadConfigFromFile(t *testing.T) {
	content := `{
		"port": 8081,
		"nats_url": "nats://localhost:4222",
		"command_timeout_seconds": 30,
		"tokens": [
			{"token": "demo-token-001", "vin": "VIN12345"}
		]
	}`
	f, err := os.CreateTemp("", "cloud-gw-config-*.json")
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(f.Name())
	if _, err := f.WriteString(content); err != nil {
		t.Fatal(err)
	}
	f.Close()

	cfg, err := config.LoadConfig(f.Name())
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig returned nil config")
	}
	if cfg.Port != 8081 {
		t.Errorf("expected port 8081, got %d", cfg.Port)
	}
	if cfg.NatsURL != "nats://localhost:4222" {
		t.Errorf("expected nats_url nats://localhost:4222, got %s", cfg.NatsURL)
	}
	if cfg.CommandTimeoutSeconds != 30 {
		t.Errorf("expected command_timeout_seconds 30, got %d", cfg.CommandTimeoutSeconds)
	}
	if len(cfg.Tokens) < 1 {
		t.Fatal("expected at least 1 token entry")
	}
	if cfg.Tokens[0].Token == "" {
		t.Error("expected non-empty token string")
	}
	if cfg.Tokens[0].VIN == "" {
		t.Error("expected non-empty VIN string")
	}
}

// TestConfigTokenVINLookup verifies that GetVINForToken returns correct VIN for known token
// and ("", false) for unknown token.
// Test Spec: TS-06-12
// Requirements: 06-REQ-6.2
func TestConfigTokenVINLookup(t *testing.T) {
	cfg := &model.Config{
		Tokens: []model.TokenMapping{
			{Token: "demo-token-001", VIN: "VIN12345"},
		},
	}

	vin, ok := config.GetVINForToken(cfg, "demo-token-001")
	if !ok {
		t.Error("expected token 'demo-token-001' to be found")
	}
	if vin != "VIN12345" {
		t.Errorf("expected VIN 'VIN12345', got %q", vin)
	}

	vin2, ok2 := config.GetVINForToken(cfg, "unknown-token")
	if ok2 {
		t.Error("expected unknown-token not to be found")
	}
	if vin2 != "" {
		t.Errorf("expected empty VIN for unknown token, got %q", vin2)
	}
}

// TestConfigFileMissing verifies that LoadConfig returns an error for a missing file.
// Test Spec: TS-06-E7
// Requirements: 06-REQ-6.E1
func TestConfigFileMissing(t *testing.T) {
	_, err := config.LoadConfig("/nonexistent/cloud-gw-config-999999.json")
	if err == nil {
		t.Error("expected non-nil error for missing config file, got nil")
	}
}

// TestConfigFileInvalidJSON verifies that LoadConfig returns an error for invalid JSON.
// Test Spec: TS-06-E8
// Requirements: 06-REQ-6.E1
func TestConfigFileInvalidJSON(t *testing.T) {
	f, err := os.CreateTemp("", "cloud-gw-invalid-*.json")
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(f.Name())
	if _, err := f.WriteString("{invalid json"); err != nil {
		t.Fatal(err)
	}
	f.Close()

	_, err = config.LoadConfig(f.Name())
	if err == nil {
		t.Error("expected non-nil error for invalid JSON, got nil")
	}
}
