package setup_test

import (
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// TS-01-9: Cargo build succeeds for entire workspace
// Requirement: 01-REQ-2.4
func TestCargoBuildWorkspace(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH; skipping Rust build test")
	}

	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Errorf("cargo build --workspace failed (exit error: %v)\noutput:\n%s", err, string(output))
	}
}

// TS-01-12: Go build succeeds for all modules
// Requirement: 01-REQ-3.4
func TestGoBuildAllModules(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH; skipping Go build test")
	}

	cmd := exec.Command("go", "build", "./...")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Errorf("go build ./... failed (exit error: %v)\noutput:\n%s", err, string(output))
	}
}

// TS-01-19: make build succeeds
// Requirement: 01-REQ-6.2
func TestMakeBuild(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("make not found on PATH; skipping")
	}

	cmd := exec.Command("make", "build")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Errorf("make build failed (exit error: %v)\noutput:\n%s", err, string(output))
	}
}

// TS-01-20: make test succeeds
// Requirement: 01-REQ-6.3
func TestMakeTest(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("make not found on PATH; skipping")
	}

	cmd := exec.Command("make", "test")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Errorf("make test failed (exit error: %v)\noutput:\n%s", err, string(output))
	}
}

// TS-01-21: make clean removes build artifacts
// Requirement: 01-REQ-6.4
func TestMakeClean(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("make not found on PATH; skipping")
	}

	// First build to create artifacts
	buildCmd := exec.Command("make", "build")
	buildCmd.Dir = root
	buildOutput, err := buildCmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make build failed (prerequisite for clean test): %v\noutput:\n%s", err, string(buildOutput))
	}

	// Then clean
	cleanCmd := exec.Command("make", "clean")
	cleanCmd.Dir = root
	cleanOutput, err := cleanCmd.CombinedOutput()
	if err != nil {
		t.Errorf("make clean failed (exit error: %v)\noutput:\n%s", err, string(cleanOutput))
	}

	// Verify Rust target/ directory was removed
	targetPath := filepath.Join(root, "rhivos", "target")
	assertPathNotExists(t, targetPath)
}

// TS-01-22: make check runs lint and tests
// Requirement: 01-REQ-6.5
func TestMakeCheck(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("make not found on PATH; skipping")
	}

	cmd := exec.Command("make", "check")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Errorf("make check failed (exit error: %v)\noutput:\n%s", err, string(output))
	}
}

// TS-01-28: cargo test passes for all Rust crates
// Requirement: 01-REQ-8.3
func TestCargoTestPasses(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH; skipping Rust test verification")
	}

	cmd := exec.Command("cargo", "test", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Errorf("cargo test --workspace failed (exit error: %v)\noutput:\n%s", err, string(output))
	}
}

// TS-01-29: go test passes for all Go modules
// Requirement: 01-REQ-8.4
func TestGoTestPasses(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH; skipping Go test verification")
	}

	cmd := exec.Command("go", "test", "./...")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Errorf("go test ./... failed (exit error: %v)\noutput:\n%s", err, string(output))
	}
}

// TS-01-30: Setup verification tests exist and are runnable
// Requirements: 01-REQ-9.1, 01-REQ-9.2, 01-REQ-9.3
func TestSetupTestsRunnable(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("make not found on PATH; skipping")
	}

	cmd := exec.Command("make", "test-setup")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Errorf("make test-setup failed (exit error: %v)\noutput:\n%s", err, string(output))
	}

	// Verify PASS appears in output
	if !strings.Contains(string(output), "PASS") {
		t.Errorf("make test-setup output does not contain PASS:\n%s", string(output))
	}
}

// TS-01-31: Setup tests report clear pass/fail
// Requirement: 01-REQ-9.4
func TestSetupTestsVerboseOutput(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH; skipping")
	}

	cmd := exec.Command("go", "test", "-v", "./...")
	cmd.Dir = filepath.Join(root, "tests", "setup")
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Logf("go test -v output:\n%s", string(output))
		// Note: test may fail if individual tests fail, but we check for named results
	}

	out := string(output)

	// Verify verbose output shows individual test names (at least some known tests)
	knownTests := []string{
		"TestRhivosDirectoryStructure",
		"TestBackendDirectoryStructure",
		"TestGoWorkspaceReferences",
	}

	for _, testName := range knownTests {
		t.Run("has_"+testName, func(t *testing.T) {
			if !strings.Contains(out, testName) {
				t.Errorf("verbose test output does not contain test name %q", testName)
			}
		})
	}

	_ = err // we already checked output content
}

// TS-01-32: make proto generates Go code
// Requirements: 01-REQ-10.1, 01-REQ-10.2, 01-REQ-10.3
func TestMakeProtoGeneratesGoCode(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("make not found on PATH; skipping")
	}
	if _, err := exec.LookPath("protoc"); err != nil {
		t.Skip("protoc not found on PATH; skipping")
	}

	// Run make proto
	protoCmd := exec.Command("make", "proto")
	protoCmd.Dir = root
	protoOutput, err := protoCmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make proto failed (exit error: %v)\noutput:\n%s", err, string(protoOutput))
	}

	// Verify generated Go code compiles
	buildCmd := exec.Command("go", "build", "./...")
	buildCmd.Dir = root
	buildOutput, err := buildCmd.CombinedOutput()
	if err != nil {
		t.Errorf("go build ./... failed after make proto (exit error: %v)\noutput:\n%s", err, string(buildOutput))
	}
}
