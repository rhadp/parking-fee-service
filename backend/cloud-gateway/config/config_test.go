package config

import (
	"os"
	"path/filepath"
	"testing"
)

// ---------------------------------------------------------------------------
// TS-06-11: Config Loading
// Requirement: 06-REQ-6.1, 06-REQ-6.2
// ---------------------------------------------------------------------------

func TestLoadConfigFromFile(t *testing.T) {
	dir := t.TempDir()
	cfgPath := filepath.Join(dir, "config.json")
	cfgJSON := `{
		"port": 8081,
		"nats_url": "nats://localhost:4222",
		"command_timeout_seconds": 30,
		"tokens": [
			{"token": "demo-token-001", "vin": "VIN12345"},
			{"token": "demo-token-002", "vin": "VIN67890"}
		]
	}`
	if err := os.WriteFile(cfgPath, []byte(cfgJSON), 0644); err != nil {
		t.Fatalf("failed to write test config: %v", err)
	}

	cfg, err := LoadConfig(cfgPath)
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
	}

	if cfg.Port != 8081 {
		t.Errorf("expected Port 8081, got %d", cfg.Port)
	}
	if cfg.NatsURL != "nats://localhost:4222" {
		t.Errorf("expected NatsURL 'nats://localhost:4222', got %q", cfg.NatsURL)
	}
	if cfg.CommandTimeoutSeconds != 30 {
		t.Errorf("expected CommandTimeoutSeconds 30, got %d", cfg.CommandTimeoutSeconds)
	}
	if len(cfg.Tokens) < 1 {
		t.Fatalf("expected at least 1 token, got %d", len(cfg.Tokens))
	}
	if cfg.Tokens[0].Token == "" {
		t.Error("expected non-empty token")
	}
	if cfg.Tokens[0].VIN == "" {
		t.Error("expected non-empty VIN")
	}
}

// ---------------------------------------------------------------------------
// TS-06-12: Config Token-VIN Lookup
// Requirement: 06-REQ-6.2
// ---------------------------------------------------------------------------

func TestConfigTokenVINLookup(t *testing.T) {
	cfg := &Config{
		Tokens: []TokenMapping{
			{Token: "demo-token-001", VIN: "VIN12345"},
			{Token: "demo-token-002", VIN: "VIN67890"},
		},
	}

	vin, ok := cfg.GetVINForToken("demo-token-001")
	if !ok {
		t.Error("expected ok=true for demo-token-001")
	}
	if vin != "VIN12345" {
		t.Errorf("expected VIN 'VIN12345', got %q", vin)
	}

	vin, ok = cfg.GetVINForToken("unknown-token")
	if ok {
		t.Error("expected ok=false for unknown-token")
	}
	if vin != "" {
		t.Errorf("expected empty VIN for unknown token, got %q", vin)
	}
}

// ---------------------------------------------------------------------------
// TS-06-E7: Config File Missing
// Requirement: 06-REQ-6.E1
// ---------------------------------------------------------------------------

func TestConfigFileMissing(t *testing.T) {
	_, err := LoadConfig("/nonexistent/config.json")
	if err == nil {
		t.Error("expected error for missing config file, got nil")
	}
}

// ---------------------------------------------------------------------------
// TS-06-E8: Config File Invalid JSON
// Requirement: 06-REQ-6.E1
// ---------------------------------------------------------------------------

func TestConfigFileInvalidJSON(t *testing.T) {
	dir := t.TempDir()
	cfgPath := filepath.Join(dir, "invalid.json")
	if err := os.WriteFile(cfgPath, []byte("{invalid json"), 0644); err != nil {
		t.Fatalf("failed to write invalid config: %v", err)
	}

	_, err := LoadConfig(cfgPath)
	if err == nil {
		t.Error("expected error for invalid JSON config, got nil")
	}
}
