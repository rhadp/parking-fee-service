package main

import (
	"bytes"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"
)

// ── Flag parsing tests ──────────────────────────────────────────────────────

func TestParseGlobalFlagsDefaults(t *testing.T) {
	t.Setenv("GATEWAY_ADDR", "")
	t.Setenv("VIN", "")
	t.Setenv("TOKEN", "")
	t.Setenv("PIN", "")
	os.Unsetenv("GATEWAY_ADDR")
	os.Unsetenv("VIN")
	os.Unsetenv("TOKEN")
	os.Unsetenv("PIN")

	cfg, remaining, err := parseGlobalFlags([]string{"status"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if cfg.gatewayAddr != "http://localhost:8081" {
		t.Errorf("expected default gateway-addr 'http://localhost:8081', got %q", cfg.gatewayAddr)
	}
	if cfg.token != "" {
		t.Errorf("expected empty token by default, got %q", cfg.token)
	}
	if cfg.vin != "" {
		t.Errorf("expected empty vin by default, got %q", cfg.vin)
	}
	if cfg.pin != "" {
		t.Errorf("expected empty pin by default, got %q", cfg.pin)
	}
	if len(remaining) != 1 || remaining[0] != "status" {
		t.Errorf("expected remaining [status], got %v", remaining)
	}
}

func TestParseGlobalFlagsCustom(t *testing.T) {
	os.Unsetenv("GATEWAY_ADDR")
	os.Unsetenv("VIN")
	os.Unsetenv("TOKEN")
	os.Unsetenv("PIN")

	cfg, remaining, err := parseGlobalFlags([]string{
		"--gateway-addr", "http://10.0.0.1:8081",
		"--vin", "WBA12345678901234",
		"--token", "my-secret-token",
		"--pin", "123456",
		"lock",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if cfg.gatewayAddr != "http://10.0.0.1:8081" {
		t.Errorf("expected gateway-addr 'http://10.0.0.1:8081', got %q", cfg.gatewayAddr)
	}
	if cfg.vin != "WBA12345678901234" {
		t.Errorf("expected vin 'WBA12345678901234', got %q", cfg.vin)
	}
	if cfg.token != "my-secret-token" {
		t.Errorf("expected token 'my-secret-token', got %q", cfg.token)
	}
	if cfg.pin != "123456" {
		t.Errorf("expected pin '123456', got %q", cfg.pin)
	}
	if len(remaining) != 1 || remaining[0] != "lock" {
		t.Errorf("expected remaining [lock], got %v", remaining)
	}
}

func TestParseGlobalFlagsFromEnv(t *testing.T) {
	t.Setenv("GATEWAY_ADDR", "http://envhost:8081")
	t.Setenv("VIN", "ENVVIN123456789")
	t.Setenv("TOKEN", "env-token")
	t.Setenv("PIN", "654321")

	cfg, remaining, err := parseGlobalFlags([]string{"unlock"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if cfg.gatewayAddr != "http://envhost:8081" {
		t.Errorf("expected gateway-addr from env 'http://envhost:8081', got %q", cfg.gatewayAddr)
	}
	if cfg.vin != "ENVVIN123456789" {
		t.Errorf("expected vin from env 'ENVVIN123456789', got %q", cfg.vin)
	}
	if cfg.token != "env-token" {
		t.Errorf("expected token from env 'env-token', got %q", cfg.token)
	}
	if cfg.pin != "654321" {
		t.Errorf("expected pin from env '654321', got %q", cfg.pin)
	}
	if len(remaining) != 1 || remaining[0] != "unlock" {
		t.Errorf("expected remaining [unlock], got %v", remaining)
	}
}

func TestParseGlobalFlagsMissingValues(t *testing.T) {
	tests := []struct {
		name string
		args []string
		want string
	}{
		{"gateway-addr", []string{"--gateway-addr"}, "--gateway-addr requires a value"},
		{"vin", []string{"--vin"}, "--vin requires a value"},
		{"token", []string{"--token"}, "--token requires a value"},
		{"pin", []string{"--pin"}, "--pin requires a value"},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, _, err := parseGlobalFlags(tt.args)
			if err == nil {
				t.Fatalf("expected error for %v", tt.args)
			}
			if err.Error() != tt.want {
				t.Errorf("expected %q, got %q", tt.want, err.Error())
			}
		})
	}
}

// ── CLI dispatch tests ──────────────────────────────────────────────────────

func TestRunNoArgs(t *testing.T) {
	os.Unsetenv("GATEWAY_ADDR")
	os.Unsetenv("VIN")
	os.Unsetenv("TOKEN")
	os.Unsetenv("PIN")

	err := run(nil, io.Discard, io.Discard)
	if err == nil {
		t.Fatal("expected error for no args")
	}
	if err.Error() != "no command specified" {
		t.Errorf("expected 'no command specified', got %q", err.Error())
	}
}

func TestRunVINRequired(t *testing.T) {
	os.Unsetenv("VIN")

	err := run([]string{"lock"}, io.Discard, io.Discard)
	if err == nil {
		t.Fatal("expected error when --vin is missing")
	}
	if err.Error() != "--vin is required" {
		t.Errorf("expected '--vin is required', got %q", err.Error())
	}
}

func TestRunUnknownCommand(t *testing.T) {
	os.Unsetenv("VIN")

	err := run([]string{"--vin", "TEST123", "nonexistent"}, io.Discard, io.Discard)
	if err == nil {
		t.Fatal("expected error for unknown command")
	}
	expected := "unknown command: nonexistent"
	if err.Error() != expected {
		t.Errorf("expected %q, got %q", expected, err.Error())
	}
}

func TestRunHelp(t *testing.T) {
	os.Unsetenv("VIN")

	for _, helpFlag := range []string{"help", "--help", "-h"} {
		t.Run(helpFlag, func(t *testing.T) {
			var stderr bytes.Buffer
			err := run([]string{helpFlag}, io.Discard, &stderr)
			if err != nil {
				t.Fatalf("unexpected error for %s: %v", helpFlag, err)
			}
			output := stderr.String()
			if !strings.Contains(output, "pair") {
				t.Errorf("help output should mention 'pair' command, got %q", output)
			}
			if !strings.Contains(output, "--pin") {
				t.Errorf("help output should mention '--pin' flag, got %q", output)
			}
		})
	}
}

func TestRunPairRequiresPin(t *testing.T) {
	os.Unsetenv("PIN")

	err := run([]string{"--vin", "TEST123", "pair"}, io.Discard, io.Discard)
	if err == nil {
		t.Fatal("expected error when --pin is missing for pair")
	}
	if !strings.Contains(err.Error(), "--pin is required") {
		t.Errorf("expected '--pin is required', got %q", err.Error())
	}
}

func TestRunLockRequiresToken(t *testing.T) {
	os.Unsetenv("TOKEN")

	err := run([]string{"--vin", "TEST123", "lock"}, io.Discard, io.Discard)
	if err == nil {
		t.Fatal("expected error when --token is missing for lock")
	}
	if !strings.Contains(err.Error(), "--token is required") {
		t.Errorf("expected '--token is required', got %q", err.Error())
	}
}

func TestRunUnlockRequiresToken(t *testing.T) {
	os.Unsetenv("TOKEN")

	err := run([]string{"--vin", "TEST123", "unlock"}, io.Discard, io.Discard)
	if err == nil {
		t.Fatal("expected error when --token is missing for unlock")
	}
	if !strings.Contains(err.Error(), "--token is required") {
		t.Errorf("expected '--token is required', got %q", err.Error())
	}
}

func TestRunStatusRequiresToken(t *testing.T) {
	os.Unsetenv("TOKEN")

	err := run([]string{"--vin", "TEST123", "status"}, io.Discard, io.Discard)
	if err == nil {
		t.Fatal("expected error when --token is missing for status")
	}
	if !strings.Contains(err.Error(), "--token is required") {
		t.Errorf("expected '--token is required', got %q", err.Error())
	}
}

// ── HTTP request construction tests using httptest.Server ───────────────────

func TestPairHTTPRequest(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Verify method and path.
		if r.Method != http.MethodPost {
			t.Errorf("expected POST, got %s", r.Method)
		}
		if r.URL.Path != "/api/v1/pair" {
			t.Errorf("expected path /api/v1/pair, got %s", r.URL.Path)
		}

		// Verify Content-Type.
		if ct := r.Header.Get("Content-Type"); ct != "application/json" {
			t.Errorf("expected Content-Type application/json, got %s", ct)
		}

		// No Authorization header for pair.
		if auth := r.Header.Get("Authorization"); auth != "" {
			t.Errorf("expected no Authorization header for pair, got %s", auth)
		}

		// Verify request body.
		var req pairRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			t.Fatalf("failed to decode pair request body: %v", err)
		}
		if req.VIN != "DEMO0000000000001" {
			t.Errorf("expected VIN 'DEMO0000000000001', got %q", req.VIN)
		}
		if req.PIN != "482916" {
			t.Errorf("expected PIN '482916', got %q", req.PIN)
		}

		// Return successful pair response.
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(pairResponse{
			Token: "test-token-abc123",
			VIN:   "DEMO0000000000001",
		})
	}))
	defer server.Close()

	var stdout bytes.Buffer
	err := run([]string{
		"--gateway-addr", server.URL,
		"--vin", "DEMO0000000000001",
		"--pin", "482916",
		"pair",
	}, &stdout, io.Discard)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	output := stdout.String()
	if !strings.Contains(output, "Paired successfully") {
		t.Errorf("expected 'Paired successfully' in output, got %q", output)
	}
	if !strings.Contains(output, "test-token-abc123") {
		t.Errorf("expected token in output, got %q", output)
	}
}

