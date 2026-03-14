package config_test

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/config"
)

// writeTempConfig writes content to a temporary JSON file and returns the path.
func writeTempConfig(t *testing.T, content string) string {
	t.Helper()
	_ = json.Valid([]byte(content))
	dir := t.TempDir()
	path := filepath.Join(dir, "config.json")
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("writeTempConfig: %v", err)
	}
	return path
}

// TS-06-18: Config File Loading
// Configuration is loaded from the specified file path.
func TestLoadConfigFromFile(t *testing.T) {
	tmp := writeTempConfig(t, `{
		"port": 9090,
		"nats_url": "nats://example.com:4222",
		"command_timeout_seconds": 45,
		"tokens": [{"token": "tok1", "vin": "VIN1"}]
	}`)

	cfg, err := config.LoadConfig(tmp)
	if err != nil {
		t.Fatalf("LoadConfig(%q) returned error: %v", tmp, err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig returned nil config")
	}
	if cfg.Port != 9090 {
		t.Errorf("cfg.Port = %d, want 9090", cfg.Port)
	}
}

// TS-06-19: Config Fields
// Config includes port, NATS URL, command timeout, and token mappings.
func TestConfigFields(t *testing.T) {
	tmp := writeTempConfig(t, `{
		"port": 8081,
		"nats_url": "nats://localhost:4222",
		"command_timeout_seconds": 30,
		"tokens": [{"token": "abc", "vin": "VIN1"}]
	}`)

	cfg, err := config.LoadConfig(tmp)
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig returned nil config")
	}
	if cfg.Port <= 0 {
		t.Errorf("cfg.Port = %d, want > 0", cfg.Port)
	}
	if cfg.NatsURL == "" {
		t.Error("cfg.NatsURL is empty, want non-empty")
	}
	if cfg.CommandTimeout <= 0 {
		t.Errorf("cfg.CommandTimeout = %d, want > 0", cfg.CommandTimeout)
	}
	if len(cfg.Tokens) == 0 {
		t.Error("cfg.Tokens is empty, want at least one token")
	}
}

// TS-06-20: Config Defaults
// Missing config fields use defaults: port 8081, NATS nats://localhost:4222, timeout 30.
func TestConfigDefaults(t *testing.T) {
	tmp := writeTempConfig(t, `{}`)

	cfg, err := config.LoadConfig(tmp)
	if err != nil {
		t.Fatalf("LoadConfig(empty) returned error: %v", err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig(empty) returned nil config")
	}
	if cfg.Port != 8081 {
		t.Errorf("cfg.Port = %d, want 8081", cfg.Port)
	}
	if cfg.NatsURL != "nats://localhost:4222" {
		t.Errorf("cfg.NatsURL = %q, want %q", cfg.NatsURL, "nats://localhost:4222")
	}
	if cfg.CommandTimeout != 30 {
		t.Errorf("cfg.CommandTimeout = %d, want 30", cfg.CommandTimeout)
	}
}

// TS-06-E10: Config File Missing
// Missing config file returns default config with no error.
func TestConfigFileMissing(t *testing.T) {
	cfg, err := config.LoadConfig("/nonexistent/config.json")
	if err != nil {
		t.Fatalf("LoadConfig(nonexistent) returned error: %v", err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig(nonexistent) returned nil config")
	}
	if cfg.Port != 8081 {
		t.Errorf("cfg.Port = %d, want 8081", cfg.Port)
	}
	if cfg.NatsURL != "nats://localhost:4222" {
		t.Errorf("cfg.NatsURL = %q, want %q", cfg.NatsURL, "nats://localhost:4222")
	}
}

// TS-06-E11: Config File Invalid JSON
// Invalid JSON config file returns an error.
func TestConfigInvalidJSON(t *testing.T) {
	tmp := writeTempConfig(t, `{invalid`)

	_, err := config.LoadConfig(tmp)
	if err == nil {
		t.Error("LoadConfig(invalid JSON) returned nil error, want error")
	}
}

// TS-06-16: Token-VIN Loading from Config
// Token-to-VIN mappings are loaded from the JSON config file.
func TestTokenVINLoading(t *testing.T) {
	tmp := writeTempConfig(t, `{
		"tokens": [{"token": "abc", "vin": "VIN1"}]
	}`)

	cfg, err := config.LoadConfig(tmp)
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig returned nil config")
	}
	if len(cfg.Tokens) != 1 {
		t.Fatalf("len(cfg.Tokens) = %d, want 1", len(cfg.Tokens))
	}
	if cfg.Tokens[0].Token != "abc" {
		t.Errorf("Tokens[0].Token = %q, want %q", cfg.Tokens[0].Token, "abc")
	}
	if cfg.Tokens[0].VIN != "VIN1" {
		t.Errorf("Tokens[0].VIN = %q, want %q", cfg.Tokens[0].VIN, "VIN1")
	}
}

// TS-06-12: Configurable Timeout
// The timeout duration is loaded from the config file.
func TestConfigurableTimeout(t *testing.T) {
	tmp := writeTempConfig(t, `{"command_timeout_seconds": 60}`)

	cfg, err := config.LoadConfig(tmp)
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig returned nil config")
	}
	if cfg.CommandTimeout != 60 {
		t.Errorf("cfg.CommandTimeout = %d, want 60", cfg.CommandTimeout)
	}
}

// TS-06-P6: Config Defaults Property
// For any missing config file, defaults are applied.
func TestPropertyConfigDefaults(t *testing.T) {
	paths := []string{
		"/nonexistent/a/b/c.json",
		"/tmp/does-not-exist-99999.json",
		"/var/missing/config.json",
	}
	for _, p := range paths {
		t.Run(p, func(t *testing.T) {
			cfg, err := config.LoadConfig(p)
			if err != nil {
				t.Fatalf("LoadConfig(%q) returned error: %v", p, err)
			}
			if cfg == nil {
				t.Fatal("LoadConfig returned nil config")
			}
			if cfg.Port != 8081 {
				t.Errorf("cfg.Port = %d, want 8081", cfg.Port)
			}
			if cfg.NatsURL != "nats://localhost:4222" {
				t.Errorf("cfg.NatsURL = %q, want %q", cfg.NatsURL, "nats://localhost:4222")
			}
			if cfg.CommandTimeout != 30 {
				t.Errorf("cfg.CommandTimeout = %d, want 30", cfg.CommandTimeout)
			}
		})
	}
}
