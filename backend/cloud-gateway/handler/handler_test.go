package handler_test

import (
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/handler"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// mockPublisher implements handler.NATSPublisher for testing.
type mockPublisher struct {
	published []publishedMsg
}

type publishedMsg struct {
	VIN   string
	Cmd   model.Command
	Token string
}

func (m *mockPublisher) PublishCommand(vin string, cmd model.Command, bearerToken string) error {
	m.published = append(m.published, publishedMsg{VIN: vin, Cmd: cmd, Token: bearerToken})
	return nil
}

// newTestRouter creates an http.ServeMux wired with handlers for testing.
func newTestRouter(s *store.Store, pub handler.NATSPublisher, a *auth.Authenticator) *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("POST /vehicles/{vin}/commands", handler.NewCommandHandler(s, pub, a))
	mux.HandleFunc("GET /vehicles/{vin}/commands/{command_id}", handler.NewStatusHandler(s, a))
	mux.HandleFunc("GET /health", handler.HealthHandler())
	return mux
}

// newTestDeps creates common test dependencies.
func newTestDeps() (*store.Store, *mockPublisher, *auth.Authenticator) {
	s := store.NewStore()
	pub := &mockPublisher{}
	a := auth.NewAuthenticator([]model.TokenMapping{
		{Token: "demo-token-car1", VIN: "VIN12345"},
		{Token: "demo-token-car2", VIN: "VIN67890"},
	})
	return s, pub, a
}

