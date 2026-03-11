package cmd

import (
	"strings"
	"testing"
)

// TS-09-E3: install with missing --image-ref and --checksum produces error.
func TestInstall_MissingFlags(t *testing.T) {
	err := RunInstall([]string{}, "localhost:50051")
	if err == nil {
		t.Fatal("expected error when --image-ref and --checksum are missing")
	}
	errMsg := err.Error()
	if !strings.Contains(errMsg, "image") && !strings.Contains(errMsg, "required") {
		t.Errorf("error should mention missing flag, got: %v", err)
	}
}
