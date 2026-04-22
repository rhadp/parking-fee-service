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

	// Verify workspace Cargo.toml exists
	workspaceToml := filepath.Join(root, "rhivos", "Cargo.toml")
	data, err := os.ReadFile(workspaceToml)
	if err != nil {
		t.Fatalf("expected %s to exist: %v", workspaceToml, err)
	}

	content := string(data)

	// Verify workspace section exists
	if !strings.Contains(content, "[workspace]") {
		t.Error("expected rhivos/Cargo.toml to contain [workspace] section")
	}

	// Verify all expected members are listed
	expectedMembers := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"mock-sensors",
	}

	for _, member := range expectedMembers {
		t.Run("member/"+member, func(t *testing.T) {
			if !strings.Contains(content, member) {
				t.Errorf("expected workspace Cargo.toml to reference member %q", member)
			}
		})
	}

	// Verify each member has Cargo.toml and src/main.rs
	for _, member := range expectedMembers {
		t.Run("crate/"+member, func(t *testing.T) {
			memberToml := filepath.Join(root, "rhivos", member, "Cargo.toml")
			if _, err := os.Stat(memberToml); err != nil {
				t.Errorf("expected %s to exist: %v", memberToml, err)
			}

			mainRs := filepath.Join(root, "rhivos", member, "src", "main.rs")
			if _, err := os.Stat(mainRs); err != nil {
				t.Errorf("expected %s to exist: %v", mainRs, err)
			}
		})
	}
}

// TS-01-8: Mock sensors declares three binary targets
// Requirement: 01-REQ-2.3
func TestMockSensorsBinaryTargets(t *testing.T) {
	root := repoRoot(t)

	cargoToml := filepath.Join(root, "rhivos", "mock-sensors", "Cargo.toml")
	data, err := os.ReadFile(cargoToml)
	if err != nil {
		t.Fatalf("expected %s to exist: %v", cargoToml, err)
	}

	content := string(data)

	expectedBinaries := []string{
		"location-sensor",
		"speed-sensor",
		"door-sensor",
	}

	// Verify [[bin]] section exists
	if !strings.Contains(content, "[[bin]]") {
		t.Fatal("expected mock-sensors/Cargo.toml to contain [[bin]] entries")
	}

	for _, bin := range expectedBinaries {
		t.Run(bin, func(t *testing.T) {
			// Check that the binary name appears in the Cargo.toml
			if !strings.Contains(content, bin) {
				t.Errorf("expected mock-sensors/Cargo.toml to declare binary target %q", bin)
			}
		})
	}
}
