package cloud_connectivity_test

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
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

// execResult holds the result of an executed command.
type execResult struct {
	ExitCode int
	Stdout   string
	Stderr   string
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

// httpGetWithAuth performs an HTTP GET request with a bearer token.
func httpGetWithAuth(t *testing.T, url, token string) (statusCode int, body string, err error) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return 0, "", err
	}
	req.Header.Set("Authorization", "Bearer "+token)
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return 0, "", err
	}
	defer resp.Body.Close()
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return resp.StatusCode, "", err
	}
	return resp.StatusCode, string(data), nil
}

// httpPostJSON performs an HTTP POST request with a JSON body and optional bearer token.
func httpPostJSON(t *testing.T, url, jsonBody, token string) (statusCode int, body string, err error) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, url, strings.NewReader(jsonBody))
	if err != nil {
		return 0, "", err
	}
	req.Header.Set("Content-Type", "application/json")
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return 0, "", err
	}
	defer resp.Body.Close()
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return resp.StatusCode, "", err
	}
	return resp.StatusCode, string(data), nil
}

// httpPostJSONWithTimeout performs an HTTP POST with a custom timeout.
func httpPostJSONWithTimeout(t *testing.T, url, jsonBody, token string, timeout time.Duration) (statusCode int, body string, err error) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, url, strings.NewReader(jsonBody))
	if err != nil {
		return 0, "", err
	}
	req.Header.Set("Content-Type", "application/json")
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return 0, "", err
	}
	defer resp.Body.Close()
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return resp.StatusCode, "", err
	}
	return resp.StatusCode, string(data), nil
}

// startProcess starts a command as a background process and returns it.
// The caller is responsible for killing the process.
func startProcess(t *testing.T, root, dir string, env []string, name string, args ...string) *exec.Cmd {
	t.Helper()
	cwd := filepath.Join(root, dir)
	cmd := exec.Command(name, args...)
	cmd.Dir = cwd
	if env != nil {
		cmd.Env = env
	}
	cmd.Stdout = io.Discard
	cmd.Stderr = io.Discard

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start process %s: %v", name, err)
	}
	return cmd
}

// startProcessWithOutput starts a command as a background process and captures
// stdout and stderr. The caller is responsible for killing the process.
func startProcessWithOutput(t *testing.T, root, dir string, env []string, name string, args ...string) (*exec.Cmd, *strings.Builder, *strings.Builder) {
	t.Helper()
	cwd := filepath.Join(root, dir)
	cmd := exec.Command(name, args...)
	cmd.Dir = cwd
	if env != nil {
		cmd.Env = env
	}
	var stdout, stderr strings.Builder
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start process %s: %v", name, err)
	}
	return cmd, &stdout, &stderr
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

// parseJSON parses a JSON string into a map. Returns nil if parsing fails.
func parseJSON(t *testing.T, data string) map[string]interface{} {
	t.Helper()
	var result map[string]interface{}
	if err := json.Unmarshal([]byte(data), &result); err != nil {
		t.Errorf("failed to parse JSON: %v\ndata: %s", err, data)
		return nil
	}
	return result
}

// mqttBrokerPort is the default MQTT broker port for testing.
const mqttBrokerPort = 1883

// defaultAuthToken is the default bearer token for testing.
const defaultAuthToken = "demo-token"

// defaultGatewayPort is the default CLOUD_GATEWAY port for testing.
const defaultGatewayPort = 8081

// skipIfNoMosquitto skips the test if the MQTT broker is not reachable.
func skipIfNoMosquitto(t *testing.T) {
	t.Helper()
	if !portIsOpen(t, mqttBrokerPort) {
		t.Skip("Mosquitto MQTT broker not running on localhost:1883; skipping integration test")
	}
}

// freePort returns an available TCP port on localhost by binding to port 0.
func freePort(t *testing.T) int {
	t.Helper()
	l, err := net.Listen("tcp", "localhost:0")
	if err != nil {
		t.Fatalf("could not find free port: %v", err)
	}
	port := l.Addr().(*net.TCPAddr).Port
	l.Close()
	return port
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
