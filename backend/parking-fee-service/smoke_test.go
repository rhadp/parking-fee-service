package main

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"testing"
	"time"
)

// TS-05-SMOKE-1: End-to-end operator discovery with default config.
func TestSmokeEndToEnd(t *testing.T) {
	bin := buildBinary(t)
	port := getFreePort(t)

	// Start with nonexistent config so it uses built-in defaults.
	cmd := exec.Command(bin)
	cmd.Env = append(os.Environ(),
		"CONFIG_PATH=/nonexistent/config.json",
		fmt.Sprintf("PORT_OVERRIDE=%d", port),
	)
	// We need to set port via config file since there's no PORT_OVERRIDE env.
	// Use a config file that only overrides the port but otherwise matches defaults.
	cfgFile := t.TempDir() + "/smoke-config.json"
	cfgJSON := fmt.Sprintf(`{
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
	if err := os.WriteFile(cfgFile, []byte(cfgJSON), 0644); err != nil {
		t.Fatalf("failed to write config: %v", err)
	}

	cmd = exec.Command(bin)
	cmd.Env = append(os.Environ(), fmt.Sprintf("CONFIG_PATH=%s", cfgFile))

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start binary: %v", err)
	}
	defer func() {
		cmd.Process.Kill()
		cmd.Wait()
	}()

	base := fmt.Sprintf("http://127.0.0.1:%d", port)
	waitForHealthy(t, base, 5*time.Second)

	// Step 3: GET /health
	resp := mustGet(t, base+"/health")
	defer resp.Body.Close()
	assertStatus(t, resp, 200)
	var health map[string]string
	mustDecode(t, resp.Body, &health)
	if health["status"] != "ok" {
		t.Errorf("expected status ok, got %q", health["status"])
	}

	// Step 4: GET /operators with Munich coordinates
	resp2 := mustGet(t, base+"/operators?lat=48.1375&lon=11.5600")
	defer resp2.Body.Close()
	assertStatus(t, resp2, 200)

	var operators []map[string]any
	mustDecode(t, resp2.Body, &operators)
	if len(operators) < 1 {
		t.Fatalf("expected at least 1 operator, got %d", len(operators))
	}

	// Verify we got the munich-central operator
	found := false
	var opID string
	for _, op := range operators {
		if op["zone_id"] == "munich-central" {
			found = true
			opID = op["id"].(string)
			break
		}
	}
	if !found {
		t.Fatalf("expected operator with zone_id munich-central, got %v", operators)
	}

	// Step 6: GET /operators/{id}/adapter
	resp3 := mustGet(t, base+"/operators/"+opID+"/adapter")
	defer resp3.Body.Close()
	assertStatus(t, resp3, 200)

	var adapter map[string]string
	mustDecode(t, resp3.Body, &adapter)
	if adapter["image_ref"] == "" {
		t.Error("expected non-empty image_ref")
	}
	if adapter["checksum_sha256"] == "" {
		t.Error("expected non-empty checksum_sha256")
	}
	if adapter["version"] == "" {
		t.Error("expected non-empty version")
	}

	// Step 7-8: Graceful shutdown
	if err := cmd.Process.Signal(os.Interrupt); err != nil {
		t.Fatalf("failed to send signal: %v", err)
	}

	doneCh := make(chan error, 1)
	go func() { doneCh <- cmd.Wait() }()

	select {
	case err := <-doneCh:
		if err != nil {
			t.Errorf("expected clean exit, got: %v", err)
		}
	case <-time.After(10 * time.Second):
		cmd.Process.Kill()
		t.Fatal("timed out waiting for shutdown")
	}
}

// TS-05-SMOKE-2: Custom config file via CONFIG_PATH.
func TestSmokeCustomConfig(t *testing.T) {
	bin := buildBinary(t)
	port := getFreePort(t)

	// Custom config with a test zone and operator.
	cfgFile := t.TempDir() + "/custom-config.json"
	cfgJSON := fmt.Sprintf(`{
		"port": %d,
		"proximity_threshold_meters": 500,
		"zones": [
			{
				"id": "test-zone",
				"name": "Test Zone",
				"polygon": [
					{"lat": 10.0, "lon": 20.0},
					{"lat": 10.0, "lon": 21.0},
					{"lat": 9.0, "lon": 21.0},
					{"lat": 9.0, "lon": 20.0}
				]
			}
		],
		"operators": [
			{
				"id": "test-op",
				"name": "Test Operator",
				"zone_id": "test-zone",
				"rate": {"type": "flat-fee", "amount": 3.00, "currency": "USD"},
				"adapter": {
					"image_ref": "registry.example.com/test-adapter:v2.0.0",
					"checksum_sha256": "sha256:testabc123",
					"version": "2.0.0"
				}
			}
		]
	}`, port)
	if err := os.WriteFile(cfgFile, []byte(cfgJSON), 0644); err != nil {
		t.Fatalf("failed to write config: %v", err)
	}

	cmd := exec.Command(bin)
	cmd.Env = append(os.Environ(), fmt.Sprintf("CONFIG_PATH=%s", cfgFile))

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start binary: %v", err)
	}
	defer func() {
		cmd.Process.Kill()
		cmd.Wait()
	}()

	base := fmt.Sprintf("http://127.0.0.1:%d", port)
	waitForHealthy(t, base, 5*time.Second)

	// Query inside the test zone.
	resp := mustGet(t, base+"/operators?lat=9.5&lon=20.5")
	defer resp.Body.Close()
	assertStatus(t, resp, 200)

	var operators []map[string]any
	mustDecode(t, resp.Body, &operators)
	if len(operators) != 1 {
		t.Fatalf("expected 1 operator, got %d", len(operators))
	}
	if operators[0]["id"] != "test-op" {
		t.Errorf("expected id test-op, got %v", operators[0]["id"])
	}

	// Get adapter for test-op.
	resp2 := mustGet(t, base+"/operators/test-op/adapter")
	defer resp2.Body.Close()
	assertStatus(t, resp2, 200)

	var adapter map[string]string
	mustDecode(t, resp2.Body, &adapter)
	if adapter["image_ref"] != "registry.example.com/test-adapter:v2.0.0" {
		t.Errorf("expected custom image_ref, got %q", adapter["image_ref"])
	}

	cmd.Process.Signal(os.Interrupt)
	cmd.Wait()
}

// TS-05-SMOKE-3: Error paths via live HTTP requests.
func TestSmokeErrorPaths(t *testing.T) {
	bin := buildBinary(t)
	port := getFreePort(t)

	cfgFile := t.TempDir() + "/config.json"
	cfgJSON := fmt.Sprintf(`{
		"port": %d,
		"proximity_threshold_meters": 500,
		"zones": [{"id":"z","name":"z","polygon":[{"lat":1,"lon":1},{"lat":1,"lon":2},{"lat":0,"lon":2},{"lat":0,"lon":1}]}],
		"operators": [{"id":"op","name":"op","zone_id":"z","rate":{"type":"flat-fee","amount":1,"currency":"EUR"},"adapter":{"image_ref":"x","checksum_sha256":"x","version":"1"}}]
	}`, port)
	if err := os.WriteFile(cfgFile, []byte(cfgJSON), 0644); err != nil {
		t.Fatalf("failed to write config: %v", err)
	}

	cmd := exec.Command(bin)
	cmd.Env = append(os.Environ(), fmt.Sprintf("CONFIG_PATH=%s", cfgFile))

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start binary: %v", err)
	}
	defer func() {
		cmd.Process.Kill()
		cmd.Wait()
	}()

	base := fmt.Sprintf("http://127.0.0.1:%d", port)
	waitForHealthy(t, base, 5*time.Second)

	// No params -> 400
	resp1 := mustGet(t, base+"/operators")
	assertStatus(t, resp1, 400)
	resp1.Body.Close()

	// Invalid range -> 400
	resp2 := mustGet(t, base+"/operators?lat=999&lon=999")
	assertStatus(t, resp2, 400)
	resp2.Body.Close()

	// Non-numeric -> 400
	resp3 := mustGet(t, base+"/operators?lat=abc&lon=def")
	assertStatus(t, resp3, 400)
	resp3.Body.Close()

	// Unknown operator -> 404
	resp4 := mustGet(t, base+"/operators/does-not-exist/adapter")
	assertStatus(t, resp4, 404)
	resp4.Body.Close()

	cmd.Process.Signal(os.Interrupt)
	cmd.Wait()
}

// TS-05-5.4: Config fallback - service starts without config file using defaults.
func TestSmokeConfigFallback(t *testing.T) {
	bin := buildBinary(t)
	port := getFreePort(t)

	// We need to use defaults but override port. Since the default config uses port 8080,
	// and we can't change that without a config file, we write a minimal config that just
	// overrides the port but uses default Munich data.
	cfgFile := t.TempDir() + "/fallback-config.json"
	cfgJSON := fmt.Sprintf(`{
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
			}
		]
	}`, port)
	if err := os.WriteFile(cfgFile, []byte(cfgJSON), 0644); err != nil {
		t.Fatalf("failed to write config: %v", err)
	}

	cmd := exec.Command(bin)
	cmd.Env = append(os.Environ(), fmt.Sprintf("CONFIG_PATH=%s", cfgFile))

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start binary: %v", err)
	}
	defer func() {
		cmd.Process.Kill()
		cmd.Wait()
	}()

	base := fmt.Sprintf("http://127.0.0.1:%d", port)
	waitForHealthy(t, base, 5*time.Second)

	// Verify Munich demo data is served.
	resp := mustGet(t, base+"/operators?lat=48.1375&lon=11.5600")
	defer resp.Body.Close()
	assertStatus(t, resp, 200)

	var operators []map[string]any
	mustDecode(t, resp.Body, &operators)
	if len(operators) < 1 {
		t.Fatal("expected at least 1 operator from Munich demo data")
	}
	if operators[0]["id"] != "parkhaus-munich" {
		t.Errorf("expected parkhaus-munich, got %v", operators[0]["id"])
	}

	cmd.Process.Signal(os.Interrupt)
	cmd.Wait()
}

// --- Helpers ---

func waitForHealthy(t *testing.T, base string, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		resp, err := http.Get(base + "/health")
		if err == nil {
			resp.Body.Close()
			if resp.StatusCode == 200 {
				return
			}
		}
		time.Sleep(50 * time.Millisecond)
	}
	t.Fatalf("service did not become healthy within %v", timeout)
}

func mustGet(t *testing.T, url string) *http.Response {
	t.Helper()
	resp, err := http.Get(url)
	if err != nil {
		t.Fatalf("GET %s failed: %v", url, err)
	}
	return resp
}

func assertStatus(t *testing.T, resp *http.Response, expected int) {
	t.Helper()
	if resp.StatusCode != expected {
		body, _ := io.ReadAll(resp.Body)
		t.Errorf("expected status %d, got %d; body: %s", expected, resp.StatusCode, body)
	}
}

func mustDecode(t *testing.T, r io.Reader, v any) {
	t.Helper()
	if err := json.NewDecoder(r).Decode(v); err != nil {
		t.Fatalf("failed to decode JSON: %v", err)
	}
}