func TestLockHTTPRequest(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			t.Errorf("expected POST, got %s", r.Method)
		}
		if r.URL.Path != "/api/v1/vehicles/DEMO0000000000001/lock" {
			t.Errorf("expected path /api/v1/vehicles/DEMO0000000000001/lock, got %s", r.URL.Path)
		}

		// Verify bearer token.
		auth := r.Header.Get("Authorization")
		if auth != "Bearer my-token" {
			t.Errorf("expected 'Bearer my-token', got %q", auth)
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusAccepted)
		json.NewEncoder(w).Encode(map[string]string{
			"command_id": "cmd-123",
			"status":     "accepted",
		})
	}))
	defer server.Close()

	var stdout bytes.Buffer
	err := run([]string{
		"--gateway-addr", server.URL,
		"--vin", "DEMO0000000000001",
		"--token", "my-token",
		"lock",
	}, &stdout, io.Discard)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	output := stdout.String()
	if !strings.Contains(output, "Lock command accepted") {
		t.Errorf("expected 'Lock command accepted' in output, got %q", output)
	}
	if !strings.Contains(output, "cmd-123") {
		t.Errorf("expected command_id in output, got %q", output)
	}
}

func TestUnlockHTTPRequest(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			t.Errorf("expected POST, got %s", r.Method)
		}
		if r.URL.Path != "/api/v1/vehicles/DEMO0000000000001/unlock" {
			t.Errorf("expected path /api/v1/vehicles/DEMO0000000000001/unlock, got %s", r.URL.Path)
		}

		auth := r.Header.Get("Authorization")
		if auth != "Bearer my-token" {
			t.Errorf("expected 'Bearer my-token', got %q", auth)
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusAccepted)
		json.NewEncoder(w).Encode(map[string]string{
			"command_id": "cmd-456",
			"status":     "accepted",
		})
	}))
	defer server.Close()

	var stdout bytes.Buffer
	err := run([]string{
		"--gateway-addr", server.URL,
		"--vin", "DEMO0000000000001",
		"--token", "my-token",
		"unlock",
	}, &stdout, io.Discard)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	output := stdout.String()
	if !strings.Contains(output, "Unlock command accepted") {
		t.Errorf("expected 'Unlock command accepted' in output, got %q", output)
	}
	if !strings.Contains(output, "cmd-456") {
		t.Errorf("expected command_id in output, got %q", output)
	}
}

