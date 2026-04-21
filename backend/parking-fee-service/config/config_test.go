// Package config_test contains tests for the config package.
package config_test

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/config"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

// writeConfigFile creates a temporary config file and returns its path.
func writeConfigFile(t *testing.T, cfg model.Config) string {
	t.Helper()
	data, err := json.Marshal(cfg)
	if err != nil {
		t.Fatalf("failed to marshal config: %v", err)
	}
	tmpDir := t.TempDir()
	path := filepath.Join(tmpDir, "config.json")
	if err := os.WriteFile(path, data, 0o600); err != nil {
		t.Fatalf("failed to write config file: %v", err)
	}
	return path
}

// TestLoadConfigFromFile verifies that LoadConfig reads values from a JSON file.
// TS-05-9
func TestLoadConfigFromFile(t *testing.T) {
	expected := model.Config{
		Port:               9090,
		ProximityThreshold: 300.0,
		Zones: []model.Zone{
			{
				ID:   "test-zone",
				Name: "Test Zone",
				Polygon: []model.Coordinate{
					{Lat: 1.0, Lon: 1.0},
					{Lat: 1.0, Lon: 2.0},
					{Lat: 0.0, Lon: 2.0},
					{Lat: 0.0, Lon: 1.0},
				},
			},
		},
		Operators: []model.Operator{
			{
				ID:     "test-op",
				Name:   "Test Operator",
				ZoneID: "test-zone",
				Rate:   model.Rate{Type: "per-hour", Amount: 1.50, Currency: "EUR"},
				Adapter: model.AdapterMeta{
					ImageRef:       "example.com/adapter:v1",
					ChecksumSHA256: "sha256:abc123",
					Version:        "1.0.0",
				},
			},
		},
	}
	path := writeConfigFile(t, expected)

	cfg, err := config.LoadConfig(path)
	if err != nil {
		t.Fatalf("LoadConfig returned unexpected error: %v", err)
	}
	if cfg.Port != 9090 {
		t.Errorf("expected Port=9090, got %d", cfg.Port)
	}
	if len(cfg.Zones) != 1 {
		t.Errorf("expected 1 zone, got %d", len(cfg.Zones))
	}
	if len(cfg.Operators) != 1 {
		t.Errorf("expected 1 operator, got %d", len(cfg.Operators))
	}
}

// TestConfigStructureValidation verifies that all required config fields are present
// after loading a complete config file.
// TS-05-10
func TestConfigStructureValidation(t *testing.T) {
	cfg := model.Config{
		Port:               8080,
		ProximityThreshold: 500.0,
		Zones: []model.Zone{
			{
				ID:   "zone-a",
				Name: "Zone A",
				Polygon: []model.Coordinate{
					{Lat: 48.14, Lon: 11.555},
					{Lat: 48.14, Lon: 11.565},
					{Lat: 48.135, Lon: 11.565},
					{Lat: 48.135, Lon: 11.555},
				},
			},
		},
		Operators: []model.Operator{
			{
				ID:     "op-a",
				Name:   "Operator A",
				ZoneID: "zone-a",
				Rate:   model.Rate{Type: "per-hour", Amount: 2.50, Currency: "EUR"},
				Adapter: model.AdapterMeta{
					ImageRef:       "us-docker.pkg.dev/project/adapters/op-a:v1.0",
					ChecksumSHA256: "sha256:deadbeef",
					Version:        "1.0.0",
				},
			},
		},
	}
	path := writeConfigFile(t, cfg)

	loaded, err := config.LoadConfig(path)
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
	}
	if loaded.Port <= 0 {
		t.Errorf("expected Port > 0, got %d", loaded.Port)
	}
	if loaded.ProximityThreshold <= 0 {
		t.Errorf("expected ProximityThreshold > 0, got %f", loaded.ProximityThreshold)
	}
	if len(loaded.Zones) == 0 {
		t.Errorf("expected at least one zone")
	}
	if len(loaded.Zones[0].Polygon) < 3 {
		t.Errorf("expected polygon with at least 3 vertices, got %d", len(loaded.Zones[0].Polygon))
	}
	if len(loaded.Operators) == 0 {
		t.Errorf("expected at least one operator")
	}
	if loaded.Operators[0].Adapter.ImageRef == "" {
		t.Errorf("expected non-empty adapter image_ref")
	}
}

// TestConfigFileMissingDefaults verifies that LoadConfig returns default Munich demo
// data when the config file does not exist.
// TS-05-E5
func TestConfigFileMissingDefaults(t *testing.T) {
	cfg, err := config.LoadConfig("/nonexistent/path/config.json")
	if err != nil {
		t.Fatalf("expected no error for missing config file, got: %v", err)
	}
	if len(cfg.Zones) < 1 {
		t.Errorf("expected at least one zone in default config, got 0")
	}
	if len(cfg.Operators) < 1 {
		t.Errorf("expected at least one operator in default config, got 0")
	}
	if cfg.Port != 8080 {
		t.Errorf("expected default port 8080, got %d", cfg.Port)
	}
	if cfg.ProximityThreshold != 500.0 {
		t.Errorf("expected default proximity threshold 500.0, got %f", cfg.ProximityThreshold)
	}
}

// TestConfigInvalidJSON verifies that LoadConfig returns an error for malformed JSON.
// TS-05-E6
func TestConfigInvalidJSON(t *testing.T) {
	tmpDir := t.TempDir()
	path := filepath.Join(tmpDir, "bad-config.json")
	if err := os.WriteFile(path, []byte("{invalid json"), 0o600); err != nil {
		t.Fatalf("failed to write bad config file: %v", err)
	}

	_, err := config.LoadConfig(path)
	if err == nil {
		t.Error("expected error for invalid JSON config, got nil")
	}
}

// TestPropertyConfigDefaults is a property test verifying that any nonexistent
// config path yields a valid default configuration.
// TS-05-P6
func TestPropertyConfigDefaults(t *testing.T) {
	// Try multiple nonexistent paths — all must return valid defaults.
	nonexistentPaths := []string{
		"/nonexistent/a.json",
		"/tmp/__no_such_file_abc123__.json",
		"/var/no/such/path/config.json",
	}
	for _, path := range nonexistentPaths {
		t.Run(path, func(t *testing.T) {
			// Ensure the file truly does not exist.
			if _, err := os.Stat(path); err == nil {
				t.Skipf("file %s unexpectedly exists; skipping", path)
			}

			cfg, err := config.LoadConfig(path)
			if err != nil {
				t.Errorf("LoadConfig(%q) returned error for nonexistent path: %v", path, err)
				return
			}
			if len(cfg.Zones) < 1 {
				t.Errorf("LoadConfig(%q) returned config with no zones", path)
			}
			if len(cfg.Operators) < 1 {
				t.Errorf("LoadConfig(%q) returned config with no operators", path)
			}
			if cfg.Port <= 0 {
				t.Errorf("LoadConfig(%q) returned config with invalid port %d", path, cfg.Port)
			}
			if cfg.ProximityThreshold <= 0 {
				t.Errorf("LoadConfig(%q) returned config with invalid threshold %f", path, cfg.ProximityThreshold)
			}
		})
	}
}
