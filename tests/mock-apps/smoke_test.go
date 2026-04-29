package mockapps_test

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"syscall"
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// TS-09-SMOKE-2: End-to-End Parking Operator Start-Stop
// Requirement: 09-REQ-8.1, 09-REQ-8.2, 09-REQ-8.3
// ---------------------------------------------------------------------------

func TestParkingOperatorSmoke(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("signal tests not supported on Windows")
	}

	root := findRepoRoot(t)
	moduleDir := filepath.Join(root, "mock", "parking-operator")
	bin := buildGoBinary(t, moduleDir, "parking-operator")

	// Find a free port.
	port := getFreePort(t)

	// Start the server.
	cmd := exec.Command(bin, "serve", fmt.Sprintf("--port=%d", port))
	cmd.Env = baseEnv()
	cmd.Stderr = os.Stderr
	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start parking-operator: %v", err)
	}

	// Wait for server to be ready.
	baseURL := fmt.Sprintf("http://localhost:%d", port)
	waitUntilListening(t, baseURL, 5*time.Second)

	// Start a session.
	startBody := `{"vehicle_id":"V1","zone_id":"z1","timestamp":1700000000}`
	startResp := doHTTPPost(t, baseURL+"/parking/start", startBody)
	if startResp.StatusCode != 200 {
		t.Fatalf("start session: expected 200, got %d", startResp.StatusCode)
	}

	var startResult map[string]any
	if err := json.NewDecoder(startResp.Body).Decode(&startResult); err != nil {
		t.Fatalf("failed to parse start response: %v", err)
	}
	startResp.Body.Close()

	sessionID, _ := startResult["session_id"].(string)
	if sessionID == "" {
		t.Fatal("start response missing session_id")
	}

	// Stop the session.
	stopBody := fmt.Sprintf(`{"session_id":"%s","timestamp":1700003600}`, sessionID)
	stopResp := doHTTPPost(t, baseURL+"/parking/stop", stopBody)
	if stopResp.StatusCode != 200 {
		t.Fatalf("stop session: expected 200, got %d", stopResp.StatusCode)
	}

	var stopResult map[string]any
	if err := json.NewDecoder(stopResp.Body).Decode(&stopResult); err != nil {
		t.Fatalf("failed to parse stop response: %v", err)
	}
	stopResp.Body.Close()

	duration, _ := stopResult["duration_seconds"].(float64)
	if duration != 3600 {
		t.Errorf("expected duration_seconds=3600, got %v", duration)
	}

	// Send SIGTERM and verify graceful shutdown.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	if err := cmd.Wait(); err != nil {
		// Exit code 0 expected.
		if exitErr, ok := err.(*exec.ExitError); ok {
			t.Errorf("expected exit 0 after SIGTERM, got %d", exitErr.ExitCode())
		} else {
			t.Errorf("wait error: %v", err)
		}
	}
}

// ---------------------------------------------------------------------------
// TS-09-SMOKE-3: End-to-End Companion App Lock-Status
// Requirement: 09-REQ-7.1, 09-REQ-7.3
// ---------------------------------------------------------------------------

func TestCompanionAppSmoke(t *testing.T) {
	// Start a mock CLOUD_GATEWAY server that tracks state.
	var commandCounter int
	mock := newMockHTTPServer(t, 200, nil)
	mock.handlerFunc = func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(200)

		if r.Method == "POST" {
			commandCounter++
			json.NewEncoder(w).Encode(map[string]any{
				"command_id": fmt.Sprintf("cmd-%d", commandCounter),
				"status":     "pending",
			})
		} else {
			json.NewEncoder(w).Encode(map[string]any{
				"command_id": "cmd-1",
				"status":     "success",
			})
		}
	}

	bin := companionBinary(t)
	env := baseEnv()

	// Lock
	lockStdout, lockStderr, lockExit := runBinary(t, bin, []string{
		"lock", "--vin=VIN001", "--token=test-token",
		"--gateway-addr=" + mock.URL(),
	}, env)

	if lockExit != 0 {
		t.Fatalf("lock: expected exit 0, got %d\nstderr: %s", lockExit, lockStderr)
	}

	// Parse command_id from lock response.
	var lockResp map[string]any
	if err := json.Unmarshal([]byte(strings.TrimSpace(lockStdout)), &lockResp); err != nil {
		t.Fatalf("failed to parse lock response: %v\nstdout: %s", err, lockStdout)
	}
	commandID, _ := lockResp["command_id"].(string)
	if commandID == "" {
		t.Fatal("lock response missing command_id")
	}

	// Status
	statusStdout, statusStderr, statusExit := runBinary(t, bin, []string{
		"status", "--vin=VIN001", "--command-id=" + commandID, "--token=test-token",
		"--gateway-addr=" + mock.URL(),
	}, env)

	if statusExit != 0 {
		t.Fatalf("status: expected exit 0, got %d\nstderr: %s", statusExit, statusStderr)
	}

	if !strings.Contains(statusStdout, "success") {
		t.Errorf("expected 'success' in status output, got: %s", statusStdout)
	}
}

// ---------------------------------------------------------------------------
// TS-09-17: Parking Operator Graceful Shutdown
// Requirement: 09-REQ-8.1
// ---------------------------------------------------------------------------

func TestGracefulShutdown(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("signal tests not supported on Windows")
	}

	root := findRepoRoot(t)
	moduleDir := filepath.Join(root, "mock", "parking-operator")
	bin := buildGoBinary(t, moduleDir, "parking-operator")

	port := getFreePort(t)
	cmd := exec.Command(bin, "serve", fmt.Sprintf("--port=%d", port))
	cmd.Env = baseEnv()
	cmd.Stderr = os.Stderr
	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start parking-operator: %v", err)
	}

	// Wait for it to start listening.
	baseURL := fmt.Sprintf("http://localhost:%d", port)
	waitUntilListening(t, baseURL, 5*time.Second)

	// Send SIGTERM.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	if err := cmd.Wait(); err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			t.Errorf("expected exit 0, got %d", exitErr.ExitCode())
		} else {
			t.Errorf("wait error: %v", err)
		}
	}
}
