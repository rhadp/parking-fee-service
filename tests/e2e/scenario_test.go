package e2e

import (
	"fmt"
	"testing"
	"time"
)

// TestHappyPathScenario exercises the full parking demo happy path across all
// services, mirroring the three message flows from the PRD:
//
//   Flow 3: Operator discovery and adapter lookup
//   Flow 1: Parking session start (lock event → autonomous session start)
//   Flow 2: Remote unlock via companion app → autonomous session stop
//
// The test uses the mock companion-app and parking-app flows via HTTP/NATS,
// and verifies the parking session on the mock-parking-operator.
func TestHappyPathScenario(t *testing.T) {
	ensureFullStack(t)

	// ── Flow 3: Operator Discovery & Adapter Lookup ─────────────────────────
	//
	// The PARKING_APP queries the PARKING_FEE_SERVICE for nearby operators,
	// picks one, and fetches adapter metadata. In this E2E setup the adaptor
	// is already running, so we skip the UPDATE_SERVICE install step.

	t.Log("=== Flow 3: Operator Discovery ===")

	listURL := parkingFeeServiceURL + "/operators?lat=48.1375&lon=11.5600"
	listResp := httpGet(t, listURL)
	if listResp.StatusCode != 200 {
		t.Fatalf("operator lookup: %d", listResp.StatusCode)
	}
	operators := decodeJSONArray(t, listResp)
	if len(operators) == 0 {
		t.Fatal("expected at least one operator for Munich coordinates")
	}

	operatorID := operators[0]["id"].(string)
	t.Logf("discovered operator: %s", operatorID)

	adapterURL := fmt.Sprintf("%s/operators/%s/adapter", parkingFeeServiceURL, operatorID)
	adapterResp := httpGet(t, adapterURL)
	if adapterResp.StatusCode != 200 {
		t.Fatalf("adapter metadata lookup: %d", adapterResp.StatusCode)
	}
	adapter := decodeJSON(t, adapterResp)
	t.Logf("adapter: image_ref=%s version=%s", adapter["image_ref"], adapter["version"])

	// ── Flow 2 (lock path): Lock via COMPANION_APP ──────────────────────────
	//
	// The companion app sends a lock command via the CLOUD_GATEWAY REST API.
	// The full chain: COMPANION_APP → CLOUD_GATEWAY → NATS →
	//   CLOUD_GATEWAY_CLIENT → DATA_BROKER → LOCKING_SERVICE →
	//   DATA_BROKER (IsLocked=true).
	//
	// This triggers Flow 1: PARKING_OPERATOR_ADAPTOR detects the lock event
	// via DATA_BROKER and autonomously starts a parking session with the
	// PARKING_OPERATOR.

	t.Log("=== Flow 1+2: Lock → Parking Session Start ===")

	// Verify no sessions exist yet on the mock parking operator.
	sessions := listParkingSessions(t)
	activeBefore := countSessionsByStatus(sessions, "active")
	t.Logf("active sessions before lock: %d", activeBefore)

	// Send lock command via companion app → cloud-gateway.
	lockBody := `{"command_id":"e2e-scenario-lock","type":"lock","doors":["driver"]}`
	lockResp := httpPostJSON(t, commandURL(testVIN), testToken, lockBody)
	if lockResp.StatusCode != 202 {
		body := decodeJSON(t, lockResp)
		t.Fatalf("lock command: expected 202, got %d: %v", lockResp.StatusCode, body)
	}
	lockResp.Body.Close()
	t.Log("lock command accepted by cloud-gateway")

	// Wait for the lock command to be processed end-to-end.
	waitForCommandStatus(t, testVIN, "e2e-scenario-lock", "success", 15*time.Second)
	t.Log("lock command completed successfully")

	// Wait for the parking-operator-adaptor to autonomously start a session
	// on the mock-parking-operator. This involves the chain:
	//   DATA_BROKER (IsLocked=true) → PARKING_OPERATOR_ADAPTOR →
	//   POST /parking/start → MOCK_PARKING_OPERATOR
	waitForActiveSessions(t, activeBefore+1, 15*time.Second)
	t.Log("parking session started autonomously after lock")

	// ── Flow 2 (unlock path): Unlock via COMPANION_APP ──────────────────────
	//
	// The companion app sends an unlock command. This triggers the adaptor to
	// autonomously stop the parking session.

	t.Log("=== Flow 2: Unlock → Parking Session Stop ===")

	unlockBody := `{"command_id":"e2e-scenario-unlock","type":"unlock","doors":["driver"]}`
	unlockResp := httpPostJSON(t, commandURL(testVIN), testToken, unlockBody)
	if unlockResp.StatusCode != 202 {
		body := decodeJSON(t, unlockResp)
		t.Fatalf("unlock command: expected 202, got %d: %v", unlockResp.StatusCode, body)
	}
	unlockResp.Body.Close()
	t.Log("unlock command accepted by cloud-gateway")

	// Wait for the unlock command to be processed.
	waitForCommandStatus(t, testVIN, "e2e-scenario-unlock", "success", 15*time.Second)
	t.Log("unlock command completed successfully")

	// Wait for the parking session to be stopped.
	waitForStoppedSession(t, 15*time.Second)
	t.Log("parking session stopped autonomously after unlock")

	t.Log("=== Happy path scenario completed ===")
}

