package integration

import (
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// repoRoot returns the absolute path to the repository root.
func repoRoot(t *testing.T) string {
	t.Helper()
	// Walk up from the test file to find the repo root (contains .git)
	dir, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get working directory: %v", err)
	}
	for {
		if _, err := os.Stat(filepath.Join(dir, ".git")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatalf("could not find repo root")
		}
		dir = parent
	}
}

// execCommand runs a command and returns stdout, stderr, and exit code.
func execCommand(t *testing.T, name string, args ...string) (string, string, int) {
	t.Helper()
	cmd := exec.Command(name, args...)
	var stdout, stderr strings.Builder
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	err := cmd.Run()
	exitCode := 0
	if err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			exitCode = exitErr.ExitCode()
		} else {
			t.Fatalf("failed to run %s: %v", name, err)
		}
	}
	return stdout.String(), stderr.String(), exitCode
}

// execCommandWithEnv runs a command with additional environment variables.
func execCommandWithEnv(t *testing.T, env map[string]string, name string, args ...string) (string, string, int) {
	t.Helper()
	cmd := exec.Command(name, args...)
	cmd.Env = os.Environ()
	for k, v := range env {
		cmd.Env = append(cmd.Env, fmt.Sprintf("%s=%s", k, v))
	}
	var stdout, stderr strings.Builder
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	err := cmd.Run()
	exitCode := 0
	if err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			exitCode = exitErr.ExitCode()
		} else {
			t.Fatalf("failed to run %s: %v", name, err)
		}
	}
	return stdout.String(), stderr.String(), exitCode
}

// waitForPort waits for a TCP port to become available, with timeout.
// If timeout is 0, a single probe is attempted.
func waitForPort(t *testing.T, host string, port int, timeout time.Duration) bool {
	t.Helper()
	addr := net.JoinHostPort(host, fmt.Sprintf("%d", port))
	if timeout == 0 {
		// Single probe
		conn, err := net.DialTimeout("tcp", addr, 500*time.Millisecond)
		if err == nil {
			conn.Close()
			return true
		}
		return false
	}
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", addr, 200*time.Millisecond)
		if err == nil {
			conn.Close()
			return true
		}
		time.Sleep(100 * time.Millisecond)
	}
	return false
}

// cliBinary returns the path to the built parking-app-cli binary.
func cliBinary(t *testing.T) string {
	t.Helper()
	root := repoRoot(t)
	binary := filepath.Join(root, "mock", "parking-app-cli", "parking-app-cli")
	// Try to build if not exists
	if _, err := os.Stat(binary); os.IsNotExist(err) {
		cmd := exec.Command("go", "build", "-o", binary, ".")
		cmd.Dir = filepath.Join(root, "mock", "parking-app-cli")
		if out, err := cmd.CombinedOutput(); err != nil {
			t.Skipf("could not build parking-app-cli: %v\n%s", err, string(out))
		}
	}
	return binary
}
