package handler

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"parking-fee-service/backend/cloud-gateway/auth"
	"parking-fee-service/backend/cloud-gateway/model"
	"parking-fee-service/backend/cloud-gateway/store"
)

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

// mockPublisher implements NATSPublisher for testing.
type mockPublisher struct {
	published []publishedMsg
}

type publishedMsg struct {
	VIN   string
	Cmd   model.Command
	Token string
}

func (m *mockPublisher) PublishCommand(vin string, cmd model.Command, token string) error {
	m.published = append(m.published, publishedMsg{VIN: vin, Cmd: cmd, Token: token})
	return nil
}

// setupMux creates a test ServeMux with auth middleware and handlers wired up.
func setupMux(cfg *model.Config, pub NATSPublisher, s *store.Store) *http.ServeMux {
	mux := http.NewServeMux()
	authMw := auth.Middleware(cfg)

	mux.Handle("POST /vehicles/{vin}/commands",
		authMw(NewSubmitCommandHandler(pub, s, time.Duration(cfg.CommandTimeoutSeconds)*time.Second)))
	mux.Handle("GET /vehicles/{vin}/commands/{command_id}",
		authMw(NewGetCommandStatusHandler(s)))
	mux.Handle("GET /health", HealthHandler())

	return mux
}

// TS-06-1: Command Submission Success (handler-level, mock NATS)
func TestCommandSubmissionSuccess(t *testing.T) {
	cfg := testConfig()
	pub := &mockPublisher{}
	s := store.NewStore()
	mux := setupMux(cfg, pub, s)

	body := `{"command_id":"cmd-001","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands", strings.NewReader(body))
	req.Header.Set("Authorization", "Bearer demo-token-001")
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != 202 {
		t.Errorf("status = %d, want 202", rec.Code)
	}

	var resp model.Command
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if resp.CommandID != "cmd-001" {
		t.Errorf("command_id = %q, want %q", resp.CommandID, "cmd-001")
	}
	if resp.Type != "lock" {
		t.Errorf("type = %q, want %q", resp.Type, "lock")
	}
	if len(resp.Doors) != 1 || resp.Doors[0] != "driver" {
		t.Errorf("doors = %v, want [driver]", resp.Doors)
	}

	// Verify NATS publish was called
	if len(pub.published) != 1 {
		t.Fatalf("published count = %d, want 1", len(pub.published))
	}
	if pub.published[0].VIN != "VIN12345" {
		t.Errorf("published VIN = %q, want %q", pub.published[0].VIN, "VIN12345")
	}
}

// TS-06-4: Command Status Query Success
func TestCommandStatusQuerySuccess(t *testing.T) {
	cfg := testConfig()
	pub := &mockPublisher{}
	s := store.NewStore()
	s.StoreResponse(model.CommandResponse{CommandID: "cmd-004", Status: "success"})
	mux := setupMux(cfg, pub, s)

	req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/cmd-004", nil)
	req.Header.Set("Authorization", "Bearer demo-token-001")
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != 200 {
		t.Errorf("status = %d, want 200", rec.Code)
	}

	var resp model.CommandResponse
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if resp.CommandID != "cmd-004" {
		t.Errorf("command_id = %q, want %q", resp.CommandID, "cmd-004")
	}
	if resp.Status != "success" {
		t.Errorf("status = %q, want %q", resp.Status, "success")
	}
}

// TS-06-10: Health Check
func TestHealthCheck(t *testing.T) {
	cfg := testConfig()
	pub := &mockPublisher{}
	s := store.NewStore()
	mux := setupMux(cfg, pub, s)

	req := httptest.NewRequest("GET", "/health", nil)
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != 200 {
		t.Errorf("status = %d, want 200", rec.Code)
	}

	var body map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode body: %v", err)
	}
	if body["status"] != "ok" {
		t.Errorf("status = %q, want %q", body["status"], "ok")
	}
}

// TS-06-13: Content-Type Header
func TestContentTypeHeader(t *testing.T) {
	cfg := testConfig()
	pub := &mockPublisher{}
	s := store.NewStore()
	mux := setupMux(cfg, pub, s)

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
				req = httptest.NewRequest(tt.method, tt.path, strings.NewReader(tt.body))
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
func TestInvalidCommandPayload(t *testing.T) {
	cfg := testConfig()
	pub := &mockPublisher{}
	s := store.NewStore()
	mux := setupMux(cfg, pub, s)

	bodies := []string{
		`{}`,
		`{"command_id":"x"}`,
		`{"command_id":"x","type":"lock"}`,
	}

	for _, body := range bodies {
		t.Run(body, func(t *testing.T) {
			req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands", strings.NewReader(body))
			req.Header.Set("Authorization", "Bearer demo-token-001")
			rec := httptest.NewRecorder()
			mux.ServeHTTP(rec, req)

			if rec.Code != 400 {
				t.Errorf("status = %d, want 400 for body %s", rec.Code, body)
			}
			var resp map[string]string
			if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
				t.Fatalf("failed to decode: %v", err)
			}
			if resp["error"] != "invalid command payload" {
				t.Errorf("error = %q, want %q", resp["error"], "invalid command payload")
			}
		})
	}
}

// TS-06-E2: Invalid Command Type
func TestInvalidCommandType(t *testing.T) {
	cfg := testConfig()
	pub := &mockPublisher{}
	s := store.NewStore()
	mux := setupMux(cfg, pub, s)

	body := `{"command_id":"x","type":"start","doors":["driver"]}`
	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands", strings.NewReader(body))
	req.Header.Set("Authorization", "Bearer demo-token-001")
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != 400 {
		t.Errorf("status = %d, want 400", rec.Code)
	}
	var resp map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("failed to decode: %v", err)
	}
	if resp["error"] != "invalid command type" {
		t.Errorf("error = %q, want %q", resp["error"], "invalid command type")
	}
}

// TS-06-E3: Command Not Found
func TestCommandNotFound(t *testing.T) {
	cfg := testConfig()
	pub := &mockPublisher{}
	s := store.NewStore()
	mux := setupMux(cfg, pub, s)

	req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/nonexistent", nil)
	req.Header.Set("Authorization", "Bearer demo-token-001")
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != 404 {
		t.Errorf("status = %d, want 404", rec.Code)
	}
	var resp map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("failed to decode: %v", err)
	}
	if resp["error"] != "command not found" {
		t.Errorf("error = %q, want %q", resp["error"], "command not found")
	}
}

// TS-06-E9: Error Response Format
func TestErrorResponseFormat(t *testing.T) {
	cfg := testConfig()
	pub := &mockPublisher{}
	s := store.NewStore()
	mux := setupMux(cfg, pub, s)

	tests := []struct {
		name       string
		method     string
		path       string
		auth       string
		wantStatus int
	}{
		{
			name:       "401_no_auth",
			method:     "POST",
			path:       "/vehicles/VIN12345/commands",
			auth:       "",
			wantStatus: 401,
		},
		{
			name:       "403_wrong_vin",
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
				t.Fatalf("failed to decode body: %v", err)
			}
			if resp["error"] == "" {
				t.Error("error field is empty, want non-empty error message")
			}
		})
	}
}
