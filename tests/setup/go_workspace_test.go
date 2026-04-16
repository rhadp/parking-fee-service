package setup_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TestGoWorkfileReferencesAllModules verifies that go.work at the repo root
// references all required Go modules via "use" directives.
// Test Spec: TS-01-10
// Requirements: 01-REQ-3.1
func TestGoWorkfileReferencesAllModules(t *testing.T) {
	root := repoRoot(t)
	goWorkPath := filepath.Join(root, "go.work")

	data, err := os.ReadFile(goWorkPath)
	if err != nil {
		t.Fatalf("cannot read go.work: %v", err)
	}
	content := string(data)

	requiredModules := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
		"mock/parking-operator",
		"tests/setup",
	}

	for _, modPath := range requiredModules {
		t.Run(modPath, func(t *testing.T) {
			if !strings.Contains(content, modPath) {
				t.Errorf("go.work does not reference module %q", modPath)
			}
		})
	}
}

// TestGoModulesHaveGoModAndMainGo verifies that each Go module (except tests/setup)
// has both a go.mod file and a main.go file.
// Test Spec: TS-01-11
// Requirements: 01-REQ-3.2, 01-REQ-3.3
func TestGoModulesHaveGoModAndMainGo(t *testing.T) {
	root := repoRoot(t)

	modules := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
		"mock/parking-operator",
	}

	for _, mod := range modules {
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
