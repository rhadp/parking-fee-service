package e2e

import (
	"fmt"
	"testing"
	"time"

	"github.com/nats-io/nats.go"
)

// ── E2E-1: Service Health ────────────────────────────────────────────────────

// TestServicesHealthy verifies all services are up and responding to health checks.
func TestServicesHealthy(t *testing.T) {
	ensureStack(t)

	tests := []struct {
		name string
		url  string
	}{
		{"parking-fee-service", parkingFeeServiceURL + "/health"},
		{"cloud-gateway", cloudGatewayURL + "/health"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			resp := httpGet(t, tt.url)
			defer resp.Body.Close()
			if resp.StatusCode != 200 {
				t.Errorf("expected 200, got %d", resp.StatusCode)
			}
			body := decodeJSON(t, resp)
			if body["status"] != "ok" {
				t.Errorf("expected status=ok, got %v", body["status"])
			}
		})
	}
}

// ── E2E-2: Operator Discovery ────────────────────────────────────────────────

// TestOperatorDiscovery verifies the full operator lookup flow:
// query parking-fee-service with Munich coordinates → get matching operators.
func TestOperatorDiscovery(t *testing.T) {
	ensureStack(t)

	// Munich Central Station area (inside the default "munich-central" zone).
	url := parkingFeeServiceURL + "/operators?lat=48.1375&lon=11.5600"
	resp := httpGet(t, url)
	if resp.StatusCode != 200 {
		body := decodeJSON(t, resp)
		t.Fatalf("expected 200, got %d: %v", resp.StatusCode, body)
	}

	operators := decodeJSONArray(t, resp)
	if len(operators) == 0 {
		t.Fatal("expected at least one operator for Munich Central coordinates")
	}

	// Verify the response has required fields.
	op := operators[0]
	for _, field := range []string{"id", "name", "zone_id"} {
		if _, ok := op[field]; !ok {
			t.Errorf("operator missing field %q: %v", field, op)
		}
	}

	// Adapter field should NOT be in the operator list response.
	if _, ok := op["adapter"]; ok {
		t.Error("operator list response should not include adapter metadata")
	}
}

// TestAdapterMetadataLookup verifies fetching adapter metadata for a known operator.
func TestAdapterMetadataLookup(t *testing.T) {
	ensureStack(t)

	// First discover operators to get a valid ID.
	listURL := parkingFeeServiceURL + "/operators?lat=48.1375&lon=11.5600"
	listResp := httpGet(t, listURL)
	if listResp.StatusCode != 200 {
		t.Fatalf("operator list failed: %d", listResp.StatusCode)
	}
	operators := decodeJSONArray(t, listResp)
	if len(operators) == 0 {
		t.Fatal("no operators found")
	}

	operatorID := operators[0]["id"].(string)

	// Fetch adapter metadata.
	adapterURL := fmt.Sprintf("%s/operators/%s/adapter", parkingFeeServiceURL, operatorID)
	adapterResp := httpGet(t, adapterURL)
	if adapterResp.StatusCode != 200 {
		t.Fatalf("adapter lookup failed: %d", adapterResp.StatusCode)
	}
	adapter := decodeJSON(t, adapterResp)

	for _, field := range []string{"image_ref", "checksum_sha256", "version"} {
		if _, ok := adapter[field]; !ok {
			t.Errorf("adapter metadata missing field %q: %v", field, adapter)
		}
	}
}

// ── E2E-3: Command Relay via Cloud Gateway ───────────────────────────────────

