package config_test

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/config"
)

// TS-06-11: Config Loading
// Requirement: 06-REQ-6.1, 06-REQ-6.2
// LoadConfig reads configuration from the specified file path with all required fields.
func TestLoadConfigFromFile(t *testing.T) {
	content := `{
		"port": 8081,
		"nats_url": "nats://localhost:4222",
		"command_timeout_seconds": 30,
		"tokens": [
			{"token": "demo-token-001", "vin": "VIN12345"},
			{"token": "demo-token-002", "vin": "VIN67890"}
		]
	}`

	tmpDir := t.TempDir()
	cfgPath := filepath.Join(tmpDir, "config.json")
	if err := os.WriteFile(cfgPath, []byte(content), 0644); err != nil {
		t.Fatalf("failed to write temp config: %v", err)
	}

	cfg, err := config.LoadConfig(cfgPath)
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig returned nil config")
	}
	if cfg.Port != 8081 {
		t.Errorf("Port = %d, want 8081", cfg.Port)
	}
	if cfg.NatsURL != "nats://localhost:4222" {
		t.Errorf("NatsURL = %q, want %q", cfg.NatsURL, "nats://localhost:4222")
	}
	if cfg.CommandTimeoutSeconds != 30 {
		t.Errorf("CommandTimeoutSeconds = %d, want 30", cfg.CommandTimeoutSeconds)
	}
	if len(cfg.Tokens) < 1 {
		t.Fatal("expected at least 1 token")
	}
	if cfg.Tokens[0].Token == "" {
		t.Error("first token string is empty")
	}
	if cfg.Tokens[0].VIN == "" {
		t.Error("first VIN string is empty")
	}
}

// TS-06-12: Config Token-VIN Lookup
// Requirement: 06-REQ-6.2
// GetVINForToken returns the correct VIN for a configured token.
func TestConfigTokenVINLookup(t *testing.T) {
	content := `{
		"port": 8081,
		"nats_url": "nats://localhost:4222",
		"command_timeout_seconds": 30,
		"tokens": [
			{"token": "demo-token-001", "vin": "VIN12345"}
		]
	}`

	tmpDir := t.TempDir()
	cfgPath := filepath.Join(tmpDir, "config.json")
	if err := os.WriteFile(cfgPath, []byte(content), 0644); err != nil {
		t.Fatalf("failed to write temp config: %v", err)
	}

	cfg, err := config.LoadConfig(cfgPath)
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig returned nil config")
	}

	// Known token should return correct VIN
	vin, ok := config.GetVINForToken(cfg, "demo-token-001")
	if !ok {
		t.Error("GetVINForToken returned false for known token")
	}
	if vin != "VIN12345" {
		t.Errorf("VIN = %q, want %q", vin, "VIN12345")
	}

	// Unknown token should return false
	vin, ok = config.GetVINForToken(cfg, "unknown-token")
	if ok {
		t.Error("GetVINForToken returned true for unknown token")
	}
	if vin != "" {
		t.Errorf("VIN = %q, want empty string", vin)
	}
}

// TS-06-E7: Config File Missing
// Requirement: 06-REQ-6.E1
// Missing config file causes LoadConfig to return an error.
func TestConfigFileMissing(t *testing.T) {
	_, err := config.LoadConfig("/nonexistent/config.json")
	if err == nil {
		t.Error("LoadConfig should return error for missing file")
	}
}

// TS-06-E8: Config File Invalid JSON
// Requirement: 06-REQ-6.E1
// Invalid JSON config causes LoadConfig to return an error.
func TestConfigFileInvalidJSON(t *testing.T) {
	tmpDir := t.TempDir()
	cfgPath := filepath.Join(tmpDir, "invalid.json")
	if err := os.WriteFile(cfgPath, []byte("{invalid json"), 0644); err != nil {
		t.Fatalf("failed to write temp file: %v", err)
	}

	_, err := config.LoadConfig(cfgPath)
	if err == nil {
		t.Error("LoadConfig should return error for invalid JSON")
	}
}
