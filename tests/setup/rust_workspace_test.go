package setup_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TS-01-7: Cargo workspace is correctly configured
// Requirements: 01-REQ-2.1, 01-REQ-2.2
func TestCargoWorkspaceConfiguration(t *testing.T) {
	root := repoRoot(t)
	cargoPath := filepath.Join(root, "rhivos", "Cargo.toml")

	data, err := os.ReadFile(cargoPath)
	if err != nil {
		t.Fatalf("failed to read %s: %v", cargoPath, err)
	}
	content := string(data)

	expectedMembers := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"mock-sensors",
	}

	t.Run("workspace members declared", func(t *testing.T) {
		for _, member := range expectedMembers {
			if !strings.Contains(content, member) {
				t.Errorf("Cargo.toml does not reference workspace member %q", member)
			}
		}
	})

	t.Run("each member is a valid crate", func(t *testing.T) {
		for _, member := range expectedMembers {
			memberDir := filepath.Join(root, "rhivos", member)
			assertPathExists(t, filepath.Join(memberDir, "Cargo.toml"))
			assertPathExists(t, filepath.Join(memberDir, "src", "main.rs"))
		}
	})
}

// TS-01-8: Mock sensors declares three binary targets
// Requirement: 01-REQ-2.3
func TestMockSensorsBinaryTargets(t *testing.T) {
	root := repoRoot(t)
	cargoPath := filepath.Join(root, "rhivos", "mock-sensors", "Cargo.toml")

	data, err := os.ReadFile(cargoPath)
	if err != nil {
		t.Fatalf("failed to read %s: %v", cargoPath, err)
	}
	content := string(data)

	binaries := []string{
		"location-sensor",
		"speed-sensor",
		"door-sensor",
	}

	for _, bin := range binaries {
		t.Run(bin, func(t *testing.T) {
			// Check that the binary name appears in a [[bin]] section
			if !strings.Contains(content, bin) {
				t.Errorf("mock-sensors Cargo.toml does not declare binary target %q", bin)
			}
		})
	}
}
