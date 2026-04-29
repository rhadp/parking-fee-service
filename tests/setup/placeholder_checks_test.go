package setup_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

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
			crateDir := filepath.Join(root, "rhivos", crate, "src")
			found := findTestAnnotationInDir(t, crateDir, "#[test]")
			if !found {
				t.Errorf("Rust crate %q does not contain any #[test] annotations in src/", crate)
			}
		})
	}
}

// TS-01-27: Go modules have placeholder tests
// Requirement: 01-REQ-8.2
func TestGoModulesHavePlaceholderTests(t *testing.T) {
	root := repoRoot(t)

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

			// Check for *_test.go files
			testFiles := findTestGoFiles(t, modDir)
			if len(testFiles) == 0 {
				t.Errorf("Go module %q does not contain any *_test.go files", mod)
				return
			}

			// Check for func Test in at least one test file
			found := false
			for _, tf := range testFiles {
				content, err := os.ReadFile(tf)
				if err != nil {
					t.Errorf("failed to read %s: %v", tf, err)
					continue
				}
				if strings.Contains(string(content), "func Test") {
					found = true
					break
				}
			}
			if !found {
				t.Errorf("Go module %q has test files but none contain func Test", mod)
			}
		})
	}
}

// findTestAnnotationInDir searches recursively in dir for files containing the
// given annotation string (e.g., "#[test]" for Rust).
func findTestAnnotationInDir(t *testing.T, dir, annotation string) bool {
	t.Helper()
	found := false

	err := filepath.Walk(dir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if info.IsDir() {
			return nil
		}
		if !strings.HasSuffix(path, ".rs") {
			return nil
		}
		content, readErr := os.ReadFile(path)
		if readErr != nil {
			return readErr
		}
		if strings.Contains(string(content), annotation) {
			found = true
		}
		return nil
	})
	if err != nil {
		// Directory may not exist yet — that's expected in Group 1
		return false
	}
	return found
}

// findTestGoFiles finds all *_test.go files in the given directory.
func findTestGoFiles(t *testing.T, dir string) []string {
	t.Helper()
	var files []string

	entries, err := os.ReadDir(dir)
	if err != nil {
		// Directory may not exist yet — that's expected in Group 1
		return nil
	}
	for _, entry := range entries {
		if !entry.IsDir() && strings.HasSuffix(entry.Name(), "_test.go") {
			files = append(files, filepath.Join(dir, entry.Name()))
		}
	}
	return files
}
