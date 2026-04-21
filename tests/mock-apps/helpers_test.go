// Package mockappstests provides integration tests for mock CLI apps.
package mockappstests

import (
	"encoding/json"
	"io"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// findProjectRoot walks up from the current directory to find the go.work file,
// which identifies the project root.
func findProjectRoot(t *testing.T) string {
	t.Helper()
	dir, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	for {
		if _, err := os.Stat(filepath.Join(dir, "go.work")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("could not find project root (go.work not found)")
		}
		dir = parent
	}
}

// buildBinary compiles a Go binary from the mock/ directory and returns its path.
// The binary is placed in a temp directory that is cleaned up when the test ends.
func buildBinary(t *testing.T, name string) string {
	t.Helper()
	root := findProjectRoot(t)
	binDir := t.TempDir()
	binPath := filepath.Join(binDir, name)

	pkgDir := filepath.Join(root, "mock", name)
	cmd := exec.Command("go", "build", "-o", binPath, pkgDir)
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("build %s: %v\n%s", name, err, out)
	}
	return binPath
}

// mockHTTPServer creates a test HTTP server that records the last received request
// and serves the given response JSON.
type mockHTTPServer struct {
	ts          *httptest.Server
	lastMethod  string
	lastPath    string
	lastBody    []byte
	lastHeaders http.Header
	respStatus  int
	respBody    string
}

// newMockHTTPServer creates a new mock HTTP server returning the given status and body.
func newMockHTTPServer(t *testing.T, status int, body string) *mockHTTPServer {
	t.Helper()
	m := &mockHTTPServer{respStatus: status, respBody: body}
	m.ts = httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		m.lastMethod = r.Method
		m.lastPath = r.URL.Path
		if r.URL.RawQuery != "" {
			m.lastPath += "?" + r.URL.RawQuery
		}
		m.lastHeaders = r.Header.Clone()
		body, _ := io.ReadAll(r.Body)
		m.lastBody = body
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(m.respStatus)
		w.Write([]byte(m.respBody)) //nolint
	}))
	t.Cleanup(m.ts.Close)
	return m
}

// URL returns the base URL of the mock server.
func (m *mockHTTPServer) URL() string { return m.ts.URL }

// Header returns the value of a request header received by the mock server.
func (m *mockHTTPServer) Header(key string) string { return m.lastHeaders.Get(key) }

// decodeLastBody decodes the last received request body as JSON.
func (m *mockHTTPServer) decodeLastBody(t *testing.T) map[string]any {
	t.Helper()
	var v map[string]any
	if err := json.Unmarshal(m.lastBody, &v); err != nil {
		t.Fatalf("decode request body: %v (body=%q)", err, m.lastBody)
	}
	return v
}

// waitForPort polls until the given TCP port is listening or the deadline is reached.
func waitForPort(addr string, timeout time.Duration) error {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", addr, 100*time.Millisecond)
		if err == nil {
			conn.Close()
			return nil
		}
		time.Sleep(50 * time.Millisecond)
	}
	return &timeoutError{addr: addr}
}

type timeoutError struct{ addr string }

func (e *timeoutError) Error() string { return "timeout waiting for " + e.addr + " to listen" }

// runBinary runs the given binary with args, captures stdout and stderr,
// and returns the exit code. Returns -1 if the process could not be started.
func runBinary(t *testing.T, binary string, args ...string) (stdout, stderr string, exitCode int) {
	t.Helper()
	cmd := exec.Command(binary, args...)
	var outBuf, errBuf strings.Builder
	cmd.Stdout = &outBuf
	cmd.Stderr = &errBuf
	err := cmd.Run()
	stdout = outBuf.String()
	stderr = errBuf.String()
	if err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			exitCode = exitErr.ExitCode()
		} else {
			exitCode = -1
		}
	} else {
		exitCode = 0
	}
	return
}