// ── Scenario helpers ────────────────────────────────────────────────────────

// listParkingSessions queries the mock-parking-operator for all sessions.
func listParkingSessions(t *testing.T) []map[string]interface{} {
	t.Helper()
	resp := httpGet(t, mockParkingOperatorURL+"/parking/sessions")
	if resp.StatusCode != 200 {
		t.Fatalf("list sessions: %d", resp.StatusCode)
	}
	return decodeJSONArray(t, resp)
}

// countSessionsByStatus counts sessions with the given status.
func countSessionsByStatus(sessions []map[string]interface{}, status string) int {
	count := 0
	for _, s := range sessions {
		if s["status"] == status {
			count++
		}
	}
	return count
}

// waitForCommandStatus polls the cloud-gateway command status endpoint until
// the command reaches the expected status or the timeout elapses.
func waitForCommandStatus(t *testing.T, vin, commandID, expected string, timeout time.Duration) {
	t.Helper()
	statusURL := commandStatusURL(vin, commandID)
	deadline := time.Now().Add(timeout)
	var lastStatus string
	for time.Now().Before(deadline) {
		resp := httpGetAuth(t, statusURL, testToken)
		body := decodeJSON(t, resp)
		if s, ok := body["status"].(string); ok {
			lastStatus = s
			if s == expected {
				return
			}
		}
		time.Sleep(500 * time.Millisecond)
	}
	t.Fatalf("command %s: expected status %q, last seen %q (timed out after %s)",
		commandID, expected, lastStatus, timeout)
}

// waitForActiveSessions polls the mock-parking-operator until the number of
// active sessions reaches the expected count.
func waitForActiveSessions(t *testing.T, expected int, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	var lastCount int
	for time.Now().Before(deadline) {
		sessions := listParkingSessions(t)
		lastCount = countSessionsByStatus(sessions, "active")
		if lastCount >= expected {
			return
		}
		time.Sleep(500 * time.Millisecond)
	}
	t.Fatalf("expected %d active sessions, got %d (timed out after %s)",
		expected, lastCount, timeout)
}

// waitForStoppedSession polls the mock-parking-operator until at least one
// stopped session is found.
func waitForStoppedSession(t *testing.T, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		sessions := listParkingSessions(t)
		if countSessionsByStatus(sessions, "stopped") > 0 {
			return
		}
		time.Sleep(500 * time.Millisecond)
	}
	t.Fatal("no stopped sessions found (timed out)")
}
