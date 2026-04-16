package config_test

import (
	"encoding/json"
	"os"
	"testing"

	"parking-fee-service/backend/parking-fee-service/config"
)

// TestLoadConfigFromFile verifies that LoadConfig reads values from a JSON
// file (TS-05-9).
func TestLoadConfigFromFile(t *testing.T) {
	const data = `{
		"port": 9090,
		"proximity_threshold_meters": 300,
		"zones": [
			{
				"id": "test-zone",
				"name": "Test Zone",
				"polygon": [
					{"lat": 10.0, "lon": 20.0},
					{"lat": 10.0, "lon": 21.0},
					{"lat": 11.0, "lon": 21.0},
					{"lat": 11.0, "lon": 20.0}
				]
			}
		],
		"operators": [
			{
				"id": "test-op",
				"name": "Test Operator",
				"zone_id": "test-zone",
				"rate": {"type": "per-hour", "amount": 1.50, "currency": "EUR"},
				"adapter": {
					"image_ref": "example.com/test-op:v1",
					"checksum_sha256": "sha256:aabbcc",
					"version": "1.0.0"
				}
			}
		]
	}`

	f, err := os.CreateTemp("", "test-config-*.json")
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(f.Name())
	if _, err := f.WriteString(data); err != nil {
		t.Fatal(err)
	}
	f.Close()

	cfg, err := config.LoadConfig(f.Name())
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
	}
	if cfg.Port != 9090 {
		t.Errorf("Port: want 9090, got %d", cfg.Port)
	}
	if len(cfg.Zones) != 1 {
		t.Errorf("Zones: want 1, got %d", len(cfg.Zones))
	}
	if len(cfg.Operators) != 1 {
		t.Errorf("Operators: want 1, got %d", len(cfg.Operators))
	}
}

// TestConfigStructureValidation verifies that a loaded config has all required
// structural fields (TS-05-10).
func TestConfigStructureValidation(t *testing.T) {
	const data = `{
		"port": 8080,
		"proximity_threshold_meters": 500,
		"zones": [
			{
				"id": "z1",
				"name": "Zone One",
				"polygon": [
					{"lat": 1.0, "lon": 1.0},
					{"lat": 1.0, "lon": 2.0},
					{"lat": 2.0, "lon": 2.0}
				]
			}
		],
		"operators": [
			{
				"id": "op1",
				"name": "Operator One",
				"zone_id": "z1",
				"rate": {"type": "flat-fee", "amount": 5.0, "currency": "EUR"},
				"adapter": {
					"image_ref": "example.com/op1:v1",
					"checksum_sha256": "sha256:deadbeef",
					"version": "1.0.0"
				}
			}
		]
	}`

	f, err := os.CreateTemp("", "test-config-*.json")
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(f.Name())
	if _, err := f.WriteString(data); err != nil {
		t.Fatal(err)
	}
	f.Close()

	cfg, err := config.LoadConfig(f.Name())
	if err != nil {
		t.Fatalf("LoadConfig error: %v", err)
	}
	if cfg.Port <= 0 {
		t.Errorf("Port must be > 0, got %d", cfg.Port)
	}
	if cfg.ProximityThreshold <= 0 {
		t.Errorf("ProximityThreshold must be > 0, got %f", cfg.ProximityThreshold)
	}
	if len(cfg.Zones) == 0 {
		t.Error("expected at least one zone")
	}
	if len(cfg.Zones[0].Polygon) < 3 {
		t.Errorf("polygon must have >= 3 vertices, got %d", len(cfg.Zones[0].Polygon))
	}
	if cfg.Operators[0].Adapter.ImageRef == "" {
		t.Error("adapter ImageRef must not be empty")
	}
}

// TestConfigFileMissingDefaults verifies that LoadConfig falls back to default
// Munich demo data when the file does not exist (TS-05-E5).
func TestConfigFileMissingDefaults(t *testing.T) {
	cfg, err := config.LoadConfig("/nonexistent/path/config.json")
	if err != nil {
		t.Fatalf("LoadConfig with missing file should not error, got: %v", err)
	}
	if len(cfg.Zones) < 1 {
		t.Error("default config must have at least one zone")
	}
	if len(cfg.Operators) < 1 {
		t.Error("default config must have at least one operator")
	}
	if cfg.Port != 8080 {
		t.Errorf("default port: want 8080, got %d", cfg.Port)
	}
	if cfg.ProximityThreshold != 500.0 {
		t.Errorf("default proximity threshold: want 500.0, got %f", cfg.ProximityThreshold)
	}
}

// TestConfigInvalidJSON verifies that LoadConfig returns an error when the
// file contains invalid JSON (TS-05-E6).
func TestConfigInvalidJSON(t *testing.T) {
	f, err := os.CreateTemp("", "bad-config-*.json")
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
		t.Error("LoadConfig with invalid JSON should return an error")
	}
}

// TestPropertyConfigDefaults verifies that for any nonexistent path, LoadConfig
// returns a valid default configuration (TS-05-P6).
func TestPropertyConfigDefaults(t *testing.T) {
	paths := []string{
		"/nonexistent/a.json",
		"/tmp/definitely-not-there-12345.json",
		"/no/such/file",
	}
	for _, p := range paths {
		cfg, err := config.LoadConfig(p)
		if err != nil {
			t.Errorf("path %q: LoadConfig returned error: %v", p, err)
			continue
		}
		if cfg == nil {
			t.Errorf("path %q: LoadConfig returned nil config", p)
			continue
		}
		if len(cfg.Zones) < 1 {
			t.Errorf("path %q: default config has no zones", p)
		}
		if len(cfg.Operators) < 1 {
			t.Errorf("path %q: default config has no operators", p)
		}
		if cfg.Port <= 0 {
			t.Errorf("path %q: default config port <= 0", p)
		}
		if cfg.ProximityThreshold <= 0 {
			t.Errorf("path %q: default config proximity threshold <= 0", p)
		}
		// Verify operators have complete adapter metadata
		for _, op := range cfg.Operators {
			if op.Adapter.ImageRef == "" {
				t.Errorf("path %q: operator %q has empty image_ref", p, op.ID)
			}
		}
	}
}

// TestDefaultConfigJSON verifies DefaultConfig() can be marshalled to JSON
// without error (sanity check).
func TestDefaultConfigJSON(t *testing.T) {
	cfg := config.DefaultConfig()
	if cfg == nil {
		t.Fatal("DefaultConfig returned nil")
	}
	b, err := json.Marshal(cfg)
	if err != nil {
		t.Fatalf("json.Marshal(DefaultConfig()): %v", err)
	}
	if len(b) == 0 {
		t.Error("marshalled config is empty")
	}
}
