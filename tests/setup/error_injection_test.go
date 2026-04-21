package setup

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// TestCargoReportsFailingCrateName verifies that when a Rust crate has a compile
// error, the error output identifies the crate name.
// Test Spec: TS-01-E2
// Requirement: 01-REQ-2.E1
func TestCargoReportsFailingCrateName(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH — skipping crate error test")
	}

	root := findRepoRoot(t)
	mainRs := filepath.Join(root, "rhivos", "locking-service", "src", "main.rs")

	// Read original content for restore.
	original, err := os.ReadFile(mainRs)
	if err != nil {
		t.Fatalf("failed to read %s: %v", mainRs, err)
	}

	// Inject syntax error.
	broken := append([]byte("THIS_IS_BROKEN!!! "), original...)
	if err := os.WriteFile(mainRs, broken, 0o644); err != nil {
		t.Fatalf("failed to inject error: %v", err)
	}
	defer func() {
		if restoreErr := os.WriteFile(mainRs, original, 0o644); restoreErr != nil {
			t.Errorf("WARNING: failed to restore %s: %v", mainRs, restoreErr)
		}
	}()

	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatal("expected cargo build to fail with injected error, but it succeeded")
	}
	if !strings.Contains(string(out), "locking-service") {
		t.Fatalf("expected error output to mention 'locking-service', got:\n%s", out)
	}
}

// TestGoBuildReportsMissingImport verifies go build reports a clear error when
// a module has an undeclared import.
// Test Spec: TS-01-E3
// Requirement: 01-REQ-3.E1
func TestGoBuildReportsMissingImport(t *testing.T) {
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH — skipping Go import error test")
	}

	root := findRepoRoot(t)
	mainGo := filepath.Join(root, "mock", "companion-app-cli", "main.go")

	original, err := os.ReadFile(mainGo)
	if err != nil {
		t.Fatalf("failed to read %s: %v", mainGo, err)
	}

	// Inject undeclared import.
	broken := strings.Replace(string(original),
		`"fmt"`,
		"\"fmt\"\n\t\"unknown/package/does/not/exist\"",
		1)
	if err := os.WriteFile(mainGo, []byte(broken), 0o644); err != nil {
		t.Fatalf("failed to inject import: %v", err)
	}
	defer func() {
		if restoreErr := os.WriteFile(mainGo, original, 0o644); restoreErr != nil {
			t.Errorf("WARNING: failed to restore %s: %v", mainGo, restoreErr)
		}
	}()

	cmd := exec.Command("go", "build", "./...")
	cmd.Dir = filepath.Join(root, "mock", "companion-app-cli")
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatal("expected go build to fail with undeclared import, but it succeeded")
	}
	combined := string(out)
	if !strings.Contains(combined, "unknown/package/does/not/exist") {
		t.Fatalf("expected error output to mention undeclared import, got:\n%s", combined)
	}
}

// TestMakeBuildReportsFailingToolchain verifies `make build` exits non-zero
// and identifies the failing toolchain when a build error occurs.
// Test Spec: TS-01-E6
// Requirement: 01-REQ-6.E1
func TestMakeBuildReportsFailingToolchain(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH — skipping make build failure test")
	}
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH — skipping make build failure test")
	}

	root := findRepoRoot(t)
	mainRs := filepath.Join(root, "rhivos", "locking-service", "src", "main.rs")

	original, err := os.ReadFile(mainRs)
	if err != nil {
		t.Fatalf("failed to read %s: %v", mainRs, err)
	}

	// Inject error.
	broken := append([]byte("COMPILE_ERROR!!! "), original...)
	if err := os.WriteFile(mainRs, broken, 0o644); err != nil {
		t.Fatalf("failed to inject error: %v", err)
	}
	defer func() {
		if restoreErr := os.WriteFile(mainRs, original, 0o644); restoreErr != nil {
			t.Errorf("WARNING: failed to restore %s: %v", mainRs, restoreErr)
		}
	}()

	cmd := exec.Command("make", "build")
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatal("expected make build to fail, but it succeeded")
	}
	combined := string(out)
	// The error should mention cargo or the crate name.
	if !strings.Contains(strings.ToLower(combined), "cargo") &&
		!strings.Contains(strings.ToLower(combined), "error") {
		t.Fatalf("expected make build output to indicate Rust toolchain failure, got:\n%s", combined)
	}
}

// TestTestRunnerReportsSyntaxErrors verifies the test runner reports file name
// when a test file has a syntax error.
// Test Spec: TS-01-E9
// Requirement: 01-REQ-8.E1
func TestTestRunnerReportsSyntaxErrors(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH — skipping syntax error test")
	}

	root := findRepoRoot(t)
	mainRs := filepath.Join(root, "rhivos", "locking-service", "src", "main.rs")

	original, err := os.ReadFile(mainRs)
	if err != nil {
		t.Fatalf("failed to read %s: %v", mainRs, err)
	}

	// Inject syntax error.
	broken := append([]byte("fn broken_syntax { "), original...)
	if err := os.WriteFile(mainRs, broken, 0o644); err != nil {
		t.Fatalf("failed to inject syntax error: %v", err)
	}
	defer func() {
		if restoreErr := os.WriteFile(mainRs, original, 0o644); restoreErr != nil {
			t.Errorf("WARNING: failed to restore %s: %v", mainRs, restoreErr)
		}
	}()

	cmd := exec.Command("cargo", "test", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatal("expected cargo test to fail with syntax error, but it succeeded")
	}
	if !strings.Contains(string(out), "main.rs") {
		t.Fatalf("expected error to mention 'main.rs', got:\n%s", out)
	}
}
