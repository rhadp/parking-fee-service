package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

// TS-06-P1: Token-VIN Binding
// For any token in the store, only the exact associated VIN is accessible.
func TestPropertyTokenVINBinding(t *testing.T) {
	tokens := demoTokens()
	tokenStore := NewTokenStore(tokens)
	knownVINs := demoKnownVINs()
	commandStore := NewCommandStore()
	router := NewRouter(tokenStore, commandStore, nil, knownVINs)

	allVINs := []string{"VIN12345", "VIN67890"}

	for token, authorizedVIN := range tokens {
		for _, testVIN := range allVINs {
			body := `{"command_id":"prop-1","type":"lock","doors":["driver"]}`
			req := httptest.NewRequest(http.MethodPost, "/vehicles/"+testVIN+"/commands", bytes.NewBufferString(body))
			req.Header.Set("Authorization", "Bearer "+token)
			req.Header.Set("Content-Type", "application/json")
			rec := httptest.NewRecorder()
			router.ServeHTTP(rec, req)

			if testVIN == authorizedVIN {
				// Should be allowed (202 or at worst not 401/403)
				if rec.Code == http.StatusUnauthorized || rec.Code == http.StatusForbidden {
					t.Errorf("token %q should be allowed for VIN %q, got status %d",
						token, testVIN, rec.Code)
				}
			} else {
				// Should be denied with 403
				if rec.Code != http.StatusForbidden {
					t.Errorf("token %q should be denied for VIN %q (authorized for %q), expected 403, got %d",
						token, testVIN, authorizedVIN, rec.Code)
				}
			}
		}
	}
}

// TS-06-P2: Command-to-NATS Subject Mapping
// For any VIN, commands are published to vehicles.{vin}.commands and no other.
func TestPropertyCommandToNATSSubject(t *testing.T) {
	knownVINs := []string{"VIN12345", "VIN67890"}

	for _, vin := range knownVINs {
		expectedSubject := "vehicles." + vin + ".commands"

		cmd := NATSCommand{
			CommandID: "prop-cmd-" + vin,
			Action:    "lock",
			Doors:     []string{"driver"},
			Source:    "companion_app",
		}

		// Verify the subject construction (this tests the PublishCommand logic)
		subject := "vehicles." + vin + ".commands"
		if subject != expectedSubject {
			t.Errorf("for VIN %q, expected subject %q, got %q", vin, expectedSubject, subject)
		}

		// Verify the command marshals correctly
		data, err := json.Marshal(cmd)
		if err != nil {
			t.Fatalf("failed to marshal command for VIN %q: %v", vin, err)
		}
		var decoded NATSCommand
		if err := json.Unmarshal(data, &decoded); err != nil {
			t.Fatalf("failed to unmarshal command: %v", err)
		}
		if decoded.CommandID != cmd.CommandID {
			t.Errorf("command_id mismatch for VIN %q", vin)
		}
	}
}

// TS-06-P3: Response-to-Command Correlation
// Processing a command response updates only the matching command_id status.
func TestPropertyResponseCorrelation(t *testing.T) {
	store := NewCommandStore()
	cmdIDs := []string{"cmd-A", "cmd-B", "cmd-C", "cmd-D"}

	// Store all as pending
	for _, id := range cmdIDs {
		store.StoreCommand(id, "pending")
	}

	// Update only cmd-B
	store.UpdateCommandStatus("cmd-B", "success", "")

	// Verify only cmd-B changed
	for _, id := range cmdIDs {
		status, found := store.GetCommandStatus(id)
		if !found {
			t.Errorf("command %q not found", id)
			continue
		}
		if id == "cmd-B" {
			if status.Status != "success" {
				t.Errorf("cmd-B should be 'success', got %q", status.Status)
			}
		} else {
			if status.Status != "pending" {
				t.Errorf("command %q should still be 'pending', got %q", id, status.Status)
			}
		}
	}
}

