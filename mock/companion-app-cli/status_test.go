package main

import (
	"net/http"
	"net/http/httptest"
	"testing"
)

// TestStatusCommand_SendsGETRequest verifies the status command sends a GET
// request to /vehicles/{vin}/status with the correct headers.
// Requirement: 03-REQ-4.3
func TestStatusCommand_SendsGETRequest(t *testing.T) {
	var receivedMethod string
	var receivedPath string
	var receivedAuth string

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedMethod = r.Method
		receivedPath = r.URL.Path
		receivedAuth = r.Header.Get("Authorization")
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"vin":"VIN12345","locked":true,"timestamp":1234}`))
	}))
	defer server.Close()

	gatewayURL = server.URL
	vin = "VIN12345"
	token = "demo-token"

	err := queryStatus()
	if err != nil {
		t.Fatalf("queryStatus returned error: %v", err)
	}

	if receivedMethod != "GET" {
		t.Errorf("expected GET, got %s", receivedMethod)
	}
	if receivedPath != "/vehicles/VIN12345/status" {
		t.Errorf("expected /vehicles/VIN12345/status, got %s", receivedPath)
	}
	if receivedAuth != "Bearer demo-token" {
		t.Errorf("expected 'Bearer demo-token', got %q", receivedAuth)
	}
}

// TestStatusCommand_VINInURL verifies the VIN flag is used in the URL path.
// Requirement: 03-REQ-4.5
func TestStatusCommand_VINInURL(t *testing.T) {
	var receivedPath string

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedPath = r.URL.Path
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"vin":"CUSTOM_VIN","locked":false,"timestamp":9999}`))
	}))
	defer server.Close()

	gatewayURL = server.URL
	vin = "CUSTOM_VIN"
	token = "demo-token"

	err := queryStatus()
	if err != nil {
		t.Fatalf("queryStatus returned error: %v", err)
	}

	if receivedPath != "/vehicles/CUSTOM_VIN/status" {
		t.Errorf("expected /vehicles/CUSTOM_VIN/status, got %s", receivedPath)
	}
}

// TestStatusCommand_ServerError verifies the status command returns an error on
// non-2xx HTTP responses.
// Requirement: 03-REQ-4.7
func TestStatusCommand_ServerError(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusNotFound)
		w.Write([]byte(`{"error":"no status available for vehicle"}`))
	}))
	defer server.Close()

	gatewayURL = server.URL
	vin = "UNKNOWN_VIN"
	token = "demo-token"

	err := queryStatus()
	if err == nil {
		t.Error("expected error on 404 response, got nil")
	}
}

// TestStatusCommand_MissingToken verifies the status command returns an error
// when no token is provided.
// Requirement: 03-REQ-4.E1
func TestStatusCommand_MissingToken(t *testing.T) {
	gatewayURL = "http://localhost:8081"
	vin = "VIN12345"
	token = ""

	err := queryStatus()
	if err == nil {
		t.Error("expected error when token is empty, got nil")
	}
}
