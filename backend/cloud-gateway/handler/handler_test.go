package handler_test

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/handler"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// mockCommander is a mock implementation of handler.Commander for testing.
type mockCommander struct {
	lastVIN   string
	lastCmd   any
	lastToken string
	err       error
}

func (m *mockCommander) PublishCommand(vin string, cmd any, token string) error {
	m.lastVIN = vin
	m.lastCmd = cmd
	m.lastToken = token
	return m.err
}

// testConfig returns a config with demo token-VIN mappings.
func testConfig() *model.Config {
	return &model.Config{
		Port:                  8081,
		NatsURL:               "nats://localhost:4222",
		CommandTimeoutSeconds: 30,
		Tokens: []model.TokenMapping{
			{Token: "demo-token-001", VIN: "VIN12345"},
			{Token: "demo-token-002", VIN: "VIN67890"},
		},
	}
}

// setupMux creates an http.ServeMux wired with auth middleware and handlers for testing.
func setupMux(mc *mockCommander, s *store.Store, cfg *model.Config) *http.ServeMux {
	mux := http.NewServeMux()
	authMw := auth.Middleware(cfg)

	mux.Handle("POST /vehicles/{vin}/commands",
		authMw(handler.NewSubmitCommandHandler(mc, s, 30*time.Second)))
	mux.Handle("GET /vehicles/{vin}/commands/{command_id}",
		authMw(handler.NewGetCommandStatusHandler(s)))
	mux.Handle("GET /health", handler.HealthHandler())

	return mux
}

