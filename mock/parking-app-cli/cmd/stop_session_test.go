package cmd

import (
	"strings"
	"testing"
)

// TS-09-E3: stop-session with missing --session-id produces error.
func TestStopSession_MissingFlags(t *testing.T) {
	err := RunStopSession([]string{}, "localhost:50052")
	if err == nil {
		t.Fatal("expected error when --session-id is missing")
	}
	errMsg := err.Error()
	if !strings.Contains(errMsg, "session") && !strings.Contains(errMsg, "required") {
		t.Errorf("error should mention missing flag, got: %v", err)
	}
}
