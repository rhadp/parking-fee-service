//go:build integration

package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"os/exec"
	"testing"
	"time"

	nats "github.com/nats-io/nats.go"
)

// writeTestConfigWithTimeout writes a test config with a custom command timeout (in seconds).
func writeTestConfigWithTimeout(t *testing.T, port, timeoutSeconds int) string {
	t.Helper()
	content := fmt.Sprintf(`{
  "port": %d,
  "nats_url": "nats://localhost:4222",
  "command_timeout_seconds": %d,
  "tokens": [
    {"token": "smoke-token-001", "vin": "SMOKEVIN1"},
    {"token": "smoke-token-002", "vin": "SMOKEVIN2"}
  ]
}`, port, timeoutSeconds)

	path := fmt.Sprintf("%s/config.json", t.TempDir())
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		t.Fatalf("write test config: %v", err)
	}
	return path
}

// startServiceProcess starts the cloud-gateway binary with the given config and
// waits until the service logs "ready". Returns the running command and the
// captured output buffer. The caller is responsible for killing the process.
func startServiceProcess(t *testing.T, cfgPath string) (*exec.Cmd, *safeBuf) {
	t.Helper()
	var out safeBuf
	cmd := exec.Command(binaryPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)
	cmd.Stdout = &out
	cmd.Stderr = &out

	if err := cmd.Start(); err != nil {
		t.Fatalf("start service: %v", err)
	}

	// Wait for the "ready" log message before proceeding.
	if !waitForOutput(&out, "ready", 15*time.Second) {
		cmd.Process.Kill() //nolint:errcheck
		t.Fatalf("service did not log 'ready' within timeout; output:\n%s", out.String())
	}
	return cmd, &out
}

// connectNATS connects to the local NATS server and returns the connection.
func connectNATS(t *testing.T) *nats.Conn {
	t.Helper()
	nc, err := nats.Connect("nats://localhost:4222")
	if err != nil {
		t.Fatalf("connect to NATS: %v", err)
	}
	t.Cleanup(nc.Close)
	return nc
}

// httpGet issues a GET request with an Authorization header.
func httpGet(t *testing.T, url, token string) *http.Response {
	t.Helper()
	req, err := http.NewRequest(http.MethodGet, url, nil)
	if err != nil {
		t.Fatalf("NewRequest GET %s: %v", url, err)
	}
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("GET %s: %v", url, err)
	}
	return resp
}

// httpPost issues a POST request with a JSON body and Authorization header.
func httpPost(t *testing.T, url, body, token string) *http.Response {
	t.Helper()
	req, err := http.NewRequest(http.MethodPost, url, bytes.NewBufferString(body))
	if err != nil {
		t.Fatalf("NewRequest POST %s: %v", url, err)
	}
	req.Header.Set("Content-Type", "application/json")
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("POST %s: %v", url, err)
	}
	return resp
}

// decodeJSON decodes the JSON response body into a map.
func decodeJSON(t *testing.T, resp *http.Response) map[string]any {
	t.Helper()
	defer resp.Body.Close()
	var m map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&m); err != nil {
		t.Fatalf("decode JSON response: %v", err)
	}
	return m
}

