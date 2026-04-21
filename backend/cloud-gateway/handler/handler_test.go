package handler_test

import (
	"bytes"
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

// mockNATSPublisher implements handler.NATSPublisher for tests.
type mockNATSPublisher struct {
	published []publishedMsg
}

type publishedMsg struct {
	VIN   string
	Cmd   model.Command
	Token string
}

func (m *mockNATSPublisher) PublishCommand(vin string, cmd model.Command, token string) error {
	m.published = append(m.published, publishedMsg{VIN: vin, Cmd: cmd, Token: token})
	return nil
}

// buildMux creates a ServeMux with auth bypassed (direct handler wiring for handler-level tests).
func buildMux(nc handler.NATSPublisher, s *store.Store) *http.ServeMux {
	mux := http.NewServeMux()
	mux.Handle("POST /vehicles/{vin}/commands", handler.NewSubmitCommandHandler(nc, s, 30*time.Second))
	mux.Handle("GET /vehicles/{vin}/commands/{command_id}", handler.NewGetCommandStatusHandler(s))
	mux.Handle("GET /health", handler.HealthHandler())
	return mux
}

// buildMuxWithAuth creates a ServeMux with auth middleware applied for integration-style tests.
func buildMuxWithAuth(nc handler.NATSPublisher, s *store.Store, cfg *config.Config) *http.ServeMux {
	mux := http.NewServeMux()
	mw := auth.Middleware(cfg)
	mux.Handle("POST /vehicles/{vin}/commands", mw(handler.NewSubmitCommandHandler(nc, s, 30*time.Second)))
	mux.Handle("GET /vehicles/{vin}/commands/{command_id}", mw(handler.NewGetCommandStatusHandler(s)))
	mux.Handle("GET /health", handler.HealthHandler())
	return mux
}

// contextWithToken returns a handler that injects the auth token into the context before the real handler.
// Since auth is bypassed in these tests, we simulate the token being available via a custom header.
// Handlers may look for "X-Auth-Token" from context or r.Header.
// For simplicity, handler tests bypass auth entirely.

// TestCommandSubmissionSuccess verifies POST /vehicles/{vin}/commands returns 202 (TS-06-1).
// Also verifies that the command was published to the mock NATS client with the correct VIN
// and command payload (per review finding about nc.published not being asserted).
func TestCommandSubmissionSuccess(t *testing.T) {
	nc := &mockNATSPublisher{}
	s := store.NewStore()
	mux := buildMux(nc, s)

	body := `{"command_id":"cmd-001","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusAccepted {
		t.Errorf("StatusCode: got %d, want 202", rec.Code)
	}
	var resp model.Command
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("failed to decode body: %v", err)
	}
	if resp.CommandID != "cmd-001" {
		t.Errorf("command_id: got %q, want %q", resp.CommandID, "cmd-001")
	}
	if resp.Type != "lock" {
		t.Errorf("type: got %q, want %q", resp.Type, "lock")
	}

	// Verify the command was published to the mock NATS client.
	if len(nc.published) != 1 {
		t.Fatalf("expected 1 published message, got %d", len(nc.published))
	}
	pub := nc.published[0]
	if pub.VIN != "VIN12345" {
		t.Errorf("published VIN: got %q, want %q", pub.VIN, "VIN12345")
	}
	if pub.Cmd.CommandID != "cmd-001" {
		t.Errorf("published command_id: got %q, want %q", pub.Cmd.CommandID, "cmd-001")
	}
	if pub.Cmd.Type != "lock" {
		t.Errorf("published type: got %q, want %q", pub.Cmd.Type, "lock")
	}
}

// TestCommandStatusQuerySuccess verifies GET /vehicles/{vin}/commands/{id} returns 200 (TS-06-4).
func TestCommandStatusQuerySuccess(t *testing.T) {
	nc := &mockNATSPublisher{}
	s := store.NewStore()
	s.StoreResponse(model.CommandResponse{CommandID: "cmd-004", Status: "success"})
	mux := buildMux(nc, s)

	req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/cmd-004", nil)
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("StatusCode: got %d, want 200", rec.Code)
	}
	var resp model.CommandResponse
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("failed to decode body: %v", err)
	}
	if resp.CommandID != "cmd-004" {
		t.Errorf("command_id: got %q, want %q", resp.CommandID, "cmd-004")
	}
	if resp.Status != "success" {
		t.Errorf("status: got %q, want %q", resp.Status, "success")
	}
}

// TestHealthCheck verifies GET /health returns 200 with {"status":"ok"} (TS-06-10).
func TestHealthCheck(t *testing.T) {
	nc := &mockNATSPublisher{}
	s := store.NewStore()
	mux := buildMux(nc, s)

	req := httptest.NewRequest("GET", "/health", nil)
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("StatusCode: got %d, want 200", rec.Code)
	}
	var resp map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("failed to decode body: %v", err)
	}
	if resp["status"] != "ok" {
		t.Errorf("status: got %q, want %q", resp["status"], "ok")
	}
}

// TestContentTypeHeader verifies all responses set Content-Type: application/json (TS-06-13).
func TestContentTypeHeader(t *testing.T) {
	nc := &mockNATSPublisher{}
	s := store.NewStore()
	s.StoreResponse(model.CommandResponse{CommandID: "cmd-ct", Status: "success"})
	mux := buildMux(nc, s)

	endpoints := []struct {
		method string
		path   string
		body   string
	}{
		{"GET", "/health", ""},
		{"POST", "/vehicles/VIN12345/commands", `{"command_id":"cmd-ct2","type":"lock","doors":["driver"]}`},
		{"GET", "/vehicles/VIN12345/commands/nonexistent", ""},
	}
	for _, ep := range endpoints {
		var reqBody *bytes.Buffer
		if ep.body != "" {
			reqBody = bytes.NewBufferString(ep.body)
		} else {
			reqBody = &bytes.Buffer{}
		}
		req := httptest.NewRequest(ep.method, ep.path, reqBody)
		if ep.body != "" {
			req.Header.Set("Content-Type", "application/json")
		}
		rec := httptest.NewRecorder()
		mux.ServeHTTP(rec, req)
		ct := rec.Header().Get("Content-Type")
		if !strings.HasPrefix(ct, "application/json") {
			t.Errorf("Content-Type [%s %s]: got %q, want application/json", ep.method, ep.path, ct)
		}
	}
}

// TestInvalidCommandPayload verifies missing fields return 400 (TS-06-E1).
func TestInvalidCommandPayload(t *testing.T) {
	nc := &mockNATSPublisher{}
	s := store.NewStore()
	mux := buildMux(nc, s)

	badBodies := []string{
		`{}`,
		`{"command_id":"x"}`,
		`{"command_id":"x","type":"lock"}`,
	}
	for _, body := range badBodies {
		req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands", strings.NewReader(body))
		req.Header.Set("Content-Type", "application/json")
		rec := httptest.NewRecorder()
		mux.ServeHTTP(rec, req)
		if rec.Code != http.StatusBadRequest {
			t.Errorf("InvalidCommandPayload [%s]: got %d, want 400", body, rec.Code)
		}
		var resp map[string]string
		if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
			t.Fatalf("failed to decode body: %v", err)
		}
		if resp["error"] != "invalid command payload" {
			t.Errorf("error: got %q, want %q", resp["error"], "invalid command payload")
		}
	}
}

// TestInvalidCommandType verifies invalid type returns 400 (TS-06-E2).
func TestInvalidCommandType(t *testing.T) {
	nc := &mockNATSPublisher{}
	s := store.NewStore()
	mux := buildMux(nc, s)

	body := `{"command_id":"x","type":"start","doors":["driver"]}`
	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	if rec.Code != http.StatusBadRequest {
		t.Errorf("InvalidCommandType: got %d, want 400", rec.Code)
	}
	var resp map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("failed to decode body: %v", err)
	}
	if resp["error"] != "invalid command type" {
		t.Errorf("error: got %q, want %q", resp["error"], "invalid command type")
	}
}

// TestCommandNotFound verifies nonexistent command_id returns 404 (TS-06-E3).
func TestCommandNotFound(t *testing.T) {
	nc := &mockNATSPublisher{}
	s := store.NewStore()
	mux := buildMux(nc, s)

	req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/nonexistent", nil)
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	if rec.Code != http.StatusNotFound {
		t.Errorf("CommandNotFound: got %d, want 404", rec.Code)
	}
	var resp map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("failed to decode body: %v", err)
	}
	if resp["error"] != "command not found" {
		t.Errorf("error: got %q, want %q", resp["error"], "command not found")
	}
}

// TestErrorResponseFormat verifies error responses use {"error":"..."} format (TS-06-E9).
// Per TS-06-E9 preconditions, this tests 401 (no auth) and 403 (valid token, wrong VIN)
// scenarios with auth middleware wired in.
func TestErrorResponseFormat(t *testing.T) {
	cfg := &config.Config{
		Tokens: []config.TokenMapping{
			{Token: "demo-token-001", VIN: "VIN12345"},
		},
	}
	nc := &mockNATSPublisher{}
	s := store.NewStore()
	mux := buildMuxWithAuth(nc, s, cfg)

	cases := []struct {
		name       string
		method     string
		path       string
		authHeader string
		wantStatus int
	}{
		{
			name:       "401 no auth",
			method:     "POST",
			path:       "/vehicles/VIN12345/commands",
			authHeader: "",
			wantStatus: http.StatusUnauthorized,
		},
		{
			name:       "403 wrong VIN",
			method:     "POST",
			path:       "/vehicles/VIN99999/commands",
			authHeader: "Bearer demo-token-001",
			wantStatus: http.StatusForbidden,
		},
	}
	for _, tc := range cases {
		req := httptest.NewRequest(tc.method, tc.path, &bytes.Buffer{})
		if tc.authHeader != "" {
			req.Header.Set("Authorization", tc.authHeader)
		}
		rec := httptest.NewRecorder()
		mux.ServeHTTP(rec, req)
		if rec.Code != tc.wantStatus {
			t.Errorf("ErrorResponseFormat [%s]: got status %d, want %d", tc.name, rec.Code, tc.wantStatus)
		}
		var resp map[string]string
		if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
			t.Fatalf("ErrorResponseFormat [%s]: failed to decode error body: %v", tc.name, err)
		}
		if resp["error"] == "" {
			t.Errorf("ErrorResponseFormat [%s]: body.error is empty", tc.name)
		}
	}
}
