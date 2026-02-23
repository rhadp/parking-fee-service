package main

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

// TestLockCommand_SendsPOSTRequest verifies the lock command sends a POST request
// to /vehicles/{vin}/commands with the correct body and headers.
// Requirement: 03-REQ-4.1
func TestLockCommand_SendsPOSTRequest(t *testing.T) {
	var receivedMethod string
	var receivedPath string
	var receivedAuth string
	var receivedBody map[string]interface{}

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedMethod = r.Method
		receivedPath = r.URL.Path
		receivedAuth = r.Header.Get("Authorization")

		var body map[string]interface{}
		if err := json.NewDecoder(r.Body).Decode(&body); err == nil {
			receivedBody = body
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"command_id":"test","status":"success"}`))
	}))
	defer server.Close()

	// Set global flags
	gatewayURL = server.URL
	vin = "VIN12345"
	token = "demo-token"

	err := sendCommand("lock")
	if err != nil {
		t.Fatalf("sendCommand returned error: %v", err)
	}

	if receivedMethod != "POST" {
		t.Errorf("expected POST, got %s", receivedMethod)
	}
	if receivedPath != "/vehicles/VIN12345/commands" {
		t.Errorf("expected /vehicles/VIN12345/commands, got %s", receivedPath)
	}
	if receivedAuth != "Bearer demo-token" {
		t.Errorf("expected 'Bearer demo-token', got %q", receivedAuth)
	}
	if receivedBody == nil {
		t.Fatal("no request body received")
	}
	if receivedBody["type"] != "lock" {
		t.Errorf("expected type 'lock', got %v", receivedBody["type"])
	}

	doors, ok := receivedBody["doors"].([]interface{})
	if !ok || len(doors) == 0 {
		t.Error("expected doors array with at least one entry")
	} else if doors[0] != "driver" {
		t.Errorf("expected first door 'driver', got %v", doors[0])
	}

	cmdID, ok := receivedBody["command_id"].(string)
	if !ok || cmdID == "" {
		t.Error("expected non-empty command_id")
	}
}

// TestLockCommand_UniqueCommandIDs verifies each lock command generates a unique
// command_id (UUID).
func TestLockCommand_UniqueCommandIDs(t *testing.T) {
	var ids []string

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		var body map[string]interface{}
		if err := json.NewDecoder(r.Body).Decode(&body); err == nil {
			if id, ok := body["command_id"].(string); ok {
				ids = append(ids, id)
			}
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"command_id":"test","status":"success"}`))
	}))
	defer server.Close()

	gatewayURL = server.URL
	vin = "VIN12345"
	token = "demo-token"

	for i := 0; i < 5; i++ {
		if err := sendCommand("lock"); err != nil {
			t.Fatalf("sendCommand failed on iteration %d: %v", i, err)
		}
	}

	if len(ids) != 5 {
		t.Fatalf("expected 5 command IDs, got %d", len(ids))
	}

	seen := make(map[string]bool)
	for _, id := range ids {
		if seen[id] {
			t.Errorf("duplicate command_id: %s", id)
		}
		seen[id] = true
	}
}

// TestLockCommand_ServerError verifies the lock command returns an error on
// non-2xx HTTP responses.
// Requirement: 03-REQ-4.7
func TestLockCommand_ServerError(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusUnauthorized)
		w.Write([]byte(`{"error":"unauthorized"}`))
	}))
	defer server.Close()

	gatewayURL = server.URL
	vin = "VIN12345"
	token = "wrong-token"

	err := sendCommand("lock")
	if err == nil {
		t.Error("expected error on 401 response, got nil")
	}
}
