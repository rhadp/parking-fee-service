package main

import (
	"encoding/json"
	"fmt"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
	"time"
)

func TestCompiles(t *testing.T) {
	// Placeholder test: confirms the module compiles successfully.
}

// buildBinary builds the service binary to a temp directory and returns the path.
func buildBinary(t *testing.T) string {
	t.Helper()
	binPath := filepath.Join(t.TempDir(), "parking-fee-service")
	cmd := exec.Command("go", "build", "-o", binPath, ".")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build binary: %v\n%s", err, out)
	}
	return binPath
}

// getFreePort finds an available TCP port.
func getFreePort(t *testing.T) int {
	t.Helper()
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to get free port: %v", err)
	}
	defer l.Close()
	return l.Addr().(*net.TCPAddr).Port
}

// waitForReady polls the health endpoint until the service is ready or timeout.
func waitForReady(port int, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	client := &http.Client{Timeout: 500 * time.Millisecond}
	for time.Now().Before(deadline) {
		resp, err := client.Get(fmt.Sprintf("http://127.0.0.1:%d/health", port))
		if err == nil {
			resp.Body.Close()
			if resp.StatusCode == http.StatusOK {
				return true
			}
		}
		time.Sleep(100 * time.Millisecond)
	}
	return false
}

// writeDefaultConfig writes the default Munich demo config with a custom port.
func writeDefaultConfig(t *testing.T, port int) string {
	t.Helper()
	cfg := fmt.Sprintf(`{
	"port": %d,
	"proximity_threshold_meters": 500,
	"zones": [
		{
			"id": "munich-central",
			"name": "Munich Central Station Area",
			"polygon": [
				{"lat": 48.1400, "lon": 11.5550},
				{"lat": 48.1400, "lon": 11.5650},
				{"lat": 48.1350, "lon": 11.5650},
				{"lat": 48.1350, "lon": 11.5550}
			]
		},
		{
			"id": "munich-marienplatz",
			"name": "Marienplatz Area",
			"polygon": [
				{"lat": 48.1380, "lon": 11.5730},
				{"lat": 48.1380, "lon": 11.5790},
				{"lat": 48.1350, "lon": 11.5790},
				{"lat": 48.1350, "lon": 11.5730}
			]
		}
	],
	"operators": [
		{
			"id": "parkhaus-munich",
			"name": "Parkhaus Muenchen GmbH",
			"zone_id": "munich-central",
			"rate": {"type": "per-hour", "amount": 2.50, "currency": "EUR"},
			"adapter": {
				"image_ref": "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0",
				"checksum_sha256": "sha256:abc123def456",
				"version": "1.0.0"
			}
		},
		{
			"id": "city-park-munich",
			"name": "CityPark Muenchen",
			"zone_id": "munich-marienplatz",
			"rate": {"type": "flat-fee", "amount": 5.00, "currency": "EUR"},
			"adapter": {
				"image_ref": "us-docker.pkg.dev/sdv-demo/adapters/citypark-munich:v1.0.0",
				"checksum_sha256": "sha256:789ghi012jkl",
				"version": "1.0.0"
			}
		}
	]
}`, port)

	cfgPath := filepath.Join(t.TempDir(), "config.json")
	if err := os.WriteFile(cfgPath, []byte(cfg), 0644); err != nil {
		t.Fatalf("failed to write test config: %v", err)
	}
	return cfgPath
}

// writeCustomConfig writes a custom config with a single test zone and operator.
func writeCustomConfig(t *testing.T, port int) string {
	t.Helper()
	cfg := fmt.Sprintf(`{
	"port": %d,
	"proximity_threshold_meters": 500,
	"zones": [
		{
			"id": "test-zone",
			"name": "Test Zone",
			"polygon": [
				{"lat": 50.0000, "lon": 10.0000},
				{"lat": 50.0000, "lon": 10.0100},
				{"lat": 49.9900, "lon": 10.0100},
				{"lat": 49.9900, "lon": 10.0000}
			]
		}
	],
	"operators": [
		{
			"id": "test-op",
			"name": "Test Operator",
			"zone_id": "test-zone",
			"rate": {"type": "per-hour", "amount": 1.50, "currency": "EUR"},
			"adapter": {
				"image_ref": "registry/test-op:v1",
				"checksum_sha256": "sha256:test123",
				"version": "1.0.0"
			}
		}
	]
}`, port)

	cfgPath := filepath.Join(t.TempDir(), "custom-config.json")
	if err := os.WriteFile(cfgPath, []byte(cfg), 0644); err != nil {
		t.Fatalf("failed to write custom config: %v", err)
	}
	return cfgPath
}

