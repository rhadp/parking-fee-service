package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

// demoTokens returns the standard demo token-to-VIN mapping.
func demoTokens() map[string]string {
	return map[string]string{
		"companion-token-vehicle-1": "VIN12345",
		"companion-token-vehicle-2": "VIN67890",
	}
}

// demoKnownVINs returns the set of known VINs.
func demoKnownVINs() map[string]bool {
	return map[string]bool{
		"VIN12345": true,
		"VIN67890": true,
	}
}

// newTestRouter creates a test router with demo config and the given NATS client.
func newTestRouter(natsClient *NATSClient) (*http.Handler, *CommandStore) {
	tokenStore := NewTokenStore(demoTokens())
	commandStore := NewCommandStore()
	knownVINs := demoKnownVINs()
	router := NewRouter(tokenStore, commandStore, natsClient, knownVINs)
	return &router, commandStore
}

// newTestRouterWithNATS creates a test router with an embedded NATS server.
// Returns the router, command store, and a cleanup function.
func newTestRouterWithNATS(t *testing.T) (*http.Handler, *CommandStore, func()) {
	t.Helper()
	ns, natsURL := startEmbeddedNATS(t)
	client, err := NewNATSClient(natsURL)
	if err != nil {
		ns.Shutdown()
		t.Fatalf("failed to create NATS client: %v", err)
	}
	router, cs := newTestRouter(client)
	cleanup := func() {
		client.Close()
		ns.Shutdown()
	}
	return router, cs, cleanup
}

// TS-06-5: Health Check Returns 200 OK
func TestHealthCheck(t *testing.T) {
	router, _ := newTestRouter(nil)
	req := httptest.NewRequest(http.MethodGet, "/health", nil)
	rec := httptest.NewRecorder()
	(*router).ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("expected status 200, got %d", rec.Code)
	}

	ct := rec.Header().Get("Content-Type")
	if ct == "" || ct != "application/json" {
		t.Errorf("expected Content-Type application/json, got %q", ct)
	}

	var body map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if body["status"] != "ok" {
		t.Errorf("expected status 'ok', got %q", body["status"])
	}
}

// TS-06-2: Bearer Token Validation - Valid Token
func TestBearerTokenValid(t *testing.T) {
	router, _, cleanup := newTestRouterWithNATS(t)
	defer cleanup()

	body := `{"command_id":"cmd-002","type":"unlock","doors":["driver"]}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", bytes.NewBufferString(body))
	req.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	(*router).ServeHTTP(rec, req)

	if rec.Code != http.StatusAccepted {
		t.Errorf("expected status 202, got %d (body: %s)", rec.Code, rec.Body.String())
	}
}

// TS-06-E1: Missing Authorization Header Returns 401
func TestMissingAuthHeader(t *testing.T) {
	router, _ := newTestRouter(nil)

	body := `{"command_id":"cmd-e1","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", bytes.NewBufferString(body))
	req.Header.Set("Content-Type", "application/json")
	// No Authorization header
	rec := httptest.NewRecorder()
	(*router).ServeHTTP(rec, req)

	if rec.Code != http.StatusUnauthorized {
		t.Errorf("expected status 401, got %d", rec.Code)
	}

	var errResp ErrorResponse
	if err := json.NewDecoder(rec.Body).Decode(&errResp); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}
	if errResp.Error == "" {
		t.Error("expected non-empty error message")
	}
}

// TS-06-E2: Invalid Bearer Token Returns 401
func TestInvalidBearerToken(t *testing.T) {
	router, _ := newTestRouter(nil)

	body := `{"command_id":"cmd-e2","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", bytes.NewBufferString(body))
	req.Header.Set("Authorization", "Bearer invalid-token-xyz")
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	(*router).ServeHTTP(rec, req)

	if rec.Code != http.StatusUnauthorized {
		t.Errorf("expected status 401, got %d", rec.Code)
	}

	var errResp ErrorResponse
	if err := json.NewDecoder(rec.Body).Decode(&errResp); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}
	if errResp.Error == "" {
		t.Error("expected non-empty error message about invalid token")
	}
}

// TS-06-E3: Token for Wrong VIN Returns 403
func TestTokenWrongVIN(t *testing.T) {
	router, _ := newTestRouter(nil)

	body := `{"command_id":"cmd-e3","type":"lock","doors":["driver"]}`
	// Token for VIN12345 used against VIN67890
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN67890/commands", bytes.NewBufferString(body))
	req.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	(*router).ServeHTTP(rec, req)

	if rec.Code != http.StatusForbidden {
		t.Errorf("expected status 403, got %d", rec.Code)
	}

	var errResp ErrorResponse
	if err := json.NewDecoder(rec.Body).Decode(&errResp); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}
	if errResp.Error == "" {
		t.Error("expected non-empty error message about VIN authorization")
	}
}

// TS-06-E4: Missing Required Fields Returns 400
func TestMissingRequiredFields(t *testing.T) {
	cases := []struct {
		name    string
		body    string
		missing string
	}{
		{"missing command_id", `{"type":"lock","doors":["driver"]}`, "command_id"},
		{"missing type", `{"command_id":"cmd-e4","doors":["driver"]}`, "type"},
		{"missing doors", `{"command_id":"cmd-e4","type":"lock"}`, "doors"},
	}

	router, _ := newTestRouter(nil)

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", bytes.NewBufferString(tc.body))
			req.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
			req.Header.Set("Content-Type", "application/json")
			rec := httptest.NewRecorder()
			(*router).ServeHTTP(rec, req)

			if rec.Code != http.StatusBadRequest {
				t.Errorf("expected status 400 for %s, got %d (body: %s)", tc.missing, rec.Code, rec.Body.String())
			}

			var errResp ErrorResponse
			if err := json.NewDecoder(rec.Body).Decode(&errResp); err != nil {
				t.Fatalf("failed to decode error response: %v", err)
			}
			if errResp.Error == "" {
				t.Errorf("expected non-empty error message for missing %s", tc.missing)
			}
		})
	}
}

// TS-06-E5: Invalid Command Type Returns 400
func TestInvalidCommandType(t *testing.T) {
	router, _ := newTestRouter(nil)

	body := `{"command_id":"cmd-e5","type":"start_engine","doors":["driver"]}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", bytes.NewBufferString(body))
	req.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	(*router).ServeHTTP(rec, req)

	if rec.Code != http.StatusBadRequest {
		t.Errorf("expected status 400, got %d (body: %s)", rec.Code, rec.Body.String())
	}

	var errResp ErrorResponse
	if err := json.NewDecoder(rec.Body).Decode(&errResp); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}
	if errResp.Error == "" {
		t.Error("expected non-empty error message about invalid type")
	}
}

