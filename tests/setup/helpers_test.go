package setup_test

import (
	"os"
	"path/filepath"
	"runtime"
	"testing"
)

// repoRoot walks up from the test file's directory until it finds the repository
// root (identified by the presence of a .git file or directory).
func repoRoot(t *testing.T) string {
	t.Helper()
	_, filename, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller failed")
	}
	dir := filepath.Dir(filename)
	for {
		if _, err := os.Stat(filepath.Join(dir, ".git")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("could not find repository root (no .git found)")
		}
		dir = parent
	}
}

// assertPathExists fails the test if the given path does not exist.
func assertPathExists(t *testing.T, path string) {
	t.Helper()
	if _, err := os.Stat(path); os.IsNotExist(err) {
		t.Errorf("expected path to exist: %s", path)
	} else if err != nil {
		t.Errorf("unexpected error checking path %s: %v", path, err)
	}
}

// assertFileContains fails the test if the file at path does not contain the
// given substring.
func assertFileContains(t *testing.T, path, substr string) {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Errorf("cannot read file %s: %v", path, err)
		return
	}
	if !containsStr(string(data), substr) {
		t.Errorf("expected file %s to contain %q", path, substr)
	}
}

func containsStr(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(substr) == 0 ||
		findSubstr(s, substr))
}

func findSubstr(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}
