package setup_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TestCargoWorkspaceConfiguration verifies rhivos/Cargo.toml declares the
// correct workspace members and each member has Cargo.toml and src/main.rs
// (TS-01-7, 01-REQ-2.1, 01-REQ-2.2).
func TestCargoWorkspaceConfiguration(t *testing.T) {
	root := repoRoot(t)
	workspaceToml := filepath.Join(root, "rhivos", "Cargo.toml")
	assertPathExists(t, workspaceToml)

	data, err := os.ReadFile(workspaceToml)
	if err != nil {
		t.Fatalf("cannot read rhivos/Cargo.toml: %v", err)
	}
	content := string(data)

	expectedMembers := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"mock-sensors",
	}
	for _, member := range expectedMembers {
		if !strings.Contains(content, member) {
			t.Errorf("rhivos/Cargo.toml does not mention workspace member %q", member)
		}
		assertPathExists(t, filepath.Join(root, "rhivos", member, "Cargo.toml"))
		assertPathExists(t, filepath.Join(root, "rhivos", member, "src", "main.rs"))
	}
}

// TestMockSensorsBinaryTargets verifies the mock-sensors crate declares three
// binary targets (TS-01-8, 01-REQ-2.3).
func TestMockSensorsBinaryTargets(t *testing.T) {
	root := repoRoot(t)
	cargoToml := filepath.Join(root, "rhivos", "mock-sensors", "Cargo.toml")
	assertPathExists(t, cargoToml)

	data, err := os.ReadFile(cargoToml)
	if err != nil {
		t.Fatalf("cannot read rhivos/mock-sensors/Cargo.toml: %v", err)
	}
	content := string(data)

	expectedBins := []string{
		"location-sensor",
		"speed-sensor",
		"door-sensor",
	}
	for _, bin := range expectedBins {
		if !strings.Contains(content, bin) {
			t.Errorf("mock-sensors/Cargo.toml does not declare binary target %q", bin)
		}
	}
}
