// Integration tests for UPDATE_SERVICE (spec 07_update_service).
//
// Tests cover:
//   - TS-07-17: Startup logging (port number and "ready" message appear in logs)
//   - TS-07-18: Graceful shutdown (SIGTERM causes exit code 0)
//   - TS-07-SMOKE-1: ListAdapters returns empty list via gRPC
//   - TS-07-SMOKE-2: GetAdapterStatus with unknown ID returns NOT_FOUND
//   - TS-07-SMOKE-3: RemoveAdapter with unknown ID returns NOT_FOUND
//
// All tests skip gracefully when cargo or grpcurl is absent.
package updateservice

import (
	"fmt"
	"strings"
	"testing"
	"time"
)

// ── TS-07-17: Startup Logging ─────────────────────────────────────────────────

// TestStartupLogging verifies that the update-service logs its gRPC port and a
// "ready" message on startup (07-REQ-10.1).
//
// Requirements: 07-REQ-10.1
// Test Spec: TS-07-17
func TestStartupLogging(t *testing.T) {
	usp := startUpdateService(t)

	// Allow up to 2 seconds for log lines to propagate after port is open.
	deadline := time.Now().Add(2 * time.Second)
	for time.Now().Before(deadline) {
		logs := usp.logs()
		portStr := fmt.Sprintf("%d", testPort)
		if strings.Contains(logs, portStr) && strings.Contains(strings.ToLower(logs), "ready") {
			return // pass
		}
		time.Sleep(100 * time.Millisecond)
	}

	logs := usp.logs()
	portStr := fmt.Sprintf("%d", testPort)
	if !strings.Contains(logs, portStr) {
		t.Errorf("expected port %d in startup logs; logs:\n%s", testPort, logs)
	}
	if !strings.Contains(strings.ToLower(logs), "ready") {
		t.Errorf(`expected "ready" in startup logs; logs:\n%s`, logs)
	}
}

// ── TS-07-18: Graceful Shutdown ───────────────────────────────────────────────

// TestGracefulShutdown verifies that the update-service exits with code 0 when
// it receives SIGTERM (07-REQ-10.2, 07-REQ-10.E1).
//
// Requirements: 07-REQ-10.2, 07-REQ-10.E1
// Test Spec: TS-07-18
func TestGracefulShutdown(t *testing.T) {
	usp := startUpdateService(t)

	usp.sendSIGTERM()

	code, ok := usp.waitForExit(15 * time.Second)
	if !ok {
		usp.kill()
		t.Fatalf("update-service did not exit within 15s after SIGTERM; logs:\n%s", usp.logs())
	}
	if code != 0 {
		t.Errorf("expected exit code 0 after SIGTERM, got %d; logs:\n%s", code, usp.logs())
	}
}

// ── TS-07-SMOKE-1: ListAdapters Returns Empty ─────────────────────────────────

// TestListAdaptersGRPC verifies that ListAdapters returns an empty list via
// gRPC when no adapters have been installed (07-REQ-4.1, 07-REQ-4.E2).
//
// Requirements: 07-REQ-4.1, 07-REQ-4.E2
// Test Spec: TS-07-SMOKE-1 (partial)
func TestListAdaptersGRPC(t *testing.T) {
	requireGrpcurl(t)
	startUpdateService(t)

	out, err := grpcCall(t, "ListAdapters", "")
	if err != nil {
		// A non-zero exit from grpcurl could indicate a legitimate RPC error;
		// print the output for diagnosis.
		t.Fatalf("ListAdapters grpcurl call failed: %v\nOutput: %s", err, out)
	}

	// The response should contain an empty adapters list or an empty JSON object.
	// grpcurl omits fields with zero/default values, so an empty list renders as
	// "{}" or "{\n}\n". Verify there are no adapter entries.
	if strings.Contains(out, `"adapterId"`) || strings.Contains(out, `"adapter_id"`) {
		t.Errorf("expected empty adapters list, but found adapter entries; output: %s", out)
	}
}

// ── TS-07-SMOKE-2: GetAdapterStatus Returns NOT_FOUND ────────────────────────

// TestGetAdapterStatusNotFound verifies that GetAdapterStatus with an unknown
// adapter ID returns gRPC status NOT_FOUND (07-REQ-4.E1).
//
// Requirements: 07-REQ-4.E1
// Test Spec: TS-07-SMOKE-2 (partial), TS-07-E8
func TestGetAdapterStatusNotFound(t *testing.T) {
	requireGrpcurl(t)
	startUpdateService(t)

	out, err := grpcCall(t, "GetAdapterStatus", `{"adapter_id":"nonexistent-adapter"}`)
	// grpcurl exits non-zero for gRPC error status codes.
	if err == nil {
		t.Fatalf("expected grpcurl to report an error for NOT_FOUND, but it succeeded; output: %s", out)
	}

	if !strings.Contains(out, "NotFound") && !strings.Contains(out, "NOT_FOUND") {
		t.Errorf("expected NOT_FOUND status code; output: %s", out)
	}
	if !strings.Contains(out, "adapter not found") {
		t.Errorf(`expected "adapter not found" message; output: %s`, out)
	}
}

// ── TS-07-SMOKE-3: RemoveAdapter Returns NOT_FOUND ───────────────────────────

// TestRemoveAdapterNotFound verifies that RemoveAdapter with an unknown adapter
// ID returns gRPC status NOT_FOUND (07-REQ-5.E1).
//
// Requirements: 07-REQ-5.E1
// Test Spec: TS-07-E10
func TestRemoveAdapterNotFound(t *testing.T) {
	requireGrpcurl(t)
	startUpdateService(t)

	out, err := grpcCall(t, "RemoveAdapter", `{"adapter_id":"nonexistent-adapter"}`)
	if err == nil {
		t.Fatalf("expected grpcurl to report an error for NOT_FOUND, but it succeeded; output: %s", out)
	}

	if !strings.Contains(out, "NotFound") && !strings.Contains(out, "NOT_FOUND") {
		t.Errorf("expected NOT_FOUND status code; output: %s", out)
	}
	if !strings.Contains(out, "adapter not found") {
		t.Errorf(`expected "adapter not found" message; output: %s`, out)
	}
}
