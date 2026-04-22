package mockapps_test

import (
	"encoding/json"
	"fmt"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"os/exec"
	"strings"
	"syscall"
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// TS-09-SMOKE-2: End-to-End Parking Operator Start-Stop
// Requirement: 09-REQ-8.1, 09-REQ-8.2, 09-REQ-8.3
// Also covers TS-09-17: Graceful Shutdown
// ---------------------------------------------------------------------------

func TestParkingOperatorSmoke(t *testing.T) {
	binary := buildBinary(t, "parking-operator")

	// Pick a free port.
	port := getFreePort(t)

	// Start the parking-operator serve process.
	cmd := exec.Command(binary, "serve", fmt.Sprintf("--port=%d", port))
	cmd.Stderr = os.Stderr
	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start parking-operator: %v", err)
	}

	// Ensure cleanup on test failure.
	defer func() {
		if cmd.Process != nil {
			_ = cmd.Process.Kill()
		}
	}()

	// Wait until the server is listening.
	waitForServer(t, port, 5*time.Second)

	baseURL := fmt.Sprintf("http://localhost:%d", port)

	// --- Start a parking session ---
	startBody := `{"vehicle_id":"V1","zone_id":"z1","timestamp":1700000000}`
	startResp := httpPost(t, baseURL+"/parking/start", startBody)
	if startResp.StatusCode != http.StatusOK {
		t.Fatalf("start: expected 200, got %d", startResp.StatusCode)
	}

	var startResult map[string]interface{}
	if err := json.NewDecoder(startResp.Body).Decode(&startResult); err != nil {
		t.Fatalf("start: failed to decode response: %v", err)
	}
	startResp.Body.Close()

	sessionID, ok := startResult["session_id"].(string)
	if !ok || sessionID == "" {
		t.Fatal("start: missing session_id in response")
	}

	if startResult["status"] != "active" {
		t.Errorf("start: expected status 'active', got %v", startResult["status"])
	}

	// --- Stop the session (1 hour later) ---
	stopBody := fmt.Sprintf(`{"session_id":"%s","timestamp":1700003600}`, sessionID)
	stopResp := httpPost(t, baseURL+"/parking/stop", stopBody)
	if stopResp.StatusCode != http.StatusOK {
		t.Fatalf("stop: expected 200, got %d", stopResp.StatusCode)
	}

	var stopResult map[string]interface{}
	if err := json.NewDecoder(stopResp.Body).Decode(&stopResult); err != nil {
		t.Fatalf("stop: failed to decode response: %v", err)
	}
	stopResp.Body.Close()

	if dur, ok := stopResult["duration_seconds"].(float64); !ok || int(dur) != 3600 {
		t.Errorf("stop: expected duration_seconds 3600, got %v", stopResult["duration_seconds"])
	}

	if amt, ok := stopResult["total_amount"].(float64); !ok || amt != 2.50 {
		t.Errorf("stop: expected total_amount 2.50, got %v", stopResult["total_amount"])
	}

	// --- Graceful shutdown (TS-09-17) ---
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	// Wait for the process to exit.
	done := make(chan error, 1)
	go func() {
		done <- cmd.Wait()
	}()

	select {
	case err := <-done:
		if err != nil {
			t.Fatalf("parking-operator exited with error after SIGTERM: %v", err)
		}
		// cmd.ProcessState.ExitCode() should be 0.
		if code := cmd.ProcessState.ExitCode(); code != 0 {
			t.Errorf("expected exit code 0 after SIGTERM, got %d", code)
		}
	case <-time.After(10 * time.Second):
		t.Fatal("parking-operator did not shut down within 10 seconds")
	}

	// Prevent double-kill in defer.
	cmd.Process = nil
}

// ---------------------------------------------------------------------------
// TS-09-SMOKE-3: End-to-End Companion App Lock-Status
// Requirement: 09-REQ-7.1, 09-REQ-7.3
// ---------------------------------------------------------------------------

func TestCompanionAppSmoke(t *testing.T) {
	// Start a mock CLOUD_GATEWAY that handles lock and status requests.
	requestCount := 0
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		requestCount++
		w.Header().Set("Content-Type", "application/json")

		if r.Method == "POST" && strings.HasSuffix(r.URL.Path, "/commands") {
			// Lock response
			json.NewEncoder(w).Encode(map[string]string{
				"command_id": "cmd-smoke-1",
				"status":     "pending",
			})
			return
		}

		if r.Method == "GET" && strings.Contains(r.URL.Path, "/commands/") {
			// Status response
			json.NewEncoder(w).Encode(map[string]string{
				"command_id": "cmd-smoke-1",
				"status":     "success",
			})
			return
		}

		w.WriteHeader(http.StatusNotFound)
	}))
	defer srv.Close()

	binary := buildBinary(t, "companion-app-cli")

	// --- Lock command ---
	lockStdout, lockStderr, lockExit := runBinary(t, binary,
		"lock",
		"--vin=VIN001",
		"--token=test-token",
		"--gateway-addr="+srv.URL,
	)

	if lockExit != 0 {
		t.Fatalf("lock: expected exit code 0, got %d\nstderr: %s", lockExit, lockStderr)
	}

	// Parse command_id from lock response.
	var lockResp map[string]string
	if err := json.Unmarshal([]byte(lockStdout), &lockResp); err != nil {
		t.Fatalf("lock: failed to parse stdout as JSON: %v\nstdout: %s", err, lockStdout)
	}

	commandID := lockResp["command_id"]
	if commandID == "" {
		t.Fatal("lock: missing command_id in response")
	}

	// --- Status command ---
	statusStdout, statusStderr, statusExit := runBinary(t, binary,
		"status",
		"--vin=VIN001",
		"--command-id="+commandID,
		"--token=test-token",
		"--gateway-addr="+srv.URL,
	)

	if statusExit != 0 {
		t.Fatalf("status: expected exit code 0, got %d\nstderr: %s", statusExit, statusStderr)
	}

	if !strings.Contains(statusStdout, "success") {
		t.Errorf("status: expected 'success' in stdout, got: %s", statusStdout)
	}

	// Verify both requests were made.
	if requestCount < 2 {
		t.Errorf("expected at least 2 requests to mock server, got %d", requestCount)
	}
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// getFreePort returns an available TCP port.
func getFreePort(t *testing.T) int {
	t.Helper()
	lis, err := net.Listen("tcp", "localhost:0")
	if err != nil {
		t.Fatalf("failed to get free port: %v", err)
	}
	port := lis.Addr().(*net.TCPAddr).Port
	lis.Close()
	return port
}

// waitForServer waits until the given port is accepting connections.
func waitForServer(t *testing.T, port int, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	addr := fmt.Sprintf("localhost:%d", port)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", addr, 100*time.Millisecond)
		if err == nil {
			conn.Close()
			return
		}
		time.Sleep(50 * time.Millisecond)
	}
	t.Fatalf("server on port %d did not start within %v", port, timeout)
}

// httpPost sends a POST request with JSON body.
func httpPost(t *testing.T, url, body string) *http.Response {
	t.Helper()
	resp, err := http.Post(url, "application/json", strings.NewReader(body))
	if err != nil {
		t.Fatalf("POST %s failed: %v", url, err)
	}
	return resp
}

