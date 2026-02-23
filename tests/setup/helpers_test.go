package setup_test

import (
	"bufio"
	"context"
	"fmt"
	"net"
	"net/http"
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

	// Start from the directory containing this test file.
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
	Combined string // stdout + stderr interleaved
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

// execCommandWithEnv runs a command with a custom environment.
func execCommandWithEnv(t *testing.T, root, dir string, env []string, name string, args ...string) execResult {
	t.Helper()
	cwd := filepath.Join(root, dir)
	cmd := exec.Command(name, args...)
	cmd.Dir = cwd
	cmd.Env = env

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

// httpGet performs an HTTP GET request and returns the status code and body.
func httpGet(t *testing.T, url string) (statusCode int, body string, err error) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return 0, "", err
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return 0, "", err
	}
	defer resp.Body.Close()
	scanner := bufio.NewScanner(resp.Body)
	var sb strings.Builder
	for scanner.Scan() {
		sb.WriteString(scanner.Text())
	}
	return resp.StatusCode, sb.String(), nil
}

// pathWithout returns the current PATH with directories containing the named
// binary removed.
func pathWithout(t *testing.T, binaryNames ...string) string {
	t.Helper()
	origPath := os.Getenv("PATH")
	dirs := strings.Split(origPath, string(os.PathListSeparator))

	var filtered []string
	for _, dir := range dirs {
		exclude := false
		for _, name := range binaryNames {
			if _, err := os.Stat(filepath.Join(dir, name)); err == nil {
				exclude = true
				break
			}
		}
		if !exclude {
			filtered = append(filtered, dir)
		}
	}
	return strings.Join(filtered, string(os.PathListSeparator))
}

// envWithPath returns a copy of the current environment with PATH replaced.
func envWithPath(newPath string) []string {
	env := os.Environ()
	var result []string
	for _, e := range env {
		if strings.HasPrefix(e, "PATH=") {
			continue
		}
		result = append(result, e)
	}
	result = append(result, "PATH="+newPath)
	return result
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

// lookPath wraps exec.LookPath for use in tests.
func lookPath(name string) (string, error) {
	return exec.LookPath(name)
}

// relPath wraps filepath.Rel.
func relPath(basePath, targPath string) (string, error) {
	return filepath.Rel(basePath, targPath)
}