// TestCommandSubmitAndNATSRelay verifies the full command flow:
//  1. Submit a lock command via cloud-gateway REST API
//  2. Verify it arrives on the NATS command subject with the bearer token
//  3. Publish a success response on NATS
//  4. Verify the command status updates to "success" via the REST API
func TestCommandSubmitAndNATSRelay(t *testing.T) {
	ensureStack(t)

	nc := connectNATS(t)

	// Subscribe to the NATS command subject before submitting.
	commandSubject := fmt.Sprintf("vehicles.%s.commands", testVIN)
	received := make(chan struct{ payload, auth string }, 1)
	sub, err := nc.Subscribe(commandSubject, func(msg *nats.Msg) {
		received <- struct{ payload, auth string }{
			payload: string(msg.Data),
			auth:    msg.Header.Get("Authorization"),
		}
	})
	if err != nil {
		t.Fatalf("subscribe: %v", err)
	}
	defer sub.Unsubscribe() //nolint:errcheck
	nc.Flush()              //nolint:errcheck

	// Step 1: Submit a command via REST.
	cmdBody := `{"command_id":"e2e-cmd-001","type":"lock","doors":["driver"]}`
	resp := httpPostJSON(t, commandURL(testVIN), testToken, cmdBody)
	if resp.StatusCode != 202 {
		body := decodeJSON(t, resp)
		t.Fatalf("expected 202 Accepted, got %d: %v", resp.StatusCode, body)
	}
	resp.Body.Close()

	// Step 2: Verify the command arrived on NATS with the bearer token.
	select {
	case msg := <-received:
		expectedAuth := "Bearer " + testToken
		if msg.auth != expectedAuth {
			t.Errorf("NATS Authorization header = %q, want %q", msg.auth, expectedAuth)
		}
		t.Logf("NATS command received: %s", msg.payload)
	case <-time.After(10 * time.Second):
		t.Fatal("timed out waiting for command on NATS")
	}

	// Step 3: Publish a success response via NATS.
	responseSubject := fmt.Sprintf("vehicles.%s.command_responses", testVIN)
	responsePayload := `{"command_id":"e2e-cmd-001","status":"success"}`
	if err := nc.Publish(responseSubject, []byte(responsePayload)); err != nil {
		t.Fatalf("publish response: %v", err)
	}
	nc.Flush() //nolint:errcheck

	// Step 4: Poll the status endpoint until the command is resolved.
	statusURL := commandStatusURL(testVIN, "e2e-cmd-001")
	deadline := time.Now().Add(10 * time.Second)
	var finalStatus string
	for time.Now().Before(deadline) {
		statusResp := httpGetAuth(t, statusURL, testToken)
		body := decodeJSON(t, statusResp)
		if s, ok := body["status"].(string); ok && s != "pending" {
			finalStatus = s
			break
		}
		time.Sleep(300 * time.Millisecond)
	}

	if finalStatus != "success" {
		t.Errorf("expected command status 'success', got %q", finalStatus)
	}
}

// ── E2E-4: Operator Discovery → Adapter Lookup (full chain) ─────────────────

// TestDiscoveryToAdapterChain exercises the end-to-end discovery flow:
//  1. Query operators by location
//  2. Pick the first operator
//  3. Fetch its adapter metadata
//
// This mirrors what the PARKING_APP would do on the vehicle.
func TestDiscoveryToAdapterChain(t *testing.T) {
	ensureStack(t)

	// Marienplatz area (inside "munich-marienplatz" zone).
	listURL := parkingFeeServiceURL + "/operators?lat=48.1365&lon=11.5760"
	listResp := httpGet(t, listURL)
	if listResp.StatusCode != 200 {
		t.Fatalf("operator lookup: %d", listResp.StatusCode)
	}
	operators := decodeJSONArray(t, listResp)
	if len(operators) == 0 {
		t.Fatal("no operators found near Marienplatz")
	}

	// Should find the CityPark operator.
	var found bool
	var operatorID string
	for _, op := range operators {
		if id, ok := op["id"].(string); ok {
			operatorID = id
			if id == "city-park-munich" {
				found = true
				break
			}
		}
	}
	if !found {
		t.Logf("expected city-park-munich, found operators: %v", operators)
	}

	// Fetch adapter metadata for the operator.
	adapterURL := fmt.Sprintf("%s/operators/%s/adapter", parkingFeeServiceURL, operatorID)
	adapterResp := httpGet(t, adapterURL)
	if adapterResp.StatusCode != 200 {
		t.Fatalf("adapter fetch for %s: %d", operatorID, adapterResp.StatusCode)
	}
	adapter := decodeJSON(t, adapterResp)
	if _, ok := adapter["image_ref"]; !ok {
		t.Errorf("adapter metadata missing image_ref: %v", adapter)
	}
	t.Logf("adapter for %s: image_ref=%s version=%s", operatorID, adapter["image_ref"], adapter["version"])
}
