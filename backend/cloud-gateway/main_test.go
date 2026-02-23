package main

import (
	"encoding/json"
	"testing"
)

func TestParseJSON(t *testing.T) {
	var target struct {
		CommandID string `json:"command_id"`
		Status    string `json:"status"`
	}

	payload := []byte(`{"command_id":"abc","status":"success"}`)
	if err := parseJSON(payload, &target); err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if target.CommandID != "abc" {
		t.Errorf("expected command_id 'abc', got %q", target.CommandID)
	}
	if target.Status != "success" {
		t.Errorf("expected status 'success', got %q", target.Status)
	}
}

func TestParseJSON_Invalid(t *testing.T) {
	var target struct{}
	if err := parseJSON([]byte("not json"), &target); err == nil {
		t.Error("expected error for invalid JSON")
	}
}

// Ensure json import is used
var _ = json.Unmarshal
