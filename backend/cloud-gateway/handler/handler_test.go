package handler_test

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/config"
	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/handler"
	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/model"
	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/store"
)

// mockPublisher is a NATSPublisher that records every call to PublishCommand.
type mockPublisher struct {
	calls []publishCall
	err   error // if non-nil, PublishCommand returns this error
}

type publishCall struct {
	vin   string
	cmd   model.Command
	token string
}

func (m *mockPublisher) PublishCommand(vin string, cmd model.Command, token string) error {
	m.calls = append(m.calls, publishCall{vin: vin, cmd: cmd, token: token})
	return m.err
}

// testCfg returns a Config with two demo tokens for test use.
func testCfg() *config.Config {
	return &config.Config{
		Port:                  8081,
		NatsURL:               "nats://localhost:4222",
		CommandTimeoutSeconds: 30,
		Tokens: []config.TokenMapping{
			{Token: "demo-token-001", VIN: "VIN12345"},
			{Token: "demo-token-002", VIN: "VIN67890"},
		},
	}
}

// newTestServer builds an httptest.Server wired with auth middleware and all handlers.
// Requests flow through Go 1.22 ServeMux so r.PathValue works inside handlers.
func newTestServer(t *testing.T, nc *mockPublisher, s *store.Store) *httptest.Server {
	t.Helper()
	cfg := testCfg()
	timeout := 30 * time.Second

	mux := http.NewServeMux()
	mux.Handle("POST /vehicles/{vin}/commands",
		auth.Middleware(cfg)(handler.NewSubmitCommandHandler(nc, s, timeout)))
	mux.Handle("GET /vehicles/{vin}/commands/{command_id}",
		auth.Middleware(cfg)(handler.NewGetCommandStatusHandler(s)))
	mux.Handle("GET /health", handler.HealthHandler())

	return httptest.NewServer(mux)
}

// postJSON sends a POST request with a JSON body to the given URL and returns the response.
func postJSON(t *testing.T, url, body, token string) *http.Response {
	t.Helper()
	req, err := http.NewRequest(http.MethodPost, url, bytes.NewBufferString(body))
	if err != nil {
		t.Fatalf("NewRequest: %v", err)
	}
	req.Header.Set("Content-Type", "application/json")
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("Do: %v", err)
	}
	return resp
}

// getWithToken sends a GET request with an Authorization header.
func getWithToken(t *testing.T, url, token string) *http.Response {
	t.Helper()
	req, err := http.NewRequest(http.MethodGet, url, nil)
	if err != nil {
		t.Fatalf("NewRequest: %v", err)
	}
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("Do: %v", err)
	}
	return resp
}

// decodeBody decodes the response body into a map[string]any.
func decodeBody(t *testing.T, resp *http.Response) map[string]any {
	t.Helper()
	var body map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("decode body: %v", err)
	}
	resp.Body.Close()
	return body
}

// TestCommandSubmissionSuccess verifies that a valid POST request returns HTTP 202
// with the command echoed back in the response body.
// TS-06-1
func TestCommandSubmissionSuccess(t *testing.T) {
	nc := &mockPublisher{}
	s := store.NewStore()
	srv := newTestServer(t, nc, s)
	defer srv.Close()

	body := `{"command_id":"cmd-001","type":"lock","doors":["driver"]}`
	resp := postJSON(t, srv.URL+"/vehicles/VIN12345/commands", body, "demo-token-001")
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusAccepted {
		t.Errorf("status: want 202, got %d", resp.StatusCode)
	}
	decoded := decodeBody(t, resp)
	if decoded["command_id"] != "cmd-001" {
		t.Errorf("command_id: want 'cmd-001', got %v", decoded["command_id"])
	}
	if decoded["type"] != "lock" {
		t.Errorf("type: want 'lock', got %v", decoded["type"])
	}
	doors, ok := decoded["doors"].([]any)
	if !ok || len(doors) != 1 || doors[0] != "driver" {
		t.Errorf("doors: want [driver], got %v", decoded["doors"])
	}
}

