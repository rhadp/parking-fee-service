package setup

import (
	"os"
	"os/exec"
	"path/filepath"
	"testing"
)

// TestMakeBuildSucceeds verifies `make build` compiles all components.
// Test Spec: TS-01-19
// Requirement: 01-REQ-6.2
func TestMakeBuildSucceeds(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH — skipping make build test")
	}
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH — skipping make build test")
	}

	root := findRepoRoot(t)
	cmd := exec.Command("make", "build")
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make build failed:\n%s\nerror: %v", out, err)
	}
}

// TestMakeTestSucceeds verifies `make test` runs all tests.
// Test Spec: TS-01-20
// Requirement: 01-REQ-6.3
func TestMakeTestSucceeds(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH — skipping make test test")
	}
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH — skipping make test test")
	}

	root := findRepoRoot(t)
	cmd := exec.Command("make", "test")
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make test failed:\n%s\nerror: %v", out, err)
	}
}

// TestMakeCleanRemovesArtifacts verifies `make clean` removes build artifacts.
// Test Spec: TS-01-21
// Requirement: 01-REQ-6.4
func TestMakeCleanRemovesArtifacts(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH — skipping make clean test")
	}
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH — skipping make clean test")
	}

	root := findRepoRoot(t)

	// First build to create artifacts.
	buildCmd := exec.Command("make", "build")
	buildCmd.Dir = root
	if out, err := buildCmd.CombinedOutput(); err != nil {
		t.Fatalf("make build failed (prerequisite):\n%s\n%v", out, err)
	}

	// Now clean.
	cleanCmd := exec.Command("make", "clean")
	cleanCmd.Dir = root
	out, err := cleanCmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make clean failed:\n%s\nerror: %v", out, err)
	}

	// Verify Rust target/ directory is removed.
	targetDir := filepath.Join(root, "rhivos", "target")
	if _, statErr := os.Stat(targetDir); statErr == nil {
		t.Fatalf("rhivos/target/ still exists after make clean")
	}
}

// TestMakeCheckSucceeds verifies `make check` runs lint + tests.
// Test Spec: TS-01-22
// Requirement: 01-REQ-6.5
func TestMakeCheckSucceeds(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH — skipping make check test")
	}
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH — skipping make check test")
	}

	root := findRepoRoot(t)
	cmd := exec.Command("make", "check")
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make check failed:\n%s\nerror: %v", out, err)
	}
}

// TestBuildCompletenessArtifactsExist verifies all expected binary artifacts
// exist after make build.
// Test Spec: TS-01-P1
// Property: Property 1 (Build Completeness)
func TestBuildCompletenessArtifactsExist(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH — skipping build completeness test")
	}
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH — skipping build completeness test")
	}

	root := findRepoRoot(t)

	// Build all.
	cmd := exec.Command("make", "build")
	cmd.Dir = root
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("make build failed:\n%s\n%v", out, err)
	}

	// Verify Rust binary artifacts exist.
	rustBinaries := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"location-sensor",
		"speed-sensor",
		"door-sensor",
	}
	for _, bin := range rustBinaries {
		binPath := filepath.Join(root, "rhivos", "target", "debug", bin)
		if _, err := os.Stat(binPath); os.IsNotExist(err) {
			t.Errorf("binary artifact missing: %s", binPath)
		}
	}
}

// TestBuildWithExtraneousFile verifies the build succeeds even with extra files
// in the repository root.
// Test Spec: TS-01-E1
// Requirement: 01-REQ-1.E1
func TestBuildWithExtraneousFile(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH — skipping extraneous file test")
	}
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH — skipping extraneous file test")
	}

	root := findRepoRoot(t)

	// Create extraneous file.
	strayFile := filepath.Join(root, "stray_file_test.txt")
	if err := os.WriteFile(strayFile, []byte("test content"), 0o644); err != nil {
		t.Fatalf("failed to create stray file: %v", err)
	}
	defer os.Remove(strayFile)

	// Build should still succeed.
	cmd := exec.Command("make", "build")
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make build failed with extraneous file present:\n%s\nerror: %v", out, err)
	}
}

// TestFullBuildTestCycle verifies clean → build → test cycle works end-to-end.
// Test Spec: TS-01-SMOKE-1
func TestFullBuildTestCycle(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH — skipping full cycle test")
	}
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH — skipping full cycle test")
	}

	root := findRepoRoot(t)

	steps := []string{"clean", "build", "test"}
	for _, step := range steps {
		t.Run(step, func(t *testing.T) {
			cmd := exec.Command("make", step)
			cmd.Dir = root
			out, err := cmd.CombinedOutput()
			if err != nil {
				t.Fatalf("make %s failed:\n%s\nerror: %v", step, out, err)
			}
		})
	}
}

// TestTestIsolationWithoutInfrastructure verifies all tests pass without any
// infrastructure containers running.
// Test Spec: TS-01-P4
// Property: Property 4 (Test Isolation)
func TestTestIsolationWithoutInfrastructure(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH — skipping test isolation test")
	}
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH — skipping test isolation test")
	}

	root := findRepoRoot(t)

	// Attempt to stop infrastructure (ignore errors — it may not be running).
	downCmd := exec.Command("make", "infra-down")
	downCmd.Dir = root
	downCmd.CombinedOutput() //nolint:errcheck

	// Run tests — they should pass without infrastructure.
	testCmd := exec.Command("make", "test")
	testCmd.Dir = root
	out, err := testCmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make test failed without infrastructure:\n%s\nerror: %v", out, err)
	}
}
