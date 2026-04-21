// Package setup provides setup verification tests for the parking-fee-service
// monorepo. Tests validate directory structure, workspace configurations,
// skeleton binaries, proto definitions, Makefile targets, and infrastructure
// configuration files.
package setup

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// findRepoRoot locates the repository root by running `git rev-parse --show-toplevel`.
// It fails the test if the root cannot be determined.
func findRepoRoot(t *testing.T) string {
	t.Helper()
	cmd := exec.Command("git", "rev-parse", "--show-toplevel")
	out, err := cmd.Output()
	if err != nil {
		t.Fatalf("failed to find repo root via git: %v", err)
	}
	return strings.TrimSpace(string(out))
}

// assertDirExists fails the test if the given path does not exist as a directory.
func assertDirExists(t *testing.T, path string) {
	t.Helper()
	info, err := os.Stat(path)
	if os.IsNotExist(err) {
		t.Fatalf("expected directory to exist: %s", path)
	}
	if err != nil {
		t.Fatalf("error checking directory %s: %v", path, err)
	}
	if !info.IsDir() {
		t.Fatalf("expected %s to be a directory, but it is a file", path)
	}
}

// assertFileExists fails the test if the given path does not exist as a file.
func assertFileExists(t *testing.T, path string) {
	t.Helper()
	info, err := os.Stat(path)
	if os.IsNotExist(err) {
		t.Fatalf("expected file to exist: %s", path)
	}
	if err != nil {
		t.Fatalf("error checking file %s: %v", path, err)
	}
	if info.IsDir() {
		t.Fatalf("expected %s to be a file, but it is a directory", path)
	}
}

// assertFileContains fails the test if the file does not contain the given substring.
func assertFileContains(t *testing.T, path, substr string) {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("failed to read file %s: %v", path, err)
	}
	if !strings.Contains(string(data), substr) {
		t.Fatalf("expected file %s to contain %q, but it does not", path, substr)
	}
}

// assertFileNonEmpty fails the test if the file is empty (zero bytes).
func assertFileNonEmpty(t *testing.T, path string) {
	t.Helper()
	info, err := os.Stat(path)
	if err != nil {
		t.Fatalf("failed to stat file %s: %v", path, err)
	}
	if info.Size() == 0 {
		t.Fatalf("expected file %s to be non-empty, but it is empty", path)
	}
}

// repoPath joins the repo root with path segments.
func repoPath(root string, parts ...string) string {
	all := append([]string{root}, parts...)
	return filepath.Join(all...)
}
