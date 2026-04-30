package config

import (
	"os"
	"path/filepath"
	"testing"
)

// TS-05-9: LoadConfig reads configuration from the specified file path.
func TestLoadConfigFromFile(t *testing.T) {
	content := `{
		"port": 9090,
		"proximity_threshold_meters": 200,
		"zones": [
			{
				"id": "test-zone",
				"name": "Test Zone",
				"polygon": [
					{"lat": 48.14, "lon": 11.55},
					{"lat": 48.14, "lon": 11.56},
					{"lat": 48.13, "lon": 11.56},
					{"lat": 48.13, "lon": 11.55}
				]
			}
		],
		"operators": [
			{
				"id": "test-op",
				"name": "Test Operator",
				"zone_id": "test-zone",
				"rate": {"type": "per-hour", "amount": 3.00, "currency": "EUR"},
				"adapter": {
					"image_ref": "registry/repo:v1",
					"checksum_sha256": "sha256:abc123",
					"version": "1.0.0"
				}
			}
		]
	}`

	tmpDir := t.TempDir()
	cfgPath := filepath.Join(tmpDir, "test-config.json")
	if err := os.WriteFile(cfgPath, []byte(content), 0644); err != nil {
		t.Fatalf("failed to write test config file: %v", err)
	}

	cfg, err := LoadConfig(cfgPath)
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
	}
	if cfg.Port != 9090 {
		t.Errorf("expected port 9090, got %d", cfg.Port)
	}
	if len(cfg.Zones) != 1 {
		t.Errorf("expected 1 zone, got %d", len(cfg.Zones))
	}
	if len(cfg.Operators) != 1 {
		t.Errorf("expected 1 operator, got %d", len(cfg.Operators))
	}
}

// TS-05-10: The loaded configuration includes proximity threshold, port, zones
// with polygons, and operators with adapter metadata.
func TestConfigStructureValidation(t *testing.T) {
	content := `{
		"port": 8080,
		"proximity_threshold_meters": 500,
		"zones": [
			{
				"id": "zone-1",
				"name": "Zone One",
				"polygon": [
					{"lat": 48.14, "lon": 11.55},
					{"lat": 48.14, "lon": 11.56},
					{"lat": 48.13, "lon": 11.56},
					{"lat": 48.13, "lon": 11.55}
				]
			}
		],
		"operators": [
			{
				"id": "op-1",
				"name": "Operator One",
				"zone_id": "zone-1",
				"rate": {"type": "per-hour", "amount": 2.50, "currency": "EUR"},
				"adapter": {
					"image_ref": "registry/repo:v1",
					"checksum_sha256": "sha256:abc",
					"version": "1.0.0"
				}
			}
		]
	}`

	tmpDir := t.TempDir()
	cfgPath := filepath.Join(tmpDir, "config.json")
	if err := os.WriteFile(cfgPath, []byte(content), 0644); err != nil {
		t.Fatalf("failed to write config file: %v", err)
	}

	cfg, err := LoadConfig(cfgPath)
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
	}
	if cfg.Port <= 0 {
		t.Error("expected port > 0")
	}
	if cfg.ProximityThreshold <= 0 {
		t.Error("expected proximity threshold > 0")
	}
	if len(cfg.Zones) == 0 {
		t.Fatal("expected at least one zone")
	}
	if len(cfg.Zones[0].Polygon) < 3 {
		t.Error("expected zone polygon to have at least 3 coordinates")
	}
	if len(cfg.Operators) == 0 {
		t.Fatal("expected at least one operator")
	}
	if cfg.Operators[0].Adapter.ImageRef == "" {
		t.Error("expected operator adapter image_ref to be non-empty")
	}
}

// TS-05-E5: When config file does not exist, LoadConfig returns default
// configuration with Munich demo data.
func TestConfigFileMissingDefaults(t *testing.T) {
	cfg, err := LoadConfig("/nonexistent/path/config.json")
	if err != nil {
		t.Fatalf("expected no error for missing config file, got: %v", err)
	}
	if len(cfg.Zones) < 1 {
		t.Error("expected at least 1 zone in default config")
	}
	if len(cfg.Operators) < 1 {
		t.Error("expected at least 1 operator in default config")
	}
	if cfg.Port != 8080 {
		t.Errorf("expected default port 8080, got %d", cfg.Port)
	}
	if cfg.ProximityThreshold != 500.0 {
		t.Errorf("expected default proximity threshold 500.0, got %f", cfg.ProximityThreshold)
	}
}

// TS-05-E6: When config file contains invalid JSON, LoadConfig returns an error.
func TestConfigInvalidJSON(t *testing.T) {
	tmpDir := t.TempDir()
	cfgPath := filepath.Join(tmpDir, "invalid-config.json")
	if err := os.WriteFile(cfgPath, []byte("{invalid json"), 0644); err != nil {
		t.Fatalf("failed to write invalid config file: %v", err)
	}

	_, err := LoadConfig(cfgPath)
	if err == nil {
		t.Error("expected error for invalid JSON config file, got nil")
	}
}

// TS-05-P6: Property test — for any missing or nonexistent config file path,
// LoadConfig returns a valid default configuration.
func TestPropertyConfigDefaults(t *testing.T) {
	paths := []string{
		"/nonexistent/path/config.json",
		"/tmp/does-not-exist-" + t.Name() + ".json",
		"",
		"/a/b/c/d/e/f.json",
		"no-such-file.json",
	}

	for _, path := range paths {
		t.Run(path, func(t *testing.T) {
			cfg, err := LoadConfig(path)
			if err != nil {
				t.Fatalf("expected no error for missing config at %q, got: %v", path, err)
			}
			if cfg == nil {
				t.Fatal("expected non-nil config")
			}
			if len(cfg.Zones) < 1 {
				t.Error("expected at least 1 zone in default config")
			}
			if len(cfg.Operators) < 1 {
				t.Error("expected at least 1 operator in default config")
			}
			if cfg.Port <= 0 {
				t.Error("expected port > 0")
			}
			if cfg.ProximityThreshold <= 0 {
				t.Error("expected proximity threshold > 0")
			}
		})
	}
}
