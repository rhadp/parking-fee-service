package mockapps_test

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"sync"
	"testing"
)

// findRepoRoot walks up from the current directory until it finds .git.
func findRepoRoot(t *testing.T) string {
	t.Helper()
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
			t.Fatal("could not find repo root (.git)")
		}
		dir = parent
	}
}

// buildGoBinary builds a Go binary from the given module directory and returns
// the path to the compiled binary. Each call builds fresh to avoid temp dir
// lifecycle issues.
func buildGoBinary(t *testing.T, moduleDir, binaryName string) string {
	t.Helper()

	tmpDir := t.TempDir()
	outputPath := filepath.Join(tmpDir, binaryName)
	if runtime.GOOS == "windows" {
		outputPath += ".exe"
	}

	cmd := exec.Command("go", "build", "-o", outputPath, ".")
	cmd.Dir = moduleDir
	cmd.Env = append(os.Environ(), "CGO_ENABLED=0")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build %s: %v\noutput: %s", binaryName, err, string(out))
	}

	return outputPath
}

// runBinary executes a binary with the given args and env, returning stdout,
// stderr, and exit code.
func runBinary(t *testing.T, binaryPath string, args []string, env []string) (stdout, stderr string, exitCode int) {
	t.Helper()

	cmd := exec.Command(binaryPath, args...)
	if env != nil {
		cmd.Env = env
	}

	var stdoutBuf, stderrBuf []byte
	stdoutPipe, _ := cmd.StdoutPipe()
	stderrPipe, _ := cmd.StderrPipe()

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start %s: %v", binaryPath, err)
	}

	stdoutBuf, _ = io.ReadAll(stdoutPipe)
	stderrBuf, _ = io.ReadAll(stderrPipe)

	err := cmd.Wait()
	exitCode = 0
	if err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			exitCode = exitErr.ExitCode()
		} else {
			t.Fatalf("failed to wait for %s: %v", binaryPath, err)
		}
	}

	return string(stdoutBuf), string(stderrBuf), exitCode
}

// mockHTTPServer is a test helper that creates a mock HTTP server with
// configurable responses and request capture.
type mockHTTPServer struct {
	server      *httptest.Server
	mu          sync.Mutex
	requests    []capturedRequest
	response    []byte
	statusCode  int
	handlerFunc http.HandlerFunc
}

type capturedRequest struct {
	Method  string
	Path    string
	Body    string
	Headers http.Header
}

func newMockHTTPServer(t *testing.T, statusCode int, response any) *mockHTTPServer {
	t.Helper()

	respBytes, err := json.Marshal(response)
	if err != nil {
		t.Fatalf("failed to marshal mock response: %v", err)
	}

	mock := &mockHTTPServer{
		response:   respBytes,
		statusCode: statusCode,
	}

	mock.server = httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		body, _ := io.ReadAll(r.Body)
		mock.mu.Lock()
		mock.requests = append(mock.requests, capturedRequest{
			Method:  r.Method,
			Path:    r.URL.Path,
			Body:    string(body),
			Headers: r.Header.Clone(),
		})
		mock.mu.Unlock()

		if mock.handlerFunc != nil {
			mock.handlerFunc(w, r)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(mock.statusCode)
		w.Write(mock.response)
	}))

	t.Cleanup(func() { mock.server.Close() })
	return mock
}

func (m *mockHTTPServer) URL() string {
	return m.server.URL
}

func (m *mockHTTPServer) getRequests() []capturedRequest {
	m.mu.Lock()
	defer m.mu.Unlock()
	cp := make([]capturedRequest, len(m.requests))
	copy(cp, m.requests)
	return cp
}

// baseEnv returns an environment with only essential variables (PATH, HOME, etc.)
// to avoid inheriting tokens or addresses from the test runner environment.
func baseEnv() []string {
	env := []string{
		fmt.Sprintf("PATH=%s", os.Getenv("PATH")),
		fmt.Sprintf("HOME=%s", os.Getenv("HOME")),
	}
	if gopath := os.Getenv("GOPATH"); gopath != "" {
		env = append(env, fmt.Sprintf("GOPATH=%s", gopath))
	}
	return env
}