// TS-05-15: On startup, the service logs version, port, zone count, operator
// count, and ready message.
func TestStartupLogging(t *testing.T) {
	binPath := buildBinary(t)
	port := getFreePort(t)
	cfgPath := writeDefaultConfig(t, port)

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)

	// Capture combined stdout/stderr via a buffer instead of CombinedOutput,
	// which would block forever on a long-running HTTP server.
	var outBuf strings.Builder
	cmd.Stdout = &outBuf
	cmd.Stderr = &outBuf

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start service: %v", err)
	}

	// Wait for the service to be ready (or give it time to log startup info).
	if !waitForReady(port, 5*time.Second) {
		// Service didn't become ready -- still check whatever was logged.
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
		t.Log("service did not become ready, checking partial output")
	}

	// Send SIGTERM so the server exits cleanly.
	_ = cmd.Process.Signal(syscall.SIGTERM)

	// Wait for the process to exit with a timeout.
	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()
	select {
	case <-done:
	case <-time.After(5 * time.Second):
		_ = cmd.Process.Kill()
	}

	output := outBuf.String()
	portStr := fmt.Sprintf("%d", port)

	if !strings.Contains(output, portStr) {
		t.Errorf("startup logs should contain port number %s, got:\n%s", portStr, output)
	}
	if !strings.Contains(strings.ToLower(output), "zone") {
		t.Errorf("startup logs should mention zones, got:\n%s", output)
	}
	if !strings.Contains(strings.ToLower(output), "operator") {
		t.Errorf("startup logs should mention operators, got:\n%s", output)
	}
	// 05-REQ-6.1: Must log the service version.
	if !strings.Contains(strings.ToLower(output), "version") {
		t.Errorf("startup logs should mention version, got:\n%s", output)
	}
	// 05-REQ-6.1: Must log a ready message.
	if !strings.Contains(strings.ToLower(output), "ready") {
		t.Errorf("startup logs should contain a ready message, got:\n%s", output)
	}
}

// TS-05-16: On SIGTERM or SIGINT, the service gracefully shuts down the HTTP
// server and exits with code 0.
func TestGracefulShutdown(t *testing.T) {
	binPath := buildBinary(t)
	port := getFreePort(t)
	cfgPath := writeDefaultConfig(t, port)

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start service: %v", err)
	}

	// Wait for the service to be ready.
	if !waitForReady(port, 5*time.Second) {
		// Try to clean up.
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
		t.Fatal("service did not become ready within 5 seconds")
	}

	// Send SIGTERM.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	// Wait for clean exit.
	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()

	select {
	case err := <-done:
		if err != nil {
			t.Errorf("expected exit code 0 after SIGTERM, got: %v", err)
		}
	case <-time.After(5 * time.Second):
		_ = cmd.Process.Kill()
		t.Fatal("service did not exit within 5 seconds after SIGTERM")
	}
}

// TS-05-SMOKE-1: End-to-end operator discovery. Start the service binary,
// query an operator by Munich coordinates, retrieve its adapter metadata,
// and verify the health endpoint.
func TestSmokeEndToEnd(t *testing.T) {
	binPath := buildBinary(t)
	port := getFreePort(t)
	cfgPath := writeDefaultConfig(t, port)

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start service: %v", err)
	}
	defer func() {
		_ = cmd.Process.Signal(syscall.SIGTERM)
		_ = cmd.Wait()
	}()

	if !waitForReady(port, 5*time.Second) {
		t.Fatal("service did not become ready within 5 seconds")
	}

	baseURL := fmt.Sprintf("http://127.0.0.1:%d", port)

	// Step 1: Health check.
	resp, err := http.Get(baseURL + "/health")
	if err != nil {
		t.Fatalf("health check request failed: %v", err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		t.Errorf("health check: expected 200, got %d", resp.StatusCode)
	}
	var health map[string]string
	if err := json.NewDecoder(resp.Body).Decode(&health); err != nil {
		t.Fatalf("failed to decode health response: %v", err)
	}
	if health["status"] != "ok" {
		t.Errorf("health check: expected status ok, got %q", health["status"])
	}

	// Step 2: Operator lookup inside munich-central.
	resp2, err := http.Get(baseURL + "/operators?lat=48.1375&lon=11.5600")
	if err != nil {
		t.Fatalf("operator lookup request failed: %v", err)
	}
	defer resp2.Body.Close()
	if resp2.StatusCode != http.StatusOK {
		t.Errorf("operator lookup: expected 200, got %d", resp2.StatusCode)
	}
	var operators []map[string]any
	if err := json.NewDecoder(resp2.Body).Decode(&operators); err != nil {
		t.Fatalf("failed to decode operator response: %v", err)
	}
	if len(operators) < 1 {
		t.Fatal("operator lookup: expected at least 1 operator")
	}
	opID, _ := operators[0]["id"].(string)
	if opID == "" {
		t.Fatal("operator lookup: expected non-empty operator id")
	}
	foundMunichCentral := false
	for _, op := range operators {
		if op["zone_id"] == "munich-central" {
			foundMunichCentral = true
			break
		}
	}
	if !foundMunichCentral {
		t.Error("operator lookup: expected at least one operator with zone_id munich-central")
	}

	// Step 3: Adapter metadata for the discovered operator.
	resp3, err := http.Get(fmt.Sprintf("%s/operators/%s/adapter", baseURL, opID))
	if err != nil {
		t.Fatalf("adapter metadata request failed: %v", err)
	}
	defer resp3.Body.Close()
	if resp3.StatusCode != http.StatusOK {
		t.Errorf("adapter metadata: expected 200, got %d", resp3.StatusCode)
	}
	var adapter map[string]string
	if err := json.NewDecoder(resp3.Body).Decode(&adapter); err != nil {
		t.Fatalf("failed to decode adapter response: %v", err)
	}
	if adapter["image_ref"] == "" {
		t.Error("adapter metadata: expected non-empty image_ref")
	}
	if adapter["checksum_sha256"] == "" {
		t.Error("adapter metadata: expected non-empty checksum_sha256")
	}
	if adapter["version"] == "" {
		t.Error("adapter metadata: expected non-empty version")
	}
}

