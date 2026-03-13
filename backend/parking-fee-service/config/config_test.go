package config_test

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/config"
)

// TS-05-9: LoadConfig reads configuration from the specified file path.
func TestLoadConfigFromFile(t *testing.T) {
	// Write a temporary config file with a custom port and one zone/operator.
	tmp := writeTempConfig(t, `{
		"port": 9090,
		"proximity_threshold_meters": 300,
		"zones": [
			{
				"id": "test-zone",
				"name": "Test Zone",
				"polygon": [
					{"lat": 1.0, "lon": 1.0},
					{"lat": 1.0, "lon": 2.0},
					{"lat": 2.0, "lon": 2.0},
					{"lat": 2.0, "lon": 1.0}
				]
			}
		],
		"operators": [
			{
				"id": "test-op",
				"name": "Test Operator",
				"zone_id": "test-zone",
				"rate": {"type": "per-hour", "amount": 1.0, "currency": "EUR"},
				"adapter": {
					"image_ref": "registry/repo/adapter:v1",
					"checksum_sha256": "sha256:aabbcc",
					"version": "1.0.0"
				}
			}
		]
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
	if len(cfg.Zones) != 1 {
		t.Errorf("len(cfg.Zones) = %d, want 1", len(cfg.Zones))
	}
	if len(cfg.Operators) != 1 {
		t.Errorf("len(cfg.Operators) = %d, want 1", len(cfg.Operators))
	}
}

// TS-05-10: The loaded configuration includes proximity threshold, port, zones,
// and operators with adapter metadata.
func TestConfigStructureValidation(t *testing.T) {
	tmp := writeTempConfig(t, `{
		"port": 8080,
		"proximity_threshold_meters": 500,
		"zones": [
			{
				"id": "z1",
				"name": "Zone One",
				"polygon": [
					{"lat": 1.0, "lon": 1.0},
					{"lat": 1.0, "lon": 2.0},
					{"lat": 2.0, "lon": 1.0}
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
					"image_ref": "registry/repo/op1:v1",
					"checksum_sha256": "sha256:deadbeef",
					"version": "1.0.0"
				}
			}
		]
	}`)

	cfg, err := config.LoadConfig(tmp)
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
	}
	if cfg.Port <= 0 {
		t.Errorf("cfg.Port = %d, want > 0", cfg.Port)
	}
	if cfg.ProximityThreshold <= 0 {
		t.Errorf("cfg.ProximityThreshold = %v, want > 0", cfg.ProximityThreshold)
	}
	if len(cfg.Zones) == 0 {
		t.Error("cfg.Zones is empty, want at least one zone")
	}
	if len(cfg.Zones) > 0 && len(cfg.Zones[0].Polygon) < 3 {
		t.Errorf("cfg.Zones[0].Polygon has %d points, want >= 3", len(cfg.Zones[0].Polygon))
	}
	if len(cfg.Operators) == 0 {
		t.Error("cfg.Operators is empty, want at least one operator")
	}
	if len(cfg.Operators) > 0 && cfg.Operators[0].Adapter.ImageRef == "" {
		t.Error("cfg.Operators[0].Adapter.ImageRef is empty")
	}
}

// TS-05-E5: When config file does not exist, LoadConfig returns default Munich config.
func TestConfigFileMissingDefaults(t *testing.T) {
	cfg, err := config.LoadConfig("/nonexistent/path/config.json")
	if err != nil {
		t.Fatalf("LoadConfig with missing file returned error: %v", err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig with missing file returned nil config")
	}
	if cfg.Port != 8080 {
		t.Errorf("default cfg.Port = %d, want 8080", cfg.Port)
	}
	if cfg.ProximityThreshold != 500.0 {
		t.Errorf("default cfg.ProximityThreshold = %v, want 500.0", cfg.ProximityThreshold)
	}
	if len(cfg.Zones) < 1 {
		t.Errorf("default cfg.Zones has %d zones, want >= 1", len(cfg.Zones))
	}
	if len(cfg.Operators) < 1 {
		t.Errorf("default cfg.Operators has %d operators, want >= 1", len(cfg.Operators))
	}
}

// TS-05-E6: When config file contains invalid JSON, LoadConfig returns a non-nil error.
func TestConfigInvalidJSON(t *testing.T) {
	tmp := writeTempConfig(t, `{invalid json`)

	cfg, err := config.LoadConfig(tmp)
	if err == nil {
		t.Errorf("LoadConfig with invalid JSON returned nil error; cfg = %+v", cfg)
	}
}

// TS-05-P6: For any nonexistent config path, LoadConfig returns a valid default config.
func TestPropertyConfigDefaults(t *testing.T) {
	paths := []string{
		"/nonexistent/a/b/c.json",
		"/tmp/does-not-exist-12345.json",
		"",
	}
	for _, p := range paths {
		t.Run(p, func(t *testing.T) {
			// Skip paths that might exist (empty string resolves to cwd config.json).
			if p == "" {
				if _, err := os.Stat("config.json"); err == nil {
					t.Skip("config.json exists in cwd, skipping")
				}
			}
			cfg, err := config.LoadConfig(p)
			if err != nil {
				t.Fatalf("LoadConfig(%q) returned error: %v", p, err)
			}
			if cfg == nil {
				t.Fatal("LoadConfig returned nil config for missing file")
			}
			if len(cfg.Zones) < 1 {
				t.Error("default config has no zones")
			}
			if len(cfg.Operators) < 1 {
				t.Error("default config has no operators")
			}
			if cfg.Port <= 0 {
				t.Errorf("default config port = %d, want > 0", cfg.Port)
			}
			if cfg.ProximityThreshold <= 0 {
				t.Errorf("default config proximity threshold = %v, want > 0", cfg.ProximityThreshold)
			}
		})
	}
}

// writeTempConfig writes content to a temporary JSON file and returns the path.
func writeTempConfig(t *testing.T, content string) string {
	t.Helper()
	// Validate that content is the JSON we intend (invalid JSON is intentional in some tests).
	_ = json.Valid([]byte(content))

	dir := t.TempDir()
	path := filepath.Join(dir, "config.json")
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("writeTempConfig: %v", err)
	}
	return path
}
