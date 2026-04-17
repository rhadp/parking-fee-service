package config_test

import (
	"os"
	"testing"

	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/config"
)

// testConfig returns a Config pre-populated with demo tokens for test use.
func testConfig() *config.Config {
	return &config.Config{
		Port:                  8081,
		NatsURL:               "nats://localhost:4222",
		CommandTimeoutSeconds: 30,
		Tokens: []config.TokenMapping{
			{Token: "demo-token-001", VIN: "VIN12345"},
			{Token: "demo-token-002", VIN: "VIN67890"},
		},
	}
}

// TestLoadConfigFromFile verifies that LoadConfig reads all fields from a JSON file.
// TS-06-11
func TestLoadConfigFromFile(t *testing.T) {
	const content = `{
		"port": 8081,
		"nats_url": "nats://localhost:4222",
		"command_timeout_seconds": 30,
		"tokens": [{"token":"demo-token-001","vin":"VIN12345"}]
	}`
	f, err := os.CreateTemp(t.TempDir(), "config-*.json")
	if err != nil {
		t.Fatalf("CreateTemp: %v", err)
	}
	if _, err := f.WriteString(content); err != nil {
		t.Fatalf("WriteString: %v", err)
	}
	f.Close()

	cfg, err := config.LoadConfig(f.Name())
	if err != nil {
		t.Fatalf("LoadConfig(%q): unexpected error: %v", f.Name(), err)
	}
	if cfg.Port != 8081 {
		t.Errorf("Port: want 8081, got %d", cfg.Port)
	}
	if cfg.NatsURL != "nats://localhost:4222" {
		t.Errorf("NatsURL: want 'nats://localhost:4222', got %q", cfg.NatsURL)
	}
	if cfg.CommandTimeoutSeconds != 30 {
		t.Errorf("CommandTimeoutSeconds: want 30, got %d", cfg.CommandTimeoutSeconds)
	}
	if len(cfg.Tokens) != 1 {
		t.Fatalf("len(Tokens): want 1, got %d", len(cfg.Tokens))
	}
	if cfg.Tokens[0].Token != "demo-token-001" {
		t.Errorf("Tokens[0].Token: want 'demo-token-001', got %q", cfg.Tokens[0].Token)
	}
	if cfg.Tokens[0].VIN != "VIN12345" {
		t.Errorf("Tokens[0].VIN: want 'VIN12345', got %q", cfg.Tokens[0].VIN)
	}
}

// TestConfigTokenVINLookup verifies that GetVINForToken returns the correct VIN for a
// configured token and ("", false) for an unknown token.
// TS-06-12
func TestConfigTokenVINLookup(t *testing.T) {
	cfg := testConfig()

	vin, ok := cfg.GetVINForToken("demo-token-001")
	if !ok {
		t.Error("GetVINForToken('demo-token-001'): want ok=true, got false")
	}
	if vin != "VIN12345" {
		t.Errorf("GetVINForToken('demo-token-001'): want 'VIN12345', got %q", vin)
	}

	vin, ok = cfg.GetVINForToken("unknown-token")
	if ok {
		t.Error("GetVINForToken('unknown-token'): want ok=false, got true")
	}
	if vin != "" {
		t.Errorf("GetVINForToken('unknown-token'): want '', got %q", vin)
	}
}

// TestConfigFileMissing verifies that LoadConfig returns a non-nil error when the
// specified file does not exist.
// TS-06-E7
func TestConfigFileMissing(t *testing.T) {
	_, err := config.LoadConfig("/nonexistent/path/to/config.json")
	if err == nil {
		t.Error("LoadConfig with missing file: want non-nil error, got nil")
	}
}

// TestConfigFileInvalidJSON verifies that LoadConfig returns a non-nil error when the
// file contains malformed JSON.
// TS-06-E8
func TestConfigFileInvalidJSON(t *testing.T) {
	f, err := os.CreateTemp(t.TempDir(), "invalid-*.json")
	if err != nil {
		t.Fatalf("CreateTemp: %v", err)
	}
	if _, err := f.WriteString("{invalid json"); err != nil {
		t.Fatalf("WriteString: %v", err)
	}
	f.Close()

	_, err = config.LoadConfig(f.Name())
	if err == nil {
		t.Error("LoadConfig with invalid JSON: want non-nil error, got nil")
	}
}
