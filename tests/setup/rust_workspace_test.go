package setup

import (
	"os"
	"strings"
	"testing"
)

// TestCargoWorkspaceConfiguration verifies rhivos/Cargo.toml declares the correct
// workspace members and each member is a valid crate with Cargo.toml and src/main.rs.
// Test Spec: TS-01-7
// Requirements: 01-REQ-2.1, 01-REQ-2.2
func TestCargoWorkspaceConfiguration(t *testing.T) {
	root := findRepoRoot(t)

	// Verify workspace Cargo.toml exists.
	workspaceToml := repoPath(root, "rhivos", "Cargo.toml")
	assertFileExists(t, workspaceToml)

	// Read workspace Cargo.toml and verify it declares a workspace.
	data, err := os.ReadFile(workspaceToml)
	if err != nil {
		t.Fatalf("failed to read %s: %v", workspaceToml, err)
	}
	content := string(data)

	if !strings.Contains(content, "[workspace]") {
		t.Fatalf("rhivos/Cargo.toml does not declare a [workspace] section")
	}

	// Verify each expected workspace member is referenced in Cargo.toml
	// and has the required crate structure.
	members := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"mock-sensors",
	}

	for _, member := range members {
		t.Run(member, func(t *testing.T) {
			// Member must appear in workspace Cargo.toml.
			if !strings.Contains(content, member) {
				t.Fatalf("member %q not found in rhivos/Cargo.toml", member)
			}

			// Each member must have its own Cargo.toml.
			memberCargoToml := repoPath(root, "rhivos", member, "Cargo.toml")
			assertFileExists(t, memberCargoToml)

			// Each member must have src/main.rs.
			memberMainRs := repoPath(root, "rhivos", member, "src", "main.rs")
			assertFileExists(t, memberMainRs)
		})
	}
}

// TestMockSensorsBinaryTargets verifies mock-sensors crate declares three binary targets.
// Test Spec: TS-01-8
// Requirement: 01-REQ-2.3
func TestMockSensorsBinaryTargets(t *testing.T) {
	root := findRepoRoot(t)

	mockSensorsToml := repoPath(root, "rhivos", "mock-sensors", "Cargo.toml")
	assertFileExists(t, mockSensorsToml)

	data, err := os.ReadFile(mockSensorsToml)
	if err != nil {
		t.Fatalf("failed to read %s: %v", mockSensorsToml, err)
	}
	content := string(data)

	// Verify three [[bin]] entries with expected names.
	expectedBins := []string{
		"location-sensor",
		"speed-sensor",
		"door-sensor",
	}

	for _, bin := range expectedBins {
		t.Run(bin, func(t *testing.T) {
			if !strings.Contains(content, bin) {
				t.Fatalf("binary target %q not found in mock-sensors/Cargo.toml", bin)
			}
		})
	}

	// Verify [[bin]] section exists.
	if !strings.Contains(content, "[[bin]]") {
		t.Fatalf("mock-sensors/Cargo.toml does not contain any [[bin]] sections")
	}
}
