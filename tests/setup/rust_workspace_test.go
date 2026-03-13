package setup

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// TS-01-7: Cargo Workspace Configuration
// Requirement: 01-REQ-2.1
func TestCargoWorkspaceMembers(t *testing.T) {
	root := repoRoot(t)

	cargoPath := filepath.Join(root, "rhivos", "Cargo.toml")
	content, err := os.ReadFile(cargoPath)
	if err != nil {
		t.Fatalf("failed to read rhivos/Cargo.toml: %v", err)
	}

	members := []string{"locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor", "mock-sensors"}
	for _, member := range members {
		if !strings.Contains(string(content), member) {
			t.Errorf("Cargo workspace missing member %q", member)
		}
	}
}

// TS-01-8: Cargo Build Succeeds
// Requirement: 01-REQ-2.2
func TestCargoBuild(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping cargo build in short mode")
	}

	root := repoRoot(t)
	cmd := exec.Command("cargo", "build")
	cmd.Dir = filepath.Join(root, "rhivos")
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("cargo build failed: %v\n%s", err, string(output))
	}
}

// TS-01-9: Cargo Test Succeeds
// Requirement: 01-REQ-2.3
func TestCargoTest(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping cargo test in short mode")
	}

	root := repoRoot(t)
	cmd := exec.Command("cargo", "test")
	cmd.Dir = filepath.Join(root, "rhivos")
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("cargo test failed: %v\n%s", err, string(output))
	}

	if !strings.Contains(string(output), "test result: ok") {
		t.Errorf("cargo test output does not contain 'test result: ok':\n%s", string(output))
	}
}

// TS-01-10: Mock Sensors Binary Targets
// Requirement: 01-REQ-2.4
func TestMockSensorsBinaries(t *testing.T) {
	root := repoRoot(t)

	bins := []string{"location-sensor", "speed-sensor", "door-sensor"}
	for _, bin := range bins {
		path := filepath.Join(root, "rhivos", "mock-sensors", "src", "bin", bin+".rs")
		if !fileExists(path) {
			t.Errorf("expected mock-sensors binary target file %s to exist", path)
		}
	}
}
