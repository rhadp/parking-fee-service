package setup

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// TS-01-29: Makefile Build Target
// Requirement: 01-REQ-7.1
func TestMakeBuild(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping make build in short mode")
	}

	root := repoRoot(t)
	cmd := exec.Command("make", "build")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make build failed: %v\n%s", err, string(output))
	}
}

// TS-01-30: Makefile Test Target
// Requirement: 01-REQ-7.2
func TestMakeTest(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping make test in short mode")
	}

	root := repoRoot(t)
	cmd := exec.Command("make", "test")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make test failed: %v\n%s", err, string(output))
	}
}

// TS-01-31: Makefile Lint Target
// Requirement: 01-REQ-7.3
func TestMakeLint(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping make lint in short mode")
	}

	root := repoRoot(t)
	cmd := exec.Command("make", "lint")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make lint failed: %v\n%s", err, string(output))
	}
}

// TS-01-32: Makefile Check Target
// Requirement: 01-REQ-7.4
func TestMakeCheck(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping make check in short mode")
	}

	root := repoRoot(t)
	cmd := exec.Command("make", "check")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make check failed: %v\n%s", err, string(output))
	}
}

// TS-01-33: Makefile Clean Target
// Requirement: 01-REQ-7.5
func TestMakeClean(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping make clean in short mode")
	}

	root := repoRoot(t)

	// Build first to ensure there are artifacts
	buildCmd := exec.Command("make", "build")
	buildCmd.Dir = root
	if output, err := buildCmd.CombinedOutput(); err != nil {
		t.Fatalf("make build failed: %v\n%s", err, string(output))
	}

	// Clean
	cleanCmd := exec.Command("make", "clean")
	cleanCmd.Dir = root
	output, err := cleanCmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make clean failed: %v\n%s", err, string(output))
	}
	_ = output

	// Verify Rust target directory is removed
	targetDir := filepath.Join(root, "rhivos", "target")
	if isDir(targetDir) {
		t.Error("rhivos/target/ still exists after make clean")
	}
}

// TS-01-34: Makefile Proto and Infra Targets Exist
// Requirement: 01-REQ-7.6
func TestMakefileTargetsDefined(t *testing.T) {
	root := repoRoot(t)

	path := filepath.Join(root, "Makefile")
	content, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("failed to read Makefile: %v", err)
	}

	text := string(content)
	targets := []string{"proto", "infra-up", "infra-down"}
	for _, target := range targets {
		// Check for target definition (e.g., "proto:" or "proto :")
		if !strings.Contains(text, target+":") && !strings.Contains(text, target+" :") {
			t.Errorf("Makefile missing target %q", target)
		}
	}
}