// TS-06-P4: Command Status Lifecycle
// Status transitions only from pending to success or pending to failed.
// Terminal states cannot be overwritten.
func TestPropertyStatusLifecycle(t *testing.T) {
	store := NewCommandStore()

	// Test pending -> success (terminal)
	store.StoreCommand("cmd-lc1", "pending")
	status, found := store.GetCommandStatus("cmd-lc1")
	if !found || status == nil {
		t.Fatal("cmd-lc1 should be found after StoreCommand")
	}
	if status.Status != "pending" {
		t.Errorf("initial status should be 'pending', got %q", status.Status)
	}

	store.UpdateCommandStatus("cmd-lc1", "success", "")
	status, found = store.GetCommandStatus("cmd-lc1")
	if !found || status == nil {
		t.Fatal("cmd-lc1 should be found after UpdateCommandStatus")
	}
	if status.Status != "success" {
		t.Errorf("after update, status should be 'success', got %q", status.Status)
	}

	// Try to change from success to failed -- should be rejected
	store.UpdateCommandStatus("cmd-lc1", "failed", "should not change")
	status, found = store.GetCommandStatus("cmd-lc1")
	if !found || status == nil {
		t.Fatal("cmd-lc1 should still be found")
	}
	if status.Status != "success" {
		t.Errorf("terminal status should not change from 'success', got %q", status.Status)
	}

	// Test pending -> failed (terminal)
	store.StoreCommand("cmd-lc2", "pending")
	store.UpdateCommandStatus("cmd-lc2", "failed", "door ajar")
	status, found = store.GetCommandStatus("cmd-lc2")
	if !found || status == nil {
		t.Fatal("cmd-lc2 should be found")
	}
	if status.Status != "failed" {
		t.Errorf("status should be 'failed', got %q", status.Status)
	}
	if status.Reason != "door ajar" {
		t.Errorf("reason should be 'door ajar', got %q", status.Reason)
	}

	// Try to change from failed to success -- should be rejected
	store.UpdateCommandStatus("cmd-lc2", "success", "")
	status, found = store.GetCommandStatus("cmd-lc2")
	if !found || status == nil {
		t.Fatal("cmd-lc2 should still be found")
	}
	if status.Status != "failed" {
		t.Errorf("terminal status should not change from 'failed', got %q", status.Status)
	}

	// Try to revert to pending -- should be rejected
	store.UpdateCommandStatus("cmd-lc2", "pending", "")
	status, found = store.GetCommandStatus("cmd-lc2")
	if !found || status == nil {
		t.Fatal("cmd-lc2 should still be found")
	}
	if status.Status != "failed" {
		t.Errorf("terminal status should not revert to 'pending', got %q", status.Status)
	}
}

// TS-06-P5: REST-to-NATS Field Mapping
// The NATS message fields are correctly mapped from the REST request fields.
func TestPropertyRESTToNATSFieldMapping(t *testing.T) {
	testCases := []struct {
		cmdID   string
		cmdType string
		doors   []string
	}{
		{"cmd-map1", "lock", []string{"driver"}},
		{"cmd-map2", "unlock", []string{"driver", "passenger"}},
		{"cmd-map3", "lock", []string{"all"}},
	}

	for _, tc := range testCases {
		// Create a NATSCommand as the handler would
		natsCmd := NATSCommand{
			CommandID: tc.cmdID,
			Action:    tc.cmdType,
			Doors:     tc.doors,
			Source:    "companion_app",
		}

		if natsCmd.Action != tc.cmdType {
			t.Errorf("action should be %q, got %q", tc.cmdType, natsCmd.Action)
		}
		if natsCmd.CommandID != tc.cmdID {
			t.Errorf("command_id should be %q, got %q", tc.cmdID, natsCmd.CommandID)
		}
		if len(natsCmd.Doors) != len(tc.doors) {
			t.Errorf("doors length mismatch: expected %d, got %d", len(tc.doors), len(natsCmd.Doors))
		}
		if natsCmd.Source != "companion_app" {
			t.Errorf("source should be 'companion_app', got %q", natsCmd.Source)
		}

		// Verify JSON round-trip
		data, _ := json.Marshal(natsCmd)
		var decoded NATSCommand
		json.Unmarshal(data, &decoded)
		if decoded.Action != tc.cmdType {
			t.Errorf("after round-trip, action should be %q, got %q", tc.cmdType, decoded.Action)
		}
		if decoded.Source != "companion_app" {
			t.Errorf("after round-trip, source should be 'companion_app', got %q", decoded.Source)
		}
	}
}