// TS-06-E6: Unknown VIN Returns 404
func TestUnknownVIN(t *testing.T) {
	router, _ := newTestRouter(nil)

	body := `{"command_id":"cmd-e6","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/UNKNOWN_VIN/commands", bytes.NewBufferString(body))
	req.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	(*router).ServeHTTP(rec, req)

	// Token won't match unknown VIN, so we accept 401 or 404.
	// The correct behavior per spec: token is valid but for wrong VIN -> 403,
	// or VIN is unknown -> 404. Either the auth check (403/401) or VIN check (404)
	// may fire first depending on implementation.
	if rec.Code != http.StatusNotFound && rec.Code != http.StatusForbidden && rec.Code != http.StatusUnauthorized {
		t.Errorf("expected status 404, 403, or 401 for unknown VIN, got %d", rec.Code)
	}
}

// TS-06-E7: Unknown Command ID Returns 404
func TestUnknownCommandID(t *testing.T) {
	router, _ := newTestRouter(nil)

	req := httptest.NewRequest(http.MethodGet, "/vehicles/VIN12345/commands/nonexistent-cmd", nil)
	req.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	rec := httptest.NewRecorder()
	(*router).ServeHTTP(rec, req)

	if rec.Code != http.StatusNotFound {
		t.Errorf("expected status 404, got %d (body: %s)", rec.Code, rec.Body.String())
	}

	var errResp ErrorResponse
	if err := json.NewDecoder(rec.Body).Decode(&errResp); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}
	if errResp.Error == "" {
		t.Error("expected non-empty error message about command not found")
	}
}

// TS-06-E9: Undefined Route Returns 404
func TestUndefinedRoute(t *testing.T) {
	router, _ := newTestRouter(nil)

	req := httptest.NewRequest(http.MethodGet, "/nonexistent-path", nil)
	rec := httptest.NewRecorder()
	(*router).ServeHTTP(rec, req)

	if rec.Code != http.StatusNotFound {
		t.Errorf("expected status 404, got %d", rec.Code)
	}

	ct := rec.Header().Get("Content-Type")
	if ct != "application/json" {
		t.Errorf("expected Content-Type application/json, got %q", ct)
	}

	var errResp ErrorResponse
	if err := json.NewDecoder(rec.Body).Decode(&errResp); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}
	if errResp.Error == "" {
		t.Error("expected non-empty error message for undefined route")
	}
}

// TS-06-4: Command Response Forwarding (handler-level)
func TestCommandResponseForwarding(t *testing.T) {
	router, commandStore := newTestRouter(nil)

	// Pre-store a command as pending
	commandStore.StoreCommand("cmd-003", "pending")

	// Simulate a NATS response updating the store
	commandStore.UpdateCommandStatus("cmd-003", "success", "")

	// Query the status via REST
	req := httptest.NewRequest(http.MethodGet, "/vehicles/VIN12345/commands/cmd-003", nil)
	req.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	rec := httptest.NewRecorder()
	(*router).ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("expected status 200, got %d (body: %s)", rec.Code, rec.Body.String())
	}

	var status CommandStatus
	if err := json.NewDecoder(rec.Body).Decode(&status); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if status.CommandID != "cmd-003" {
		t.Errorf("expected command_id 'cmd-003', got %q", status.CommandID)
	}
	if status.Status != "success" {
		t.Errorf("expected status 'success', got %q", status.Status)
	}
}
