package setup

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TestRustCratesHavePlaceholderTests verifies each Rust crate has at least one
// #[test] annotation in its source files.
// Test Spec: TS-01-26
// Requirement: 01-REQ-8.1
func TestRustCratesHavePlaceholderTests(t *testing.T) {
	root := findRepoRoot(t)

	crates := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"mock-sensors",
	}

	for _, crate := range crates {
		t.Run(crate, func(t *testing.T) {
			crateDir := filepath.Join(root, "rhivos", crate, "src")
			found := false

			err := filepath.WalkDir(crateDir, func(path string, d os.DirEntry, err error) error {
				if err != nil {
					return err
				}
				if d.IsDir() || !strings.HasSuffix(path, ".rs") {
					return nil
				}
				data, readErr := os.ReadFile(path)
				if readErr != nil {
					return readErr
				}
				if strings.Contains(string(data), "#[test]") {
					found = true
				}
				return nil
			})
			if err != nil {
				t.Fatalf("failed to walk crate directory %s: %v", crateDir, err)
			}
			if !found {
				t.Fatalf("crate %q has no #[test] annotations in src/", crate)
			}
		})
	}
}

// TestGoModulesHavePlaceholderTests verifies each Go module (except tests/setup)
// has at least one *_test.go file containing a func Test function.
// Test Spec: TS-01-27
// Requirement: 01-REQ-8.2
func TestGoModulesHavePlaceholderTests(t *testing.T) {
	root := findRepoRoot(t)

	modules := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
		"mock/parking-operator",
	}

	for _, mod := range modules {
		t.Run(mod, func(t *testing.T) {
			modDir := filepath.Join(root, mod)
			foundTestFile := false
			foundTestFunc := false

			err := filepath.WalkDir(modDir, func(path string, d os.DirEntry, err error) error {
				if err != nil {
					return err
				}
				if d.IsDir() || !strings.HasSuffix(path, "_test.go") {
					return nil
				}
				foundTestFile = true
				data, readErr := os.ReadFile(path)
				if readErr != nil {
					return readErr
				}
				if strings.Contains(string(data), "func Test") {
					foundTestFunc = true
				}
				return nil
			})
			if err != nil {
				t.Fatalf("failed to walk module directory %s: %v", modDir, err)
			}
			if !foundTestFile {
				t.Fatalf("module %q has no *_test.go files", mod)
			}
			if !foundTestFunc {
				t.Fatalf("module %q has no func Test* in test files", mod)
			}
		})
	}
}
