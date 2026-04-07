package config

import (
	"os"
	"path/filepath"
	"testing"
)

// TS-06-11: Config Loading
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
		t.Fatalf("failed to write temp config: %v", err)
	}

	cfg, err := LoadConfig(path)
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
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
		t.Fatalf("Tokens length = %d, want >= 1", len(cfg.Tokens))
	}
	if cfg.Tokens[0].Token == "" {
		t.Error("Tokens[0].Token is empty")
	}
	if cfg.Tokens[0].VIN == "" {
		t.Error("Tokens[0].VIN is empty")
	}
}

// TS-06-12: Config Token-VIN Lookup
func TestConfigTokenVINLookup(t *testing.T) {
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
		t.Fatalf("failed to write temp config: %v", err)
	}

	cfg, err := LoadConfig(path)
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
	}

	// Valid token lookup
	vin, ok := GetVINForToken(cfg, "demo-token-001")
	if !ok {
		t.Error("GetVINForToken returned ok=false for valid token")
	}
	if vin != "VIN12345" {
		t.Errorf("VIN = %q, want %q", vin, "VIN12345")
	}

	// Unknown token lookup
	vin, ok = GetVINForToken(cfg, "unknown-token")
	if ok {
		t.Error("GetVINForToken returned ok=true for unknown token")
	}
	if vin != "" {
		t.Errorf("VIN = %q, want empty string for unknown token", vin)
	}
}

// TS-06-E7: Config File Missing
func TestConfigFileMissing(t *testing.T) {
	_, err := LoadConfig("/nonexistent/config.json")
	if err == nil {
		t.Error("LoadConfig should return error for missing file")
	}
}

// TS-06-E8: Config File Invalid JSON
func TestConfigFileInvalidJSON(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "invalid.json")
	if err := os.WriteFile(path, []byte("{invalid json"), 0644); err != nil {
		t.Fatalf("failed to write temp file: %v", err)
	}

	_, err := LoadConfig(path)
	if err == nil {
		t.Error("LoadConfig should return error for invalid JSON")
	}
}
