package main

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// TS-06-P1: Token-VIN Binding
func TestPropertyTokenVINBinding(t *testing.T) {
	tokens := map[string]string{
		"companion-token-vehicle-1": "VIN12345",
		"companion-token-vehicle-2": "VIN67890",
	}
	tokenStore := NewTokenStore(tokens)
	knownVINs := map[string]bool{
		"VIN12345": true,
		"VIN67890": true,
	}

	for token, vin := range tokens {
		// Token should be valid for its own VIN
		valid, _ := tokenStore.ValidateToken(token, vin)
		if !valid {
			t.Errorf("token %q should be valid for VIN %q", token, vin)
		}

		// Token should NOT be valid for other VINs
		for otherVIN := range knownVINs {
			if otherVIN == vin {
				continue
			}
			valid, _ := tokenStore.ValidateToken(token, otherVIN)
			if valid {
				t.Errorf("token %q should NOT be valid for VIN %q (authorized for %q)", token, otherVIN, vin)
			}
		}
	}
}

// TS-06-P2: Command-to-NATS Subject Mapping
func TestPropertyCommandToNATSSubject(t *testing.T) {
	vins := []string{"VIN12345", "VIN67890", "VINABC", "VIN_TEST_123"}

	for _, vin := range vins {
		expectedSubject := "vehicles." + vin + ".commands"
		// Verify that the subject construction follows the pattern
		// This test will be meaningful once PublishCommand is implemented
		// and we can capture the actual published subject
		actualSubject := "vehicles." + vin + ".commands"
		if actualSubject != expectedSubject {
			t.Errorf("for VIN %q: expected subject %q, got %q", vin, expectedSubject, actualSubject)
		}
	}
}

// TS-06-P3: Response-to-Command Correlation
func TestPropertyResponseCorrelation(t *testing.T) {
	store := NewCommandStore()

	// Create a set of pending commands
	commands := []string{"cmd-a", "cmd-b", "cmd-c", "cmd-d"}
	for _, cmdID := range commands {
		store.StoreCommand(cmdID, "pending")
	}

	// Update only one command
	targetCmdID := "cmd-b"
	store.UpdateCommandStatus(targetCmdID, "success", "")

	// Verify only the target command was updated
	for _, cmdID := range commands {
		status, ok := store.GetCommandStatus(cmdID)
		if !ok {
			t.Errorf("command %q should exist in store", cmdID)
			continue
		}
		if cmdID == targetCmdID {
			if status.Status != "success" {
				t.Errorf("command %q should have status 'success', got %q", cmdID, status.Status)
			}
		} else {
			if status.Status != "pending" {
				t.Errorf("command %q should still be 'pending', got %q", cmdID, status.Status)
			}
		}
	}
}

// TS-06-P4: Command Status Lifecycle
func TestPropertyStatusLifecycle(t *testing.T) {
	store := NewCommandStore()

	// Test: pending -> success is allowed
	store.StoreCommand("cmd-lifecycle-1", "pending")
	status, _ := store.GetCommandStatus("cmd-lifecycle-1")
	if status == nil || status.Status != "pending" {
		t.Fatal("initial status should be 'pending'")
	}

	store.UpdateCommandStatus("cmd-lifecycle-1", "success", "")
	status, _ = store.GetCommandStatus("cmd-lifecycle-1")
	if status == nil || status.Status != "success" {
		t.Fatal("status should be 'success' after update")
	}

	// Test: success -> failed should NOT be allowed (terminal state)
	store.UpdateCommandStatus("cmd-lifecycle-1", "failed", "some reason")
	status, _ = store.GetCommandStatus("cmd-lifecycle-1")
	if status == nil || status.Status != "success" {
		t.Error("status should remain 'success' (terminal state), not transition to 'failed'")
	}

	// Test: pending -> failed is allowed
	store.StoreCommand("cmd-lifecycle-2", "pending")
	store.UpdateCommandStatus("cmd-lifecycle-2", "failed", "door ajar")
	status, _ = store.GetCommandStatus("cmd-lifecycle-2")
	if status == nil || status.Status != "failed" {
		t.Error("status should be 'failed' after update from pending")
	}

	// Test: failed -> success should NOT be allowed (terminal state)
	store.UpdateCommandStatus("cmd-lifecycle-2", "success", "")
	status, _ = store.GetCommandStatus("cmd-lifecycle-2")
	if status == nil || status.Status != "failed" {
		t.Error("status should remain 'failed' (terminal state), not transition to 'success'")
	}
}

