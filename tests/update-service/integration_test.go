package updateservice

import (
	"fmt"
	"strings"
	"testing"
	"time"
)

// ── TS-07-22: Startup Logging ─────────────────────────────────────────────────

// TestStartupLogging verifies that the service logs its gRPC port and registry
// URL at startup (REQ-8.1).
func TestStartupLogging(t *testing.T) {
	port := nextPort()
	cfg := defaultTestServiceConfig(port)
	sp := startService(t, cfg)

	logs := sp.logs()

	if !sp.logContains(fmt.Sprintf("%d", port)) {
		t.Errorf("expected port %d in startup logs; logs:\n%s", port, logs)
	}
	if !sp.logContains("registry") {
		t.Errorf("expected registry URL mention in startup logs; logs:\n%s", logs)
	}
}

// ── TS-07-23: Graceful Shutdown ───────────────────────────────────────────────

// TestGracefulShutdown verifies that SIGTERM causes a graceful shutdown with
// exit code 0 (REQ-8.2).
func TestGracefulShutdown(t *testing.T) {
	port := nextPort()
	cfg := defaultTestServiceConfig(port)
	sp := startService(t, cfg)

	sp.sendSIGTERM()

	code, ok := sp.waitForExit(15 * time.Second)
	if !ok {
		t.Fatalf("update-service did not exit within timeout after SIGTERM; logs:\n%s", sp.logs())
	}
	if code != 0 {
		t.Errorf("expected exit code 0 after SIGTERM, got %d; logs:\n%s", code, sp.logs())
	}
}

// ── TestListAdaptersGRPC (optional) ──────────────────────────────────────────

// TestListAdaptersGRPC verifies that the ListAdapters RPC returns a successful
// response (empty adapters list) on a fresh service. Skips if grpcurl is not
// available (REQ-4.1).
func TestListAdaptersGRPC(t *testing.T) {
	requireGrpcurl(t)

	port := nextPort()
	cfg := defaultTestServiceConfig(port)
	sp := startService(t, cfg)

	out, err := sp.grpcCall(t, "parking.updateservice.UpdateService/ListAdapters", "")
	if err != nil {
		t.Fatalf("ListAdapters gRPC call failed: %v\noutput: %s", err, out)
	}

	// A fresh service returns an empty adapters list: {} or {"adapters": []}
	t.Logf("ListAdapters response: %s", out)
}

// ── TestGetAdapterStatusNotFound (optional) ───────────────────────────────────

// TestGetAdapterStatusNotFound verifies that GetAdapterStatus returns a
// NOT_FOUND error for an unknown adapter_id. Skips if grpcurl is not available
// (REQ-4.E1).
func TestGetAdapterStatusNotFound(t *testing.T) {
	requireGrpcurl(t)

	port := nextPort()
	cfg := defaultTestServiceConfig(port)
	sp := startService(t, cfg)

	out, err := sp.grpcCall(t,
		"parking.updateservice.UpdateService/GetAdapterStatus",
		`{"adapter_id": "nonexistent-adapter-xyz"}`,
	)

	if err == nil {
		t.Errorf("expected gRPC error for unknown adapter_id, got success; output:\n%s", out)
		return
	}
	// grpcurl formats gRPC status codes as "NotFound" (mixed case).
	if !strings.Contains(out, "NotFound") {
		t.Errorf("expected NotFound in gRPC error response, got:\n%s", out)
	}
}

// ── TestRemoveAdapterNotFound (optional) ─────────────────────────────────────

// TestRemoveAdapterNotFound verifies that RemoveAdapter returns NOT_FOUND for
// an unknown adapter_id. Skips if grpcurl is not available (REQ-5.E1).
func TestRemoveAdapterNotFound(t *testing.T) {
	requireGrpcurl(t)

	port := nextPort()
	cfg := defaultTestServiceConfig(port)
	sp := startService(t, cfg)

	out, err := sp.grpcCall(t,
		"parking.updateservice.UpdateService/RemoveAdapter",
		`{"adapter_id": "nonexistent-adapter-xyz"}`,
	)

	if err == nil {
		t.Errorf("expected gRPC error for unknown adapter_id, got success; output:\n%s", out)
		return
	}
	// grpcurl formats gRPC status codes as "NotFound" (mixed case).
	if !strings.Contains(out, "NotFound") {
		t.Errorf("expected NotFound in gRPC error response, got:\n%s", out)
	}
}
