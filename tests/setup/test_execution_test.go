package setup

import (
	"os/exec"
	"path/filepath"
	"testing"
)

// TestCargoTestPasses verifies `cargo test --workspace` passes in rhivos/.
// Test Spec: TS-01-28
// Requirement: 01-REQ-8.3
func TestCargoTestPasses(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH — skipping cargo test")
	}

	root := findRepoRoot(t)
	cmd := exec.Command("cargo", "test", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("cargo test --workspace failed:\n%s\nerror: %v", out, err)
	}
}

// TestGoTestPasses verifies `go test ./...` passes for all Go modules.
// Test Spec: TS-01-29
// Requirement: 01-REQ-8.4
func TestGoTestPasses(t *testing.T) {
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH — skipping go test")
	}

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
			cmd := exec.Command("go", "test", "./...")
			cmd.Dir = filepath.Join(root, mod)
			out, err := cmd.CombinedOutput()
			if err != nil {
				t.Fatalf("go test ./... failed in %s:\n%s\nerror: %v", mod, out, err)
			}
		})
	}
}
