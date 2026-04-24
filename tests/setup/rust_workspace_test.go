package setup_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TS-01-7: Cargo workspace is correctly configured
// Requirement: 01-REQ-2.1, 01-REQ-2.2
func TestCargoWorkspaceConfiguration(t *testing.T) {
	root := repoRoot(t)

	workspaceToml := filepath.Join(root, "rhivos", "Cargo.toml")

	t.Run("workspace Cargo.toml exists", func(t *testing.T) {
		if !pathExists(workspaceToml) {
			t.Fatalf("expected %s to exist", workspaceToml)
		}
	})

	t.Run("workspace declares all members", func(t *testing.T) {
		if !pathExists(workspaceToml) {
			t.Skip("rhivos/Cargo.toml does not exist")
		}

		data, err := os.ReadFile(workspaceToml)
		if err != nil {
			t.Fatalf("failed to read %s: %v", workspaceToml, err)
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
				t.Errorf("Cargo.toml should reference workspace member %q", member)
			}
		}
	})

	t.Run("each member has Cargo.toml and src/main.rs", func(t *testing.T) {
		members := []string{
			"locking-service",
			"cloud-gateway-client",
			"update-service",
			"parking-operator-adaptor",
			"mock-sensors",
		}

		for _, member := range members {
			cargoPath := filepath.Join(root, "rhivos", member, "Cargo.toml")
			if !pathExists(cargoPath) {
				t.Errorf("expected %s to exist", cargoPath)
			}

			mainPath := filepath.Join(root, "rhivos", member, "src", "main.rs")
			if !pathExists(mainPath) {
				t.Errorf("expected %s to exist", mainPath)
			}
		}
	})
}

// TS-01-8: Mock sensors declares three binary targets
// Requirement: 01-REQ-2.3
func TestMockSensorsBinaryTargets(t *testing.T) {
	root := repoRoot(t)

	cargoToml := filepath.Join(root, "rhivos", "mock-sensors", "Cargo.toml")

	if !pathExists(cargoToml) {
		t.Fatalf("expected %s to exist", cargoToml)
	}

	data, err := os.ReadFile(cargoToml)
	if err != nil {
		t.Fatalf("failed to read %s: %v", cargoToml, err)
	}
	content := string(data)

	expectedBinaries := []string{
		"location-sensor",
		"speed-sensor",
		"door-sensor",
	}

	for _, bin := range expectedBinaries {
		t.Run(bin, func(t *testing.T) {
			// Check that the binary name appears in the Cargo.toml,
			// typically as: name = "location-sensor"
			if !strings.Contains(content, bin) {
				t.Errorf("mock-sensors/Cargo.toml should declare binary target %q", bin)
			}
		})
	}
}

// TS-01-26: Rust crates have placeholder tests
// Requirement: 01-REQ-8.1
func TestRustCratesHavePlaceholderTests(t *testing.T) {
	root := repoRoot(t)

	crates := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"mock-sensors",
	}

	for _, crate := range crates {
		t.Run(crate, func(t *testing.T) {
			srcDir := filepath.Join(root, "rhivos", crate, "src")
			if !pathExists(srcDir) {
				t.Fatalf("expected %s to exist", srcDir)
			}

			// Walk the src directory looking for #[test] in any .rs file
			found := false
			err := filepath.Walk(srcDir, func(path string, info os.FileInfo, err error) error {
				if err != nil {
					return err
				}
				if info.IsDir() || !strings.HasSuffix(path, ".rs") {
					return nil
				}
				data, err := os.ReadFile(path)
				if err != nil {
					return err
				}
				if strings.Contains(string(data), "#[test]") {
					found = true
				}
				return nil
			})
			if err != nil {
				t.Fatalf("failed to walk %s: %v", srcDir, err)
			}
			if !found {
				t.Errorf("crate %s should contain at least one #[test] function", crate)
			}
		})
	}
}
