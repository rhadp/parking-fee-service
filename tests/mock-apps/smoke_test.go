// Package mockappstests provides end-to-end smoke tests.
//
// TS-09-SMOKE-2: parking-operator serve handles full start/stop lifecycle.
// TS-09-SMOKE-3: companion-app-cli sends lock command and queries status via CLOUD_GATEWAY.
package mockappstests

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"os/exec"
	"strings"
	"syscall"
	"testing"
	"time"
)

// doPost sends a POST request with a JSON body and returns the response.
func doPost(t *testing.T, url string, body any) *http.Response {
	t.Helper()
	var buf bytes.Buffer
	if err := json.NewEncoder(&buf).Encode(body); err != nil {
		t.Fatalf("encode request body: %v", err)
	}
	resp, err := http.Post(url, "application/json", &buf)
	if err != nil {
		t.Fatalf("POST %s: %v", url, err)
	}
	return resp
}

// doGet sends a GET request and returns the response.
func doGet(t *testing.T, url string) *http.Response {
	t.Helper()
	resp, err := http.Get(url)
	if err != nil {
		t.Fatalf("GET %s: %v", url, err)
	}
	return resp
}

// decodeJSON decodes the response body as a map[string]any.
func decodeJSON(t *testing.T, resp *http.Response) map[string]any {
	t.Helper()
	defer resp.Body.Close()
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		t.Fatalf("read response body: %v", err)
	}
	var m map[string]any
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatalf("decode response JSON: %v (body=%q)", err, data)
	}
	return m
}

// TS-09-SMOKE-2: End-to-end test: parking-operator serve handles full start→stop lifecycle.
// Starts the parking-operator binary, sends start/stop requests, verifies responses,
// then sends SIGTERM and verifies clean exit.
func TestParkingOperatorSmoke(t *testing.T) {
	binary := parkingOperatorBin(t)

	// Use a unique port to avoid conflicts with TestGracefulShutdown
	port := "19877"
	cmd := exec.Command(binary, "serve", "--port="+port)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	if err := cmd.Start(); err != nil {
		t.Fatalf("start parking-operator: %v", err)
	}
	defer func() {
		cmd.Process.Signal(syscall.SIGTERM) //nolint
		cmd.Wait()                          //nolint
	}()

	baseURL := fmt.Sprintf("http://127.0.0.1:%s", port)

	// Wait for server to listen
	if err := waitForPort(fmt.Sprintf("127.0.0.1:%s", port), 5*time.Second); err != nil {
		t.Fatalf("parking-operator did not start listening: %v", err)
	}

	// POST /parking/start
	startBody := map[string]any{
		"vehicle_id": "VIN001",
		"zone_id":    "zone-1",
		"timestamp":  int64(1700000000),
	}
	startResp := doPost(t, baseURL+"/parking/start", startBody)
	if startResp.StatusCode != http.StatusOK {
		t.Fatalf("POST /parking/start: expected 200, got %d", startResp.StatusCode)
	}
	startData := decodeJSON(t, startResp)

	sessionID, _ := startData["session_id"].(string)
	if sessionID == "" {
		t.Fatalf("POST /parking/start: no session_id in response")
	}
	if status, _ := startData["status"].(string); status != "active" {
		t.Errorf("POST /parking/start: expected status=active, got %q", status)
	}

	// GET /parking/status/{session_id}
	statusResp := doGet(t, baseURL+"/parking/status/"+sessionID)
	if statusResp.StatusCode != http.StatusOK {
		t.Errorf("GET /parking/status: expected 200, got %d", statusResp.StatusCode)
	}
	statusData := decodeJSON(t, statusResp)
	if id, _ := statusData["session_id"].(string); id != sessionID {
		t.Errorf("GET /parking/status: expected session_id=%q, got %q", sessionID, id)
	}

	// POST /parking/stop (1 hour later → duration=3600, total_amount=2.50)
	stopBody := map[string]any{
		"session_id": sessionID,
		"timestamp":  int64(1700003600),
	}
	stopResp := doPost(t, baseURL+"/parking/stop", stopBody)
	if stopResp.StatusCode != http.StatusOK {
		t.Fatalf("POST /parking/stop: expected 200, got %d", stopResp.StatusCode)
	}
	stopData := decodeJSON(t, stopResp)

	duration, _ := stopData["duration_seconds"].(float64)
	if uint64(duration) != 3600 {
		t.Errorf("POST /parking/stop: expected duration_seconds=3600, got %v", duration)
	}
	totalAmount, _ := stopData["total_amount"].(float64)
	if absFloat(totalAmount-2.50) > 0.01 {
		t.Errorf("POST /parking/stop: expected total_amount≈2.50, got %v", totalAmount)
	}

	// Send SIGTERM and verify clean exit
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("send SIGTERM: %v", err)
	}

	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()

	select {
	case err := <-done:
		if err != nil {
			if exitErr, ok := err.(*exec.ExitError); ok {
				if exitErr.ExitCode() != 0 {
					t.Errorf("parking-operator: expected exit 0 on SIGTERM, got %d", exitErr.ExitCode())
				}
			} else {
				t.Errorf("parking-operator Wait: unexpected error: %v", err)
			}
		}
	case <-time.After(10 * time.Second):
		cmd.Process.Kill() //nolint
		t.Error("parking-operator did not exit within 10s after SIGTERM")
	}
}