func TestStatusHTTPRequest(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			t.Errorf("expected GET, got %s", r.Method)
		}
		if r.URL.Path != "/api/v1/vehicles/DEMO0000000000001/status" {
			t.Errorf("expected path /api/v1/vehicles/DEMO0000000000001/status, got %s", r.URL.Path)
		}

		auth := r.Header.Get("Authorization")
		if auth != "Bearer my-token" {
			t.Errorf("expected 'Bearer my-token', got %q", auth)
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(map[string]interface{}{
			"vin":                    "DEMO0000000000001",
			"is_locked":             true,
			"is_door_open":          false,
			"speed":                 0.0,
			"latitude":             48.1351,
			"longitude":            11.5820,
			"parking_session_active": false,
			"updated_at":           "2024-02-19T10:00:00Z",
		})
	}))
	defer server.Close()

	var stdout bytes.Buffer
	err := run([]string{
		"--gateway-addr", server.URL,
		"--vin", "DEMO0000000000001",
		"--token", "my-token",
		"status",
	}, &stdout, io.Discard)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	output := stdout.String()
	if !strings.Contains(output, "Vehicle Status:") {
		t.Errorf("expected 'Vehicle Status:' in output, got %q", output)
	}
	if !strings.Contains(output, "DEMO0000000000001") {
		t.Errorf("expected VIN in output, got %q", output)
	}
	if !strings.Contains(output, "48.1351") {
		t.Errorf("expected latitude in output, got %q", output)
	}
}

