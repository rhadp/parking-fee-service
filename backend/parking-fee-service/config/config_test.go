package config_test

import (
	"encoding/json"
	"os"
	"testing"

	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/config"
	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/model"
)

// writeConfigFile writes a model.Config as JSON to a temporary file and
// returns its path. The caller is responsible for removing the file.
func writeConfigFile(t *testing.T, cfg model.Config) string {
	t.Helper()
	f, err := os.CreateTemp(t.TempDir(), "test-config-*.json")
	if err != nil {
		t.Fatalf("failed to create temp config file: %v", err)
	}
	enc := json.NewEncoder(f)
	if err := enc.Encode(cfg); err != nil {
		t.Fatalf("failed to encode config: %v", err)
	}
	f.Close()
	return f.Name()
}

// TS-05-9: LoadConfig reads configuration values from the specified file path.
func TestLoadConfigFromFile(t *testing.T) {
	t.Helper()
	want := model.Config{
		Port:               9090,
		ProximityThreshold: 250.0,
		Zones: []model.Zone{
			{
				ID:   "test-zone",
				Name: "Test Zone",
				Polygon: []model.Coordinate{
					{Lat: 48.14, Lon: 11.55},
					{Lat: 48.14, Lon: 11.56},
					{Lat: 48.13, Lon: 11.56},
					{Lat: 48.13, Lon: 11.55},
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
					ImageRef:       "registry/test-op:v1.0.0",
					ChecksumSHA256: "sha256:deadbeef",
					Version:        "1.0.0",
				},
			},
		},
	}
	path := writeConfigFile(t, want)

	cfg, err := config.LoadConfig(path)
	if err != nil {
		t.Fatalf("LoadConfig(%q): unexpected error: %v", path, err)
	}
	if cfg.Port != want.Port {
		t.Errorf("Port: want %d, got %d", want.Port, cfg.Port)
	}
	if cfg.ProximityThreshold != want.ProximityThreshold {
		t.Errorf("ProximityThreshold: want %v, got %v", want.ProximityThreshold, cfg.ProximityThreshold)
	}
	if len(cfg.Zones) != 1 {
		t.Fatalf("len(Zones): want 1, got %d", len(cfg.Zones))
	}
	if cfg.Zones[0].ID != "test-zone" {
		t.Errorf("Zones[0].ID: want 'test-zone', got %q", cfg.Zones[0].ID)
	}
	if len(cfg.Operators) != 1 {
		t.Fatalf("len(Operators): want 1, got %d", len(cfg.Operators))
	}
	if cfg.Operators[0].ID != "test-op" {
		t.Errorf("Operators[0].ID: want 'test-op', got %q", cfg.Operators[0].ID)
	}
}

// TS-05-10: Loaded configuration includes all required structural fields.
func TestConfigStructureValidation(t *testing.T) {
	t.Helper()
	full := model.Config{
		Port:               8080,
		ProximityThreshold: 500.0,
		Zones: []model.Zone{
			{
				ID:   "z1",
				Name: "Zone One",
				Polygon: []model.Coordinate{
					{Lat: 48.14, Lon: 11.55},
					{Lat: 48.14, Lon: 11.56},
					{Lat: 48.13, Lon: 11.56},
				},
			},
		},
		Operators: []model.Operator{
			{
				ID:     "op1",
				Name:   "Operator One",
				ZoneID: "z1",
				Rate:   model.Rate{Type: "flat-fee", Amount: 5.0, Currency: "EUR"},
				Adapter: model.AdapterMeta{
					ImageRef:       "registry/op1:v2.0.0",
					ChecksumSHA256: "sha256:cafebabe",
					Version:        "2.0.0",
				},
			},
		},
	}
	path := writeConfigFile(t, full)

	cfg, err := config.LoadConfig(path)
	if err != nil {
		t.Fatalf("LoadConfig: unexpected error: %v", err)
	}
	if cfg.Port <= 0 {
		t.Errorf("Port: want > 0, got %d", cfg.Port)
	}
	if cfg.ProximityThreshold <= 0 {
		t.Errorf("ProximityThreshold: want > 0, got %v", cfg.ProximityThreshold)
	}
	if len(cfg.Zones) == 0 {
		t.Fatalf("Zones: want at least 1, got 0")
	}
	if len(cfg.Zones[0].Polygon) < 3 {
		t.Errorf("Zones[0].Polygon: want at least 3 vertices, got %d", len(cfg.Zones[0].Polygon))
	}
	if len(cfg.Operators) == 0 {
		t.Fatalf("Operators: want at least 1, got 0")
	}
	if cfg.Operators[0].Adapter.ImageRef == "" {
		t.Errorf("Operators[0].Adapter.ImageRef: want non-empty, got empty")
	}
}

