package main

import (
	"os"
	"testing"
)

// TS-09-24: PARKING_OPERATOR Config Default
// Requirement: 09-REQ-5.4
func TestConfigDefault(t *testing.T) {
	os.Unsetenv("PORT")
	port := getPort(nil)
	if port != 8080 {
		t.Errorf("getPort() = %d, want 8080", port)
	}
}

// TS-09-24: PORT env var overrides default
func TestConfigEnvPort(t *testing.T) {
	os.Setenv("PORT", "9090")
	defer os.Unsetenv("PORT")
	port := getPort(nil)
	if port != 9090 {
		t.Errorf("getPort() = %d, want 9090", port)
	}
}

// TS-09-24: --port flag overrides env and default
func TestConfigFlagPort(t *testing.T) {
	os.Unsetenv("PORT")
	port := getPort([]string{"--port=7777"})
	if port != 7777 {
		t.Errorf("getPort([--port=7777]) = %d, want 7777", port)
	}
}
