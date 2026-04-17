// Smoke tests for mock Go CLI apps (task group 5 — TS-09-17, TS-09-SMOKE-2, TS-09-SMOKE-3).
//
// TestGracefulShutdown verifies that parking-operator exits 0 on SIGTERM (TS-09-17).
// TestParkingOperatorSmoke runs a full start→stop lifecycle via the parking-operator binary (TS-09-SMOKE-2).
// TestCompanionAppSmoke runs lock→status against a mock CLOUD_GATEWAY via companion-app-cli (TS-09-SMOKE-3).
package mock_apps

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"syscall"
	"testing"
	"time"
)

// ── Go binary builder ─────────────────────────────────────────────────────────

var (
	goBuildMu      sync.Mutex
	goBuildResults = make(map[string]string) // name → binary path or error marker
)

// buildGoMockBinary builds a Go mock binary into testdata/bin/ and returns its path.
// srcDir is relative to the repo root (e.g. "mock/parking-operator").
// name is the output binary name (e.g. "parking-operator").
func buildGoMockBinary(t *testing.T, srcDir, name string) string {
	t.Helper()
	root := repoRoot(t)
	outDir := binDir(t)
	if err := os.MkdirAll(outDir, 0o755); err != nil {
		t.Fatalf("failed to create testdata/bin dir: %v", err)
	}
	outPath := filepath.Join(outDir, name)

	goBuildMu.Lock()
	if cached, ok := goBuildResults[name]; ok {
		goBuildMu.Unlock()
		if strings.HasPrefix(cached, "ERR:") {
			t.Fatalf("failed to build %s (cached): %s", name, cached[4:])
		}
		return cached
	}
	goBuildMu.Unlock()

	cmd := exec.Command("go", "build", "-o", outPath, "./"+srcDir)
	cmd.Dir = root
	out, err := cmd.CombinedOutput()

	goBuildMu.Lock()
	defer goBuildMu.Unlock()
	if err != nil {
		errMsg := fmt.Sprintf("go build %s failed: %v\n%s", srcDir, err, string(out))
		goBuildResults[name] = "ERR:" + errMsg
		t.Fatalf("%s", errMsg)
	}
	goBuildResults[name] = outPath
	return outPath
}

// ── port helpers ──────────────────────────────────────────────────────────────

// freePort returns an available TCP port on 127.0.0.1.
func freePort(t *testing.T) int {
	t.Helper()
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to find free port: %v", err)
	}
	port := ln.Addr().(*net.TCPAddr).Port
	ln.Close()
	return port
}

// waitForTCP polls until addr is listening or deadline expires.
func waitForTCP(t *testing.T, addr string, timeout time.Duration) {
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
	t.Fatalf("timed out waiting for %s to listen", addr)
}

// ── TS-09-17: Graceful shutdown ───────────────────────────────────────────────

// TestGracefulShutdown verifies that parking-operator serve exits 0 on SIGTERM.
func TestGracefulShutdown(t *testing.T) {
	bin := buildGoMockBinary(t, "mock/parking-operator", "parking-operator")
	port := freePort(t)
	portStr := fmt.Sprintf("%d", port)

	cmd := exec.Command(bin, "serve", "--port="+portStr)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start parking-operator: %v", err)
	}

	waitForTCP(t, "127.0.0.1:"+portStr, 10*time.Second)

	// Send SIGTERM.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()

	select {
	case err := <-done:
		if err != nil {
			if exitErr, ok := err.(*exec.ExitError); ok {
				t.Errorf("expected exit code 0 after SIGTERM, got %d", exitErr.ExitCode())
			} else {
				t.Errorf("unexpected error: %v", err)
			}
		}
	case <-time.After(10 * time.Second):
		_ = cmd.Process.Kill()
		t.Fatal("timed out waiting for parking-operator to exit after SIGTERM")
	}
}

// ── TS-09-SMOKE-2: Parking operator start/stop lifecycle ─────────────────────

