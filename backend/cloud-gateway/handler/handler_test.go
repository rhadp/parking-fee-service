package handler_test

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/config"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/handler"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// mockPublisher records commands published to NATS.
type mockPublisher struct {
	mu       sync.Mutex
	commands []publishedCommand
}

type publishedCommand struct {
	VIN     string
	Command model.Command
	Token   string
}

func (m *mockPublisher) PublishCommand(vin string, cmd model.Command, token string) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.commands = append(m.commands, publishedCommand{VIN: vin, Command: cmd, Token: token})
	return nil
}

func testConfig() *config.Config {
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

func setupTestServer(pub handler.CommandPublisher, s *store.Store) *httptest.Server {
	cfg := testConfig()
	mux := http.NewServeMux()
	authMW := auth.Middleware(cfg)

	mux.Handle("POST /vehicles/{vin}/commands",
		authMW(handler.NewSubmitCommandHandler(pub, s, 30*time.Second)))
	mux.Handle("GET /vehicles/{vin}/commands/{command_id}",
		authMW(handler.NewGetCommandStatusHandler(s)))
	mux.HandleFunc("GET /health", handler.HealthHandler())

	return httptest.NewServer(mux)
}

// ---------------------------------------------------------------------------
// TS-06-1: Command Submission Success
// Requirement: 06-REQ-1.1
// ---------------------------------------------------------------------------

func TestCommandSubmissionSuccess(t *testing.T) {
	pub := &mockPublisher{}
	s := store.NewStore()
	ts := setupTestServer(pub, s)
	defer ts.Close()

	body := `{"command_id":"cmd-001","type":"lock","doors":["driver"]}`
	req, _ := http.NewRequest("POST", ts.URL+"/vehicles/VIN12345/commands",
		strings.NewReader(body))
	req.Header.Set("Authorization", "Bearer demo-token-001")
	req.Header.Set("Content-Type", "application/json")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("POST failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 202 {
		t.Errorf("expected status 202, got %d", resp.StatusCode)
	}

	var result model.Command
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if result.CommandID != "cmd-001" {
		t.Errorf("expected command_id 'cmd-001', got %q", result.CommandID)
	}
	if result.Type != "lock" {
		t.Errorf("expected type 'lock', got %q", result.Type)
	}
	if len(result.Doors) != 1 || result.Doors[0] != "driver" {
		t.Errorf("expected doors ['driver'], got %v", result.Doors)
	}

	// Verify command was published to mock NATS
	pub.mu.Lock()
	defer pub.mu.Unlock()
	if len(pub.commands) != 1 {
		t.Fatalf("expected 1 published command, got %d", len(pub.commands))
	}
	if pub.commands[0].VIN != "VIN12345" {
		t.Errorf("expected published VIN 'VIN12345', got %q", pub.commands[0].VIN)
	}
	if pub.commands[0].Command.CommandID != "cmd-001" {
		t.Errorf("expected published command_id 'cmd-001', got %q",
			pub.commands[0].Command.CommandID)
	}
}

// ---------------------------------------------------------------------------
// TS-06-4: Command Status Query Success
// Requirement: 06-REQ-2.1
// ---------------------------------------------------------------------------

func TestCommandStatusQuerySuccess(t *testing.T) {
	pub := &mockPublisher{}
	s := store.NewStore()
	s.StoreResponse(model.CommandResponse{CommandID: "cmd-004", Status: "success"})

	ts := setupTestServer(pub, s)
	defer ts.Close()

	req, _ := http.NewRequest("GET", ts.URL+"/vehicles/VIN12345/commands/cmd-004", nil)
	req.Header.Set("Authorization", "Bearer demo-token-001")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("GET failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		t.Errorf("expected status 200, got %d", resp.StatusCode)
	}

	var result model.CommandResponse
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if result.CommandID != "cmd-004" {
		t.Errorf("expected command_id 'cmd-004', got %q", result.CommandID)
	}
	if result.Status != "success" {
		t.Errorf("expected status 'success', got %q", result.Status)
	}
}

// ---------------------------------------------------------------------------
// TS-06-10: Health Check
// Requirement: 06-REQ-4.1
// ---------------------------------------------------------------------------

func TestHealthCheck(t *testing.T) {
	pub := &mockPublisher{}
	s := store.NewStore()
	ts := setupTestServer(pub, s)
	defer ts.Close()

	resp, err := http.Get(ts.URL + "/health")
	if err != nil {
		t.Fatalf("GET /health failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		t.Errorf("expected status 200, got %d", resp.StatusCode)
	}

	var body map[string]string
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if body["status"] != "ok" {
		t.Errorf("expected status 'ok', got %q", body["status"])
	}
}

// ---------------------------------------------------------------------------
// TS-06-13: Content-Type Header
// Requirement: 06-REQ-7.1
// ---------------------------------------------------------------------------

func TestContentTypeHeader(t *testing.T) {
	pub := &mockPublisher{}
	s := store.NewStore()
	ts := setupTestServer(pub, s)
	defer ts.Close()

	tests := []struct {
		name   string
		method string
		path   string
		auth   string
		body   string
	}{
		{"health", "GET", "/health", "", ""},
		{"submit", "POST", "/vehicles/VIN12345/commands", "Bearer demo-token-001",
			`{"command_id":"ct-001","type":"lock","doors":["driver"]}`},
		{"status_not_found", "GET", "/vehicles/VIN12345/commands/nonexistent",
			"Bearer demo-token-001", ""},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			var req *http.Request
			if tt.body != "" {
				req, _ = http.NewRequest(tt.method, ts.URL+tt.path,
					strings.NewReader(tt.body))
			} else {
				req, _ = http.NewRequest(tt.method, ts.URL+tt.path, nil)
			}
			if tt.auth != "" {
				req.Header.Set("Authorization", tt.auth)
			}

			resp, err := http.DefaultClient.Do(req)
			if err != nil {
				t.Fatalf("request failed: %v", err)
			}
			resp.Body.Close()

			ct := resp.Header.Get("Content-Type")
			if ct != "application/json" {
				t.Errorf("expected Content-Type 'application/json', got %q", ct)
			}
		})
	}
}

