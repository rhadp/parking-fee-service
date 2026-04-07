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
					{"lat": 10.0, "lon": 20.0},
					{"lat": 10.0, "lon": 20.1},
					{"lat": 10.1, "lon": 20.1},
					{"lat": 10.1, "lon": 20.0}
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
					"image_ref": "registry/test:v1",
					"checksum_sha256": "sha256:test123",
					"version": "1.0.0"
				}
			}
		]
	}`

	dir := t.TempDir()
	path := filepath.Join(dir, "test-config.json")
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		t.Fatalf("failed to write test config: %v", err)
	}

	cfg, err := LoadConfig(path)
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig returned nil config")
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

// TS-05-10: Configuration includes all required fields.
func TestConfigStructureValidation(t *testing.T) {
	content := `{
		"port": 8080,
		"proximity_threshold_meters": 500,
		"zones": [
			{
				"id": "z1",
				"name": "Zone One",
				"polygon": [
					{"lat": 1.0, "lon": 2.0},
					{"lat": 1.0, "lon": 3.0},
					{"lat": 2.0, "lon": 3.0},
					{"lat": 2.0, "lon": 2.0}
				]
			}
		],
		"operators": [
			{
				"id": "op1",
				"name": "Operator One",
				"zone_id": "z1",
				"rate": {"type": "flat-fee", "amount": 5.00, "currency": "EUR"},
				"adapter": {
					"image_ref": "registry/op1:v1",
					"checksum_sha256": "sha256:abc",
					"version": "2.0.0"
				}
			}
		]
	}`

	dir := t.TempDir()
	path := filepath.Join(dir, "config.json")
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		t.Fatalf("failed to write config: %v", err)
	}

	cfg, err := LoadConfig(path)
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig returned nil config")
	}
	if cfg.Port <= 0 {
		t.Errorf("expected positive port, got %d", cfg.Port)
	}
	if cfg.ProximityThreshold <= 0 {
		t.Errorf("expected positive proximity threshold, got %f", cfg.ProximityThreshold)
	}
	if len(cfg.Zones) == 0 {
		t.Fatal("expected at least one zone")
	}
	if len(cfg.Zones[0].Polygon) < 3 {
		t.Errorf("expected polygon with >= 3 vertices, got %d", len(cfg.Zones[0].Polygon))
	}
	if cfg.Operators[0].Adapter.ImageRef == "" {
		t.Error("expected non-empty adapter image_ref")
	}
}

// TS-05-E5: When config file does not exist, returns default configuration.
func TestConfigFileMissingDefaults(t *testing.T) {
	cfg, err := LoadConfig("/nonexistent/path/config.json")
	if err != nil {
		t.Fatalf("expected no error for missing config, got: %v", err)
	}
	if cfg == nil {
		t.Fatal("expected non-nil default config")
	}
	if len(cfg.Zones) < 1 {
		t.Errorf("expected at least 1 default zone, got %d", len(cfg.Zones))
	}
	if len(cfg.Operators) < 1 {
		t.Errorf("expected at least 1 default operator, got %d", len(cfg.Operators))
	}
	if cfg.Port != 8080 {
		t.Errorf("expected default port 8080, got %d", cfg.Port)
	}
	if cfg.ProximityThreshold != 500.0 {
		t.Errorf("expected default proximity threshold 500.0, got %f", cfg.ProximityThreshold)
	}
}

// TS-05-E6: Invalid JSON config returns error.
func TestConfigInvalidJSON(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "bad-config.json")
	if err := os.WriteFile(path, []byte("{invalid json"), 0644); err != nil {
		t.Fatalf("failed to write file: %v", err)
	}

	_, err := LoadConfig(path)
	if err == nil {
		t.Error("expected error for invalid JSON, got nil")
	}
}

// TS-05-P6: Property test — any nonexistent path returns valid defaults.
func TestPropertyConfigDefaults(t *testing.T) {
	paths := []string{
		"/nonexistent/a/b/c.json",
		"/tmp/does-not-exist-12345.json",
		"/no/such/file.json",
		"relative/missing.json",
	}
	for _, path := range paths {
		cfg, err := LoadConfig(path)
		if err != nil {
			t.Errorf("LoadConfig(%q) returned error: %v", path, err)
			continue
		}
		if cfg == nil {
			t.Errorf("LoadConfig(%q) returned nil config", path)
			continue
		}
		if len(cfg.Zones) < 1 {
			t.Errorf("LoadConfig(%q): expected >= 1 zone, got %d", path, len(cfg.Zones))
		}
		if len(cfg.Operators) < 1 {
			t.Errorf("LoadConfig(%q): expected >= 1 operator, got %d", path, len(cfg.Operators))
		}
		if cfg.Port <= 0 {
			t.Errorf("LoadConfig(%q): expected positive port, got %d", path, cfg.Port)
		}
		if cfg.ProximityThreshold <= 0 {
			t.Errorf("LoadConfig(%q): expected positive proximity threshold, got %f", path, cfg.ProximityThreshold)
		}
	}
}
