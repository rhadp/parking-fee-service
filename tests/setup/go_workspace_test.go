package setup_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TS-01-10: Go workspace file references all modules
// Requirement: 01-REQ-3.1
func TestGoWorkspaceReferences(t *testing.T) {
	root := repoRoot(t)

	goWorkPath := filepath.Join(root, "go.work")

	if !pathExists(goWorkPath) {
		t.Fatalf("expected %s to exist", goWorkPath)
	}

	data, err := os.ReadFile(goWorkPath)
	if err != nil {
		t.Fatalf("failed to read %s: %v", goWorkPath, err)
	}
	content := string(data)

	// The go.work file must reference at least these modules.
	requiredModules := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
		"mock/parking-operator",
		"tests/setup",
	}

	for _, mod := range requiredModules {
		t.Run(mod, func(t *testing.T) {
			if !strings.Contains(content, mod) {
				t.Errorf("go.work should reference module %q", mod)
			}
		})
	}
}

// TS-01-11: Each Go module has go.mod and main.go
// Requirement: 01-REQ-3.2, 01-REQ-3.3
func TestGoModulesHaveRequiredFiles(t *testing.T) {
	root := repoRoot(t)

	// Modules that should have both go.mod and main.go
	modulesWithMain := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
		"mock/parking-operator",
	}

	for _, mod := range modulesWithMain {
		t.Run(mod+"/go.mod", func(t *testing.T) {
			path := filepath.Join(root, mod, "go.mod")
			if !pathExists(path) {
				t.Errorf("expected %s to exist", path)
			}
		})

		t.Run(mod+"/main.go", func(t *testing.T) {
			path := filepath.Join(root, mod, "main.go")
			if !pathExists(path) {
				t.Errorf("expected %s to exist", path)
			}
		})
	}

	// tests/setup should have go.mod but NOT main.go
	t.Run("tests/setup/go.mod", func(t *testing.T) {
		path := filepath.Join(root, "tests", "setup", "go.mod")
		if !pathExists(path) {
			t.Errorf("expected %s to exist", path)
		}
	})
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
			if !pathExists(modDir) {
				t.Fatalf("module directory %s does not exist", modDir)
			}

			// Look for any _test.go file containing "func Test"
			entries, err := os.ReadDir(modDir)
			if err != nil {
				t.Fatalf("failed to read directory %s: %v", modDir, err)
			}

			testFound := false
			for _, entry := range entries {
				if entry.IsDir() || !strings.HasSuffix(entry.Name(), "_test.go") {
					continue
				}
				data, err := os.ReadFile(filepath.Join(modDir, entry.Name()))
				if err != nil {
					continue
				}
				if strings.Contains(string(data), "func Test") {
					testFound = true
					break
				}
			}

			if !testFound {
				t.Errorf("module %s should contain at least one _test.go file with a func Test function", mod)
			}
		})
	}
}