// ── Error handling tests ────────────────────────────────────────────────────

func TestPairNotFound(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusNotFound)
		json.NewEncoder(w).Encode(errorResponse{
			Error: "vehicle not found",
			Code:  "NOT_FOUND",
		})
	}))
	defer server.Close()

	err := run([]string{
		"--gateway-addr", server.URL,
		"--vin", "UNKNOWN",
		"--pin", "000000",
		"pair",
	}, io.Discard, io.Discard)
	if err == nil {
		t.Fatal("expected error for 404 response")
	}
	if !strings.Contains(err.Error(), "404") {
		t.Errorf("expected 404 in error, got %q", err.Error())
	}
	if !strings.Contains(err.Error(), "vehicle not found") {
		t.Errorf("expected 'vehicle not found' in error, got %q", err.Error())
	}
}

func TestPairWrongPin(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusForbidden)
		json.NewEncoder(w).Encode(errorResponse{
			Error: "incorrect pairing PIN",
			Code:  "FORBIDDEN",
		})
	}))
	defer server.Close()

	err := run([]string{
		"--gateway-addr", server.URL,
		"--vin", "DEMO0000000000001",
		"--pin", "999999",
		"pair",
	}, io.Discard, io.Discard)
	if err == nil {
		t.Fatal("expected error for 403 response")
	}
	if !strings.Contains(err.Error(), "403") {
		t.Errorf("expected 403 in error, got %q", err.Error())
	}
	if !strings.Contains(err.Error(), "incorrect pairing PIN") {
		t.Errorf("expected 'incorrect pairing PIN' in error, got %q", err.Error())
	}
}

func TestLockUnauthorized(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusUnauthorized)
		json.NewEncoder(w).Encode(errorResponse{
			Error: "invalid or missing bearer token",
			Code:  "UNAUTHORIZED",
		})
	}))
	defer server.Close()

	err := run([]string{
		"--gateway-addr", server.URL,
		"--vin", "DEMO0000000000001",
		"--token", "bad-token",
		"lock",
	}, io.Discard, io.Discard)
	if err == nil {
		t.Fatal("expected error for 401 response")
	}
	if !strings.Contains(err.Error(), "401") {
		t.Errorf("expected 401 in error, got %q", err.Error())
	}
}

func TestStatusVehicleNotFound(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusNotFound)
		json.NewEncoder(w).Encode(errorResponse{
			Error: "vehicle not found",
			Code:  "NOT_FOUND",
		})
	}))
	defer server.Close()

	err := run([]string{
		"--gateway-addr", server.URL,
		"--vin", "UNKNOWN",
		"--token", "some-token",
		"status",
	}, io.Discard, io.Discard)
	if err == nil {
		t.Fatal("expected error for 404 response")
	}
	if !strings.Contains(err.Error(), "404") {
		t.Errorf("expected 404 in error, got %q", err.Error())
	}
}

func TestGatewayUnreachable(t *testing.T) {
	// Use a port that's very unlikely to have anything listening.
	err := run([]string{
		"--gateway-addr", "http://127.0.0.1:1",
		"--vin", "DEMO0000000000001",
		"--pin", "123456",
		"pair",
	}, io.Discard, io.Discard)
	if err == nil {
		t.Fatal("expected error when gateway is unreachable")
	}
	if !strings.Contains(err.Error(), "is the gateway running?") {
		t.Errorf("expected gateway unreachable message, got %q", err.Error())
	}
}