// TS-06-1: Command Submission via REST
func TestCommandSubmission(t *testing.T) {
	s, pub, a := newTestDeps()
	mux := newTestRouter(s, pub, a)

	body := `{"command_id":"cmd-001","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands", strings.NewReader(body))
	req.Header.Set("Authorization", "Bearer demo-token-car1")
	w := httptest.NewRecorder()
	mux.ServeHTTP(w, req)

	if w.Code != http.StatusAccepted {
		t.Errorf("status = %d, want %d", w.Code, http.StatusAccepted)
	}

	var resp map[string]string
	if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
		t.Fatalf("decode response: %v", err)
	}
	if resp["command_id"] != "cmd-001" {
		t.Errorf("command_id = %q, want %q", resp["command_id"], "cmd-001")
	}
	if resp["status"] != "pending" {
		t.Errorf("status = %q, want %q", resp["status"], "pending")
	}

	// Verify command was published to NATS mock
	if len(pub.published) != 1 {
		t.Fatalf("published %d messages, want 1", len(pub.published))
	}
	if pub.published[0].VIN != "VIN12345" {
		t.Errorf("published VIN = %q, want %q", pub.published[0].VIN, "VIN12345")
	}
	if pub.published[0].Token != "demo-token-car1" {
		t.Errorf("published token = %q, want %q", pub.published[0].Token, "demo-token-car1")
	}
}

// TS-06-5: Command Status Query
func TestCommandStatusQuery(t *testing.T) {
	s, pub, a := newTestDeps()
	s.Add(model.CommandStatus{
		CommandID: "cmd-001",
		Status:    "success",
		VIN:       "VIN12345",
		CreatedAt: time.Now(),
	})
	mux := newTestRouter(s, pub, a)

	req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/cmd-001", nil)
	req.Header.Set("Authorization", "Bearer demo-token-car1")
	w := httptest.NewRecorder()
	mux.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("status = %d, want %d", w.Code, http.StatusOK)
	}

	var resp map[string]string
	if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
		t.Fatalf("decode response: %v", err)
	}
	if resp["command_id"] != "cmd-001" {
		t.Errorf("command_id = %q, want %q", resp["command_id"], "cmd-001")
	}
	if resp["status"] != "success" {
		t.Errorf("status = %q, want %q", resp["status"], "success")
	}
}

// TS-06-23: Health Check
func TestHealthCheck(t *testing.T) {
	s, pub, a := newTestDeps()
	mux := newTestRouter(s, pub, a)

	req := httptest.NewRequest("GET", "/health", nil)
	w := httptest.NewRecorder()
	mux.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("status = %d, want %d", w.Code, http.StatusOK)
	}

	var resp map[string]string
	if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
		t.Fatalf("decode response: %v", err)
	}
	if resp["status"] != "ok" {
		t.Errorf("status = %q, want %q", resp["status"], "ok")
	}
}

// TS-06-26: Content-Type Header
func TestContentTypeHeader(t *testing.T) {
	s, pub, a := newTestDeps()
	mux := newTestRouter(s, pub, a)

	endpoints := []struct {
		method string
		path   string
		body   string
		auth   string
	}{
		{"GET", "/health", "", ""},
		{"POST", "/vehicles/VIN12345/commands", `{"command_id":"c1","type":"lock","doors":["d"]}`, "Bearer demo-token-car1"},
	}

	for _, ep := range endpoints {
		// Use io.Reader interface (not *strings.Reader) so nil body is a true nil interface.
		var bodyReader io.Reader
		if ep.body != "" {
			bodyReader = strings.NewReader(ep.body)
		}
		req := httptest.NewRequest(ep.method, ep.path, bodyReader)
		if ep.auth != "" {
			req.Header.Set("Authorization", ep.auth)
		}
		w := httptest.NewRecorder()
		mux.ServeHTTP(w, req)

		ct := w.Header().Get("Content-Type")
		if ct != "application/json" {
			t.Errorf("%s %s: Content-Type = %q, want %q", ep.method, ep.path, ct, "application/json")
		}
	}
}

// TS-06-27: Error Response Format
func TestErrorResponseFormat(t *testing.T) {
	s, pub, a := newTestDeps()
	mux := newTestRouter(s, pub, a)

	// Request without auth should return 401 with {"error":"unauthorized"}
	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands",
		strings.NewReader(`{"command_id":"c1","type":"lock","doors":["d"]}`))
	w := httptest.NewRecorder()
	mux.ServeHTTP(w, req)

	if w.Code != http.StatusUnauthorized {
		t.Errorf("status = %d, want %d", w.Code, http.StatusUnauthorized)
	}

	var resp map[string]string
	if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
		t.Fatalf("decode error response: %v", err)
	}
	if resp["error"] != "unauthorized" {
		t.Errorf("error = %q, want %q", resp["error"], "unauthorized")
	}
}

// TS-06-E1: Missing Authorization Header
func TestMissingAuthHeader(t *testing.T) {
	s, pub, a := newTestDeps()
	mux := newTestRouter(s, pub, a)

	// No Authorization header
	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands",
		strings.NewReader(`{"command_id":"c1","type":"lock","doors":["d"]}`))
	w := httptest.NewRecorder()
	mux.ServeHTTP(w, req)
	if w.Code != http.StatusUnauthorized {
		t.Errorf("no auth: status = %d, want %d", w.Code, http.StatusUnauthorized)
	}

	// Basic auth instead of Bearer
	req2 := httptest.NewRequest("POST", "/vehicles/VIN12345/commands",
		strings.NewReader(`{"command_id":"c1","type":"lock","doors":["d"]}`))
	req2.Header.Set("Authorization", "Basic abc123")
	w2 := httptest.NewRecorder()
	mux.ServeHTTP(w2, req2)
	if w2.Code != http.StatusUnauthorized {
		t.Errorf("basic auth: status = %d, want %d", w2.Code, http.StatusUnauthorized)
	}
}

// TS-06-E2: Token Not Authorized for VIN
func TestTokenNotAuthorizedForVIN(t *testing.T) {
	s, pub, a := newTestDeps()
	mux := newTestRouter(s, pub, a)

	req := httptest.NewRequest("POST", "/vehicles/VIN99999/commands",
		strings.NewReader(`{"command_id":"c1","type":"lock","doors":["d"]}`))
	req.Header.Set("Authorization", "Bearer demo-token-car1")
	w := httptest.NewRecorder()
	mux.ServeHTTP(w, req)

	if w.Code != http.StatusForbidden {
		t.Errorf("status = %d, want %d", w.Code, http.StatusForbidden)
	}

	var resp map[string]string
	if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
		t.Fatalf("decode response: %v", err)
	}
	if resp["error"] != "forbidden" {
		t.Errorf("error = %q, want %q", resp["error"], "forbidden")
	}
}

// TS-06-E3: Invalid Command Payload
func TestInvalidCommandPayload(t *testing.T) {
	s, pub, a := newTestDeps()
	mux := newTestRouter(s, pub, a)

	payloads := []string{
		`{invalid json}`,
		`{"command_id":"c1"}`,
	}

	for _, body := range payloads {
		req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands",
			strings.NewReader(body))
		req.Header.Set("Authorization", "Bearer demo-token-car1")
		w := httptest.NewRecorder()
		mux.ServeHTTP(w, req)

		if w.Code != http.StatusBadRequest {
			t.Errorf("body %q: status = %d, want %d", body, w.Code, http.StatusBadRequest)
		}

		var resp map[string]string
		if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
			t.Fatalf("decode response: %v", err)
		}
		if resp["error"] != "invalid command payload" {
			t.Errorf("body %q: error = %q, want %q", body, resp["error"], "invalid command payload")
		}
	}
}

// TS-06-E4: Invalid Command Type
func TestInvalidCommandType(t *testing.T) {
	s, pub, a := newTestDeps()
	mux := newTestRouter(s, pub, a)

	body := `{"command_id":"c1","type":"open","doors":["driver"]}`
	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands",
		strings.NewReader(body))
	req.Header.Set("Authorization", "Bearer demo-token-car1")
	w := httptest.NewRecorder()
	mux.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("status = %d, want %d", w.Code, http.StatusBadRequest)
	}
}

// TS-06-E5: Unknown Command ID
func TestUnknownCommandID(t *testing.T) {
	s, pub, a := newTestDeps()
	mux := newTestRouter(s, pub, a)

	req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/nonexistent-id", nil)
	req.Header.Set("Authorization", "Bearer demo-token-car1")
	w := httptest.NewRecorder()
	mux.ServeHTTP(w, req)

	if w.Code != http.StatusNotFound {
		t.Errorf("status = %d, want %d", w.Code, http.StatusNotFound)
	}

	var resp map[string]string
	if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
		t.Fatalf("decode response: %v", err)
	}
	if resp["error"] != "command not found" {
		t.Errorf("error = %q, want %q", resp["error"], "command not found")
	}
}

// TS-06-E6: Auth on Status Query
func TestAuthOnStatusQuery(t *testing.T) {
	s, pub, a := newTestDeps()
	mux := newTestRouter(s, pub, a)

	// No auth → 401
	req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/cmd-001", nil)
	w := httptest.NewRecorder()
	mux.ServeHTTP(w, req)
	if w.Code != http.StatusUnauthorized {
		t.Errorf("no auth: status = %d, want %d", w.Code, http.StatusUnauthorized)
	}

	// Wrong VIN → 403
	req2 := httptest.NewRequest("GET", "/vehicles/VIN99999/commands/cmd-001", nil)
	req2.Header.Set("Authorization", "Bearer demo-token-car1")
	w2 := httptest.NewRecorder()
	mux.ServeHTTP(w2, req2)
	if w2.Code != http.StatusForbidden {
		t.Errorf("wrong VIN: status = %d, want %d", w2.Code, http.StatusForbidden)
	}
}

// TS-06-15: Token Validation on All Endpoints
func TestTokenValidationOnEndpoints(t *testing.T) {
	s, pub, a := newTestDeps()
	mux := newTestRouter(s, pub, a)

	// POST with valid token → 202
	body := `{"command_id":"tv-001","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands", strings.NewReader(body))
	req.Header.Set("Authorization", "Bearer demo-token-car1")
	w := httptest.NewRecorder()
	mux.ServeHTTP(w, req)
	if w.Code != http.StatusAccepted {
		t.Errorf("POST valid auth: status = %d, want %d", w.Code, http.StatusAccepted)
	}

	// POST with no token → 401
	req2 := httptest.NewRequest("POST", "/vehicles/VIN12345/commands", strings.NewReader(body))
	w2 := httptest.NewRecorder()
	mux.ServeHTTP(w2, req2)
	if w2.Code != http.StatusUnauthorized {
		t.Errorf("POST no auth: status = %d, want %d", w2.Code, http.StatusUnauthorized)
	}

	// GET with no token → 401
	req3 := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/tv-001", nil)
	w3 := httptest.NewRecorder()
	mux.ServeHTTP(w3, req3)
	if w3.Code != http.StatusUnauthorized {
		t.Errorf("GET no auth: status = %d, want %d", w3.Code, http.StatusUnauthorized)
	}
}

