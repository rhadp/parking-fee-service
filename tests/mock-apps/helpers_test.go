// Package mock_apps provides integration tests for the six mock CLI tools.
// Tests invoke compiled binaries via os/exec and verify stdout/stderr/exit-code contracts.
// In task group 1, tests FAIL because the binaries have not been implemented yet.
package mock_apps

import (
	"encoding/json"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

// repoRoot returns the absolute path to the repository root.
func repoRoot(t *testing.T) string {
	t.Helper()
	// Walk up from the test file location to find the repo root.
	_, filename, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("could not determine test file location")
	}
	// tests/mock-apps/helpers_test.go → repo root is two dirs up
	return filepath.Join(filepath.Dir(filename), "..", "..")
}

// binDir returns the directory where built mock binaries are placed.
func binDir(t *testing.T) string {
	t.Helper()
	return filepath.Join(repoRoot(t), "tests", "mock-apps", "testdata", "bin")
}

// findBinary returns the path to a compiled mock binary.
// The binary must have been built into tests/mock-apps/testdata/bin/ first.
// Fails the test (t.Fatal) if the binary is not found.
func findBinary(t *testing.T, name string) string {
	t.Helper()
	// Prefer pre-built binary in testdata/bin/
	candidate := filepath.Join(binDir(t), name)
	if runtime.GOOS == "windows" {
		candidate += ".exe"
	}
	if _, err := os.Stat(candidate); err == nil {
		return candidate
	}
	// Fall back to PATH
	path, err := exec.LookPath(name)
	if err != nil {
		t.Fatalf("binary %q not found in testdata/bin/ or PATH; build mock apps first: %v", name, err)
		return ""
	}
	return path
}

// startMockHTTPServer starts a test HTTP server with the given handler.
// Returns the server's URL.
func startMockHTTPServer(t *testing.T, handler http.Handler) string {
	t.Helper()
	srv := httptest.NewServer(handler)
	t.Cleanup(srv.Close)
	return srv.URL
}

// startMockHTTPServerWithCapture starts a test HTTP server that records requests
// and responds with the given JSON response body (status 200).
// Returns (serverURL, requestCapture).
type requestCapture struct {
	Method string
	Path   string
	Body   []byte
	Header http.Header
}

func startMockHTTPServerWithCapture(t *testing.T, responseBody any, statusCode int) (string, *requestCapture) {
	t.Helper()
	cap := &requestCapture{}
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		cap.Method = r.Method
		cap.Path = r.URL.Path + "?" + r.URL.RawQuery
		cap.Header = r.Header.Clone()
		if r.Body != nil {
			buf := make([]byte, 1024*64)
			n, _ := r.Body.Read(buf)
			cap.Body = buf[:n]
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(statusCode)
		if responseBody != nil {
			_ = json.NewEncoder(w).Encode(responseBody)
		}
	}))
	t.Cleanup(srv.Close)
	return srv.URL, cap
}

// startMockTCPListener starts a plain TCP listener on a random local port.
// This is used as a placeholder for gRPC mock servers in task group 1.
// Task group 4 will replace this with real proto-based gRPC mock servers.
func startMockTCPListener(t *testing.T) string {
	t.Helper()
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to start TCP listener: %v", err)
	}
	t.Cleanup(func() { _ = ln.Close() })
	return ln.Addr().String()
}

// runCmd executes the binary with the given args and environment overrides.
// Returns (stdout, stderr, exitCode).
func runCmd(t *testing.T, binary string, args []string, envOverrides map[string]string) (string, string, int) {
	t.Helper()
	cmd := exec.Command(binary, args...)

	// Build environment: start from current env, apply overrides.
	env := os.Environ()
	// Remove any vars we're overriding.
	filtered := env[:0]
	for _, e := range env {
		keep := true
		for k := range envOverrides {
			if strings.HasPrefix(e, k+"=") {
				keep = false
				break
			}
		}
		if keep {
			filtered = append(filtered, e)
		}
	}
	for k, v := range envOverrides {
		if v != "" {
			filtered = append(filtered, k+"="+v)
		}
		// If v == "", the variable is removed (not added back).
	}
	cmd.Env = filtered

	var stdoutBuf, stderrBuf strings.Builder
	cmd.Stdout = &stdoutBuf
	cmd.Stderr = &stderrBuf

	err := cmd.Run()
	exitCode := 0
	if err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			exitCode = exitErr.ExitCode()
		} else {
			t.Fatalf("failed to run %s %v: %v", binary, args, err)
		}
	}
	return stdoutBuf.String(), stderrBuf.String(), exitCode
}
