package config

import (
	"os"
	"testing"
)

// TS-09-23: parking-app-cli uses correct default service addresses.
func TestConfigDefaults(t *testing.T) {
	os.Unsetenv("PARKING_FEE_SERVICE_URL")
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTOR_ADDR")

	if url := ParkingFeeServiceURL(); url != "http://localhost:8080" {
		t.Errorf("expected http://localhost:8080, got %q", url)
	}
	if addr := UpdateServiceAddr(); addr != "localhost:50052" {
		t.Errorf("expected localhost:50052, got %q", addr)
	}
	if addr := ParkingAdaptorAddr(); addr != "localhost:50053" {
		t.Errorf("expected localhost:50053, got %q", addr)
	}
}

// TS-09-23: env vars override defaults.
func TestConfigOverrides(t *testing.T) {
	os.Setenv("PARKING_FEE_SERVICE_URL", "http://custom:9090")
	os.Setenv("UPDATE_SERVICE_ADDR", "custom:60000")
	os.Setenv("ADAPTOR_ADDR", "custom:60001")
	defer func() {
		os.Unsetenv("PARKING_FEE_SERVICE_URL")
		os.Unsetenv("UPDATE_SERVICE_ADDR")
		os.Unsetenv("ADAPTOR_ADDR")
	}()

	if url := ParkingFeeServiceURL(); url != "http://custom:9090" {
		t.Errorf("expected http://custom:9090, got %q", url)
	}
	if addr := UpdateServiceAddr(); addr != "custom:60000" {
		t.Errorf("expected custom:60000, got %q", addr)
	}
	if addr := ParkingAdaptorAddr(); addr != "custom:60001" {
		t.Errorf("expected custom:60001, got %q", addr)
	}
}

// TS-09-P4: Configuration defaults property test for parking-app-cli.
func TestPropertyConfigDefaults(t *testing.T) {
	defaults := map[string]struct {
		envVar   string
		expected string
		getter   func() string
	}{
		"PARKING_FEE_SERVICE_URL": {"PARKING_FEE_SERVICE_URL", DefaultParkingFeeServiceURL, ParkingFeeServiceURL},
		"UPDATE_SERVICE_ADDR":     {"UPDATE_SERVICE_ADDR", DefaultUpdateServiceAddr, UpdateServiceAddr},
		"ADAPTOR_ADDR":            {"ADAPTOR_ADDR", DefaultParkingAdaptorAddr, ParkingAdaptorAddr},
	}

	// Test defaults when env vars are cleared
	for name, tc := range defaults {
		os.Unsetenv(tc.envVar)
		if got := tc.getter(); got != tc.expected {
			t.Errorf("%s: expected default %q, got %q", name, tc.expected, got)
		}
	}

	// Test overrides
	for name, tc := range defaults {
		customVal := "custom-" + name
		os.Setenv(tc.envVar, customVal)
		if got := tc.getter(); got != customVal {
			t.Errorf("%s: expected override %q, got %q", name, customVal, got)
		}
		os.Unsetenv(tc.envVar)
	}
}
