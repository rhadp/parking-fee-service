package config_test

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/config"
)

// TestLoadConfigFromFile verifies LoadConfig reads all fields (TS-06-11).
func TestLoadConfigFromFile(t *testing.T) {
	content := `{
        "port": 8081,
        "nats_url": "nats://localhost:4222",
        "command_timeout_seconds": 30,
        "tokens": [{"token": "demo-token-001", "vin": "VIN12345"}]
    }`
	tmp := t.TempDir()
	path := filepath.Join(tmp, "config.json")
	if err := os.WriteFile(path, []byte(content), 0600); err != nil {
		t.Fatal(err)
	}
	cfg, err := config.LoadConfig(path)
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
	}
	if cfg.Port != 8081 {
		t.Errorf("Port: got %d, want 8081", cfg.Port)
	}
	if cfg.NatsURL != "nats://localhost:4222" {
		t.Errorf("NatsURL: got %q, want %q", cfg.NatsURL, "nats://localhost:4222")
	}
	if cfg.CommandTimeoutSeconds != 30 {
		t.Errorf("CommandTimeoutSeconds: got %d, want 30", cfg.CommandTimeoutSeconds)
	}
	if len(cfg.Tokens) < 1 {
		t.Errorf("Tokens: got %d, want >= 1", len(cfg.Tokens))
	} else {
		if cfg.Tokens[0].Token == "" {
			t.Error("Tokens[0].Token: got empty string")
		}
		if cfg.Tokens[0].VIN == "" {
			t.Error("Tokens[0].VIN: got empty string")
		}
	}
}

// TestConfigTokenVINLookup verifies GetVINForToken returns correct VIN (TS-06-12).
func TestConfigTokenVINLookup(t *testing.T) {
	cfg := &config.Config{
		Tokens: []config.TokenMapping{
			{Token: "demo-token-001", VIN: "VIN12345"},
		},
	}
	vin, ok := cfg.GetVINForToken("demo-token-001")
	if !ok {
		t.Error("GetVINForToken: expected ok=true for known token")
	}
	if vin != "VIN12345" {
		t.Errorf("GetVINForToken: got %q, want %q", vin, "VIN12345")
	}

	vin, ok = cfg.GetVINForToken("unknown-token")
	if ok {
		t.Error("GetVINForToken: expected ok=false for unknown token")
	}
	if vin != "" {
		t.Errorf("GetVINForToken: got %q, want empty string", vin)
	}
}

// TestConfigFileMissing verifies LoadConfig returns error for missing file (TS-06-E7).
func TestConfigFileMissing(t *testing.T) {
	_, err := config.LoadConfig("/nonexistent/path/config.json")
	if err == nil {
		t.Error("LoadConfig: expected error for missing file, got nil")
	}
}

// TestConfigFileInvalidJSON verifies LoadConfig returns error for invalid JSON (TS-06-E8).
func TestConfigFileInvalidJSON(t *testing.T) {
	tmp := t.TempDir()
	path := filepath.Join(tmp, "invalid.json")
	if err := os.WriteFile(path, []byte("{invalid json"), 0600); err != nil {
		t.Fatal(err)
	}
	_, err := config.LoadConfig(path)
	if err == nil {
		t.Error("LoadConfig: expected error for invalid JSON, got nil")
	}
}
