package main

import (
	"testing"
)

// TestValidateToken_Empty verifies that an empty token returns an error.
// Requirement: 03-REQ-4.E1
func TestValidateToken_Empty(t *testing.T) {
	token = ""
	err := validateToken()
	if err == nil {
		t.Error("expected error for empty token, got nil")
	}
}

// TestValidateToken_Valid verifies that a non-empty token passes validation.
func TestValidateToken_Valid(t *testing.T) {
	token = "demo-token"
	err := validateToken()
	if err != nil {
		t.Errorf("expected nil error for valid token, got: %v", err)
	}
}

// TestSendCommand_MissingToken verifies sendCommand returns error when token is empty.
// Requirement: 03-REQ-4.E1
func TestSendCommand_MissingToken(t *testing.T) {
	gatewayURL = "http://localhost:8081"
	vin = "VIN12345"
	token = ""

	err := sendCommand("lock")
	if err == nil {
		t.Error("expected error when token is empty")
	}
}

// TestSendCommand_ConnectionError verifies sendCommand returns error when
// the gateway is unreachable.
// Requirement: 03-REQ-4.E2
func TestSendCommand_ConnectionError(t *testing.T) {
	gatewayURL = "http://localhost:19999"
	vin = "VIN12345"
	token = "demo-token"

	err := sendCommand("lock")
	if err == nil {
		t.Error("expected connection error, got nil")
	}
}
