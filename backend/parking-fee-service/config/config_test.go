package config_test

import (
	"encoding/json"
	"math/rand"
	"os"
	"path/filepath"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/config"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

// testConfig returns a valid config JSON and the expected config values.
func testConfig() ([]byte, model.Config) {
	cfg := model.Config{
		Port:               9090,
		ProximityThreshold: 200,
		Zones: []model.Zone{
			{
				ID:   "test-zone",
				Name: "Test Zone",
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
				ID:     "test-op",
				Name:   "Test Operator",
				ZoneID: "test-zone",
				Rate:   model.Rate{Type: "per-hour", Amount: 3.00, Currency: "EUR"},
				Adapter: model.AdapterMeta{
					ImageRef:       "registry.example.com/test:v1.0.0",
					ChecksumSHA256: "sha256:test123",
					Version:        "1.0.0",
				},
			},
		},
	}
	data, _ := json.Marshal(cfg)
	return data, cfg
}

// TS-05-9: LoadConfig reads configuration from the specified file path.
func TestLoadConfigFromFile(t *testing.T) {
	data, expected := testConfig()

	dir := t.TempDir()
	path := filepath.Join(dir, "config.json")
	if err := os.WriteFile(path, data, 0644); err != nil {
		t.Fatalf("failed to write test config: %v", err)
	}

	cfg, err := config.LoadConfig(path)
	if err != nil {
		t.Fatalf("LoadConfig(%q) returned error: %v", path, err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig returned nil config")
	}
	if cfg.Port != expected.Port {
		t.Errorf("Port = %d, want %d", cfg.Port, expected.Port)
	}
	if len(cfg.Zones) != len(expected.Zones) {
		t.Errorf("len(Zones) = %d, want %d", len(cfg.Zones), len(expected.Zones))
	}
	if len(cfg.Operators) != len(expected.Operators) {
		t.Errorf("len(Operators) = %d, want %d", len(cfg.Operators), len(expected.Operators))
	}
}

// TS-05-10: Loaded configuration includes proximity threshold, port, zones
// with polygons, and operators with adapter metadata.
func TestConfigStructureValidation(t *testing.T) {
	data, _ := testConfig()

	dir := t.TempDir()
	path := filepath.Join(dir, "config.json")
	if err := os.WriteFile(path, data, 0644); err != nil {
		t.Fatalf("failed to write test config: %v", err)
	}

	cfg, err := config.LoadConfig(path)
	if err != nil {
		t.Fatalf("LoadConfig returned error: %v", err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig returned nil config")
	}
	if cfg.Port <= 0 {
		t.Errorf("Port = %d, want > 0", cfg.Port)
	}
	if cfg.ProximityThreshold <= 0 {
		t.Errorf("ProximityThreshold = %f, want > 0", cfg.ProximityThreshold)
	}
	if len(cfg.Zones) == 0 {
		t.Fatal("Zones is empty")
	}
	if len(cfg.Zones[0].Polygon) < 3 {
		t.Errorf("Zones[0].Polygon has %d points, want >= 3", len(cfg.Zones[0].Polygon))
	}
	if len(cfg.Operators) == 0 {
		t.Fatal("Operators is empty")
	}
	if cfg.Operators[0].Adapter.ImageRef == "" {
		t.Error("Operators[0].Adapter.ImageRef is empty")
	}
}

// TS-05-E5: When config file does not exist, LoadConfig returns default
// configuration with Munich demo data.
func TestConfigFileMissingDefaults(t *testing.T) {
	cfg, err := config.LoadConfig("/nonexistent/path/config.json")
	if err != nil {
		t.Fatalf("LoadConfig(missing path) returned error: %v", err)
	}
	if cfg == nil {
		t.Fatal("LoadConfig returned nil config")
	}
	if len(cfg.Zones) < 1 {
		t.Errorf("default config has %d zones, want >= 1", len(cfg.Zones))
	}
	if len(cfg.Operators) < 1 {
		t.Errorf("default config has %d operators, want >= 1", len(cfg.Operators))
	}
	if cfg.Port != 8080 {
		t.Errorf("default Port = %d, want 8080", cfg.Port)
	}
	if cfg.ProximityThreshold != 500.0 {
		t.Errorf("default ProximityThreshold = %f, want 500.0", cfg.ProximityThreshold)
	}
}

// TS-05-E6: When config file contains invalid JSON, LoadConfig returns an error.
func TestConfigInvalidJSON(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "bad-config.json")
	if err := os.WriteFile(path, []byte("{invalid json"), 0644); err != nil {
		t.Fatalf("failed to write invalid config: %v", err)
	}

	_, err := config.LoadConfig(path)
	if err == nil {
		t.Error("LoadConfig(invalid JSON) returned nil error, want non-nil")
	}
}

// TS-05-P6: Property test for config defaults.
// For any nonexistent file path, LoadConfig returns valid default configuration.
func TestPropertyConfigDefaults(t *testing.T) {
	rng := rand.New(rand.NewSource(42))
	paths := []string{
		"/nonexistent/config.json",
		"/tmp/does-not-exist-abc123.json",
		"/var/missing/data.json",
	}
	// Generate some random nonexistent paths
	for i := 0; i < 10; i++ {
		path := "/nonexistent/" + randomString(rng, 10) + ".json"
		paths = append(paths, path)
	}

	for _, path := range paths {
		cfg, err := config.LoadConfig(path)
		if err != nil {
			t.Errorf("LoadConfig(%q) returned error: %v", path, err)
			continue
		}
		if cfg == nil {
			t.Errorf("LoadConfig(%q) returned nil config", path)
			continue
		}
		if len(cfg.Zones) < 1 {
			t.Errorf("LoadConfig(%q): zones count = %d, want >= 1", path, len(cfg.Zones))
		}
		if len(cfg.Operators) < 1 {
			t.Errorf("LoadConfig(%q): operators count = %d, want >= 1", path, len(cfg.Operators))
		}
		if cfg.Port <= 0 {
			t.Errorf("LoadConfig(%q): port = %d, want > 0", path, cfg.Port)
		}
		if cfg.ProximityThreshold <= 0 {
			t.Errorf("LoadConfig(%q): threshold = %f, want > 0", path, cfg.ProximityThreshold)
		}
	}
}

func randomString(rng *rand.Rand, n int) string {
	const letters = "abcdefghijklmnopqrstuvwxyz"
	b := make([]byte, n)
	for i := range b {
		b[i] = letters[rng.Intn(len(letters))]
	}
	return string(b)
}