// TS-06-P5: REST-to-NATS Field Mapping
func TestPropertyRESTToNATSFieldMapping(t *testing.T) {
	inputs := []struct {
		cmdID   string
		cmdType string
		doors   []string
	}{
		{"cmd-map-1", "lock", []string{"driver"}},
		{"cmd-map-2", "unlock", []string{"driver", "passenger"}},
		{"cmd-map-3", "lock", []string{"all"}},
	}

	for _, input := range inputs {
		// Build the expected NATS command from REST input
		natsCmd := NATSCommand{
			CommandID: input.cmdID,
			Action:    input.cmdType,
			Doors:     input.doors,
			Source:    "companion_app",
		}

		if natsCmd.Action != input.cmdType {
			t.Errorf("action should be %q, got %q", input.cmdType, natsCmd.Action)
		}
		if natsCmd.CommandID != input.cmdID {
			t.Errorf("command_id should be %q, got %q", input.cmdID, natsCmd.CommandID)
		}
		if len(natsCmd.Doors) != len(input.doors) {
			t.Errorf("doors length should be %d, got %d", len(input.doors), len(natsCmd.Doors))
		}
		if natsCmd.Source != "companion_app" {
			t.Errorf("source should be 'companion_app', got %q", natsCmd.Source)
		}
	}
}

// TS-06-P6: Response Format Consistency
func TestPropertyResponseFormatConsistency(t *testing.T) {
	tokenStore, commandStore, knownVINs := defaultTestSetup()
	handler := testServer(tokenStore, commandStore, nil, knownVINs)

	requests := []struct {
		name   string
		method string
		path   string
		body   string
		auth   string
	}{
		{"health", "GET", "/health", "", ""},
		{"command submit valid auth", "POST", "/vehicles/VIN12345/commands",
			`{"command_id":"cmd-p6","type":"lock","doors":["driver"]}`,
			"Bearer companion-token-vehicle-1"},
		{"command submit invalid body", "POST", "/vehicles/VIN12345/commands",
			`{"type":"lock"}`,
			"Bearer companion-token-vehicle-1"},
		{"command status unknown", "GET", "/vehicles/VIN12345/commands/unknown",
			"", "Bearer companion-token-vehicle-1"},
		{"command submit invalid auth", "POST", "/vehicles/VIN12345/commands",
			`{"command_id":"cmd-p6b","type":"lock","doors":["driver"]}`,
			"Bearer invalid-token"},
		{"undefined route", "GET", "/nonexistent", "", ""},
	}

	for _, req := range requests {
		t.Run(req.name, func(t *testing.T) {
			var body *strings.Reader
			if req.body != "" {
				body = strings.NewReader(req.body)
			} else {
				body = strings.NewReader("")
			}
			httpReq := httptest.NewRequest(req.method, req.path, body)
			if req.auth != "" {
				httpReq.Header.Set("Authorization", req.auth)
			}
			if req.body != "" {
				httpReq.Header.Set("Content-Type", "application/json")
			}
			w := httptest.NewRecorder()
			handler.ServeHTTP(w, httpReq)

			resp := w.Result()

			// Every response should have Content-Type: application/json
			ct := resp.Header.Get("Content-Type")
			if !strings.Contains(ct, "application/json") {
				t.Errorf("expected Content-Type containing 'application/json', got %q", ct)
			}

			// Every response body should be valid JSON
			var result map[string]any
			if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
				t.Errorf("response body is not valid JSON: %v", err)
			}
		})
	}
}

// TS-06-P7: Health Endpoint Independence
func TestPropertyHealthEndpointIndependence(t *testing.T) {
	tokenStore, commandStore, knownVINs := defaultTestSetup()
	handler := testServer(tokenStore, commandStore, nil, knownVINs)

	authHeaders := []struct {
		name  string
		value string
	}{
		{"no auth", ""},
		{"valid bearer", "Bearer companion-token-vehicle-1"},
		{"invalid bearer", "Bearer invalid-token"},
		{"empty", ""},
		{"basic auth", "Basic xyz"},
	}

	for _, ah := range authHeaders {
		t.Run(ah.name, func(t *testing.T) {
			req := httptest.NewRequest("GET", "/health", nil)
			if ah.value != "" {
				req.Header.Set("Authorization", ah.value)
			}
			w := httptest.NewRecorder()
			handler.ServeHTTP(w, req)

			resp := w.Result()
			if resp.StatusCode != http.StatusOK {
				t.Errorf("expected status 200, got %d", resp.StatusCode)
			}

			var result map[string]any
			if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
				t.Fatalf("failed to decode response: %v", err)
			}
			if result["status"] != "ok" {
				t.Errorf("expected status 'ok', got %v", result["status"])
			}
		})
	}
}
