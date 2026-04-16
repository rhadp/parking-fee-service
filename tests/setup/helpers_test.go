package setup_test

import (
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

// repoRoot returns the absolute path to the repository root.
// It navigates two levels up from the directory containing this test file
// (tests/setup/ -> tests/ -> repo root).
func repoRoot(t *testing.T) string {
	t.Helper()
	_, filename, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller failed")
	}
	// filename is .../tests/setup/helpers_test.go
	root := filepath.Join(filepath.Dir(filename), "..", "..")
	abs, err := filepath.Abs(root)
	if err != nil {
		t.Fatalf("filepath.Abs failed: %v", err)
	}
	return abs
}

// assertPathExists fails the test if the given path does not exist.
func assertPathExists(t *testing.T, path string) {
	t.Helper()
	if _, err := os.Stat(path); os.IsNotExist(err) {
		t.Errorf("expected path to exist: %s", path)
	} else if err != nil {
		t.Errorf("error stat-ing %s: %v", path, err)
	}
}

// assertDirExists fails the test if the given path does not exist or is not a directory.
func assertDirExists(t *testing.T, path string) {
	t.Helper()
	info, err := os.Stat(path)
	if os.IsNotExist(err) {
		t.Errorf("expected directory to exist: %s", path)
		return
	}
	if err != nil {
		t.Errorf("error stat-ing %s: %v", path, err)
		return
	}
	if !info.IsDir() {
		t.Errorf("expected %s to be a directory, but it is a file", path)
	}
}

// assertFileContains fails the test if the file does not exist or does not contain
// the given substring.
func assertFileContains(t *testing.T, path, substring string) {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Errorf("cannot read file %s: %v", path, err)
		return
	}
	if !strings.Contains(string(data), substring) {
		t.Errorf("file %s does not contain %q", path, substring)
	}
}

// assertFileNotEmpty fails the test if the file does not exist or is empty.
func assertFileNotEmpty(t *testing.T, path string) {
	t.Helper()
	info, err := os.Stat(path)
	if err != nil {
		t.Errorf("cannot stat file %s: %v", path, err)
		return
	}
	if info.Size() == 0 {
		t.Errorf("file %s is empty", path)
	}
}
