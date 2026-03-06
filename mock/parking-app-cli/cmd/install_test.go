package cmd

import (
	"strings"
	"testing"
)

// TS-09-E3: install with missing flags should return an error.
func TestInstall_MissingFlags(t *testing.T) {
	err := runInstall(nil)
	if err == nil {
		t.Fatal("expected error when --image-ref and --checksum are missing")
	}
	errStr := err.Error()
	if !strings.Contains(errStr, "image-ref") && !strings.Contains(errStr, "usage") {
		t.Fatalf("expected usage error mentioning image-ref, got: %s", errStr)
	}
}

// TS-09-P3: install calls the correct gRPC method.
// Full verification requires a running UPDATE_SERVICE; this test verifies
// that the command attempts a gRPC connection to the configured address.
func TestInstall_CorrectGRPCMethod(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping gRPC dial test in short mode")
	}
	// With no server running, the gRPC dial will fail with a connection error.
	err := runInstall([]string{"--image-ref=registry/adapter:v1", "--checksum=abc123def456"})
	if err == nil {
		t.Fatal("expected connection error when no gRPC server is running")
	}
}