func TestGatewayUnreachableLock(t *testing.T) {
	err := run([]string{
		"--gateway-addr", "http://127.0.0.1:1",
		"--vin", "DEMO0000000000001",
		"--token", "some-token",
		"lock",
	}, io.Discard, io.Discard)
	if err == nil {
		t.Fatal("expected error when gateway is unreachable")
	}
	if !strings.Contains(err.Error(), "is the gateway running?") {
		t.Errorf("expected gateway unreachable message, got %q", err.Error())
	}
}

func TestServiceUnavailable(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(errorResponse{
			Error: "MQTT broker unreachable",
			Code:  "SERVICE_UNAVAILABLE",
		})
	}))
	defer server.Close()

	err := run([]string{
		"--gateway-addr", server.URL,
		"--vin", "DEMO0000000000001",
		"--token", "some-token",
		"lock",
	}, io.Discard, io.Discard)
	if err == nil {
		t.Fatal("expected error for 503 response")
	}
	if !strings.Contains(err.Error(), "503") {
		t.Errorf("expected 503 in error, got %q", err.Error())
	}
	if !strings.Contains(err.Error(), "MQTT broker unreachable") {
		t.Errorf("expected 'MQTT broker unreachable' in error, got %q", err.Error())
	}
}

// ── handleErrorResponse tests ───────────────────────────────────────────────

func TestHandleErrorResponseWithValidJSON(t *testing.T) {
	body, _ := json.Marshal(errorResponse{
		Error: "vehicle not found",
		Code:  "NOT_FOUND",
	})

	err := handleErrorResponse(404, body)
	if err == nil {
		t.Fatal("expected error")
	}
	if err.Error() != "HTTP 404: vehicle not found" {
		t.Errorf("expected 'HTTP 404: vehicle not found', got %q", err.Error())
	}
}

func TestHandleErrorResponseWithInvalidJSON(t *testing.T) {
	err := handleErrorResponse(500, []byte("not json"))
	if err == nil {
		t.Fatal("expected error")
	}
	if err.Error() != "HTTP 500: Internal Server Error" {
		t.Errorf("expected 'HTTP 500: Internal Server Error', got %q", err.Error())
	}
}

func TestHandleErrorResponseWithEmptyErrorField(t *testing.T) {
	body, _ := json.Marshal(errorResponse{
		Error: "",
		Code:  "UNKNOWN",
	})

	err := handleErrorResponse(400, body)
	if err == nil {
		t.Fatal("expected error")
	}
	// Falls back to generic message because Error field is empty.
	if err.Error() != "HTTP 400: Bad Request" {
		t.Errorf("expected 'HTTP 400: Bad Request', got %q", err.Error())
	}
}

// ── envOrDefault tests ──────────────────────────────────────────────────────

func TestEnvOrDefault(t *testing.T) {
	t.Setenv("TEST_COMPANION_VAR", "custom")
	if v := envOrDefault("TEST_COMPANION_VAR", "default"); v != "custom" {
		t.Errorf("expected 'custom', got %q", v)
	}

	os.Unsetenv("TEST_COMPANION_VAR_UNSET")
	if v := envOrDefault("TEST_COMPANION_VAR_UNSET", "fallback"); v != "fallback" {
		t.Errorf("expected 'fallback', got %q", v)
	}
}

// ── Subcommand recognition tests ────────────────────────────────────────────

func TestAllSubcommandsRecognized(t *testing.T) {
	// Test that all subcommands are dispatched (they will reach the HTTP
	// layer and fail due to unreachable server, but should not be "unknown").
	subcommands := []string{"lock", "unlock", "status"}

	for _, cmd := range subcommands {
		t.Run(cmd, func(t *testing.T) {
			os.Unsetenv("VIN")
			os.Unsetenv("GATEWAY_ADDR")
			os.Unsetenv("TOKEN")
			os.Unsetenv("PIN")

			err := run([]string{
				"--gateway-addr", "http://127.0.0.1:1",
				"--vin", "TESTVIN123456789",
				"--token", "test-token",
				cmd,
			}, io.Discard, io.Discard)
			if err == nil {
				return
			}
			if strings.Contains(err.Error(), "unknown command") {
				t.Errorf("command %q was not recognized", cmd)
			}
		})
	}
}