// TS-05-E5: When config file does not exist, LoadConfig returns default
// configuration (Munich demo data) with no error.
func TestConfigFileMissingDefaults(t *testing.T) {
	t.Helper()
	cfg, err := config.LoadConfig("/nonexistent/path/that/cannot/exist/config.json")
	if err != nil {
		t.Fatalf("LoadConfig with missing file: want nil error, got: %v", err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig with missing file: want non-nil Config, got nil")
	}
	if len(cfg.Zones) < 1 {
		t.Errorf("Zones: want >= 1, got %d", len(cfg.Zones))
	}
	if len(cfg.Operators) < 1 {
		t.Errorf("Operators: want >= 1, got %d", len(cfg.Operators))
	}
	if cfg.Port <= 0 {
		t.Errorf("Port: want > 0, got %d", cfg.Port)
	}
	if cfg.ProximityThreshold <= 0 {
		t.Errorf("ProximityThreshold: want > 0, got %v", cfg.ProximityThreshold)
	}
}

// TS-05-E6: When config file contains invalid JSON, LoadConfig returns a
// non-nil error.
func TestConfigInvalidJSON(t *testing.T) {
	t.Helper()
	f, err := os.CreateTemp(t.TempDir(), "invalid-config-*.json")
	if err != nil {
		t.Fatalf("failed to create temp file: %v", err)
	}
	f.WriteString("{invalid json")
	f.Close()

	_, err = config.LoadConfig(f.Name())
	if err == nil {
		t.Error("LoadConfig with invalid JSON: want non-nil error, got nil")
	}
}

// TS-05-P6: Property — for any nonexistent file path, LoadConfig returns a
// valid default configuration with at least one zone and one operator.
func TestPropertyConfigDefaults(t *testing.T) {
	t.Helper()
	paths := []string{
		"/nonexistent/a",
		"/nonexistent/b/c/d",
		"/tmp/does_not_exist_12345.json",
	}

	for _, path := range paths {
		// Skip if the file somehow exists (extreme edge case).
		if _, statErr := os.Stat(path); statErr == nil {
			t.Skipf("path %q unexpectedly exists, skipping", path)
		}

		cfg, err := config.LoadConfig(path)
		if err != nil {
			t.Errorf("LoadConfig(%q): want nil error, got: %v", path, err)
			continue
		}
		if cfg == nil {
			t.Errorf("LoadConfig(%q): want non-nil Config, got nil", path)
			continue
		}
		if len(cfg.Zones) < 1 {
			t.Errorf("LoadConfig(%q): Zones: want >= 1, got %d", path, len(cfg.Zones))
		}
		if len(cfg.Operators) < 1 {
			t.Errorf("LoadConfig(%q): Operators: want >= 1, got %d", path, len(cfg.Operators))
		}
		if cfg.Port <= 0 {
			t.Errorf("LoadConfig(%q): Port: want > 0, got %d", path, cfg.Port)
		}
		if cfg.ProximityThreshold <= 0 {
			t.Errorf("LoadConfig(%q): ProximityThreshold: want > 0, got %v", path, cfg.ProximityThreshold)
		}
	}
}