// TestParkingOperatorSmoke runs a full start→stop lifecycle against the parking-operator binary.
func TestParkingOperatorSmoke(t *testing.T) {
	bin := buildGoMockBinary(t, "mock/parking-operator", "parking-operator")
	port := freePort(t)
	portStr := fmt.Sprintf("%d", port)
	baseURL := fmt.Sprintf("http://127.0.0.1:%d", port)

	cmd := exec.Command(bin, "serve", "--port="+portStr)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start parking-operator: %v", err)
	}
	t.Cleanup(func() { _ = cmd.Process.Kill() })

	waitForTCP(t, "127.0.0.1:"+portStr, 10*time.Second)

	// POST /parking/start
	startBody := `{"vehicle_id":"V1","zone_id":"zone-smoke","timestamp":1700000000}`
	startResp, err := http.Post(baseURL+"/parking/start", "application/json",
		strings.NewReader(startBody))
	if err != nil {
		t.Fatalf("POST /parking/start failed: %v", err)
	}
	defer startResp.Body.Close()
	if startResp.StatusCode != http.StatusOK {
		t.Fatalf("POST /parking/start: expected 200, got %d", startResp.StatusCode)
	}

	var startResult map[string]any
	if err := json.NewDecoder(startResp.Body).Decode(&startResult); err != nil {
		t.Fatalf("decode start response: %v", err)
	}
	sessionID, ok := startResult["session_id"].(string)
	if !ok || sessionID == "" {
		t.Fatalf("expected session_id in start response, got: %v", startResult)
	}
	if startResult["status"] != "active" {
		t.Errorf("expected status 'active', got: %v", startResult["status"])
	}

	// POST /parking/stop
	stopBody, _ := json.Marshal(map[string]any{
		"session_id": sessionID,
		"timestamp":  1700003600, // 1 hour later
	})
	stopResp, err := http.Post(baseURL+"/parking/stop", "application/json",
		bytes.NewReader(stopBody))
	if err != nil {
		t.Fatalf("POST /parking/stop failed: %v", err)
	}
	defer stopResp.Body.Close()
	if stopResp.StatusCode != http.StatusOK {
		t.Fatalf("POST /parking/stop: expected 200, got %d", stopResp.StatusCode)
	}

	var stopResult map[string]any
	if err := json.NewDecoder(stopResp.Body).Decode(&stopResult); err != nil {
		t.Fatalf("decode stop response: %v", err)
	}
	if stopResult["status"] != "stopped" {
		t.Errorf("expected status 'stopped', got: %v", stopResult["status"])
	}
	if dur, ok := stopResult["duration_seconds"].(float64); !ok || dur != 3600 {
		t.Errorf("expected duration_seconds 3600, got: %v", stopResult["duration_seconds"])
	}

	// Graceful shutdown.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}
	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()
	select {
	case err := <-done:
		if err != nil {
			if exitErr, ok := err.(*exec.ExitError); ok {
				t.Errorf("expected exit code 0 after SIGTERM, got %d", exitErr.ExitCode())
			} else {
				t.Errorf("unexpected shutdown error: %v", err)
			}
		}
	case <-time.After(10 * time.Second):
		t.Fatal("timed out waiting for parking-operator to shut down")
	}
}

// ── TS-09-SMOKE-3: Companion app lock→status lifecycle ───────────────────────

// TestCompanionAppSmoke tests the lock → status sequence via companion-app-cli
// against a mock CLOUD_GATEWAY.
func TestCompanionAppSmoke(t *testing.T) {
	bin := buildGoMockBinary(t, "mock/companion-app-cli", "companion-app-cli")

	// Build a stateful mock CLOUD_GATEWAY that records the lock command and
	// serves a status query.
	type commandState struct {
		mu        sync.Mutex
		commandID string
	}
	state := &commandState{}

	mux := http.NewServeMux()
	mux.HandleFunc("/vehicles/VIN001/commands", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}
		cmdID := "cmd-smoke-1"
		state.mu.Lock()
		state.commandID = cmdID
		state.mu.Unlock()

		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]string{
			"command_id": cmdID,
			"status":     "pending",
		})
	})
	mux.HandleFunc("/vehicles/VIN001/commands/cmd-smoke-1", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]string{
			"command_id": "cmd-smoke-1",
			"status":     "success",
		})
	})

	mockSrv := httptest.NewServer(mux)
	defer mockSrv.Close()

	// Run lock command.
	stdout, stderr, code := runCmd(t, bin,
		[]string{"lock", "--vin=VIN001", "--token=smoke-token", "--gateway-addr=" + mockSrv.URL},
		nil,
	)
	if code != 0 {
		t.Fatalf("companion-app-cli lock exited %d; stderr: %s", code, stderr)
	}
	if !strings.Contains(stdout, "cmd-smoke-1") {
		t.Errorf("expected 'cmd-smoke-1' in lock stdout, got: %q", stdout)
	}

	// Run status command.
	stdout2, stderr2, code2 := runCmd(t, bin,
		[]string{"status", "--vin=VIN001", "--command-id=cmd-smoke-1",
			"--token=smoke-token", "--gateway-addr=" + mockSrv.URL},
		nil,
	)
	if code2 != 0 {
		t.Fatalf("companion-app-cli status exited %d; stderr: %s", code2, stderr2)
	}
	if !strings.Contains(stdout2, "success") {
		t.Errorf("expected 'success' in status stdout, got: %q", stdout2)
	}
}
