package main

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

// TestUnlockCommand_SendsPOSTWithUnlockType verifies the unlock command sends
// a POST request with type "unlock".
// Requirement: 03-REQ-4.2
func TestUnlockCommand_SendsPOSTWithUnlockType(t *testing.T) {
	var receivedBody map[string]interface{}

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		var body map[string]interface{}
		if err := json.NewDecoder(r.Body).Decode(&body); err == nil {
			receivedBody = body
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"command_id":"test","status":"success"}`))
	}))
	defer server.Close()

	gatewayURL = server.URL
	vin = "VIN12345"
	token = "demo-token"

	err := sendCommand("unlock")
	if err != nil {
		t.Fatalf("sendCommand returned error: %v", err)
	}

	if receivedBody == nil {
		t.Fatal("no request body received")
	}
	if receivedBody["type"] != "unlock" {
		t.Errorf("expected type 'unlock', got %v", receivedBody["type"])
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

// TestUnlockCommand_ServerError verifies the unlock command returns an error on
// non-2xx HTTP responses.
// Requirement: 03-REQ-4.7
func TestUnlockCommand_ServerError(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		w.Write([]byte(`{"error":"bad request"}`))
	}))
	defer server.Close()

	gatewayURL = server.URL
	vin = "VIN12345"
	token = "demo-token"

	err := sendCommand("unlock")
	if err == nil {
		t.Error("expected error on 400 response, got nil")
	}
}
