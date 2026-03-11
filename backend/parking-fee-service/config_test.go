package main

import (
	"os"
	"path/filepath"
	"testing"
)

func TestConfigLoadFromFile(t *testing.T) {
	// TS-05-7: Load configuration from a temp JSON file
	tmpDir := t.TempDir()
	cfgPath := filepath.Join(tmpDir, "config.json")

	cfgData := `{
		"settings": {"port": 9090, "proximity_threshold_meters": 250},
		"zones": [{"id": "z1", "name": "Zone 1", "polygon": [{"lat": 1, "lon": 1}, {"lat": 1, "lon": 2}, {"lat": 2, "lon": 2}]}],
		"operators": [{"operator_id": "op1", "name": "Op 1", "zone_id": "z1", "rate_type": "per_hour", "rate_amount": 1.0, "rate_currency": "USD", "adapter": {"image_ref": "img:v1", "checksum_sha256": "sha256:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789", "version": "v1"}}]
	}`
	if err := os.WriteFile(cfgPath, []byte(cfgData), 0644); err != nil {
		t.Fatalf("failed to write temp config: %v", err)
	}

	cfg, err := LoadConfig(cfgPath)
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig returned nil config")
	}
	if cfg.Settings.ProximityThresholdMeters != 250 {
		t.Errorf("expected proximity threshold 250, got %v", cfg.Settings.ProximityThresholdMeters)
	}
	if cfg.Settings.Port != 9090 {
		t.Errorf("expected port 9090, got %v", cfg.Settings.Port)
	}
	if len(cfg.Zones) < 1 {
		t.Errorf("expected at least 1 zone, got %d", len(cfg.Zones))
	}
	if len(cfg.Operators) < 1 {
		t.Errorf("expected at least 1 operator, got %d", len(cfg.Operators))
	}
}

func TestConfigLoadDefault(t *testing.T) {
	// TS-05-7: Load embedded default config when no file is specified
	cfg, err := LoadConfig("")
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig returned nil config")
	}
	if cfg.Settings.ProximityThresholdMeters != 500 {
		t.Errorf("expected proximity threshold 500, got %v", cfg.Settings.ProximityThresholdMeters)
	}
	if len(cfg.Zones) < 2 {
		t.Errorf("expected at least 2 zones, got %d", len(cfg.Zones))
	}
	if len(cfg.Operators) < 2 {
		t.Errorf("expected at least 2 operators, got %d", len(cfg.Operators))
	}
}

func TestConfigLoadInvalidFile(t *testing.T) {
	// TS-05-E5: Non-existent and invalid JSON config files return errors

	// Non-existent file
	_, err := LoadConfig("/nonexistent/path.json")
	if err == nil {
		t.Error("expected error for non-existent config file, got nil")
	}

	// Invalid JSON
	tmpDir := t.TempDir()
	invalidPath := filepath.Join(tmpDir, "invalid.json")
	if err := os.WriteFile(invalidPath, []byte("{invalid"), 0644); err != nil {
		t.Fatalf("failed to write invalid config: %v", err)
	}
	_, err = LoadConfig(invalidPath)
	if err == nil {
		t.Error("expected error for invalid JSON config file, got nil")
	}
}
