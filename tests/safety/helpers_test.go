package safety_test

import (
	"encoding/json"
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// repoRoot returns the absolute path to the repository root by walking up from
// the test file directory until it finds a .git entry. It calls t.Fatal if the
// root cannot be located.
func repoRoot(t *testing.T) string {
	t.Helper()

	dir, err := os.Getwd()
	if err != nil {
		t.Fatalf("repoRoot: could not get working directory: %v", err)
	}

	for {
		if _, err := os.Stat(filepath.Join(dir, ".git")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("repoRoot: could not find .git directory; are tests running inside the repo?")
		}
		dir = parent
	}
}

// assertDirExists fails the test if the directory at the given path (relative
// to the repo root) does not exist.
func assertDirExists(t *testing.T, root, relPath string) {
	t.Helper()
	full := filepath.Join(root, relPath)
	info, err := os.Stat(full)
	if err != nil {
		t.Errorf("expected directory %q to exist, but got error: %v", relPath, err)
		return
	}
	if !info.IsDir() {
		t.Errorf("expected %q to be a directory, but it is a file", relPath)
	}
}

// assertFileExists fails the test if the file at the given path (relative to
// the repo root) does not exist.
func assertFileExists(t *testing.T, root, relPath string) {
	t.Helper()
	full := filepath.Join(root, relPath)
	info, err := os.Stat(full)
	if err != nil {
		t.Errorf("expected file %q to exist, but got error: %v", relPath, err)
		return
	}
	if info.IsDir() {
		t.Errorf("expected %q to be a file, but it is a directory", relPath)
	}
}

// assertFileContains fails the test if the file at the given path does not
// contain the expected substring.
func assertFileContains(t *testing.T, root, relPath, substr string) {
	t.Helper()
	full := filepath.Join(root, relPath)
	data, err := os.ReadFile(full)
	if err != nil {
		t.Errorf("could not read file %q: %v", relPath, err)
		return
	}
	if !strings.Contains(string(data), substr) {
		t.Errorf("file %q does not contain expected substring %q", relPath, substr)
	}
}

// assertFileContainsAny fails the test if the file at the given path does not
// contain at least one of the expected substrings.
func assertFileContainsAny(t *testing.T, root, relPath string, substrs ...string) {
	t.Helper()
	full := filepath.Join(root, relPath)
	data, err := os.ReadFile(full)
	if err != nil {
		t.Errorf("could not read file %q: %v", relPath, err)
		return
	}
	content := string(data)
	for _, substr := range substrs {
		if strings.Contains(content, substr) {
			return
		}
	}
	t.Errorf("file %q does not contain any of the expected substrings: %v", relPath, substrs)
}

// assertFileNotContains fails the test if the file at the given path contains
// the unexpected substring.
func assertFileNotContains(t *testing.T, root, relPath, substr string) {
	t.Helper()
	full := filepath.Join(root, relPath)
	data, err := os.ReadFile(full)
	if err != nil {
		t.Errorf("could not read file %q: %v", relPath, err)
		return
	}
	if strings.Contains(string(data), substr) {
		t.Errorf("file %q should not contain substring %q, but it does", relPath, substr)
	}
}

// execResult holds the result of an executed command.
type execResult struct {
	ExitCode int
	Stdout   string
	Stderr   string
	Combined string
	Err      error
}

// execCommand runs a command in the given directory (relative to repo root) and
// returns the result. It does NOT fail the test on error — the caller decides.
func execCommand(t *testing.T, root, dir string, name string, args ...string) execResult {
	t.Helper()
	cwd := filepath.Join(root, dir)
	cmd := exec.Command(name, args...)
	cmd.Dir = cwd

	var stdout, stderr strings.Builder
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	err := cmd.Run()
	exitCode := 0
	if err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			exitCode = exitErr.ExitCode()
		} else {
			exitCode = -1
		}
	}

	return execResult{
		ExitCode: exitCode,
		Stdout:   stdout.String(),
		Stderr:   stderr.String(),
		Combined: stdout.String() + stderr.String(),
		Err:      err,
	}
}

// waitForPort waits until a TCP connection to localhost:port succeeds or the
// timeout expires. Returns true if the port became reachable.
func waitForPort(t *testing.T, port int, timeout time.Duration) bool {
	t.Helper()
	deadline := time.Now().Add(timeout)
	addr := fmt.Sprintf("localhost:%d", port)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", addr, 500*time.Millisecond)
		if err == nil {
			conn.Close()
			return true
		}
		time.Sleep(250 * time.Millisecond)
	}
	return false
}

// portIsOpen checks whether a TCP port on localhost is currently accepting
// connections.
func portIsOpen(t *testing.T, port int) bool {
	t.Helper()
	addr := fmt.Sprintf("localhost:%d", port)
	conn, err := net.DialTimeout("tcp", addr, 1*time.Second)
	if err != nil {
		return false
	}
	conn.Close()
	return true
}

// readFile reads a file relative to root and returns its content.
func readFile(t *testing.T, root, relPath string) string {
	t.Helper()
	full := filepath.Join(root, relPath)
	data, err := os.ReadFile(full)
	if err != nil {
		t.Fatalf("could not read file %q: %v", relPath, err)
	}
	return string(data)
}

// globFiles returns file paths matching a glob pattern relative to root.
func globFiles(t *testing.T, root, pattern string) []string {
	t.Helper()
	matches, err := filepath.Glob(filepath.Join(root, pattern))
	if err != nil {
		t.Fatalf("glob error for pattern %q: %v", pattern, err)
	}
	return matches
}

// readAllRustFiles reads all .rs files under a directory (relative to root)
// and returns their concatenated content.
func readAllRustFiles(t *testing.T, root, relDir string) string {
	t.Helper()
	dir := filepath.Join(root, relDir)
	var content strings.Builder
	err := filepath.Walk(dir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if !info.IsDir() && strings.HasSuffix(path, ".rs") {
			data, readErr := os.ReadFile(path)
			if readErr != nil {
				return readErr
			}
			content.Write(data)
			content.WriteString("\n")
		}
		return nil
	})
	if err != nil {
		t.Fatalf("readAllRustFiles: could not walk %q: %v", relDir, err)
	}
	return content.String()
}

// parseJSONFile parses a JSON file and returns the result as a map.
func parseJSONFile(t *testing.T, root, relPath string) map[string]interface{} {
	t.Helper()
	data := readFile(t, root, relPath)
	var result map[string]interface{}
	if err := json.Unmarshal([]byte(data), &result); err != nil {
		t.Fatalf("could not parse JSON file %q: %v", relPath, err)
	}
	return result
}

// navigateJSON navigates a nested map[string]interface{} by dot-separated keys.
// Returns the value at the end of the path, or nil if any key is missing.
func navigateJSON(data map[string]interface{}, path string) interface{} {
	keys := strings.Split(path, ".")
	var current interface{} = data
	for _, key := range keys {
		m, ok := current.(map[string]interface{})
		if !ok {
			return nil
		}
		current, ok = m[key]
		if !ok {
			return nil
		}
	}
	return current
}
