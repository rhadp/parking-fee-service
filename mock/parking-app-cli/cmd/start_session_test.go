package cmd

import (
	"strings"
	"testing"
)

// TS-09-E3: start-session with missing --zone-id produces error.
func TestStartSession_MissingFlags(t *testing.T) {
	err := RunStartSession([]string{}, "localhost:50052")
	if err == nil {
		t.Fatal("expected error when --zone-id is missing")
	}
	errMsg := err.Error()
	if !strings.Contains(errMsg, "zone") && !strings.Contains(errMsg, "required") {
		t.Errorf("error should mention missing flag, got: %v", err)
	}
}
