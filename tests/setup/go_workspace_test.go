package setup

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// TS-01-11: Go Workspace File
// Requirement: 01-REQ-3.1
func TestGoWorkFile(t *testing.T) {
	root := repoRoot(t)

	goWorkPath := filepath.Join(root, "go.work")
	content, err := os.ReadFile(goWorkPath)
	if err != nil {
		t.Fatalf("failed to read go.work: %v", err)
	}

	modules := []string{"./backend", "./mock", "./tests/setup"}
	for _, mod := range modules {
		if !strings.Contains(string(content), mod) {
			t.Errorf("go.work missing use directive for %q", mod)
		}
	}
}

// TS-01-12: Go Build Succeeds
// Requirement: 01-REQ-3.2
func TestGoBuild(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping go build in short mode")
	}

	root := repoRoot(t)
	cmd := exec.Command("go", "build", "./...")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("go build ./... failed: %v\n%s", err, string(output))
	}
}

// TS-01-13: Go Test Succeeds
// Requirement: 01-REQ-3.3
func TestGoTest(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping go test in short mode")
	}

	root := repoRoot(t)
	cmd := exec.Command("go", "test", "./...")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("go test ./... failed: %v\n%s", err, string(output))
	}

	if !strings.Contains(string(output), "ok") {
		t.Errorf("go test output does not contain 'ok':\n%s", string(output))
	}
}
