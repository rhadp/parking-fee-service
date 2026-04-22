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
		t.Fatalf("expected %s to exist: %v", goWorkPath, err)
	}

	content := string(data)

	expectedModules := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
		"mock/parking-operator",
		"tests/setup",
	}

	for _, mod := range expectedModules {
		t.Run(mod, func(t *testing.T) {
			if !strings.Contains(content, mod) {
				t.Errorf("expected go.work to reference module %q", mod)
			}
		})
	}
}

// TS-01-11: Each Go module has go.mod and main.go
// Requirements: 01-REQ-3.2, 01-REQ-3.3
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
			goModPath := filepath.Join(root, mod, "go.mod")
			if _, err := os.Stat(goModPath); err != nil {
				t.Fatalf("expected %s to exist: %v", goModPath, err)
			}
		})

		t.Run(mod+"/main.go", func(t *testing.T) {
			mainGoPath := filepath.Join(root, mod, "main.go")
			if _, err := os.Stat(mainGoPath); err != nil {
				t.Fatalf("expected %s to exist: %v", mainGoPath, err)
			}
		})
	}

	// tests/setup should have go.mod but not main.go
	t.Run("tests/setup/go.mod", func(t *testing.T) {
		goModPath := filepath.Join(root, "tests", "setup", "go.mod")
		if _, err := os.Stat(goModPath); err != nil {
			t.Fatalf("expected %s to exist: %v", goModPath, err)
		}
	})
}