// TestCommandStatusQuerySuccess verifies that a pre-stored response is returned
// with HTTP 200 via GET /vehicles/{vin}/commands/{command_id}.
// TS-06-4
func TestCommandStatusQuerySuccess(t *testing.T) {
	nc := &mockPublisher{}
	s := store.NewStore()
	s.StoreResponse(model.CommandResponse{CommandID: "cmd-004", Status: "success"})

	srv := newTestServer(t, nc, s)
	defer srv.Close()

	resp := getWithToken(t, srv.URL+"/vehicles/VIN12345/commands/cmd-004", "demo-token-001")
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("status: want 200, got %d", resp.StatusCode)
	}
	decoded := decodeBody(t, resp)
	if decoded["command_id"] != "cmd-004" {
		t.Errorf("command_id: want 'cmd-004', got %v", decoded["command_id"])
	}
	if decoded["status"] != "success" {
		t.Errorf("status: want 'success', got %v", decoded["status"])
	}
}

// TestHealthCheck verifies that GET /health returns HTTP 200 with {"status":"ok"}.
// TS-06-10
func TestHealthCheck(t *testing.T) {
	nc := &mockPublisher{}
	s := store.NewStore()
	srv := newTestServer(t, nc, s)
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/health")
	if err != nil {
		t.Fatalf("GET /health: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("status: want 200, got %d", resp.StatusCode)
	}
	decoded := decodeBody(t, resp)
	if decoded["status"] != "ok" {
		t.Errorf("status field: want 'ok', got %v", decoded["status"])
	}
}

// TestContentTypeHeader verifies that all REST responses set Content-Type: application/json.
// TS-06-13
func TestContentTypeHeader(t *testing.T) {
	nc := &mockPublisher{}
	s := store.NewStore()
	srv := newTestServer(t, nc, s)
	defer srv.Close()

	endpoints := []struct {
		name    string
		method  string
		path    string
		body    string
		token   string
	}{
		{
			name:   "GET /health",
			method: http.MethodGet,
			path:   "/health",
		},
		{
			name:   "POST /vehicles/{vin}/commands (valid)",
			method: http.MethodPost,
			path:   "/vehicles/VIN12345/commands",
			body:   `{"command_id":"ct-001","type":"lock","doors":["driver"]}`,
			token:  "demo-token-001",
		},
		{
			name:   "GET /vehicles/{vin}/commands/{id} (not found)",
			method: http.MethodGet,
			path:   "/vehicles/VIN12345/commands/nonexistent",
			token:  "demo-token-001",
		},
	}

	for _, ep := range endpoints {
		t.Run(ep.name, func(t *testing.T) {
			var resp *http.Response
			switch ep.method {
			case http.MethodGet:
				resp = getWithToken(t, srv.URL+ep.path, ep.token)
			case http.MethodPost:
				resp = postJSON(t, srv.URL+ep.path, ep.body, ep.token)
			default:
				t.Fatalf("unsupported method: %s", ep.method)
			}
			defer resp.Body.Close()

			ct := resp.Header.Get("Content-Type")
			if !strings.HasPrefix(ct, "application/json") {
				t.Errorf("Content-Type: want 'application/json', got %q", ct)
			}
		})
	}
}

// TestInvalidCommandPayload verifies that POST with missing required fields returns
// HTTP 400 with {"error":"invalid command payload"}.
// TS-06-E1
func TestInvalidCommandPayload(t *testing.T) {
	nc := &mockPublisher{}
	s := store.NewStore()
	srv := newTestServer(t, nc, s)
	defer srv.Close()

	bodies := []string{
		`{}`,
		`{"command_id":"x"}`,
		`{"command_id":"x","type":"lock"}`,
	}

	for _, body := range bodies {
		t.Run(body, func(t *testing.T) {
			resp := postJSON(t, srv.URL+"/vehicles/VIN12345/commands", body, "demo-token-001")
			defer resp.Body.Close()

			if resp.StatusCode != http.StatusBadRequest {
				t.Errorf("status: want 400, got %d", resp.StatusCode)
			}
			decoded := decodeBody(t, resp)
			if decoded["error"] != "invalid command payload" {
				t.Errorf("error: want 'invalid command payload', got %v", decoded["error"])
			}
		})
	}
}

// TestInvalidCommandType verifies that a command type other than "lock" or "unlock"
// returns HTTP 400 with {"error":"invalid command type"}.
// TS-06-E2
func TestInvalidCommandType(t *testing.T) {
	nc := &mockPublisher{}
	s := store.NewStore()
	srv := newTestServer(t, nc, s)
	defer srv.Close()

	body := `{"command_id":"x","type":"start","doors":["driver"]}`
	resp := postJSON(t, srv.URL+"/vehicles/VIN12345/commands", body, "demo-token-001")
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusBadRequest {
		t.Errorf("status: want 400, got %d", resp.StatusCode)
	}
	decoded := decodeBody(t, resp)
	if decoded["error"] != "invalid command type" {
		t.Errorf("error: want 'invalid command type', got %v", decoded["error"])
	}
}

// TestCommandNotFound verifies that querying a nonexistent command_id returns
// HTTP 404 with {"error":"command not found"}.
// TS-06-E3
func TestCommandNotFound(t *testing.T) {
	nc := &mockPublisher{}
	s := store.NewStore()
	srv := newTestServer(t, nc, s)
	defer srv.Close()

	resp := getWithToken(t, srv.URL+"/vehicles/VIN12345/commands/nonexistent", "demo-token-001")
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusNotFound {
		t.Errorf("status: want 404, got %d", resp.StatusCode)
	}
	decoded := decodeBody(t, resp)
	if decoded["error"] != "command not found" {
		t.Errorf("error: want 'command not found', got %v", decoded["error"])
	}
}

// TestErrorResponseFormat verifies that auth error responses (401, 403) use the
// format {"error":"<message>"} with a non-empty message.
// TS-06-E9
func TestErrorResponseFormat(t *testing.T) {
	nc := &mockPublisher{}
	s := store.NewStore()
	srv := newTestServer(t, nc, s)
	defer srv.Close()

	t.Run("401 no auth", func(t *testing.T) {
		resp := postJSON(t, srv.URL+"/vehicles/VIN12345/commands",
			`{"command_id":"e9","type":"lock","doors":["driver"]}`, "")
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusUnauthorized {
			t.Errorf("status: want 401, got %d", resp.StatusCode)
		}
		decoded := decodeBody(t, resp)
		msg, _ := decoded["error"].(string)
		if msg == "" {
			t.Errorf("error field: want non-empty string, got %v", decoded["error"])
		}
	})

	t.Run("403 wrong VIN", func(t *testing.T) {
		// demo-token-001 is mapped to VIN12345, but we request VIN99999.
		resp := postJSON(t, srv.URL+"/vehicles/VIN99999/commands",
			`{"command_id":"e9b","type":"lock","doors":["driver"]}`, "demo-token-001")
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusForbidden {
			t.Errorf("status: want 403, got %d", resp.StatusCode)
		}
		decoded := decodeBody(t, resp)
		msg, _ := decoded["error"].(string)
		if msg == "" {
			t.Errorf("error field: want non-empty string, got %v", decoded["error"])
		}
	})
}

// TestPropertyNATSHeaderPropagation verifies that for any command submitted via REST,
// the NATSPublisher is called with the bearer token from the originating request.
// TS-06-P6
func TestPropertyNATSHeaderPropagation(t *testing.T) {
	cases := []struct {
		token string
		vin   string
		cmdID string
	}{
		{"demo-token-001", "VIN12345", "prop-p6-001"},
		{"demo-token-002", "VIN67890", "prop-p6-002"},
	}

	for _, c := range cases {
		t.Run(c.token, func(t *testing.T) {
			nc := &mockPublisher{}
			s := store.NewStore()
			srv := newTestServer(t, nc, s)
			defer srv.Close()

			body := `{"command_id":"` + c.cmdID + `","type":"lock","doors":["driver"]}`
			resp := postJSON(t, srv.URL+"/vehicles/"+c.vin+"/commands", body, c.token)
			defer resp.Body.Close()

			if resp.StatusCode != http.StatusAccepted {
				t.Errorf("status: want 202, got %d", resp.StatusCode)
			}

			if len(nc.calls) == 0 {
				t.Fatal("PublishCommand not called: want at least 1 call")
			}
			call := nc.calls[0]
			if call.token != c.token {
				t.Errorf("token in NATS call: want %q, got %q", c.token, call.token)
			}
			if call.vin != c.vin {
				t.Errorf("vin in NATS call: want %q, got %q", c.vin, call.vin)
			}
		})
	}
}
