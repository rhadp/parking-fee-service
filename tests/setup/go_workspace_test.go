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

	data, err := os.ReadFile(goWorkPath)
	if err != nil {
		t.Fatalf("failed to read %s: %v", goWorkPath, err)
	}
	content := string(data)

	modules := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
		"mock/parking-operator",
		"tests/setup",
	}

	for _, mod := range modules {
		t.Run(mod, func(t *testing.T) {
			if !strings.Contains(content, mod) {
				t.Errorf("go.work does not reference module %q", mod)
			}
		})
	}
}

// TS-01-11: Each Go module has go.mod and main.go
// Requirements: 01-REQ-3.2, 01-REQ-3.3
func TestGoModuleFiles(t *testing.T) {
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
			assertPathExists(t, filepath.Join(root, mod, "go.mod"))
		})
		t.Run(mod+"/main.go", func(t *testing.T) {
			assertPathExists(t, filepath.Join(root, mod, "main.go"))
		})
	}

	// tests/setup only needs go.mod (no main.go required)
	t.Run("tests/setup/go.mod", func(t *testing.T) {
		assertPathExists(t, filepath.Join(root, "tests", "setup", "go.mod"))
	})
}
