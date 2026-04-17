// End-to-end smoke tests for mock tools.
//
// TS-09-SMOKE-2: parking-operator serve handles a full start-stop lifecycle
//   via subprocess: start server → POST /parking/start → POST /parking/stop →
//   verify duration and total_amount → SIGTERM → exit 0.
//
// TS-09-SMOKE-3: companion-app-cli sends lock command and queries status via
//   a mock CLOUD_GATEWAY, verifying command_id propagation.
package integration

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/http/httptest"
	"os/exec"
	"syscall"
	"testing"
	"time"
)

// doHTTP performs an HTTP request (method, url, optional JSON body) and returns
// the response body bytes and status code.
func doHTTP(t *testing.T, method, url string, body any) (int, []byte) {
	t.Helper()
	var reqBody io.Reader
	if body != nil {
		data, err := json.Marshal(body)
		if err != nil {
			t.Fatalf("json.Marshal: %v", err)
		}
		reqBody = bytes.NewReader(data)
	}
	req, err := http.NewRequest(method, url, reqBody)
	if err != nil {
		t.Fatalf("http.NewRequest: %v", err)
	}
	if body != nil {
		req.Header.Set("Content-Type", "application/json")
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("HTTP %s %s: %v", method, url, err)
	}
	defer resp.Body.Close()
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		t.Fatalf("read response body: %v", err)
	}
	return resp.StatusCode, data
}

// findFreePort returns an unused TCP port on localhost.
func findFreePort(t *testing.T) int {
	t.Helper()
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("findFreePort: %v", err)
	}
	port := l.Addr().(*net.TCPAddr).Port
	l.Close()
	return port
}

// waitForPortReady polls addr until a TCP connection succeeds or timeout expires.
func waitForPortReady(addr string, timeout time.Duration) error {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", addr, 100*time.Millisecond)
		if err == nil {
			conn.Close()
			return nil
		}
		time.Sleep(50 * time.Millisecond)
	}
	return fmt.Errorf("timeout waiting for %s to be ready", addr)
}

// ── TS-09-SMOKE-2: parking-operator full lifecycle ────────────────────────

// TestParkingOperatorSmoke verifies the full start-stop lifecycle of the
// parking-operator server binary:
//
//  1. Start `parking-operator serve` on a free port.
//  2. POST /parking/start → get session_id and verify active status.
//  3. POST /parking/stop  → verify duration_seconds and total_amount.
//  4. GET  /parking/status/{id} → verify stopped status.
//  5. Send SIGTERM → verify exit 0.
//
// Test Spec: TS-09-SMOKE-2
// Requirements: 09-REQ-8.1, 09-REQ-8.2, 09-REQ-8.3, 09-REQ-8.4, 09-REQ-8.5
func TestParkingOperatorSmoke(t *testing.T) {
	binary := buildBinary(t, parkingOperatorPkg)
	port := findFreePort(t)
	addr := fmt.Sprintf("127.0.0.1:%d", port)
	base := fmt.Sprintf("http://%s", addr)

	cmd := exec.Command(binary, "serve", fmt.Sprintf("--port=%d", port))
	if err := cmd.Start(); err != nil {
		t.Fatalf("start parking-operator: %v", err)
	}

	// Wait for server readiness.
	if err := waitForPortReady(addr, 5*time.Second); err != nil {
		cmd.Process.Kill() //nolint
		t.Fatalf("parking-operator did not start: %v", err)
	}

	// ── Step 1: POST /parking/start ──────────────────────────────────────
	const startTS int64 = 1700000000
	const stopTS int64 = 1700003600 // 1 hour later
	startPayload := map[string]any{
		"vehicle_id": "VIN001",
		"zone_id":    "zone-smoke-1",
		"timestamp":  startTS,
	}
	code, body := doHTTP(t, http.MethodPost, base+"/parking/start", startPayload)
	if code != http.StatusOK {
		cmd.Process.Kill() //nolint
		t.Fatalf("POST /parking/start: expected 200, got %d\nbody: %s", code, body)
	}

	var startResp map[string]any
	if err := json.Unmarshal(body, &startResp); err != nil {
		cmd.Process.Kill() //nolint
		t.Fatalf("decode start response: %v", err)
	}
	sessionID, _ := startResp["session_id"].(string)
	if sessionID == "" {
		cmd.Process.Kill() //nolint
		t.Fatalf("start response missing session_id; got: %s", body)
	}
	if startResp["status"] != "active" {
		cmd.Process.Kill() //nolint
		t.Fatalf("expected status=active; got: %v", startResp["status"])
	}

	// ── Step 2: POST /parking/stop ───────────────────────────────────────
	stopPayload := map[string]any{
		"session_id": sessionID,
		"timestamp":  stopTS,
	}
	code, body = doHTTP(t, http.MethodPost, base+"/parking/stop", stopPayload)
	if code != http.StatusOK {
		cmd.Process.Kill() //nolint
		t.Fatalf("POST /parking/stop: expected 200, got %d\nbody: %s", code, body)
	}

	var stopResp map[string]any
	if err := json.Unmarshal(body, &stopResp); err != nil {
		cmd.Process.Kill() //nolint
		t.Fatalf("decode stop response: %v", err)
	}
	if stopResp["status"] != "stopped" {
		t.Errorf("expected status=stopped; got: %v", stopResp["status"])
	}
	dur, _ := stopResp["duration_seconds"].(float64)
	if dur != float64(stopTS-startTS) {
		t.Errorf("expected duration_seconds=%d, got: %v", stopTS-startTS, dur)
	}
	amt, _ := stopResp["total_amount"].(float64)
	expectedAmt := 2.50 * float64(stopTS-startTS) / 3600.0
	if amt < expectedAmt-0.01 || amt > expectedAmt+0.01 {
		t.Errorf("expected total_amount≈%.4f, got: %v", expectedAmt, amt)
	}

	// ── Step 3: GET /parking/status/{id} ─────────────────────────────────
	code, body = doHTTP(t, http.MethodGet, base+"/parking/status/"+sessionID, nil)
	if code != http.StatusOK {
		t.Errorf("GET /parking/status/%s: expected 200, got %d\nbody: %s", sessionID, code, body)
	} else {
		var statusResp map[string]any
		if err := json.Unmarshal(body, &statusResp); err == nil {
			if statusResp["session_id"] != sessionID {
				t.Errorf("status: expected session_id=%q, got: %v", sessionID, statusResp["session_id"])
			}
			if statusResp["status"] != "stopped" {
				t.Errorf("status: expected status=stopped, got: %v", statusResp["status"])
			}
		}
	}

	// ── Step 4: SIGTERM → exit 0 ─────────────────────────────────────────
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("SIGTERM: %v", err)
	}
	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()
	select {
	case err := <-done:
		if err != nil {
			t.Fatalf("parking-operator: expected exit 0 after SIGTERM, got: %v", err)
		}
	case <-time.After(5 * time.Second):
		cmd.Process.Kill() //nolint
		t.Fatal("parking-operator did not exit within 5s after SIGTERM")
	}
}

