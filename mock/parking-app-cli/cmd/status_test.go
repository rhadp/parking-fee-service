package cmd

import (
	"strings"
	"testing"
)

// TS-09-E3: status with missing --adapter-id produces error.
func TestStatus_MissingFlags(t *testing.T) {
	err := RunStatus([]string{}, "localhost:50051")
	if err == nil {
		t.Fatal("expected error when --adapter-id is missing")
	}
	errMsg := err.Error()
	if !strings.Contains(errMsg, "adapter") && !strings.Contains(errMsg, "required") {
		t.Errorf("error should mention missing flag, got: %v", err)
	}
}
