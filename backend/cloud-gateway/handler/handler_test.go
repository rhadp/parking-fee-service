package handler_test

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"parking-fee-service/backend/cloud-gateway/handler"
	"parking-fee-service/backend/cloud-gateway/model"
	"parking-fee-service/backend/cloud-gateway/store"
)

// mockNATSPublisher records published commands for test assertions.
type mockNATSPublisher struct {
	published []struct {
		vin   string
		cmd   model.Command
		token string
	}
}

func (m *mockNATSPublisher) PublishCommand(vin string, cmd model.Command, token string) error {
	m.published = append(m.published, struct {
		vin   string
		cmd   model.Command
		token string
	}{vin, cmd, token})
	return nil
}

// newTestMux creates a ServeMux with all routes registered, using Go 1.22 pattern syntax.
func newTestMux(nc handler.NATSPublisher, s *store.Store, timeout time.Duration) *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("POST /vehicles/{vin}/commands", handler.NewSubmitCommandHandler(nc, s, timeout))
	mux.HandleFunc("GET /vehicles/{vin}/commands/{command_id}", handler.NewGetCommandStatusHandler(s))
	mux.HandleFunc("GET /health", handler.HealthHandler())
	return mux
}

// decodeBody decodes a JSON body into a map.
func decodeBody(t *testing.T, body *bytes.Buffer) map[string]interface{} {
	t.Helper()
	var m map[string]interface{}
	if err := json.NewDecoder(body).Decode(&m); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	return m
}

// TestCommandSubmissionSuccess verifies that a valid POST to /vehicles/{vin}/commands
// returns HTTP 202 with the command echoed back.
// Test Spec: TS-06-1 (handler level, mock NATS)
// Requirements: 06-REQ-1.1
func TestCommandSubmissionSuccess(t *testing.T) {
	nc := &mockNATSPublisher{}
	s := store.NewStore()
	mux := newTestMux(nc, s, 30*time.Second)

	body := `{"command_id":"cmd-001","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusAccepted {
		t.Errorf("expected 202 Accepted, got %d", rec.Code)
	}
	m := decodeBody(t, rec.Body)
	if m["command_id"] != "cmd-001" {
		t.Errorf("expected command_id 'cmd-001', got %v", m["command_id"])
	}
	if m["type"] != "lock" {
		t.Errorf("expected type 'lock', got %v", m["type"])
	}
	doors, ok := m["doors"].([]interface{})
	if !ok || len(doors) != 1 || doors[0] != "driver" {
		t.Errorf("expected doors [\"driver\"], got %v", m["doors"])
	}
}

// TestCommandStatusQuerySuccess verifies that GET /vehicles/{vin}/commands/{command_id}
// returns HTTP 200 with the stored command response.
// Test Spec: TS-06-4
// Requirements: 06-REQ-2.1
func TestCommandStatusQuerySuccess(t *testing.T) {
	nc := &mockNATSPublisher{}
	s := store.NewStore()
	s.StoreResponse(model.CommandResponse{CommandID: "cmd-004", Status: "success"})
	mux := newTestMux(nc, s, 30*time.Second)

	req := httptest.NewRequest(http.MethodGet, "/vehicles/VIN12345/commands/cmd-004", nil)
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("expected 200 OK, got %d", rec.Code)
	}
	m := decodeBody(t, rec.Body)
	if m["command_id"] != "cmd-004" {
		t.Errorf("expected command_id 'cmd-004', got %v", m["command_id"])
	}
	if m["status"] != "success" {
		t.Errorf("expected status 'success', got %v", m["status"])
	}
}

// TestHealthCheck verifies that GET /health returns HTTP 200 with {"status":"ok"}.
// Test Spec: TS-06-10
// Requirements: 06-REQ-4.1
func TestHealthCheck(t *testing.T) {
	nc := &mockNATSPublisher{}
	s := store.NewStore()
	mux := newTestMux(nc, s, 30*time.Second)

	req := httptest.NewRequest(http.MethodGet, "/health", nil)
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("expected 200 OK, got %d", rec.Code)
	}
	m := decodeBody(t, rec.Body)
	if m["status"] != "ok" {
		t.Errorf(`expected {"status":"ok"}, got %v`, m)
	}
}

// TestContentTypeHeader verifies that all REST responses include Content-Type: application/json.
// Test Spec: TS-06-13
// Requirements: 06-REQ-7.1
func TestContentTypeHeader(t *testing.T) {
	nc := &mockNATSPublisher{}
	s := store.NewStore()
	mux := newTestMux(nc, s, 30*time.Second)

	cases := []struct {
		method string
		path   string
		body   string
	}{
		{http.MethodGet, "/health", ""},
		{http.MethodPost, "/vehicles/VIN12345/commands",
			`{"command_id":"ct-001","type":"lock","doors":["driver"]}`},
		{http.MethodGet, "/vehicles/VIN12345/commands/nonexistent", ""},
	}

	for _, tc := range cases {
		var reqBody *strings.Reader
		if tc.body != "" {
			reqBody = strings.NewReader(tc.body)
		} else {
			reqBody = strings.NewReader("")
		}
		req := httptest.NewRequest(tc.method, tc.path, reqBody)
		if tc.body != "" {
			req.Header.Set("Content-Type", "application/json")
		}
		rec := httptest.NewRecorder()
		mux.ServeHTTP(rec, req)

		ct := rec.Header().Get("Content-Type")
		if !strings.HasPrefix(ct, "application/json") {
			t.Errorf("%s %s: expected Content-Type application/json, got %q", tc.method, tc.path, ct)
		}
	}
}

// TestInvalidCommandPayload verifies that missing required fields return HTTP 400.
// Test Spec: TS-06-E1
// Requirements: 06-REQ-1.E1
func TestInvalidCommandPayload(t *testing.T) {
	nc := &mockNATSPublisher{}
	s := store.NewStore()
	mux := newTestMux(nc, s, 30*time.Second)

	invalidBodies := []string{
		`{}`,
		`{"command_id":"x"}`,
		`{"command_id":"x","type":"lock"}`,
		`{"type":"lock","doors":["driver"]}`,
	}

	for _, body := range invalidBodies {
		req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", strings.NewReader(body))
		req.Header.Set("Content-Type", "application/json")
		rec := httptest.NewRecorder()
		mux.ServeHTTP(rec, req)

		if rec.Code != http.StatusBadRequest {
			t.Errorf("body %q: expected 400, got %d", body, rec.Code)
		}
		m := decodeBody(t, rec.Body)
		if m["error"] != "invalid command payload" {
			t.Errorf("body %q: expected error 'invalid command payload', got %v", body, m["error"])
		}
	}
}

// TestInvalidCommandType verifies that an invalid type field returns HTTP 400.
// Test Spec: TS-06-E2
// Requirements: 06-REQ-1.E2
func TestInvalidCommandType(t *testing.T) {
	nc := &mockNATSPublisher{}
	s := store.NewStore()
	mux := newTestMux(nc, s, 30*time.Second)

	body := `{"command_id":"x","type":"start","doors":["driver"]}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusBadRequest {
		t.Errorf("expected 400 for invalid type, got %d", rec.Code)
	}
	m := decodeBody(t, rec.Body)
	if m["error"] != "invalid command type" {
		t.Errorf("expected error 'invalid command type', got %v", m["error"])
	}
}

