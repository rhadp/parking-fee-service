package setup_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TestCargoWorkspaceMembers verifies that rhivos/Cargo.toml declares a workspace
// with all required member crates, and that each member has Cargo.toml and src/main.rs.
// Test Spec: TS-01-7
// Requirements: 01-REQ-2.1, 01-REQ-2.2
func TestCargoWorkspaceMembers(t *testing.T) {
	root := repoRoot(t)
	cargoTomlPath := filepath.Join(root, "rhivos", "Cargo.toml")

	data, err := os.ReadFile(cargoTomlPath)
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

	t.Run("workspace_section_present", func(t *testing.T) {
		if !strings.Contains(content, "[workspace]") {
			t.Error("rhivos/Cargo.toml does not contain [workspace] section")
		}
	})

	for _, member := range expectedMembers {
		t.Run("member_in_toml/"+member, func(t *testing.T) {
			if !strings.Contains(content, member) {
				t.Errorf("rhivos/Cargo.toml does not mention workspace member %q", member)
			}
		})

		t.Run("member_cargo_toml/"+member, func(t *testing.T) {
			assertPathExists(t, filepath.Join(root, "rhivos", member, "Cargo.toml"))
		})

		t.Run("member_main_rs/"+member, func(t *testing.T) {
			assertPathExists(t, filepath.Join(root, "rhivos", member, "src", "main.rs"))
		})
	}
}

// TestMockSensorsBinaryTargets verifies that the mock-sensors crate declares three
// binary targets: location-sensor, speed-sensor, and door-sensor.
// Test Spec: TS-01-8
// Requirements: 01-REQ-2.3
func TestMockSensorsBinaryTargets(t *testing.T) {
	root := repoRoot(t)
	cargoTomlPath := filepath.Join(root, "rhivos", "mock-sensors", "Cargo.toml")

	data, err := os.ReadFile(cargoTomlPath)
	if err != nil {
		t.Fatalf("cannot read rhivos/mock-sensors/Cargo.toml: %v", err)
	}
	content := string(data)

	binaries := []string{
		"location-sensor",
		"speed-sensor",
		"door-sensor",
	}

	for _, bin := range binaries {
		t.Run(bin, func(t *testing.T) {
			if !strings.Contains(content, bin) {
				t.Errorf("rhivos/mock-sensors/Cargo.toml does not declare binary target %q", bin)
			}
		})
	}

	// Verify [[bin]] sections are present
	t.Run("bin_sections_present", func(t *testing.T) {
		if !strings.Contains(content, "[[bin]]") {
			t.Error("rhivos/mock-sensors/Cargo.toml does not contain any [[bin]] sections")
		}
	})
}