// ---------------------------------------------------------------------------
// TS-06-E1: Invalid Command Payload
// Requirement: 06-REQ-1.E1
// ---------------------------------------------------------------------------

func TestInvalidCommandPayload(t *testing.T) {
	pub := &mockPublisher{}
	s := store.NewStore()
	ts := setupTestServer(pub, s)
	defer ts.Close()

	bodies := []struct {
		name string
		json string
	}{
		{"empty_object", `{}`},
		{"missing_type_and_doors", `{"command_id":"x"}`},
		{"missing_doors", `{"command_id":"x","type":"lock"}`},
	}

	for _, tt := range bodies {
		t.Run(tt.name, func(t *testing.T) {
			req, _ := http.NewRequest("POST", ts.URL+"/vehicles/VIN12345/commands",
				strings.NewReader(tt.json))
			req.Header.Set("Authorization", "Bearer demo-token-001")
			req.Header.Set("Content-Type", "application/json")

			resp, err := http.DefaultClient.Do(req)
			if err != nil {
				t.Fatalf("POST failed: %v", err)
			}
			defer resp.Body.Close()

			if resp.StatusCode != 400 {
				t.Errorf("expected status 400, got %d", resp.StatusCode)
			}

			var result map[string]string
			if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
				t.Fatalf("failed to decode response: %v", err)
			}
			if result["error"] != "invalid command payload" {
				t.Errorf("expected error 'invalid command payload', got %q",
					result["error"])
			}
		})
	}
}

// ---------------------------------------------------------------------------
// TS-06-E2: Invalid Command Type
// Requirement: 06-REQ-1.E2
// ---------------------------------------------------------------------------

func TestInvalidCommandType(t *testing.T) {
	pub := &mockPublisher{}
	s := store.NewStore()
	ts := setupTestServer(pub, s)
	defer ts.Close()

	body := `{"command_id":"x","type":"start","doors":["driver"]}`
	req, _ := http.NewRequest("POST", ts.URL+"/vehicles/VIN12345/commands",
		strings.NewReader(body))
	req.Header.Set("Authorization", "Bearer demo-token-001")
	req.Header.Set("Content-Type", "application/json")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("POST failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 400 {
		t.Errorf("expected status 400, got %d", resp.StatusCode)
	}

	var result map[string]string
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if result["error"] != "invalid command type" {
		t.Errorf("expected error 'invalid command type', got %q", result["error"])
	}
}

// ---------------------------------------------------------------------------
// TS-06-E3: Command Not Found
// Requirement: 06-REQ-2.E1
// ---------------------------------------------------------------------------

func TestCommandNotFound(t *testing.T) {
	pub := &mockPublisher{}
	s := store.NewStore()
	ts := setupTestServer(pub, s)
	defer ts.Close()

	req, _ := http.NewRequest("GET", ts.URL+"/vehicles/VIN12345/commands/nonexistent", nil)
	req.Header.Set("Authorization", "Bearer demo-token-001")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("GET failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 404 {
		t.Errorf("expected status 404, got %d", resp.StatusCode)
	}

	var result map[string]string
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if result["error"] != "command not found" {
		t.Errorf("expected error 'command not found', got %q", result["error"])
	}
}

// ---------------------------------------------------------------------------
// TS-06-E9: Error Response Format
// Requirement: 06-REQ-7.2
// ---------------------------------------------------------------------------

func TestErrorResponseFormat(t *testing.T) {
	pub := &mockPublisher{}
	s := store.NewStore()
	ts := setupTestServer(pub, s)
	defer ts.Close()

	// 401 case - no auth header
	t.Run("unauthorized", func(t *testing.T) {
		resp, err := http.Post(ts.URL+"/vehicles/VIN12345/commands", "application/json",
			strings.NewReader(`{"command_id":"x","type":"lock","doors":["driver"]}`))
		if err != nil {
			t.Fatalf("POST without auth failed: %v", err)
		}
		defer resp.Body.Close()

		if resp.StatusCode != 401 {
			t.Errorf("expected status 401, got %d", resp.StatusCode)
		}

		var errBody map[string]string
		if err := json.NewDecoder(resp.Body).Decode(&errBody); err != nil {
			t.Fatalf("failed to decode 401 response: %v", err)
		}
		if errBody["error"] == "" {
			t.Error("expected non-empty error field in 401 response")
		}
	})

	// 403 case - valid token, wrong VIN
	t.Run("forbidden", func(t *testing.T) {
		req, _ := http.NewRequest("POST", ts.URL+"/vehicles/VIN99999/commands",
			strings.NewReader(`{"command_id":"x","type":"lock","doors":["driver"]}`))
		req.Header.Set("Authorization", "Bearer demo-token-001")

		resp, err := http.DefaultClient.Do(req)
		if err != nil {
			t.Fatalf("POST with wrong VIN failed: %v", err)
		}
		defer resp.Body.Close()

		if resp.StatusCode != 403 {
			t.Errorf("expected status 403, got %d", resp.StatusCode)
		}

		var errBody map[string]string
		if err := json.NewDecoder(resp.Body).Decode(&errBody); err != nil {
			t.Fatalf("failed to decode 403 response: %v", err)
		}
		if errBody["error"] == "" {
			t.Error("expected non-empty error field in 403 response")
		}
	})
}