// TestCommandNotFound verifies that querying a nonexistent command_id returns HTTP 404.
// Test Spec: TS-06-E3
// Requirements: 06-REQ-2.E1
func TestCommandNotFound(t *testing.T) {
	nc := &mockNATSPublisher{}
	s := store.NewStore()
	mux := newTestMux(nc, s, 30*time.Second)

	req := httptest.NewRequest(http.MethodGet, "/vehicles/VIN12345/commands/nonexistent", nil)
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusNotFound {
		t.Errorf("expected 404 for unknown command, got %d", rec.Code)
	}
	m := decodeBody(t, rec.Body)
	if m["error"] != "command not found" {
		t.Errorf("expected error 'command not found', got %v", m["error"])
	}
}

// TestErrorResponseFormat verifies that all error responses use {"error":"<message>"} format.
// Test Spec: TS-06-E9
// Requirements: 06-REQ-7.2
func TestErrorResponseFormat(t *testing.T) {
	nc := &mockNATSPublisher{}
	s := store.NewStore()
	mux := newTestMux(nc, s, 30*time.Second)

	errorCases := []struct {
		method string
		path   string
		body   string
	}{
		// 404: command not found
		{http.MethodGet, "/vehicles/VIN12345/commands/does-not-exist", ""},
		// 400: invalid payload
		{http.MethodPost, "/vehicles/VIN12345/commands", `{}`},
	}

	for _, tc := range errorCases {
		var reqBody *strings.Reader
		if tc.body != "" {
			reqBody = strings.NewReader(tc.body)
		} else {
			reqBody = strings.NewReader("")
		}
		req := httptest.NewRequest(tc.method, tc.path, reqBody)
		if tc.body != "" {
			req.Header.Set("Content-Type", "application/json")
		}
		rec := httptest.NewRecorder()
		mux.ServeHTTP(rec, req)

		// Must be an error status code
		if rec.Code < 400 {
			t.Errorf("%s %s: expected error status code, got %d", tc.method, tc.path, rec.Code)
			continue
		}

		var m map[string]string
		if err := json.NewDecoder(rec.Body).Decode(&m); err != nil {
			t.Errorf("%s %s: could not decode error body as JSON: %v", tc.method, tc.path, err)
			continue
		}
		errMsg, ok := m["error"]
		if !ok {
			t.Errorf("%s %s: response body missing 'error' key, got %v", tc.method, tc.path, m)
			continue
		}
		if errMsg == "" {
			t.Errorf("%s %s: 'error' key is empty", tc.method, tc.path)
		}
	}
}
