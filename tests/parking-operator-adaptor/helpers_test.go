// Package parkingoperatoradaptor contains integration tests for the
// parking-operator-adaptor Rust binary.
//
// Tests build the Rust binary via `cargo build`, start it as a subprocess,
// and verify startup logging, graceful shutdown, and DATA_BROKER retry behavior.
package parkingoperatoradaptor

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"testing"
	"time"
)

// repoRoot returns the absolute path to the repository root.
// The tests live in tests/parking-operator-adaptor/ → root is ../../
func repoRoot(t *testing.T) string {
	t.Helper()
	_, filename, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller failed")
	}
	root := filepath.Join(filepath.Dir(filename), "..", "..")
	abs, err := filepath.Abs(root)
	if err != nil {
		t.Fatalf("failed to resolve repo root: %v", err)
	}
	return abs
}

// buildAdaptorBinary builds the parking-operator-adaptor Rust binary and
// returns the path to the compiled binary.
func buildAdaptorBinary(t *testing.T) string {
	t.Helper()
	root := repoRoot(t)
	rhivosDir := filepath.Join(root, "rhivos")
	cmd := exec.Command("cargo", "build", "-p", "parking-operator-adaptor")
	cmd.Dir = rhivosDir
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("cargo build -p parking-operator-adaptor failed: %v\n%s", err, string(out))
	}
	binPath := filepath.Join(rhivosDir, "target", "debug", "parking-operator-adaptor")
	if _, err := os.Stat(binPath); err != nil {
		t.Fatalf("expected binary at %s but stat failed: %v", binPath, err)
	}
	return binPath
}

// findFreePort returns an available TCP port on localhost.
func findFreePort(t *testing.T) int {
	t.Helper()
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to find free port: %v", err)
	}
	port := ln.Addr().(*net.TCPAddr).Port
	ln.Close()
	return port
}

// mockParkingOperator starts a minimal HTTP server that responds to
// POST /parking/start and POST /parking/stop with valid JSON responses.
// Returns the base URL and a cleanup function.
func mockParkingOperator(t *testing.T) string {
	t.Helper()
	mux := http.NewServeMux()

	mux.HandleFunc("POST /parking/start", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		resp := map[string]interface{}{
			"session_id": "test-session-001",
			"status":     "active",
			"rate": map[string]interface{}{
				"rate_type": "per_hour",
				"amount":    2.50,
				"currency":  "EUR",
			},
		}
		json.NewEncoder(w).Encode(resp)
	})

	mux.HandleFunc("POST /parking/stop", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		resp := map[string]interface{}{
			"session_id":       "test-session-001",
			"status":           "completed",
			"duration_seconds": 3600,
			"total_amount":     2.50,
			"currency":         "EUR",
		}
		json.NewEncoder(w).Encode(resp)
	})

	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to start mock operator: %v", err)
	}

	server := &http.Server{Handler: mux}
	go server.Serve(ln)
	t.Cleanup(func() { server.Close() })

	return fmt.Sprintf("http://127.0.0.1:%d", ln.Addr().(*net.TCPAddr).Port)
}

// startAdaptor starts the parking-operator-adaptor binary with the given
// environment variables and returns the command (for signal sending) plus
// captured stdout/stderr buffers.
func startAdaptor(t *testing.T, binPath string, env []string) (*exec.Cmd, *bytes.Buffer, *bytes.Buffer) {
	t.Helper()
	cmd := exec.Command(binPath)
	cmd.Env = env

	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start adaptor: %v", err)
	}

	t.Cleanup(func() {
		if cmd.ProcessState == nil || !cmd.ProcessState.Exited() {
			_ = cmd.Process.Kill()
			_ = cmd.Wait()
		}
	})

	return cmd, &stdout, &stderr
}

// waitForGRPCReady polls the given TCP address until a connection succeeds
// or the timeout elapses.
func waitForGRPCReady(t *testing.T, addr string, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", addr, 200*time.Millisecond)
		if err == nil {
			conn.Close()
			return
		}
		time.Sleep(100 * time.Millisecond)
	}
	t.Fatalf("gRPC server at %s did not become ready within %v", addr, timeout)
}
