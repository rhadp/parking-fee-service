package config_test

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/config"
)

// TS-06-11: Config Loading
// Requirement: 06-REQ-6.1, 06-REQ-6.2
func TestLoadConfigFromFile(t *testing.T) {
	// Create a temporary config file with all required fields.
	content := `{
		"port": 8081,
		"nats_url": "nats://localhost:4222",
		"command_timeout_seconds": 30,
		"tokens": [
			{"token": "demo-token-001", "vin": "VIN12345"},
			{"token": "demo-token-002", "vin": "VIN67890"}
		]
	}`
	dir := t.TempDir()
	path := filepath.Join(dir, "config.json")
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		t.Fatal(err)
	}

	cfg, err := config.LoadConfig(path)
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
	if len(cfg.Tokens) != 2 {
		t.Fatalf("expected 2 tokens, got %d", len(cfg.Tokens))
	}
	if cfg.Tokens[0].Token == "" {
		t.Error("first token string is empty")
	}
	if cfg.Tokens[0].VIN == "" {
		t.Error("first token VIN is empty")
	}
}

// TS-06-12: Config Token-VIN Lookup
// Requirement: 06-REQ-6.2
func TestConfigTokenVINLookup(t *testing.T) {
	cfg := &config.Config{
		Tokens: []config.TokenMapping{
			{Token: "demo-token-001", VIN: "VIN12345"},
			{Token: "demo-token-002", VIN: "VIN67890"},
		},
	}

	// Valid token lookup.
	vin, ok := cfg.GetVINForToken("demo-token-001")
	if !ok {
		t.Error("expected GetVINForToken to return true for valid token")
	}
	if vin != "VIN12345" {
		t.Errorf("expected VIN12345, got %s", vin)
	}

	// Unknown token lookup.
	vin, ok = cfg.GetVINForToken("unknown-token")
	if ok {
		t.Error("expected GetVINForToken to return false for unknown token")
	}
	if vin != "" {
		t.Errorf("expected empty VIN for unknown token, got %s", vin)
	}
}

// TS-06-E7: Config File Missing
// Requirement: 06-REQ-6.E1
func TestConfigFileMissing(t *testing.T) {
	_, err := config.LoadConfig("/nonexistent/config.json")
	if err == nil {
		t.Error("expected error for missing config file, got nil")
	}
}

// TS-06-E8: Config File Invalid JSON
// Requirement: 06-REQ-6.E1
func TestConfigFileInvalidJSON(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "invalid.json")
	if err := os.WriteFile(path, []byte(`{invalid json`), 0644); err != nil {
		t.Fatal(err)
	}

	_, err := config.LoadConfig(path)
	if err == nil {
		t.Error("expected error for invalid JSON config, got nil")
	}
}
