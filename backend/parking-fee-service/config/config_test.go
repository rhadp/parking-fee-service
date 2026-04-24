package config

import (
	"os"
	"path/filepath"
	"testing"
)

// ---------------------------------------------------------------------------
// TS-05-9: Config Loading from File
// ---------------------------------------------------------------------------

// TestLoadConfigFromFile verifies that LoadConfig reads and parses a valid
// JSON configuration file.
func TestLoadConfigFromFile(t *testing.T) {
	content := `{
		"port": 9090,
		"proximity_threshold_meters": 300,
		"zones": [
			{
				"id": "z1",
				"name": "Zone 1",
				"polygon": [
					{"lat": 0, "lon": 0},
					{"lat": 0, "lon": 1},
					{"lat": 1, "lon": 0}
				]
			}
		],
		"operators": [
			{
				"id": "op1",
				"name": "Op 1",
				"zone_id": "z1",
				"rate": {"type": "per-hour", "amount": 2.0, "currency": "EUR"},
				"adapter": {
					"image_ref": "registry/op1:v1",
					"checksum_sha256": "sha256:abc",
					"version": "1.0.0"
				}
			}
		]
	}`

	dir := t.TempDir()
	path := filepath.Join(dir, "config.json")
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		t.Fatal(err)
	}

	cfg, err := LoadConfig(path)
	if err != nil {
		t.Fatalf("LoadConfig(%q) returned error: %v", path, err)
	}
	if cfg.Port != 9090 {
		t.Errorf("cfg.Port = %d, want 9090", cfg.Port)
	}
	if len(cfg.Zones) != 1 {
		t.Errorf("len(cfg.Zones) = %d, want 1", len(cfg.Zones))
	}
	if len(cfg.Operators) != 1 {
		t.Errorf("len(cfg.Operators) = %d, want 1", len(cfg.Operators))
	}
}

// ---------------------------------------------------------------------------
// TS-05-10: Config Structure Validation
// ---------------------------------------------------------------------------

// TestConfigStructureValidation verifies that the loaded config includes
// proximity threshold, port, zones with polygons, and operators with adapter
// metadata.
func TestConfigStructureValidation(t *testing.T) {
	content := `{
		"port": 8080,
		"proximity_threshold_meters": 500,
		"zones": [
			{
				"id": "z1",
				"name": "Zone 1",
				"polygon": [
					{"lat": 48.14, "lon": 11.555},
					{"lat": 48.14, "lon": 11.565},
					{"lat": 48.135, "lon": 11.555}
				]
			}
		],
		"operators": [
			{
				"id": "op1",
				"name": "Op 1",
				"zone_id": "z1",
				"rate": {"type": "per-hour", "amount": 2.50, "currency": "EUR"},
				"adapter": {
					"image_ref": "us-docker.pkg.dev/demo/adapter:v1",
					"checksum_sha256": "sha256:abc123",
					"version": "1.0.0"
				}
			}
		]
	}`

	dir := t.TempDir()
	path := filepath.Join(dir, "config.json")
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		t.Fatal(err)
	}

	cfg, err := LoadConfig(path)
	if err != nil {
		t.Fatalf("LoadConfig(%q) returned error: %v", path, err)
	}
	if cfg.Port <= 0 {
		t.Error("cfg.Port should be > 0")
	}
	if cfg.ProximityThreshold <= 0 {
		t.Error("cfg.ProximityThreshold should be > 0")
	}
	if len(cfg.Zones) == 0 {
		t.Fatal("cfg.Zones should not be empty")
	}
	if len(cfg.Zones[0].Polygon) < 3 {
		t.Errorf("zone polygon should have at least 3 vertices, got %d",
			len(cfg.Zones[0].Polygon))
	}
	if len(cfg.Operators) == 0 {
		t.Fatal("cfg.Operators should not be empty")
	}
	if cfg.Operators[0].Adapter.ImageRef == "" {
		t.Error("operator adapter image_ref should not be empty")
	}
}

// ---------------------------------------------------------------------------
// TS-05-E5: Config File Missing Defaults
// ---------------------------------------------------------------------------

// TestConfigFileMissingDefaults verifies that LoadConfig returns the
// built-in default configuration when the file does not exist.
func TestConfigFileMissingDefaults(t *testing.T) {
	cfg, err := LoadConfig("/nonexistent/path/config.json")
	if err != nil {
		t.Fatalf("LoadConfig with missing file returned error: %v", err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig returned nil config for missing file")
	}
	if len(cfg.Zones) < 1 {
		t.Error("default config should have at least 1 zone")
	}
	if len(cfg.Operators) < 1 {
		t.Error("default config should have at least 1 operator")
	}
	if cfg.Port != 8080 {
		t.Errorf("default config port = %d, want 8080", cfg.Port)
	}
	if cfg.ProximityThreshold != 500.0 {
		t.Errorf("default config proximity threshold = %f, want 500.0",
			cfg.ProximityThreshold)
	}
}

// ---------------------------------------------------------------------------
// TS-05-E6: Invalid JSON Config
// ---------------------------------------------------------------------------

// TestConfigInvalidJSON verifies that LoadConfig returns an error when the
// config file contains invalid JSON.
func TestConfigInvalidJSON(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "invalid-config.json")
	if err := os.WriteFile(path, []byte("{invalid json"), 0644); err != nil {
		t.Fatal(err)
	}

	_, err := LoadConfig(path)
	if err == nil {
		t.Error("LoadConfig with invalid JSON should return error")
	}
}

// ---------------------------------------------------------------------------
// TS-05-P6: Property – Config Defaults
// ---------------------------------------------------------------------------

// TestPropertyConfigDefaults verifies that for any nonexistent file path,
// LoadConfig returns a valid default configuration.
func TestPropertyConfigDefaults(t *testing.T) {
	paths := []string{
		"/tmp/nonexistent-abc123-test/config.json",
		"/tmp/nonexistent-def456-test/config.json",
		"/tmp/nonexistent-ghi789-test/settings.json",
		"/does/not/exist/anywhere/config.json",
		"/var/missing/parking-fee-service/cfg.json",
	}

	for _, path := range paths {
		t.Run(path, func(t *testing.T) {
			cfg, err := LoadConfig(path)
			if err != nil {
				t.Fatalf("LoadConfig(%q) returned error: %v", path, err)
			}
			if cfg == nil {
				t.Fatal("LoadConfig returned nil config")
			}
			if len(cfg.Zones) < 1 {
				t.Error("default config should have at least 1 zone")
			}
			if len(cfg.Operators) < 1 {
				t.Error("default config should have at least 1 operator")
			}
			if cfg.Port <= 0 {
				t.Error("default config port should be > 0")
			}
			if cfg.ProximityThreshold <= 0 {
				t.Error("default config proximity threshold should be > 0")
			}
		})
	}
}
