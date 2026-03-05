package main

import (
	"os"
	"testing"
)

// TS-05-7: Configuration Loading from File
func TestConfigLoadFromFile(t *testing.T) {
	// Write a valid config to a temp file
	tmpFile, err := os.CreateTemp("", "config-*.json")
	if err != nil {
		t.Fatalf("failed to create temp file: %v", err)
	}
	defer os.Remove(tmpFile.Name())

	configJSON := `{
		"settings": {"port": 9090, "proximity_threshold_meters": 500},
		"zones": [
			{
				"id": "test-zone",
				"name": "Test Zone",
				"polygon": [
					{"lat": 48.14, "lon": 11.55},
					{"lat": 48.14, "lon": 11.57},
					{"lat": 48.13, "lon": 11.57},
					{"lat": 48.13, "lon": 11.55}
				]
			}
		],
		"operators": [
			{
				"operator_id": "test-op",
				"name": "Test Operator",
				"zone_id": "test-zone",
				"rate_type": "per_hour",
				"rate_amount": 3.00,
				"rate_currency": "EUR",
				"adapter": {
					"image_ref": "registry/test:v1",
					"checksum_sha256": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
					"version": "v1.0.0"
				}
			}
		]
	}`

	if _, err := tmpFile.WriteString(configJSON); err != nil {
		t.Fatalf("failed to write config: %v", err)
	}
	tmpFile.Close()

	cfg, err := LoadConfig(tmpFile.Name())
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig returned nil config")
	}
	if cfg.Settings.ProximityThresholdMeters != 500 {
		t.Errorf("expected proximity threshold 500, got %f", cfg.Settings.ProximityThresholdMeters)
	}
	if len(cfg.Zones) < 1 {
		t.Error("expected at least 1 zone")
	}
	if len(cfg.Operators) < 1 {
		t.Error("expected at least 1 operator")
	}
}

// TS-05-7: Configuration Loading Default
func TestConfigLoadDefault(t *testing.T) {
	cfg, err := LoadConfig("")
	if err != nil {
		t.Fatalf("LoadConfig with empty path returned error: %v", err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig returned nil config")
	}
	if cfg.Settings.ProximityThresholdMeters != 500 {
		t.Errorf("expected proximity threshold 500, got %f", cfg.Settings.ProximityThresholdMeters)
	}
	if len(cfg.Zones) < 2 {
		t.Errorf("expected at least 2 zones, got %d", len(cfg.Zones))
	}
	if len(cfg.Operators) < 2 {
		t.Errorf("expected at least 2 operators, got %d", len(cfg.Operators))
	}
}

// TS-05-E5: Invalid Config File Causes Error
func TestConfigLoadInvalidFile(t *testing.T) {
	// Non-existent file
	_, err := LoadConfig("/nonexistent/path.json")
	if err == nil {
		t.Error("expected error for non-existent config file")
	}

	// Invalid JSON
	tmpFile, err := os.CreateTemp("", "invalid-config-*.json")
	if err != nil {
		t.Fatalf("failed to create temp file: %v", err)
	}
	defer os.Remove(tmpFile.Name())

	if _, err := tmpFile.WriteString("{invalid"); err != nil {
		t.Fatalf("failed to write invalid config: %v", err)
	}
	tmpFile.Close()

	_, err = LoadConfig(tmpFile.Name())
	if err == nil {
		t.Error("expected error for invalid JSON config file")
	}
}