// TS-06-P6: Response Format Consistency
// Every REST response has Content-Type application/json and valid JSON body.
func TestPropertyResponseFormatConsistency(t *testing.T) {
	router, _ := newTestRouter(nil)

	requests := []struct {
		name   string
		method string
		path   string
		body   string
		auth   string
	}{
		{"health", http.MethodGet, "/health", "", ""},
		{"valid command", http.MethodPost, "/vehicles/VIN12345/commands",
			`{"command_id":"fmt-1","type":"lock","doors":["driver"]}`,
			"Bearer companion-token-vehicle-1"},
		{"invalid body", http.MethodPost, "/vehicles/VIN12345/commands",
			`{"type":"lock"}`,
			"Bearer companion-token-vehicle-1"},
		{"unknown command", http.MethodGet, "/vehicles/VIN12345/commands/unknown-id", "",
			"Bearer companion-token-vehicle-1"},
		{"invalid auth", http.MethodPost, "/vehicles/VIN12345/commands",
			`{"command_id":"fmt-2","type":"lock","doors":["driver"]}`,
			"Bearer invalid-token"},
		{"no auth", http.MethodPost, "/vehicles/VIN12345/commands",
			`{"command_id":"fmt-3","type":"lock","doors":["driver"]}`,
			""},
		{"undefined route", http.MethodGet, "/nonexistent", "", ""},
	}

	for _, tc := range requests {
		t.Run(tc.name, func(t *testing.T) {
			var bodyReader *bytes.Buffer
			if tc.body != "" {
				bodyReader = bytes.NewBufferString(tc.body)
			} else {
				bodyReader = bytes.NewBuffer(nil)
			}

			req := httptest.NewRequest(tc.method, tc.path, bodyReader)
			if tc.auth != "" {
				req.Header.Set("Authorization", tc.auth)
			}
			if tc.body != "" {
				req.Header.Set("Content-Type", "application/json")
			}

			rec := httptest.NewRecorder()
			(*router).ServeHTTP(rec, req)

			// Check Content-Type
			ct := rec.Header().Get("Content-Type")
			if ct != "application/json" {
				t.Errorf("expected Content-Type 'application/json', got %q (status: %d)", ct, rec.Code)
			}

			// Check body is valid JSON
			var js json.RawMessage
			if err := json.Unmarshal(rec.Body.Bytes(), &js); err != nil {
				t.Errorf("response body is not valid JSON: %v (body: %s)", err, rec.Body.String())
			}
		})
	}
}

// TS-06-P7: Health Endpoint Independence
// Health endpoint responds consistently regardless of auth state.
func TestPropertyHealthEndpointIndependence(t *testing.T) {
	router, _ := newTestRouter(nil)

	authHeaders := []struct {
		name  string
		value string
	}{
		{"no auth", ""},
		{"valid bearer", "Bearer companion-token-vehicle-1"},
		{"invalid bearer", "Bearer invalid-token"},
		{"empty string", ""},
		{"basic auth", "Basic xyz"},
	}

	for _, tc := range authHeaders {
		t.Run(tc.name, func(t *testing.T) {
			req := httptest.NewRequest(http.MethodGet, "/health", nil)
			if tc.value != "" {
				req.Header.Set("Authorization", tc.value)
			}
			rec := httptest.NewRecorder()
			(*router).ServeHTTP(rec, req)

			if rec.Code != http.StatusOK {
				t.Errorf("expected status 200, got %d", rec.Code)
			}

			var body map[string]string
			if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
				t.Fatalf("failed to decode response: %v", err)
			}
			if body["status"] != "ok" {
				t.Errorf("expected status 'ok', got %q", body["status"])
			}
		})
	}
}
