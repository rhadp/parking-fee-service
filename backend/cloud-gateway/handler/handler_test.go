package handler_test

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/config"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/handler"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// mockPublisher captures published commands for test verification.
type mockPublisher struct {
	published    bool
	lastVIN      string
	lastCommand  model.Command
	lastToken    string
	publishError error
}

func (m *mockPublisher) PublishCommand(vin string, cmd model.Command, token string) error {
	m.published = true
	m.lastVIN = vin
	m.lastCommand = cmd
	m.lastToken = token
	return m.publishError
}

// newTestMux creates a ServeMux wired with auth middleware and handlers
// using the provided mock publisher and store.
func newTestMux(pub handler.CommandPublisher, s *store.Store) http.Handler {
	cfg := &config.Config{
		Tokens: []config.TokenMapping{
			{Token: "demo-token-001", VIN: "VIN12345"},
			{Token: "demo-token-002", VIN: "VIN67890"},
		},
	}
	mux := http.NewServeMux()
	authMw := auth.Middleware(cfg)
	mux.Handle("POST /vehicles/{vin}/commands",
		authMw(handler.NewSubmitCommandHandler(pub, s, 30*time.Second)))
	mux.Handle("GET /vehicles/{vin}/commands/{command_id}",
		authMw(handler.NewGetCommandStatusHandler(s)))
	mux.Handle("GET /health", handler.HealthHandler())
	return mux
}

// TS-06-1: Command Submission Success
// Requirement: 06-REQ-1.1
func TestCommandSubmissionSuccess(t *testing.T) {
	mock := &mockPublisher{}
	s := store.NewStore()
	srv := httptest.NewServer(newTestMux(mock, s))
	defer srv.Close()

	body := `{"command_id":"cmd-001","type":"lock","doors":["driver"]}`
	req, err := http.NewRequest("POST", srv.URL+"/vehicles/VIN12345/commands",
		strings.NewReader(body))
	if err != nil {
		t.Fatal(err)
	}
	req.Header.Set("Authorization", "Bearer demo-token-001")
	req.Header.Set("Content-Type", "application/json")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusAccepted {
		t.Errorf("expected 202, got %d", resp.StatusCode)
	}

	var cmd model.Command
	if err := json.NewDecoder(resp.Body).Decode(&cmd); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	if cmd.CommandID != "cmd-001" {
		t.Errorf("expected command_id 'cmd-001', got '%s'", cmd.CommandID)
	}
	if cmd.Type != "lock" {
		t.Errorf("expected type 'lock', got '%s'", cmd.Type)
	}
	if len(cmd.Doors) != 1 || cmd.Doors[0] != "driver" {
		t.Errorf("expected doors [driver], got %v", cmd.Doors)
	}

	// Verify the command was published via the mock publisher.
	if !mock.published {
		t.Error("expected command to be published via CommandPublisher")
	}
	if mock.lastCommand.CommandID != "cmd-001" {
		t.Errorf("expected published command_id 'cmd-001', got '%s'", mock.lastCommand.CommandID)
	}
}

// TS-06-4: Command Status Query Success
// Requirement: 06-REQ-2.1
func TestCommandStatusQuerySuccess(t *testing.T) {
	mock := &mockPublisher{}
	s := store.NewStore()

	// Pre-store a command response.
	s.StoreResponse(model.CommandResponse{
		CommandID: "cmd-004",
		Status:    "success",
	})

	srv := httptest.NewServer(newTestMux(mock, s))
	defer srv.Close()

	req, err := http.NewRequest("GET", srv.URL+"/vehicles/VIN12345/commands/cmd-004", nil)
	if err != nil {
		t.Fatal(err)
	}
	req.Header.Set("Authorization", "Bearer demo-token-001")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected 200, got %d", resp.StatusCode)
	}

	var cmdResp model.CommandResponse
	if err := json.NewDecoder(resp.Body).Decode(&cmdResp); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	if cmdResp.CommandID != "cmd-004" {
		t.Errorf("expected command_id 'cmd-004', got '%s'", cmdResp.CommandID)
	}
	if cmdResp.Status != "success" {
		t.Errorf("expected status 'success', got '%s'", cmdResp.Status)
	}
}

// TS-06-10: Health Check
// Requirement: 06-REQ-4.1
func TestHealthCheck(t *testing.T) {
	mock := &mockPublisher{}
	s := store.NewStore()
	srv := httptest.NewServer(newTestMux(mock, s))
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/health")
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected 200, got %d", resp.StatusCode)
	}

	var body map[string]string
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	if body["status"] != "ok" {
		t.Errorf("expected status 'ok', got '%s'", body["status"])
	}
}