// TS-05-SMOKE-2: Custom config file. Start the service with a custom config
// file via CONFIG_PATH and verify it uses the custom data.
func TestSmokeCustomConfig(t *testing.T) {
	binPath := buildBinary(t)
	port := getFreePort(t)
	cfgPath := writeCustomConfig(t, port)

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start service: %v", err)
	}
	defer func() {
		_ = cmd.Process.Signal(syscall.SIGTERM)
		_ = cmd.Wait()
	}()

	if !waitForReady(port, 5*time.Second) {
		t.Fatal("service did not become ready within 5 seconds")
	}

	baseURL := fmt.Sprintf("http://127.0.0.1:%d", port)

	// Query coordinates inside the custom test-zone (center: 49.995, 10.005).
	resp, err := http.Get(baseURL + "/operators?lat=49.995&lon=10.005")
	if err != nil {
		t.Fatalf("operator lookup request failed: %v", err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		t.Errorf("operator lookup: expected 200, got %d", resp.StatusCode)
	}
	var operators []map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&operators); err != nil {
		t.Fatalf("failed to decode operator response: %v", err)
	}
	if len(operators) < 1 {
		t.Fatal("expected at least 1 operator from custom config")
	}
	opID, _ := operators[0]["id"].(string)
	if opID != "test-op" {
		t.Errorf("expected operator id test-op, got %q", opID)
	}

	// Retrieve adapter metadata for the custom operator.
	resp2, err := http.Get(baseURL + "/operators/test-op/adapter")
	if err != nil {
		t.Fatalf("adapter metadata request failed: %v", err)
	}
	defer resp2.Body.Close()
	if resp2.StatusCode != http.StatusOK {
		t.Errorf("adapter metadata: expected 200, got %d", resp2.StatusCode)
	}
}

// TS-05-SMOKE-3: Error paths. Verify error responses via live HTTP requests
// against the running service.
func TestSmokeErrorPaths(t *testing.T) {
	binPath := buildBinary(t)
	port := getFreePort(t)
	cfgPath := writeDefaultConfig(t, port)

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start service: %v", err)
	}
	defer func() {
		_ = cmd.Process.Signal(syscall.SIGTERM)
		_ = cmd.Wait()
	}()

	if !waitForReady(port, 5*time.Second) {
		t.Fatal("service did not become ready within 5 seconds")
	}

	baseURL := fmt.Sprintf("http://127.0.0.1:%d", port)

	// Missing params -> 400.
	resp1, err := http.Get(baseURL + "/operators")
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	resp1.Body.Close()
	if resp1.StatusCode != http.StatusBadRequest {
		t.Errorf("missing params: expected 400, got %d", resp1.StatusCode)
	}

	// Invalid coordinates (out of range) -> 400.
	resp2, err := http.Get(baseURL + "/operators?lat=999&lon=999")
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	resp2.Body.Close()
	if resp2.StatusCode != http.StatusBadRequest {
		t.Errorf("invalid range: expected 400, got %d", resp2.StatusCode)
	}

	// Non-numeric coordinates -> 400.
	resp3, err := http.Get(baseURL + "/operators?lat=abc&lon=def")
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	resp3.Body.Close()
	if resp3.StatusCode != http.StatusBadRequest {
		t.Errorf("non-numeric: expected 400, got %d", resp3.StatusCode)
	}

	// Unknown operator -> 404.
	resp4, err := http.Get(baseURL + "/operators/does-not-exist/adapter")
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	resp4.Body.Close()
	if resp4.StatusCode != http.StatusNotFound {
		t.Errorf("unknown operator: expected 404, got %d", resp4.StatusCode)
	}
}
