package setup_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TestGoWorkspaceReferencesAllModules verifies go.work references all required
// Go modules (TS-01-10, 01-REQ-3.1).
func TestGoWorkspaceReferencesAllModules(t *testing.T) {
	root := repoRoot(t)
	goWork := filepath.Join(root, "go.work")
	assertPathExists(t, goWork)

	data, err := os.ReadFile(goWork)
	if err != nil {
		t.Fatalf("cannot read go.work: %v", err)
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
		if !strings.Contains(content, mod) {
			t.Errorf("go.work does not reference module %q", mod)
		}
	}
}

// TestGoModuleFilesExist verifies each Go module has go.mod and main.go
// (TS-01-11, 01-REQ-3.2, 01-REQ-3.3).
func TestGoModuleFilesExist(t *testing.T) {
	root := repoRoot(t)

	// Modules that should have both go.mod and main.go
	modules := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
		"mock/parking-operator",
	}
	for _, mod := range modules {
		assertPathExists(t, filepath.Join(root, mod, "go.mod"))
		assertPathExists(t, filepath.Join(root, mod, "main.go"))
	}

	// tests/setup has go.mod but no main.go (it's a test-only module)
	assertPathExists(t, filepath.Join(root, "tests", "setup", "go.mod"))
}
