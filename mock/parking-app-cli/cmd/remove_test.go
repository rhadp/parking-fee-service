package cmd

import "testing"

// TS-09-E3: remove with missing --adapter-id should return an error.
func TestRemove_MissingFlags(t *testing.T) {
	err := runRemove(nil)
	if err == nil {
		t.Fatal("expected error when --adapter-id is missing")
	}
}
