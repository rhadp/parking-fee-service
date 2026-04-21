package setup

import (
	"os"
	"strings"
	"testing"
)

// TestGoWorkspaceReferencesAllModules verifies go.work references all required Go modules.
// Test Spec: TS-01-10
// Requirement: 01-REQ-3.1
func TestGoWorkspaceReferencesAllModules(t *testing.T) {
	root := findRepoRoot(t)

	goWorkPath := repoPath(root, "go.work")
	assertFileExists(t, goWorkPath)

	data, err := os.ReadFile(goWorkPath)
	if err != nil {
		t.Fatalf("failed to read go.work: %v", err)
	}
	content := string(data)

	// Verify go.work contains a "use" block or use directives.
	if !strings.Contains(content, "use") {
		t.Fatalf("go.work does not contain any use directives")
	}

	// All six Go module directories must be referenced.
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
				t.Fatalf("go.work does not reference module %q", mod)
			}
		})
	}
}

// TestGoModulesHaveGoModAndMainGo verifies each Go module has go.mod and main.go.
// Test Spec: TS-01-11
// Requirements: 01-REQ-3.2, 01-REQ-3.3
func TestGoModulesHaveGoModAndMainGo(t *testing.T) {
	root := findRepoRoot(t)

	// All Go modules except tests/setup must have both go.mod and main.go.
	modules := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
		"mock/parking-operator",
	}

	for _, mod := range modules {
		t.Run(mod+"/go.mod", func(t *testing.T) {
			assertFileExists(t, repoPath(root, mod, "go.mod"))
		})
		t.Run(mod+"/main.go", func(t *testing.T) {
			assertFileExists(t, repoPath(root, mod, "main.go"))
		})
	}

	// tests/setup must have go.mod but does not need main.go.
	t.Run("tests/setup/go.mod", func(t *testing.T) {
		assertFileExists(t, repoPath(root, "tests", "setup", "go.mod"))
	})
}

// TestGoModulePathsMatchDirectoryLayout verifies each go.mod declares a module path
// that corresponds to its location under the repository.
// Test Spec: TS-01-11 (extended)
// Requirement: 01-REQ-3.2
func TestGoModulePathsMatchDirectoryLayout(t *testing.T) {
	root := findRepoRoot(t)

	// Map from relative dir to expected module path substring.
	type modCheck struct {
		dir     string
		pathFrag string
	}

	checks := []modCheck{
		{"backend/parking-fee-service", "parking-fee-service"},
		{"backend/cloud-gateway", "cloud-gateway"},
		{"mock/parking-app-cli", "parking-app-cli"},
		{"mock/companion-app-cli", "companion-app-cli"},
		{"mock/parking-operator", "parking-operator"},
	}

	for _, c := range checks {
		t.Run(c.dir, func(t *testing.T) {
			goModPath := repoPath(root, c.dir, "go.mod")
			data, err := os.ReadFile(goModPath)
			if err != nil {
				t.Fatalf("failed to read %s: %v", goModPath, err)
			}
			if !strings.Contains(string(data), c.pathFrag) {
				t.Fatalf("go.mod at %s does not contain expected path fragment %q", goModPath, c.pathFrag)
			}
		})
	}
}