// ── TS-09-SMOKE-3: companion-app-cli lock → status sequence ───────────────

// dualMethodServer creates an httptest.Server that serves different JSON
// responses for POST (lockResp) and GET (statusResp) requests.
func dualMethodServer(t *testing.T, lockResp, statusResp string) *httptest.Server {
	t.Helper()
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		if r.Method == http.MethodGet {
			io.WriteString(w, statusResp) //nolint
		} else {
			io.WriteString(w, lockResp) //nolint
		}
	}))
	t.Cleanup(srv.Close)
	return srv
}

// TestCompanionAppSmoke verifies the full lock-then-status sequence:
//
//  1. Run `companion-app-cli lock --vin=VIN001` against a mock CLOUD_GATEWAY.
//  2. Parse command_id from the lock response.
//  3. Run `companion-app-cli status --vin=VIN001 --command-id=<id>`.
//  4. Verify status response is printed to stdout.
//
// Test Spec: TS-09-SMOKE-3
// Requirements: 09-REQ-7.1, 09-REQ-7.3, 09-REQ-7.4
func TestCompanionAppSmoke(t *testing.T) {
	const commandID = "smoke-cmd-1"
	lockJSON := fmt.Sprintf(`{"command_id":%q,"status":"pending"}`, commandID)
	statusJSON := fmt.Sprintf(`{"command_id":%q,"status":"success"}`, commandID)

	srv := dualMethodServer(t, lockJSON, statusJSON)
	binary := buildBinary(t, companionPkg)

	// Step 1: lock command.
	lockOut, err := exec.Command(binary,
		"lock",
		"--vin=VIN001",
		"--token=smoke-token",
		"--gateway-addr="+srv.URL,
	).CombinedOutput()
	if err != nil {
		t.Fatalf("lock: expected exit 0, got: %v\noutput: %s", err, lockOut)
	}
	if !bytes.Contains(lockOut, []byte(commandID)) {
		t.Fatalf("lock: expected %q in stdout, got: %s", commandID, lockOut)
	}

	// Step 2: status command using the command_id returned by lock.
	statusOut, err := exec.Command(binary,
		"status",
		"--vin=VIN001",
		fmt.Sprintf("--command-id=%s", commandID),
		"--token=smoke-token",
		"--gateway-addr="+srv.URL,
	).CombinedOutput()
	if err != nil {
		t.Fatalf("status: expected exit 0, got: %v\noutput: %s", err, statusOut)
	}
	if !bytes.Contains(statusOut, []byte("success")) {
		t.Fatalf("status: expected 'success' in stdout, got: %s", statusOut)
	}
}