func TestPairSubcommandRecognized(t *testing.T) {
	os.Unsetenv("VIN")
	os.Unsetenv("GATEWAY_ADDR")
	os.Unsetenv("TOKEN")
	os.Unsetenv("PIN")

	err := run([]string{
		"--gateway-addr", "http://127.0.0.1:1",
		"--vin", "TESTVIN123456789",
		"--pin", "123456",
		"pair",
	}, io.Discard, io.Discard)
	if err == nil {
		return
	}
	if strings.Contains(err.Error(), "unknown command") {
		t.Errorf("pair command was not recognized")
	}
}

// ── Flag override precedence tests ──────────────────────────────────────────

func TestFlagOverridesEnv(t *testing.T) {
	t.Setenv("GATEWAY_ADDR", "http://envhost:8081")
	t.Setenv("VIN", "ENVVIN")
	t.Setenv("TOKEN", "env-token")
	t.Setenv("PIN", "111111")

	cfg, _, err := parseGlobalFlags([]string{
		"--gateway-addr", "http://flaghost:9090",
		"--vin", "FLAGVIN",
		"--token", "flag-token",
		"--pin", "222222",
		"pair",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if cfg.gatewayAddr != "http://flaghost:9090" {
		t.Errorf("flag should override env for gateway-addr, got %q", cfg.gatewayAddr)
	}
	if cfg.vin != "FLAGVIN" {
		t.Errorf("flag should override env for vin, got %q", cfg.vin)
	}
	if cfg.token != "flag-token" {
		t.Errorf("flag should override env for token, got %q", cfg.token)
	}
	if cfg.pin != "222222" {
		t.Errorf("flag should override env for pin, got %q", cfg.pin)
	}
}

// ── Full flow integration test with httptest ────────────────────────────────

func TestFullPairAndLockFlow(t *testing.T) {
	// Simulate a full pairing followed by lock command flow.
	mux := http.NewServeMux()
	mux.HandleFunc("/api/v1/pair", func(w http.ResponseWriter, r *http.Request) {
		var req pairRequest
		json.NewDecoder(r.Body).Decode(&req)
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(pairResponse{
			Token: "paired-token-xyz",
			VIN:   req.VIN,
		})
	})
	mux.HandleFunc("/api/v1/vehicles/DEMO0000000000001/lock", func(w http.ResponseWriter, r *http.Request) {
		auth := r.Header.Get("Authorization")
		if auth != "Bearer paired-token-xyz" {
			w.WriteHeader(http.StatusUnauthorized)
			json.NewEncoder(w).Encode(errorResponse{Error: "unauthorized", Code: "UNAUTHORIZED"})
			return
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusAccepted)
		json.NewEncoder(w).Encode(map[string]string{
			"command_id": "flow-cmd-1",
			"status":     "accepted",
		})
	})

	server := httptest.NewServer(mux)
	defer server.Close()

	// Step 1: Pair.
	var pairOut bytes.Buffer
	err := run([]string{
		"--gateway-addr", server.URL,
		"--vin", "DEMO0000000000001",
		"--pin", "123456",
		"pair",
	}, &pairOut, io.Discard)
	if err != nil {
		t.Fatalf("pair failed: %v", err)
	}
	if !strings.Contains(pairOut.String(), "paired-token-xyz") {
		t.Fatalf("expected token in pair output, got %q", pairOut.String())
	}

	// Step 2: Lock with the obtained token.
	var lockOut bytes.Buffer
	err = run([]string{
		"--gateway-addr", server.URL,
		"--vin", "DEMO0000000000001",
		"--token", "paired-token-xyz",
		"lock",
	}, &lockOut, io.Discard)
	if err != nil {
		t.Fatalf("lock failed: %v", err)
	}
	if !strings.Contains(lockOut.String(), "Lock command accepted") {
		t.Errorf("expected 'Lock command accepted', got %q", lockOut.String())
	}
}

// ── Non-JSON error body test ────────────────────────────────────────────────

func TestNonJSONErrorBody(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusBadGateway)
		w.Write([]byte("Bad Gateway"))
	}))
	defer server.Close()

	err := run([]string{
		"--gateway-addr", server.URL,
		"--vin", "DEMO0000000000001",
		"--token", "some-token",
		"lock",
	}, io.Discard, io.Discard)
	if err == nil {
		t.Fatal("expected error for 502 response")
	}
	if !strings.Contains(err.Error(), "502") {
		t.Errorf("expected 502 in error, got %q", err.Error())
	}
}
