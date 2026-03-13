package cmd

import "testing"

// TS-09-20: stop-session requires no arguments.
func TestStopSession_NoArgsRequired(t *testing.T) {
	// stop-session should not error for missing flags;
	// it only fails if it can't reach the gRPC server.
	// With an unreachable address, it should fail with a connection error,
	// not a "missing flag" error.
	err := RunStopSession([]string{}, "localhost:19999")
	if err == nil {
		// It's OK if it fails — the important thing is it doesn't fail
		// due to missing flags. With gRPC lazy connect, it might not
		// fail until the actual RPC call, which is expected.
		return
	}
	// Should not be a "missing flag" error
	if contains(err.Error(), "required flag") {
		t.Errorf("stop-session should not require any flags, got: %v", err)
	}
}

func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(s) > 0 && containsSubstring(s, substr))
}

func containsSubstring(s, substr string) bool {
	for i := 0; i+len(substr) <= len(s); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}
