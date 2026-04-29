package setup_test

import (
	"os"
	"path/filepath"
	"testing"
)

// repoRoot returns the absolute path to the repository root.
// tests/setup/ is two levels deep from the repo root.
func repoRoot(t *testing.T) string {
	t.Helper()
	root, err := filepath.Abs("../..")
	if err != nil {
		t.Fatalf("failed to resolve repo root: %v", err)
	}
	return root
}

// assertPathExists fails the test if the given path does not exist.
func assertPathExists(t *testing.T, path string) {
	t.Helper()
	if _, err := os.Stat(path); os.IsNotExist(err) {
		t.Errorf("expected path to exist: %s", path)
	} else if err != nil {
		t.Errorf("error checking path %s: %v", path, err)
	}
}

// assertFileContains fails the test if the file does not exist or does not
// contain the given substring.
func assertFileContains(t *testing.T, path, substr string) {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Errorf("failed to read file %s: %v", path, err)
		return
	}
	content := string(data)
	if !contains(content, substr) {
		t.Errorf("file %s does not contain %q", path, substr)
	}
}

// contains checks whether s contains substr.
func contains(s, substr string) bool {
	return len(substr) > 0 && len(s) >= len(substr) && searchString(s, substr)
}

// searchString performs a simple substring search.
func searchString(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}

// assertPathNotExists fails the test if the given path exists.
func assertPathNotExists(t *testing.T, path string) {
	t.Helper()
	if _, err := os.Stat(path); err == nil {
		t.Errorf("expected path to not exist: %s", path)
	}
}

// readFileContent reads a file and returns its content as a string.
// Fails the test if the file cannot be read.
func readFileContent(t *testing.T, path string) string {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("failed to read file %s: %v", path, err)
	}
	return string(data)
}

// findProtoFiles finds all .proto files recursively under the given directory.
func findProtoFiles(t *testing.T, dir string) []string {
	t.Helper()
	var files []string
	err := filepath.Walk(dir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if !info.IsDir() && filepath.Ext(path) == ".proto" {
			files = append(files, path)
		}
		return nil
	})
	if err != nil {
		t.Fatalf("failed to walk directory %s: %v", dir, err)
	}
	return files
}
