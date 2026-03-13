package main

import "testing"

// TestStartup validates the package compiles.
func TestStartup(t *testing.T) {
	t.Log("parking-operator skeleton compiles and runs")
}

// TestGetPort_Default validates default port is 8080 when PORT is unset.
func TestGetPort_Default(t *testing.T) {
	t.Setenv("PORT", "")
	port := GetPort()
	if port != "8080" {
		t.Errorf("expected default port '8080', got %q", port)
	}
}

// TestGetPort_Override validates PORT env var overrides default.
func TestGetPort_Override(t *testing.T) {
	t.Setenv("PORT", "3000")
	port := GetPort()
	if port != "3000" {
		t.Errorf("expected port '3000', got %q", port)
	}
}
