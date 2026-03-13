package setup

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// TS-01-35: Cargo Test Discovers Tests
// Requirement: 01-REQ-8.1
func TestCargoTestDiscovery(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping cargo test discovery in short mode")
	}

	root := repoRoot(t)
	cmd := exec.Command("cargo", "test")
	cmd.Dir = filepath.Join(root, "rhivos")
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("cargo test failed: %v\n%s", err, string(output))
	}

	text := string(output)
	crates := []string{"locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor", "mock-sensors"}
	for _, crate := range crates {
		// cargo test output uses underscores for crate names with hyphens
		crateName := strings.ReplaceAll(crate, "-", "_")
		if !strings.Contains(text, crate) && !strings.Contains(text, crateName) {
			t.Errorf("cargo test output does not reference crate %q", crate)
		}
	}
}

// TS-01-36: Go Test Discovers Tests
// Requirement: 01-REQ-8.2
func TestGoTestDiscovery(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping go test discovery in short mode")
	}

	root := repoRoot(t)
	cmd := exec.Command("go", "test", "./...")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("go test ./... failed: %v\n%s", err, string(output))
	}

	if !strings.Contains(string(output), "ok") {
		t.Error("go test output does not contain 'ok'")
	}
}

// TS-01-37: Setup Tests Module
// Requirement: 01-REQ-8.3
func TestSetupTestsModule(t *testing.T) {
	root := repoRoot(t)

	goModPath := filepath.Join(root, "tests", "setup", "go.mod")
	if !fileExists(goModPath) {
		t.Error("expected tests/setup/go.mod to exist")
	}

	// Check for at least one test file
	entries, err := os.ReadDir(filepath.Join(root, "tests", "setup"))
	if err != nil {
		t.Fatalf("failed to read tests/setup/: %v", err)
	}

	hasTestFile := false
	for _, entry := range entries {
		if strings.HasSuffix(entry.Name(), "_test.go") {
			hasTestFile = true
			break
		}
	}
	if !hasTestFile {
		t.Error("tests/setup/ contains no _test.go files")
	}
}

// TS-01-38: Make Test Runs All Tests
// Requirement: 01-REQ-8.4
func TestMakeTestRunsAll(t *testing.T) {
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

	text := string(output)
	// Should contain evidence of cargo test
	if !strings.Contains(text, "test result") {
		t.Error("make test output does not contain cargo test output ('test result')")
	}
	// Should contain evidence of go test
	if !strings.Contains(text, "ok") {
		t.Error("make test output does not contain go test output ('ok')")
	}
}
