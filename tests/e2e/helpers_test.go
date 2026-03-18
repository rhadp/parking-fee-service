// Package e2e contains end-to-end tests that exercise the full service stack
// running in containers via podman compose.
//
// Prerequisites:
//   - All services running via: make e2e-up
//   - NATS on localhost:4222, parking-fee-service on :8080, cloud-gateway on :8081
//
// Run:
//
//	go test ./tests/e2e/ -v -timeout 120s
package e2e

import (
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"strings"
	"testing"
	"time"

	"github.com/nats-io/nats.go"
)

// ── Service addresses (override via env vars if testing against remote) ──────

const (
	parkingFeeServiceURL   = "http://localhost:8080"
	cloudGatewayURL        = "http://localhost:8081"
	mockParkingOperatorURL = "http://localhost:8082"
	natsURL                = "nats://localhost:4222"

	// Default credentials matching deployments/e2e/cloud-gateway.json.
	testToken = "demo-token-car1"
	testVIN   = "VIN12345"
)

// ── Readiness helpers ────────────────────────────────────────────────────────

// waitForHTTP polls a URL until it returns 200 or the timeout elapses.
func waitForHTTP(t *testing.T, url string, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		resp, err := http.Get(url) //nolint:noctx
		if err == nil {
			resp.Body.Close()
			if resp.StatusCode == http.StatusOK {
				return
			}
		}
		time.Sleep(500 * time.Millisecond)
	}
	t.Fatalf("service at %s did not become healthy within %s", url, timeout)
}

// waitForTCP polls a TCP address until it accepts connections or the timeout elapses.
func waitForTCP(t *testing.T, addr string, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", addr, 500*time.Millisecond)
		if err == nil {
			conn.Close()
			return
		}
		time.Sleep(500 * time.Millisecond)
	}
	t.Fatalf("TCP endpoint %s did not become available within %s", addr, timeout)
}

// ensureStack waits for the core services to be ready before running tests.
func ensureStack(t *testing.T) {
	t.Helper()
	waitForTCP(t, "localhost:4222", 30*time.Second)
	waitForHTTP(t, parkingFeeServiceURL+"/health", 30*time.Second)
	waitForHTTP(t, cloudGatewayURL+"/health", 30*time.Second)
}

// ensureFullStack waits for all services including mock-parking-operator and
// gRPC services to be ready.
func ensureFullStack(t *testing.T) {
	t.Helper()
	ensureStack(t)
	waitForTCP(t, "localhost:8082", 30*time.Second)
	waitForTCP(t, "localhost:50052", 30*time.Second)
}

// ── NATS helpers ─────────────────────────────────────────────────────────────

// connectNATS connects to the NATS server and registers cleanup.
func connectNATS(t *testing.T) *nats.Conn {
	t.Helper()
	nc, err := nats.Connect(natsURL)
	if err != nil {
		t.Fatalf("failed to connect to NATS at %s: %v", natsURL, err)
	}
	t.Cleanup(func() { nc.Close() })
	return nc
}

// ── HTTP helpers ─────────────────────────────────────────────────────────────

// httpGet sends a GET request and returns the response.
func httpGet(t *testing.T, url string) *http.Response {
	t.Helper()
	resp, err := http.Get(url) //nolint:noctx
	if err != nil {
		t.Fatalf("GET %s failed: %v", url, err)
	}
	return resp
}

// httpGetAuth sends an authenticated GET request.
func httpGetAuth(t *testing.T, url, token string) *http.Response {
	t.Helper()
	req, err := http.NewRequest(http.MethodGet, url, nil)
	if err != nil {
		t.Fatalf("create request: %v", err)
	}
	req.Header.Set("Authorization", "Bearer "+token)
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("GET %s failed: %v", url, err)
	}
	return resp
}

// httpPostJSON sends a POST request with a JSON body and optional bearer token.
func httpPostJSON(t *testing.T, url, token, body string) *http.Response {
	t.Helper()
	req, err := http.NewRequest(http.MethodPost, url, strings.NewReader(body))
	if err != nil {
		t.Fatalf("create request: %v", err)
	}
	req.Header.Set("Content-Type", "application/json")
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("POST %s failed: %v", url, err)
	}
	return resp
}

// decodeJSON reads the response body and unmarshals it into a map.
func decodeJSON(t *testing.T, resp *http.Response) map[string]interface{} {
	t.Helper()
	defer resp.Body.Close()
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		t.Fatalf("read response body: %v", err)
	}
	var m map[string]interface{}
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatalf("decode JSON: %v\nbody: %s", err, string(data))
	}
	return m
}

// decodeJSONArray reads the response body and unmarshals it into a slice of maps.
func decodeJSONArray(t *testing.T, resp *http.Response) []map[string]interface{} {
	t.Helper()
	defer resp.Body.Close()
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		t.Fatalf("read response body: %v", err)
	}
	var arr []map[string]interface{}
	if err := json.Unmarshal(data, &arr); err != nil {
		t.Fatalf("decode JSON array: %v\nbody: %s", err, string(data))
	}
	return arr
}

// commandURL returns the command submission URL for a given VIN.
func commandURL(vin string) string {
	return fmt.Sprintf("%s/vehicles/%s/commands", cloudGatewayURL, vin)
}

// commandStatusURL returns the command status URL.
func commandStatusURL(vin, commandID string) string {
	return fmt.Sprintf("%s/vehicles/%s/commands/%s", cloudGatewayURL, vin, commandID)
}
