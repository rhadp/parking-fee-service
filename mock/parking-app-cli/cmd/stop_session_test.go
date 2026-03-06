package cmd

import "testing"

// TS-09-E3: stop-session with missing --session-id should return an error.
func TestStopSession_MissingFlags(t *testing.T) {
	err := runStopSession(nil)
	if err == nil {
		t.Fatal("expected error when --session-id is missing")
	}
}