// dynamicMock holds request state captured by the handler closure.
type dynamicMock struct {
	ts          *httptest.Server
	lastMethod  string
	lastPath    string
	lastHeaders http.Header
	lastBody    []byte
}

// URL returns the base URL of the mock server.
func (m *dynamicMock) URL() string { return m.ts.URL }

// Header returns a captured request header value.
func (m *dynamicMock) Header(key string) string { return m.lastHeaders.Get(key) }

// newDynamicMock creates an httptest server whose handler captures each request.
// The provided handleFn is called after capturing the request metadata.
func newDynamicMock(t *testing.T, handleFn func(w http.ResponseWriter, r *http.Request, body []byte)) *dynamicMock {
	t.Helper()
	m := &dynamicMock{}
	m.ts = httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		m.lastMethod = r.Method
		m.lastPath = r.URL.Path
		m.lastHeaders = r.Header.Clone()
		data, _ := io.ReadAll(r.Body)
		m.lastBody = data
		handleFn(w, r, data)
	}))
	t.Cleanup(m.ts.Close)
	return m
}

// TS-09-SMOKE-3: companion-app-cli sends lock command and queries status via mock CLOUD_GATEWAY.
// Verifies the full lock → get command_id → status sequence.
func TestCompanionAppSmoke(t *testing.T) {
	binary := companionBin(t)

	const commandID = "smoke-cmd-1"

	// Build a mock CLOUD_GATEWAY that handles lock POST and status GET.
	mock := newDynamicMock(t, func(w http.ResponseWriter, r *http.Request, _ []byte) {
		w.Header().Set("Content-Type", "application/json")
		if r.Method == http.MethodPost && strings.Contains(r.URL.Path, "/commands") {
			w.WriteHeader(http.StatusOK)
			fmt.Fprintf(w, `{"command_id":%q,"status":"pending"}`, commandID)
		} else if r.Method == http.MethodGet && strings.Contains(r.URL.Path, "/commands/") {
			w.WriteHeader(http.StatusOK)
			fmt.Fprintf(w, `{"command_id":%q,"status":"success"}`, commandID)
		} else {
			w.WriteHeader(http.StatusNotFound)
			fmt.Fprint(w, `{"error":"not found"}`)
		}
	})

	// Step 1: lock command
	stdout, stderr, code := runBinary(t, binary,
		"lock",
		"--vin=VIN001",
		"--token=smoke-token",
		"--gateway-addr="+mock.URL(),
	)
	if code != 0 {
		t.Fatalf("companion-app-cli lock: expected exit 0, got %d (stderr=%q)", code, stderr)
	}
	if !strings.Contains(stdout, commandID) {
		t.Errorf("companion-app-cli lock: expected %q in stdout, got %q", commandID, stdout)
	}

	// Verify auth header on lock
	auth := mock.Header("Authorization")
	if auth != "Bearer smoke-token" {
		t.Errorf("companion-app-cli lock: expected Authorization=Bearer smoke-token, got %q", auth)
	}

	// Verify lock body type
	var lockBody map[string]any
	if err := json.Unmarshal(mock.lastBody, &lockBody); err != nil {
		t.Errorf("companion-app-cli lock: could not parse request body: %v", err)
	} else if typ, _ := lockBody["type"].(string); typ != "lock" {
		t.Errorf("companion-app-cli lock: expected type=lock, got %q", typ)
	}

	// Step 2: status query
	stdout2, stderr2, code2 := runBinary(t, binary,
		"status",
		"--vin=VIN001",
		"--command-id="+commandID,
		"--token=smoke-token",
		"--gateway-addr="+mock.URL(),
	)
	if code2 != 0 {
		t.Fatalf("companion-app-cli status: expected exit 0, got %d (stderr=%q)", code2, stderr2)
	}
	if !strings.Contains(stdout2, "success") {
		t.Errorf("companion-app-cli status: expected success in stdout, got %q", stdout2)
	}
	if !strings.Contains(mock.lastPath, "/commands/"+commandID) {
		t.Errorf("companion-app-cli status: expected path /commands/%s, got %q", commandID, mock.lastPath)
	}
}

// absFloat returns the absolute value of a float64.
func absFloat(x float64) float64 {
	if x < 0 {
		return -x
	}
	return x
}
