package cmd

import (
	"testing"
)

// TS-09-E3: install with missing flags should return an error.
func TestInstall_MissingFlags(t *testing.T) {
	err := runInstall(nil)
	if err == nil {
		t.Fatal("expected error when --image-ref and --checksum are missing")
	}
}

// TS-09-P3: install calls the correct gRPC method.
// This test will be fully implemented when gRPC mock server is available.
func TestInstall_CorrectGRPCMethod(t *testing.T) {
	// For now, verify the stub returns an error.
	err := runInstall([]string{"--image-ref=registry/adapter:v1", "--checksum=abc123def456"})
	if err == nil {
		t.Fatal("expected stub error until gRPC client is implemented")
	}
}
