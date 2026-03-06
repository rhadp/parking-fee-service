package cmd

import "testing"

// TS-09-E3: status with missing --adapter-id should return an error.
func TestStatus_MissingFlags(t *testing.T) {
	err := runStatus(nil)
	if err == nil {
		t.Fatal("expected error when --adapter-id is missing")
	}
}
