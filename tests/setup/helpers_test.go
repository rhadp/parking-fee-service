package setup_test

import (
	"os"
	"path/filepath"
	"testing"
)

// repoRoot returns the absolute path to the repository root.
// It walks up from the current working directory until it finds a directory
// containing both a Makefile and a .gitignore (or .git).
func repoRoot(t *testing.T) string {
	t.Helper()

	dir, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get working directory: %v", err)
	}

	// Walk up from tests/setup/ to the repo root.
	// The repo root is two levels up from tests/setup/.
	root := filepath.Join(dir, "..", "..")
	root, err = filepath.Abs(root)
	if err != nil {
		t.Fatalf("failed to resolve repo root: %v", err)
	}

	// Sanity check: the repo root should contain a .gitignore.
	if _, err := os.Stat(filepath.Join(root, ".gitignore")); os.IsNotExist(err) {
		t.Fatalf("repo root %s does not contain .gitignore; test must run from tests/setup/", root)
	}

	return root
}

// pathExists checks if a path exists in the filesystem.
func pathExists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}

// fileContains checks if a file contains a given substring.
func fileContains(t *testing.T, path, substr string) bool {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("failed to read %s: %v", path, err)
	}
	return containsString(string(data), substr)
}

// containsString checks if s contains substr.
func containsString(s, substr string) bool {
	return len(substr) > 0 && len(s) >= len(substr) && indexOf(s, substr) >= 0
}

// indexOf returns the index of substr in s, or -1 if not found.
func indexOf(s, substr string) int {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return i
		}
	}
	return -1
}