// TestEndToEndCommandFlow verifies the full flow: submit command via REST,
// receive on NATS, publish response on NATS, query status via REST.
// TS-06-SMOKE-1
func TestEndToEndCommandFlow(t *testing.T) {
	if !checkNATSAvailable() {
		t.Skip("NATS not available on localhost:4222; skipping end-to-end smoke test")
	}

	const smokePort = 18091
	const smokeToken = "smoke-token-001"
	const smokeVIN = "SMOKEVIN1"
	const cmdID = "smoke-001"

	cfgPath := writeTestConfigWithTimeout(t, smokePort, 30)
	cmd, out := startServiceProcess(t, cfgPath)
	defer cmd.Process.Kill() //nolint:errcheck

	base := fmt.Sprintf("http://localhost:%d", smokePort)

	// 1. Subscribe to vehicles.SMOKEVIN1.commands on NATS.
	nc := connectNATS(t)
	sub, err := nc.SubscribeSync(fmt.Sprintf("vehicles.%s.commands", smokeVIN))
	if err != nil {
		t.Fatalf("NATS subscribe: %v", err)
	}
	defer sub.Unsubscribe() //nolint:errcheck

	// 2. POST /vehicles/SMOKEVIN1/commands.
	postBody := fmt.Sprintf(`{"command_id":%q,"type":"lock","doors":["driver"]}`, cmdID)
	postResp := httpPost(t, base+"/vehicles/"+smokeVIN+"/commands", postBody, smokeToken)
	if postResp.StatusCode != http.StatusAccepted {
		body := decodeJSON(t, postResp)
		t.Fatalf("submit command: want 202, got %d; body: %v", postResp.StatusCode, body)
	}
	submitted := decodeJSON(t, postResp)
	if submitted["command_id"] != cmdID {
		t.Errorf("submit response command_id: want %q, got %v", cmdID, submitted["command_id"])
	}

	// 3. Receive the command on the NATS subscriber.
	msg, err := sub.NextMsg(3 * time.Second)
	if err != nil {
		t.Fatalf("receive NATS command: %v (output: %s)", err, out.String())
	}

	var receivedCmd map[string]any
	if err := json.Unmarshal(msg.Data, &receivedCmd); err != nil {
		t.Fatalf("unmarshal NATS command: %v", err)
	}
	if receivedCmd["command_id"] != cmdID {
		t.Errorf("NATS command_id: want %q, got %v", cmdID, receivedCmd["command_id"])
	}
	// Verify the Authorization header was propagated (06-REQ-1.2).
	wantAuthHeader := "Bearer " + smokeToken
	if got := msg.Header.Get("Authorization"); got != wantAuthHeader {
		t.Errorf("NATS Authorization header: want %q, got %q", wantAuthHeader, got)
	}

	// 4. Publish a success response to vehicles.SMOKEVIN1.command_responses.
	respPayload, _ := json.Marshal(map[string]string{
		"command_id": cmdID,
		"status":     "success",
	})
	if err := nc.Publish(fmt.Sprintf("vehicles.%s.command_responses", smokeVIN), respPayload); err != nil {
		t.Fatalf("publish response: %v", err)
	}

	// Give the gateway time to process the NATS response.
	time.Sleep(300 * time.Millisecond)

	// 5. GET /vehicles/SMOKEVIN1/commands/smoke-001.
	getResp := httpGet(t, base+"/vehicles/"+smokeVIN+"/commands/"+cmdID, smokeToken)
	if getResp.StatusCode != http.StatusOK {
		body := decodeJSON(t, getResp)
		t.Fatalf("query status: want 200, got %d; body: %v", getResp.StatusCode, body)
	}
	status := decodeJSON(t, getResp)
	if status["command_id"] != cmdID {
		t.Errorf("status command_id: want %q, got %v", cmdID, status["command_id"])
	}
	if status["status"] != "success" {
		t.Errorf("status field: want 'success', got %v", status["status"])
	}
}

// TestCommandTimeoutEndToEnd submits a command without sending a NATS response
// and verifies that the status becomes "timeout" after the configured timeout.
// TS-06-SMOKE-2
func TestCommandTimeoutEndToEnd(t *testing.T) {
	if !checkNATSAvailable() {
		t.Skip("NATS not available on localhost:4222; skipping timeout smoke test")
	}

	const smokePort = 18092
	const smokeToken = "smoke-token-001"
	const smokeVIN = "SMOKEVIN1"
	const cmdID = "smoke-002"
	const timeoutSeconds = 1 // fast timeout for test

	cfgPath := writeTestConfigWithTimeout(t, smokePort, timeoutSeconds)
	cmd, _ := startServiceProcess(t, cfgPath)
	defer cmd.Process.Kill() //nolint:errcheck

	base := fmt.Sprintf("http://localhost:%d", smokePort)

	// 1. POST /vehicles/SMOKEVIN1/commands.
	postBody := fmt.Sprintf(`{"command_id":%q,"type":"unlock","doors":["driver"]}`, cmdID)
	postResp := httpPost(t, base+"/vehicles/"+smokeVIN+"/commands", postBody, smokeToken)
	if postResp.StatusCode != http.StatusAccepted {
		body := decodeJSON(t, postResp)
		t.Fatalf("submit command: want 202, got %d; body: %v", postResp.StatusCode, body)
	}
	postResp.Body.Close()

	// 2. Wait past the configured 1s timeout (2 seconds total).
	time.Sleep(time.Duration(timeoutSeconds+1) * time.Second)

	// 3. GET /vehicles/SMOKEVIN1/commands/smoke-002.
	getResp := httpGet(t, base+"/vehicles/"+smokeVIN+"/commands/"+cmdID, smokeToken)
	if getResp.StatusCode != http.StatusOK {
		body := decodeJSON(t, getResp)
		t.Fatalf("query status: want 200, got %d; body: %v", getResp.StatusCode, body)
	}
	status := decodeJSON(t, getResp)
	if status["status"] != "timeout" {
		t.Errorf("status field: want 'timeout', got %v (full body: %v)", status["status"], status)
	}
}
