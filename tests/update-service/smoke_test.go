package updateservice_test

import (
	"context"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/gen/update"
)

// TestSmokeListAdaptersEmpty verifies that ListAdapters returns an empty
// list when no adapters have been installed.
//
// Requirements: 07-REQ-4.E2
// Test Spec: TS-07-SMOKE-1 (partial)
func TestSmokeListAdaptersEmpty(t *testing.T) {
	bin := buildUpdateService(t)
	svc := startUpdateService(t, bin)

	client := newUpdateServiceClient(t, svc.port)

	ctx, cancel := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel()

	resp, err := client.ListAdapters(ctx, &update.ListAdaptersRequest{})
	if err != nil {
		t.Fatalf("ListAdapters failed: %v", err)
	}

	if len(resp.Adapters) != 0 {
		t.Errorf("expected 0 adapters, got %d", len(resp.Adapters))
	}
}

// TestSmokeGetAdapterStatusNotFound verifies that GetAdapterStatus returns
// NOT_FOUND for an unknown adapter.
//
// Requirements: 07-REQ-4.E1
// Test Spec: TS-07-SMOKE-1 (partial)
func TestSmokeGetAdapterStatusNotFound(t *testing.T) {
	bin := buildUpdateService(t)
	svc := startUpdateService(t, bin)

	client := newUpdateServiceClient(t, svc.port)

	ctx, cancel := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel()

	_, err := client.GetAdapterStatus(ctx, &update.GetAdapterStatusRequest{
		AdapterId: "nonexistent-adapter",
	})
	if err == nil {
		t.Fatal("expected error for unknown adapter, got nil")
	}
}

// TestSmokeInstallAdapterValidation verifies that InstallAdapter validates
// input parameters and returns INVALID_ARGUMENT for empty fields.
//
// Requirements: 07-REQ-1.E1, 07-REQ-1.E2
// Test Spec: TS-07-SMOKE-1 (partial)
func TestSmokeInstallAdapterValidation(t *testing.T) {
	bin := buildUpdateService(t)
	svc := startUpdateService(t, bin)

	client := newUpdateServiceClient(t, svc.port)

	t.Run("empty_image_ref", func(t *testing.T) {
		ctx, cancel := context.WithTimeout(context.Background(), rpcTimeout)
		defer cancel()

		_, err := client.InstallAdapter(ctx, &update.InstallAdapterRequest{
			ImageRef:       "",
			ChecksumSha256: "sha256:abc123",
		})
		if err == nil {
			t.Fatal("expected error for empty image_ref, got nil")
		}
	})

	t.Run("empty_checksum", func(t *testing.T) {
		ctx, cancel := context.WithTimeout(context.Background(), rpcTimeout)
		defer cancel()

		_, err := client.InstallAdapter(ctx, &update.InstallAdapterRequest{
			ImageRef:       "example.com/adapter:v1",
			ChecksumSha256: "",
		})
		if err == nil {
			t.Fatal("expected error for empty checksum, got nil")
		}
	})
}

// TestSmokeRemoveAdapterNotFound verifies that RemoveAdapter returns
// NOT_FOUND for an unknown adapter.
//
// Requirements: 07-REQ-5.E1
// Test Spec: TS-07-SMOKE-1 (partial)
func TestSmokeRemoveAdapterNotFound(t *testing.T) {
	bin := buildUpdateService(t)
	svc := startUpdateService(t, bin)

	client := newUpdateServiceClient(t, svc.port)

	ctx, cancel := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel()

	_, err := client.RemoveAdapter(ctx, &update.RemoveAdapterRequest{
		AdapterId: "nonexistent-adapter",
	})
	if err == nil {
		t.Fatal("expected error for unknown adapter, got nil")
	}
}

// TestSmokeWatchAdapterStates verifies that WatchAdapterStates opens a
// server-streaming connection successfully.
//
// Requirements: 07-REQ-3.1
// Test Spec: TS-07-SMOKE-2 (partial)
func TestSmokeWatchAdapterStates(t *testing.T) {
	bin := buildUpdateService(t)
	svc := startUpdateService(t, bin)

	client := newUpdateServiceClient(t, svc.port)

	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	stream, err := client.WatchAdapterStates(ctx, &update.WatchAdapterStatesRequest{})
	if err != nil {
		t.Fatalf("WatchAdapterStates failed: %v", err)
	}

	// The stream should be open; it will block waiting for events.
	// We just verify it was created without error. When the context
	// expires, the stream will be cancelled.
	_ = stream
}
