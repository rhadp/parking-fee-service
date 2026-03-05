package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

// testServer creates an HTTP handler wired with the given stores and NATS client.
func testServer(tokenStore *TokenStore, commandStore *CommandStore, natsClient *NATSClient, knownVINs map[string]bool) http.Handler {
	mux := http.NewServeMux()

	mux.HandleFunc("GET /health", HandleHealth)
	mux.Handle("POST /vehicles/{vin}/commands",
		AuthMiddleware(tokenStore, knownVINs)(HandleCommandSubmit(commandStore, natsClient, knownVINs)))
	mux.Handle("GET /vehicles/{vin}/commands/{command_id}",
		AuthMiddleware(tokenStore, knownVINs)(HandleCommandStatus(commandStore, knownVINs)))
	mux.HandleFunc("/", NotFoundHandler())

	return mux
}

func defaultTestSetup() (*TokenStore, *CommandStore, map[string]bool) {
	tokens := map[string]string{
		"companion-token-vehicle-1": "VIN12345",
		"companion-token-vehicle-2": "VIN67890",
	}
	tokenStore := NewTokenStore(tokens)
	commandStore := NewCommandStore()
	knownVINs := map[string]bool{
		"VIN12345": true,
		"VIN67890": true,
	}
	return tokenStore, commandStore, knownVINs
}

// TS-06-1: Command Submission via REST
func TestCommandSubmission(t *testing.T) {
	tokenStore, commandStore, knownVINs := defaultTestSetup()
	// For this test, we need a NATS client that can publish.
	// Since we're testing the handler, we use a nil client for now.
	// The handler should return 202 when NATS publish succeeds.
	// This test will fail until the handler and NATS client are implemented.
	handler := testServer(tokenStore, commandStore, nil, knownVINs)

	body := `{"command_id":"cmd-001","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands", bytes.NewBufferString(body))
	req.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusAccepted {
		t.Errorf("expected status 202, got %d", resp.StatusCode)
	}

	var result map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	if result["command_id"] != "cmd-001" {
		t.Errorf("expected command_id 'cmd-001', got %v", result["command_id"])
	}
	if result["status"] != "pending" {
		t.Errorf("expected status 'pending', got %v", result["status"])
	}
}

// TS-06-2: Bearer Token Validation - Valid Token
func TestBearerTokenValid(t *testing.T) {
	tokenStore, commandStore, knownVINs := defaultTestSetup()
	handler := testServer(tokenStore, commandStore, nil, knownVINs)

	body := `{"command_id":"cmd-002","type":"unlock","doors":["driver"]}`
	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands", bytes.NewBufferString(body))
	req.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusAccepted {
		t.Errorf("expected status 202 (auth should pass), got %d", resp.StatusCode)
	}
}

// TS-06-4: Command Response Forwarding
func TestCommandResponseForwarding(t *testing.T) {
	tokenStore, commandStore, knownVINs := defaultTestSetup()
	handler := testServer(tokenStore, commandStore, nil, knownVINs)

	// Simulate a command that was stored as pending
	commandStore.StoreCommand("cmd-003", "pending")

	// Simulate receiving a response from NATS that updated the store
	commandStore.UpdateCommandStatus("cmd-003", "success", "")

	req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/cmd-003", nil)
	req.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected status 200, got %d", resp.StatusCode)
	}

	var result map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	if result["command_id"] != "cmd-003" {
		t.Errorf("expected command_id 'cmd-003', got %v", result["command_id"])
	}
	if result["status"] != "success" {
		t.Errorf("expected status 'success', got %v", result["status"])
	}
}

// TS-06-5: Health Check Returns 200 OK
func TestHealthCheck(t *testing.T) {
	tokenStore, commandStore, knownVINs := defaultTestSetup()
	handler := testServer(tokenStore, commandStore, nil, knownVINs)

	req := httptest.NewRequest("GET", "/health", nil)
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected status 200, got %d", resp.StatusCode)
	}

	ct := resp.Header.Get("Content-Type")
	if ct != "application/json" {
		t.Errorf("expected Content-Type 'application/json', got %q", ct)
	}

	var result map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	if result["status"] != "ok" {
		t.Errorf("expected status 'ok', got %v", result["status"])
	}
}

// TS-06-E1: Missing Authorization Header Returns 401
func TestMissingAuthHeader(t *testing.T) {
	tokenStore, commandStore, knownVINs := defaultTestSetup()
	handler := testServer(tokenStore, commandStore, nil, knownVINs)

	body := `{"command_id":"cmd-e1","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands", bytes.NewBufferString(body))
	// No Authorization header
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusUnauthorized {
		t.Errorf("expected status 401, got %d", resp.StatusCode)
	}

	var result map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	if result["error"] == nil || result["error"] == "" {
		t.Error("expected non-empty error field in response")
	}
}

