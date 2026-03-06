package cmd

import "testing"

// TS-09-E3: start-session with missing --zone-id should return an error.
func TestStartSession_MissingFlags(t *testing.T) {
	err := runStartSession(nil)
	if err == nil {
		t.Fatal("expected error when --zone-id is missing")
	}
}