// TS-06-13: Content-Type Header
// Requirement: 06-REQ-7.1
func TestContentTypeHeader(t *testing.T) {
	mock := &mockPublisher{}
	s := store.NewStore()
	srv := httptest.NewServer(newTestMux(mock, s))
	defer srv.Close()

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

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			var bodyReader *strings.Reader
			if tc.body != "" {
				bodyReader = strings.NewReader(tc.body)
			}

			var req *http.Request
			var err error
			if bodyReader != nil {
				req, err = http.NewRequest(tc.method, srv.URL+tc.path, bodyReader)
			} else {
				req, err = http.NewRequest(tc.method, srv.URL+tc.path, nil)
			}
			if err != nil {
				t.Fatal(err)
			}
			if tc.auth != "" {
				req.Header.Set("Authorization", tc.auth)
			}

			resp, err := http.DefaultClient.Do(req)
			if err != nil {
				t.Fatal(err)
			}
			defer resp.Body.Close()

			ct := resp.Header.Get("Content-Type")
			if ct != "application/json" {
				t.Errorf("expected Content-Type 'application/json', got '%s'", ct)
			}
		})
	}
}

// TS-06-E1: Invalid Command Payload
// Requirement: 06-REQ-1.E1
func TestInvalidCommandPayload(t *testing.T) {
	mock := &mockPublisher{}
	s := store.NewStore()
	h := handler.NewSubmitCommandHandler(mock, s, 30*time.Second)

	bodies := []struct {
		name string
		body string
	}{
		{"empty_object", `{}`},
		{"missing_type_and_doors", `{"command_id":"x"}`},
		{"missing_doors", `{"command_id":"x","type":"lock"}`},
	}

	for _, tc := range bodies {
		t.Run(tc.name, func(t *testing.T) {
			req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands",
				strings.NewReader(tc.body))
			req.SetPathValue("vin", "VIN12345")
			rec := httptest.NewRecorder()

			h.ServeHTTP(rec, req)

			if rec.Code != http.StatusBadRequest {
				t.Errorf("expected 400, got %d", rec.Code)
			}

			var body map[string]string
			if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
				t.Fatalf("failed to decode response body: %v", err)
			}
			if body["error"] != "invalid command payload" {
				t.Errorf("expected error 'invalid command payload', got '%s'", body["error"])
			}
		})
	}
}

// TS-06-E2: Invalid Command Type
// Requirement: 06-REQ-1.E2
func TestInvalidCommandType(t *testing.T) {
	mock := &mockPublisher{}
	s := store.NewStore()
	h := handler.NewSubmitCommandHandler(mock, s, 30*time.Second)

	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands",
		strings.NewReader(`{"command_id":"x","type":"start","doors":["driver"]}`))
	req.SetPathValue("vin", "VIN12345")
	rec := httptest.NewRecorder()

	h.ServeHTTP(rec, req)

	if rec.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", rec.Code)
	}

	var body map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	if body["error"] != "invalid command type" {
		t.Errorf("expected error 'invalid command type', got '%s'", body["error"])
	}
}

// TS-06-E3: Command Not Found
// Requirement: 06-REQ-2.E1
func TestCommandNotFound(t *testing.T) {
	mock := &mockPublisher{}
	s := store.NewStore()
	srv := httptest.NewServer(newTestMux(mock, s))
	defer srv.Close()

	req, err := http.NewRequest("GET", srv.URL+"/vehicles/VIN12345/commands/nonexistent", nil)
	if err != nil {
		t.Fatal(err)
	}
	req.Header.Set("Authorization", "Bearer demo-token-001")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusNotFound {
		t.Errorf("expected 404, got %d", resp.StatusCode)
	}

	var body map[string]string
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	if body["error"] != "command not found" {
		t.Errorf("expected error 'command not found', got '%s'", body["error"])
	}
}

// TS-06-E9: Error Response Format
// Requirement: 06-REQ-7.2
func TestErrorResponseFormat(t *testing.T) {
	mock := &mockPublisher{}
	s := store.NewStore()
	srv := httptest.NewServer(newTestMux(mock, s))
	defer srv.Close()

	tests := []struct {
		name           string
		method         string
		path           string
		auth           string
		expectedStatus int
	}{
		{
			name:           "unauthorized_no_auth",
			method:         "POST",
			path:           "/vehicles/VIN12345/commands",
			auth:           "",
			expectedStatus: http.StatusUnauthorized,
		},
		{
			name:           "forbidden_vin_mismatch",
			method:         "POST",
			path:           "/vehicles/VIN99999/commands",
			auth:           "Bearer demo-token-001",
			expectedStatus: http.StatusForbidden,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			req, err := http.NewRequest(tc.method, srv.URL+tc.path, nil)
			if err != nil {
				t.Fatal(err)
			}
			if tc.auth != "" {
				req.Header.Set("Authorization", tc.auth)
			}

			resp, err := http.DefaultClient.Do(req)
			if err != nil {
				t.Fatal(err)
			}
			defer resp.Body.Close()

			if resp.StatusCode != tc.expectedStatus {
				t.Errorf("expected %d, got %d", tc.expectedStatus, resp.StatusCode)
			}

			var body map[string]string
			if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
				t.Fatalf("failed to decode response body: %v", err)
			}
			if body["error"] == "" {
				t.Error("expected non-empty error field in response")
			}
		})
	}
}
