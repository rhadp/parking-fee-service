package mockapps_test

import (
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"runtime"
	"strings"
	"syscall"
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// TS-09-SMOKE-2: End-to-End Parking Operator Start-Stop
// Requirements: 09-REQ-8.1, 09-REQ-8.2, 09-REQ-8.3, 09-REQ-8.4, 09-REQ-8.5
// ---------------------------------------------------------------------------

func TestSmokeEndToEndParkingOperator(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("SIGTERM not supported on Windows")
	}

	root := findRepoRoot(t)
	moduleDir := filepath.Join(root, "mock", "parking-operator")
	bin := buildGoBinary(t, moduleDir, "parking-operator")

	port := getFreeTestPort(t)
	addr := fmt.Sprintf("http://localhost:%d", port)

	cmd := exec.Command(bin, "serve", fmt.Sprintf("--port=%d", port))
	cmd.Stderr = os.Stderr
	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start parking-operator: %v", err)
	}
	t.Cleanup(func() {
		_ = cmd.Process.Signal(syscall.SIGTERM)
		_ = cmd.Wait()
	})

	// Wait for server to start.
	if !waitForReady(addr+"/parking/status/probe", 5*time.Second) {
		t.Fatal("parking-operator did not start within timeout")
	}

	// Start a session.
	startBody := `{"vehicle_id":"V1","zone_id":"z1","timestamp":1700000000}`
	startResp, err := http.Post(addr+"/parking/start", "application/json",
		strings.NewReader(startBody))
	if err != nil {
		t.Fatalf("POST /parking/start failed: %v", err)
	}
	defer startResp.Body.Close()

	if startResp.StatusCode != 200 {
		t.Fatalf("start: expected 200, got %d", startResp.StatusCode)
	}

	var startResult map[string]any
	startData, _ := io.ReadAll(startResp.Body)
	if err := json.Unmarshal(startData, &startResult); err != nil {
		t.Fatalf("failed to parse start response: %v", err)
	}

	sessionID, _ := startResult["session_id"].(string)
	if sessionID == "" {
		t.Fatal("start response missing session_id")
	}

	uuidRegex := regexp.MustCompile(`^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$`)
	if !uuidRegex.MatchString(sessionID) {
		t.Errorf("session_id %q is not a valid UUID", sessionID)
	}

	// Query session status.
	statusResp, err := http.Get(addr + "/parking/status/" + sessionID)
	if err != nil {
		t.Fatalf("GET /parking/status failed: %v", err)
	}
	defer statusResp.Body.Close()

	if statusResp.StatusCode != 200 {
		t.Fatalf("status: expected 200, got %d", statusResp.StatusCode)
	}

	var statusResult map[string]any
	statusData, _ := io.ReadAll(statusResp.Body)
	if err := json.Unmarshal(statusData, &statusResult); err != nil {
		t.Fatalf("failed to parse status response: %v", err)
	}
	if s, _ := statusResult["status"].(string); s != "active" {
		t.Errorf("expected status 'active', got %q", s)
	}

	// Stop the session (1 hour later).
	stopBody := fmt.Sprintf(`{"session_id":"%s","timestamp":1700003600}`, sessionID)
	stopResp, err := http.Post(addr+"/parking/stop", "application/json",
		strings.NewReader(stopBody))
	if err != nil {
		t.Fatalf("POST /parking/stop failed: %v", err)
	}
	defer stopResp.Body.Close()

	if stopResp.StatusCode != 200 {
		t.Fatalf("stop: expected 200, got %d", stopResp.StatusCode)
	}

	var stopResult map[string]any
	stopData, _ := io.ReadAll(stopResp.Body)
	if err := json.Unmarshal(stopData, &stopResult); err != nil {
		t.Fatalf("failed to parse stop response: %v", err)
	}

	duration, _ := stopResult["duration_seconds"].(float64)
	if duration != 3600 {
		t.Errorf("expected duration_seconds 3600, got %v", duration)
	}

	totalAmount, _ := stopResult["total_amount"].(float64)
	if totalAmount != 2.50 {
		t.Errorf("expected total_amount 2.50, got %v", totalAmount)
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
			t.Errorf("expected exit 0, got: %v", err)
		}
	case <-time.After(10 * time.Second):
		_ = cmd.Process.Kill()
		t.Fatal("server did not shut down within timeout")
	}
}

// ---------------------------------------------------------------------------
// TS-09-SMOKE-3: End-to-End Companion App Lock-Status
// Requirements: 09-REQ-7.1, 09-REQ-7.3, 09-REQ-7.4
// ---------------------------------------------------------------------------

func TestSmokeCompanionAppLockStatus(t *testing.T) {
	// Use a mock HTTP server simulating CLOUD_GATEWAY.
	commandID := "cmd-smoke-1"

	mock := newMockHTTPServer(t, 200, nil)
	mock.handlerFunc = func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		switch {
		case r.Method == "POST" && strings.HasSuffix(r.URL.Path, "/commands"):
			json.NewEncoder(w).Encode(map[string]any{
				"command_id": commandID,
				"status":     "pending",
			})
		case r.Method == "GET" && strings.Contains(r.URL.Path, commandID):
			json.NewEncoder(w).Encode(map[string]any{
				"command_id": commandID,
				"status":     "success",
			})
		default:
			w.WriteHeader(404)
		}
	}

	bin := companionBinary(t)
	env := baseEnv()

	// Step 1: Send lock command.
	lockStdout, lockStderr, lockExit := runBinary(t, bin, []string{
		"lock",
		"--vin=VIN001",
		"--token=test-token",
		"--gateway-addr=" + mock.URL(),
	}, env)

	if lockExit != 0 {
		t.Fatalf("lock: expected exit 0, got %d\nstderr: %s", lockExit, lockStderr)
	}

	if !strings.Contains(lockStdout, commandID) {
		t.Errorf("lock: expected stdout to contain %q, got: %s", commandID, lockStdout)
	}

	// Step 2: Query command status.
	statusStdout, statusStderr, statusExit := runBinary(t, bin, []string{
		"status",
		"--vin=VIN001",
		"--command-id=" + commandID,
		"--token=test-token",
		"--gateway-addr=" + mock.URL(),
	}, env)

	if statusExit != 0 {
		t.Fatalf("status: expected exit 0, got %d\nstderr: %s", statusExit, statusStderr)
	}

	if !strings.Contains(statusStdout, "success") {
		t.Errorf("status: expected stdout to contain 'success', got: %s", statusStdout)
	}
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// getFreeTestPort returns an available TCP port.
func getFreeTestPort(t *testing.T) int {
	t.Helper()
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("getFreeTestPort: %v", err)
	}
	port := l.Addr().(*net.TCPAddr).Port
	l.Close()
	return port
}

// waitForReady polls the given URL until it responds or times out.
func waitForReady(url string, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		resp, err := http.Get(url)
		if err == nil {
			resp.Body.Close()
			return true
		}
		time.Sleep(50 * time.Millisecond)
	}
	return false
}