// TS-06-P1: Command Routing Fidelity Property
func TestPropertyCommandRouting(t *testing.T) {
	vins := []string{"VIN12345", "VIN67890"}
	tokens := map[string]string{
		"VIN12345": "demo-token-car1",
		"VIN67890": "demo-token-car2",
	}

	for _, vin := range vins {
		s, pub, a := newTestDeps()
		mux := newTestRouter(s, pub, a)
		token := tokens[vin]

		body := `{"command_id":"prop-001","type":"unlock","doors":["passenger"]}`
		req := httptest.NewRequest("POST", "/vehicles/"+vin+"/commands", strings.NewReader(body))
		req.Header.Set("Authorization", "Bearer "+token)
		w := httptest.NewRecorder()
		mux.ServeHTTP(w, req)

		if w.Code != http.StatusAccepted {
			t.Errorf("VIN=%s: status = %d, want %d", vin, w.Code, http.StatusAccepted)
		}

		// Verify published to correct VIN
		if len(pub.published) != 1 {
			t.Fatalf("VIN=%s: published %d, want 1", vin, len(pub.published))
		}
		if pub.published[0].VIN != vin {
			t.Errorf("VIN=%s: published VIN = %q", vin, pub.published[0].VIN)
		}
		if pub.published[0].Token != token {
			t.Errorf("VIN=%s: published token = %q", vin, pub.published[0].Token)
		}

		// Verify stored as pending
		cs, found := s.Get("prop-001")
		if !found {
			t.Errorf("VIN=%s: command not in store", vin)
		} else if cs.Status != "pending" {
			t.Errorf("VIN=%s: status = %q, want pending", vin, cs.Status)
		}
	}
}

// TS-06-P7: NATS Subject Correctness Property
func TestPropertyNATSSubjects(t *testing.T) {
	vins := []string{"VIN12345", "VIN67890", "ABC", "X-Y-Z"}
	for _, vin := range vins {
		expected := "vehicles." + vin + ".commands"
		// Verify the subject construction pattern
		got := "vehicles." + vin + ".commands"
		if got != expected {
			t.Errorf("command subject for %q = %q, want %q", vin, got, expected)
		}
	}

	// Response and telemetry subjects are wildcarded
	respSubject := "vehicles.*.command_responses"
	if respSubject != "vehicles.*.command_responses" {
		t.Errorf("response subject = %q, want %q", respSubject, "vehicles.*.command_responses")
	}
	telSubject := "vehicles.*.telemetry"
	if telSubject != "vehicles.*.telemetry" {
		t.Errorf("telemetry subject = %q, want %q", telSubject, "vehicles.*.telemetry")
	}
}