// TS-06-1: Command Submission Success (handler-level, mock NATS)
// Requirement: 06-REQ-1.1
// A POST to /vehicles/{vin}/commands with a valid token and body returns HTTP 202.
func TestCommandSubmissionSuccess(t *testing.T) {
	mc := &mockCommander{}
	s := store.NewStore()
	cfg := testConfig()
	mux := setupMux(mc, s, cfg)

	body := `{"command_id":"cmd-001","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands",
		bytes.NewBufferString(body))
	req.Header.Set("Authorization", "Bearer demo-token-001")
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()

	mux.ServeHTTP(rec, req)

	if rec.Code != 202 {
		t.Errorf("status = %d, want 202", rec.Code)
	}

	var resp map[string]any
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if resp["command_id"] != "cmd-001" {
		t.Errorf("command_id = %v, want %q", resp["command_id"], "cmd-001")
	}
	if resp["type"] != "lock" {
		t.Errorf("type = %v, want %q", resp["type"], "lock")
	}
	doors, ok := resp["doors"].([]any)
	if !ok || len(doors) != 1 || doors[0] != "driver" {
		t.Errorf("doors = %v, want [\"driver\"]", resp["doors"])
	}
}

// TS-06-4: Command Status Query Success
// Requirement: 06-REQ-2.1
// GET /vehicles/{vin}/commands/{command_id} returns the stored command response.
func TestCommandStatusQuerySuccess(t *testing.T) {
	mc := &mockCommander{}
	s := store.NewStore()
	cfg := testConfig()
	mux := setupMux(mc, s, cfg)

	// Pre-store a response
	s.StoreResponse(model.CommandResponse{CommandID: "cmd-004", Status: "success"})

	req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/cmd-004", nil)
	req.Header.Set("Authorization", "Bearer demo-token-001")
	rec := httptest.NewRecorder()

	mux.ServeHTTP(rec, req)

	if rec.Code != 200 {
		t.Errorf("status = %d, want 200", rec.Code)
	}

	var resp map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if resp["command_id"] != "cmd-004" {
		t.Errorf("command_id = %q, want %q", resp["command_id"], "cmd-004")
	}
	if resp["status"] != "success" {
		t.Errorf("status = %q, want %q", resp["status"], "success")
	}
}

// TS-06-10: Health Check
// Requirement: 06-REQ-4.1
// GET /health returns HTTP 200 with {"status":"ok"}.
func TestHealthCheck(t *testing.T) {
	mc := &mockCommander{}
	s := store.NewStore()
	cfg := testConfig()
	mux := setupMux(mc, s, cfg)

	req := httptest.NewRequest("GET", "/health", nil)
	rec := httptest.NewRecorder()

	mux.ServeHTTP(rec, req)

	if rec.Code != 200 {
		t.Errorf("status = %d, want 200", rec.Code)
	}

	var resp map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if resp["status"] != "ok" {
		t.Errorf("status = %q, want %q", resp["status"], "ok")
	}
}

// TS-06-13: Content-Type Header
// Requirement: 06-REQ-7.1
// All REST responses set Content-Type: application/json.
func TestContentTypeHeader(t *testing.T) {
	mc := &mockCommander{}
	s := store.NewStore()
	cfg := testConfig()
	mux := setupMux(mc, s, cfg)

	tests := []struct {
		name   string
		method string
		path   string
		body   string
		auth   string
	}{
		{
			name:   "health",
			method: "GET",
			path:   "/health",
		},
		{
			name:   "submit_command",
			method: "POST",
			path:   "/vehicles/VIN12345/commands",
			body:   `{"command_id":"ct-001","type":"lock","doors":["driver"]}`,
			auth:   "Bearer demo-token-001",
		},
		{
			name:   "get_command_not_found",
			method: "GET",
			path:   "/vehicles/VIN12345/commands/nonexistent",
			auth:   "Bearer demo-token-001",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			var req *http.Request
			if tt.body != "" {
				req = httptest.NewRequest(tt.method, tt.path,
					bytes.NewBufferString(tt.body))
			} else {
				req = httptest.NewRequest(tt.method, tt.path, nil)
			}
			if tt.auth != "" {
				req.Header.Set("Authorization", tt.auth)
			}
			rec := httptest.NewRecorder()
			mux.ServeHTTP(rec, req)

			ct := rec.Header().Get("Content-Type")
			if ct != "application/json" {
				t.Errorf("Content-Type = %q, want %q", ct, "application/json")
			}
		})
	}
}

// TS-06-E1: Invalid Command Payload
// Requirement: 06-REQ-1.E1
// Missing required fields in command body return HTTP 400.
func TestInvalidCommandPayload(t *testing.T) {
	mc := &mockCommander{}
	s := store.NewStore()
	cfg := testConfig()
	mux := setupMux(mc, s, cfg)

	payloads := []struct {
		name string
		body string
	}{
		{"empty_object", `{}`},
		{"missing_type_and_doors", `{"command_id":"x"}`},
		{"missing_doors", `{"command_id":"x","type":"lock"}`},
	}

	for _, tt := range payloads {
		t.Run(tt.name, func(t *testing.T) {
			req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands",
				bytes.NewBufferString(tt.body))
			req.Header.Set("Authorization", "Bearer demo-token-001")
			req.Header.Set("Content-Type", "application/json")
			rec := httptest.NewRecorder()

			mux.ServeHTTP(rec, req)

			if rec.Code != 400 {
				t.Errorf("status = %d, want 400", rec.Code)
			}

			var resp map[string]string
			if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
				t.Fatalf("failed to decode response: %v", err)
			}
			if resp["error"] != "invalid command payload" {
				t.Errorf("error = %q, want %q", resp["error"], "invalid command payload")
			}
		})
	}
}

// TS-06-E2: Invalid Command Type
// Requirement: 06-REQ-1.E2
// Command type other than "lock" or "unlock" returns HTTP 400.
func TestInvalidCommandType(t *testing.T) {
	mc := &mockCommander{}
	s := store.NewStore()
	cfg := testConfig()
	mux := setupMux(mc, s, cfg)

	body := `{"command_id":"x","type":"start","doors":["driver"]}`
	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands",
		bytes.NewBufferString(body))
	req.Header.Set("Authorization", "Bearer demo-token-001")
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()

	mux.ServeHTTP(rec, req)

	if rec.Code != 400 {
		t.Errorf("status = %d, want 400", rec.Code)
	}

	var resp map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if resp["error"] != "invalid command type" {
		t.Errorf("error = %q, want %q", resp["error"], "invalid command type")
	}
}

// TS-06-E3: Command Not Found
// Requirement: 06-REQ-2.E1
// Querying a nonexistent command_id returns HTTP 404.
func TestCommandNotFound(t *testing.T) {
	mc := &mockCommander{}
	s := store.NewStore()
	cfg := testConfig()
	mux := setupMux(mc, s, cfg)

	req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/nonexistent", nil)
	req.Header.Set("Authorization", "Bearer demo-token-001")
	rec := httptest.NewRecorder()

	mux.ServeHTTP(rec, req)

	if rec.Code != 404 {
		t.Errorf("status = %d, want 404", rec.Code)
	}

	var resp map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if resp["error"] != "command not found" {
		t.Errorf("error = %q, want %q", resp["error"], "command not found")
	}
}

// TS-06-E9: Error Response Format
// Requirement: 06-REQ-7.2
// All error responses use the format {"error":"<message>"}.
func TestErrorResponseFormat(t *testing.T) {
	mc := &mockCommander{}
	s := store.NewStore()
	cfg := testConfig()
	mux := setupMux(mc, s, cfg)

	tests := []struct {
		name       string
		method     string
		path       string
		auth       string
		wantStatus int
	}{
		{
			name:       "unauthorized_no_auth",
			method:     "POST",
			path:       "/vehicles/VIN12345/commands",
			auth:       "",
			wantStatus: 401,
		},
		{
			name:       "forbidden_vin_mismatch",
			method:     "POST",
			path:       "/vehicles/VIN99999/commands",
			auth:       "Bearer demo-token-001",
			wantStatus: 403,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req := httptest.NewRequest(tt.method, tt.path, nil)
			if tt.auth != "" {
				req.Header.Set("Authorization", tt.auth)
			}
			rec := httptest.NewRecorder()
			mux.ServeHTTP(rec, req)

			if rec.Code != tt.wantStatus {
				t.Errorf("status = %d, want %d", rec.Code, tt.wantStatus)
			}

			var resp map[string]string
			if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
				t.Fatalf("failed to decode response: %v", err)
			}
			if resp["error"] == "" {
				t.Error("error field is empty, want non-empty error message")
			}
		})
	}
}