// TS-06-E2: Invalid Bearer Token Returns 401
func TestInvalidBearerToken(t *testing.T) {
	tokenStore, commandStore, knownVINs := defaultTestSetup()
	handler := testServer(tokenStore, commandStore, nil, knownVINs)

	body := `{"command_id":"cmd-e2","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands", bytes.NewBufferString(body))
	req.Header.Set("Authorization", "Bearer invalid-token-xyz")
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusUnauthorized {
		t.Errorf("expected status 401, got %d", resp.StatusCode)
	}

	var result map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	errMsg, ok := result["error"].(string)
	if !ok || errMsg == "" {
		t.Error("expected non-empty error string in response")
	}
}

// TS-06-E3: Token for Wrong VIN Returns 403
func TestTokenWrongVIN(t *testing.T) {
	tokenStore, commandStore, knownVINs := defaultTestSetup()
	handler := testServer(tokenStore, commandStore, nil, knownVINs)

	body := `{"command_id":"cmd-e3","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest("POST", "/vehicles/VIN67890/commands", bytes.NewBufferString(body))
	req.Header.Set("Authorization", "Bearer companion-token-vehicle-1") // valid for VIN12345, not VIN67890
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusForbidden {
		t.Errorf("expected status 403, got %d", resp.StatusCode)
	}

	var result map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	errMsg, ok := result["error"].(string)
	if !ok || errMsg == "" {
		t.Error("expected non-empty error string in response")
	}
}

// TS-06-E4: Missing Required Fields Returns 400
func TestMissingRequiredFields(t *testing.T) {
	tokenStore, commandStore, knownVINs := defaultTestSetup()
	handler := testServer(tokenStore, commandStore, nil, knownVINs)

	tests := []struct {
		name string
		body string
	}{
		{"missing command_id", `{"type":"lock","doors":["driver"]}`},
		{"missing type", `{"command_id":"cmd-e4","doors":["driver"]}`},
		{"missing doors", `{"command_id":"cmd-e4","type":"lock"}`},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands", bytes.NewBufferString(tt.body))
			req.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
			req.Header.Set("Content-Type", "application/json")
			w := httptest.NewRecorder()

			handler.ServeHTTP(w, req)

			resp := w.Result()
			if resp.StatusCode != http.StatusBadRequest {
				t.Errorf("expected status 400, got %d", resp.StatusCode)
			}

			var result map[string]interface{}
			if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
				t.Fatalf("failed to decode response body: %v", err)
			}
			if result["error"] == nil || result["error"] == "" {
				t.Error("expected non-empty error field in response")
			}
		})
	}
}

// TS-06-E5: Invalid Command Type Returns 400
func TestInvalidCommandType(t *testing.T) {
	tokenStore, commandStore, knownVINs := defaultTestSetup()
	handler := testServer(tokenStore, commandStore, nil, knownVINs)

	body := `{"command_id":"cmd-e5","type":"start_engine","doors":["driver"]}`
	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands", bytes.NewBufferString(body))
	req.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusBadRequest {
		t.Errorf("expected status 400, got %d", resp.StatusCode)
	}

	var result map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	errMsg, ok := result["error"].(string)
	if !ok || errMsg == "" {
		t.Error("expected non-empty error string in response")
	}
}

// TS-06-E6: Unknown VIN Returns 404
func TestUnknownVIN(t *testing.T) {
	tokenStore, commandStore, knownVINs := defaultTestSetup()
	// Add a token for the unknown VIN so we can get past auth to test VIN validation
	tokenStore.tokens["token-unknown"] = "UNKNOWN_VIN"
	handler := testServer(tokenStore, commandStore, nil, knownVINs)

	body := `{"command_id":"cmd-e6","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest("POST", "/vehicles/UNKNOWN_VIN/commands", bytes.NewBufferString(body))
	req.Header.Set("Authorization", "Bearer token-unknown")
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusNotFound {
		t.Errorf("expected status 404, got %d", resp.StatusCode)
	}

	var result map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	errMsg, ok := result["error"].(string)
	if !ok || errMsg == "" {
		t.Error("expected non-empty error string in response")
	}
}

// TS-06-E7: Unknown Command ID Returns 404
func TestUnknownCommandID(t *testing.T) {
	tokenStore, commandStore, knownVINs := defaultTestSetup()
	handler := testServer(tokenStore, commandStore, nil, knownVINs)

	req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/nonexistent-cmd", nil)
	req.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusNotFound {
		t.Errorf("expected status 404, got %d", resp.StatusCode)
	}

	var result map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	errMsg, ok := result["error"].(string)
	if !ok || errMsg == "" {
		t.Error("expected non-empty error string in response")
	}
}

// TS-06-E9: Undefined Route Returns 404
func TestUndefinedRoute(t *testing.T) {
	tokenStore, commandStore, knownVINs := defaultTestSetup()
	handler := testServer(tokenStore, commandStore, nil, knownVINs)

	req := httptest.NewRequest("GET", "/nonexistent-path", nil)
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusNotFound {
		t.Errorf("expected status 404, got %d", resp.StatusCode)
	}

	ct := resp.Header.Get("Content-Type")
	if ct != "application/json" {
		t.Errorf("expected Content-Type 'application/json', got %q", ct)
	}

	var result map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	if result["error"] == nil || result["error"] == "" {
		t.Error("expected non-empty error field in response")
	}
}
