package api

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/state"
)

// mockPublisher is a test double for MQTTPublisher.
type mockPublisher struct {
	published []publishedCommand
	err       error
}

type publishedCommand struct {
	VIN       string
	CommandID string
	CmdType   string
}

func (m *mockPublisher) PublishCommand(vin, commandID, cmdType string) error {
	m.published = append(m.published, publishedCommand{vin, commandID, cmdType})
	return m.err
}

// newTestServer creates an httptest.Server with the full handler routing.
func newTestServer(t *testing.T, store *state.Store, pub MQTTPublisher) *httptest.Server {
	t.Helper()
	h := NewHandlers(store, pub)
	mux := http.NewServeMux()
	h.RegisterRoutes(mux)
	return httptest.NewServer(mux)
}

// --- Health Check ---

func TestHealthz(t *testing.T) {
	s := state.NewStore()
	srv := newTestServer(t, s, nil)
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/healthz")
	if err != nil {
		t.Fatalf("GET /healthz: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusOK)
	}
	if ct := resp.Header.Get("Content-Type"); ct != "application/json" {
		t.Errorf("Content-Type = %q, want %q", ct, "application/json")
	}
}

// --- Pairing ---

func TestPairSuccess(t *testing.T) {
	s := state.NewStore()
	s.RegisterVehicle("VIN1", "123456")

	srv := newTestServer(t, s, nil)
	defer srv.Close()

	body := `{"vin":"VIN1","pin":"123456"}`
	resp, err := http.Post(srv.URL+"/api/v1/pair", "application/json", strings.NewReader(body))
	if err != nil {
		t.Fatalf("POST /api/v1/pair: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusOK)
	}

	var pairResp PairResponse
	json.NewDecoder(resp.Body).Decode(&pairResp)

	if pairResp.Token == "" {
		t.Error("expected non-empty token")
	}
	if pairResp.VIN != "VIN1" {
		t.Errorf("VIN = %q, want %q", pairResp.VIN, "VIN1")
	}
}

func TestPairUnknownVIN(t *testing.T) {
	s := state.NewStore()
	srv := newTestServer(t, s, nil)
	defer srv.Close()

	body := `{"vin":"NONEXISTENT","pin":"123456"}`
	resp, err := http.Post(srv.URL+"/api/v1/pair", "application/json", strings.NewReader(body))
	if err != nil {
		t.Fatalf("POST /api/v1/pair: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusNotFound {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusNotFound)
	}
}

func TestPairWrongPIN(t *testing.T) {
	s := state.NewStore()
	s.RegisterVehicle("VIN1", "123456")

	srv := newTestServer(t, s, nil)
	defer srv.Close()

	body := `{"vin":"VIN1","pin":"999999"}`
	resp, err := http.Post(srv.URL+"/api/v1/pair", "application/json", strings.NewReader(body))
	if err != nil {
		t.Fatalf("POST /api/v1/pair: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusForbidden {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusForbidden)
	}
}

func TestPairInvalidBody(t *testing.T) {
	s := state.NewStore()
	srv := newTestServer(t, s, nil)
	defer srv.Close()

	resp, err := http.Post(srv.URL+"/api/v1/pair", "application/json", strings.NewReader("not json"))
	if err != nil {
		t.Fatalf("POST /api/v1/pair: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusBadRequest {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusBadRequest)
	}
}

func TestPairMissingFields(t *testing.T) {
	s := state.NewStore()
	srv := newTestServer(t, s, nil)
	defer srv.Close()

	body := `{"vin":"VIN1"}`
	resp, err := http.Post(srv.URL+"/api/v1/pair", "application/json", strings.NewReader(body))
	if err != nil {
		t.Fatalf("POST /api/v1/pair: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusBadRequest {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusBadRequest)
	}
}

// --- Lock Command ---

func TestLockAccepted(t *testing.T) {
	s := state.NewStore()
	s.RegisterVehicle("VIN1", "123456")
	token, _ := s.PairVehicle("VIN1", "123456")

	pub := &mockPublisher{}
	srv := newTestServer(t, s, pub)
	defer srv.Close()

	req, _ := http.NewRequest("POST", srv.URL+"/api/v1/vehicles/VIN1/lock", nil)
	req.Header.Set("Authorization", "Bearer "+token)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("POST lock: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusAccepted {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusAccepted)
	}

	var cmdResp CommandAcceptedResponse
	json.NewDecoder(resp.Body).Decode(&cmdResp)

	if cmdResp.CommandID == "" {
		t.Error("expected non-empty command_id")
	}
	if cmdResp.Status != "accepted" {
		t.Errorf("status = %q, want %q", cmdResp.Status, "accepted")
	}

	// Verify MQTT publish was called.
	if len(pub.published) != 1 {
		t.Fatalf("expected 1 MQTT publish, got %d", len(pub.published))
	}
	if pub.published[0].VIN != "VIN1" {
		t.Errorf("published VIN = %q, want %q", pub.published[0].VIN, "VIN1")
	}
	if pub.published[0].CmdType != "lock" {
		t.Errorf("published cmdType = %q, want %q", pub.published[0].CmdType, "lock")
	}
}

func TestLockWithoutAuth(t *testing.T) {
	s := state.NewStore()
	s.RegisterVehicle("VIN1", "123456")

	srv := newTestServer(t, s, nil)
	defer srv.Close()

	req, _ := http.NewRequest("POST", srv.URL+"/api/v1/vehicles/VIN1/lock", nil)
	// No auth header.

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("POST lock: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusUnauthorized {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusUnauthorized)
	}
}

func TestLockUnknownVIN(t *testing.T) {
	s := state.NewStore()
	s.RegisterVehicle("VIN1", "123456")
	token, _ := s.PairVehicle("VIN1", "123456")

	srv := newTestServer(t, s, nil)
	defer srv.Close()

	// Try to lock a VIN that exists but the token is for a different VIN.
	// First, since VIN2 is not registered, the auth middleware should block.
	req, _ := http.NewRequest("POST", srv.URL+"/api/v1/vehicles/VIN2/lock", nil)
	req.Header.Set("Authorization", "Bearer "+token)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("POST lock: %v", err)
	}
	defer resp.Body.Close()

	// Auth middleware rejects because token is for VIN1, not VIN2.
	if resp.StatusCode != http.StatusUnauthorized {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusUnauthorized)
	}
}

func TestLockMQTTFailure(t *testing.T) {
	s := state.NewStore()
	s.RegisterVehicle("VIN1", "123456")
	token, _ := s.PairVehicle("VIN1", "123456")

	pub := &mockPublisher{err: fmt.Errorf("broker unreachable")}
	srv := newTestServer(t, s, pub)
	defer srv.Close()

	req, _ := http.NewRequest("POST", srv.URL+"/api/v1/vehicles/VIN1/lock", nil)
	req.Header.Set("Authorization", "Bearer "+token)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("POST lock: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusServiceUnavailable {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusServiceUnavailable)
	}
}

// --- Unlock Command ---

func TestUnlockAccepted(t *testing.T) {
	s := state.NewStore()
	s.RegisterVehicle("VIN1", "123456")
	token, _ := s.PairVehicle("VIN1", "123456")

	pub := &mockPublisher{}
	srv := newTestServer(t, s, pub)
	defer srv.Close()

	req, _ := http.NewRequest("POST", srv.URL+"/api/v1/vehicles/VIN1/unlock", nil)
	req.Header.Set("Authorization", "Bearer "+token)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("POST unlock: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusAccepted {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusAccepted)
	}

	var cmdResp CommandAcceptedResponse
	json.NewDecoder(resp.Body).Decode(&cmdResp)

	if cmdResp.Status != "accepted" {
		t.Errorf("status = %q, want %q", cmdResp.Status, "accepted")
	}

	// Verify MQTT publish was called with unlock type.
	if len(pub.published) != 1 {
		t.Fatalf("expected 1 MQTT publish, got %d", len(pub.published))
	}
	if pub.published[0].CmdType != "unlock" {
		t.Errorf("published cmdType = %q, want %q", pub.published[0].CmdType, "unlock")
	}
}

// --- Status ---

func TestStatusSuccess(t *testing.T) {
	s := state.NewStore()
	s.RegisterVehicle("VIN1", "123456")
	token, _ := s.PairVehicle("VIN1", "123456")

	// Set some state.
	locked := true
	speed := 0.0
	s.UpdateState("VIN1", &locked, nil, &speed, nil, nil, nil)

	srv := newTestServer(t, s, nil)
	defer srv.Close()

	req, _ := http.NewRequest("GET", srv.URL+"/api/v1/vehicles/VIN1/status", nil)
	req.Header.Set("Authorization", "Bearer "+token)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("GET status: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusOK)
	}

	var statusResp VehicleStatusResponse
	json.NewDecoder(resp.Body).Decode(&statusResp)

	if statusResp.VIN != "VIN1" {
		t.Errorf("VIN = %q, want %q", statusResp.VIN, "VIN1")
	}
	if statusResp.IsLocked == nil || *statusResp.IsLocked != true {
		t.Errorf("IsLocked = %v, want true", statusResp.IsLocked)
	}
	if statusResp.Speed == nil || *statusResp.Speed != 0.0 {
		t.Errorf("Speed = %v, want 0.0", statusResp.Speed)
	}
}

func TestStatusWithLastCommand(t *testing.T) {
	s := state.NewStore()
	s.RegisterVehicle("VIN1", "123456")
	token, _ := s.PairVehicle("VIN1", "123456")

	s.AddCommand("VIN1", "cmd-1", "lock")
	s.UpdateCommandResult("VIN1", "cmd-1", "SUCCESS")

	srv := newTestServer(t, s, nil)
	defer srv.Close()

	req, _ := http.NewRequest("GET", srv.URL+"/api/v1/vehicles/VIN1/status", nil)
	req.Header.Set("Authorization", "Bearer "+token)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("GET status: %v", err)
	}
	defer resp.Body.Close()

	var statusResp VehicleStatusResponse
	json.NewDecoder(resp.Body).Decode(&statusResp)

	if statusResp.LastCommand == nil {
		t.Fatal("expected last_command in response")
	}
	if statusResp.LastCommand.CommandID != "cmd-1" {
		t.Errorf("last_command.command_id = %q, want %q", statusResp.LastCommand.CommandID, "cmd-1")
	}
	if statusResp.LastCommand.Result != "SUCCESS" {
		t.Errorf("last_command.result = %q, want %q", statusResp.LastCommand.Result, "SUCCESS")
	}
	if statusResp.LastCommand.Status != "success" {
		t.Errorf("last_command.status = %q, want %q", statusResp.LastCommand.Status, "success")
	}
}

func TestStatusWithoutAuth(t *testing.T) {
	s := state.NewStore()
	s.RegisterVehicle("VIN1", "123456")

	srv := newTestServer(t, s, nil)
	defer srv.Close()

	req, _ := http.NewRequest("GET", srv.URL+"/api/v1/vehicles/VIN1/status", nil)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("GET status: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusUnauthorized {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusUnauthorized)
	}
}

func TestStatusEmptyVehicle(t *testing.T) {
	s := state.NewStore()
	s.RegisterVehicle("VIN1", "123456")
	token, _ := s.PairVehicle("VIN1", "123456")

	srv := newTestServer(t, s, nil)
	defer srv.Close()

	req, _ := http.NewRequest("GET", srv.URL+"/api/v1/vehicles/VIN1/status", nil)
	req.Header.Set("Authorization", "Bearer "+token)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("GET status: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusOK)
	}

	var statusResp VehicleStatusResponse
	json.NewDecoder(resp.Body).Decode(&statusResp)

	if statusResp.VIN != "VIN1" {
		t.Errorf("VIN = %q, want %q", statusResp.VIN, "VIN1")
	}
	// All state fields should be nil for a freshly registered vehicle.
	if statusResp.IsLocked != nil {
		t.Errorf("IsLocked should be nil, got %v", *statusResp.IsLocked)
	}
	if statusResp.LastCommand != nil {
		t.Errorf("LastCommand should be nil for a vehicle with no commands")
	}
}

// --- Async Command Pattern ---

func TestLockDoesNotBlockOnMQTT(t *testing.T) {
	// Property 3: Async Command Pattern
	// Verify that lock/unlock returns 202 Accepted immediately without
	// waiting for the MQTT response round-trip.
	s := state.NewStore()
	s.RegisterVehicle("VIN1", "123456")
	token, _ := s.PairVehicle("VIN1", "123456")

	pub := &mockPublisher{}
	srv := newTestServer(t, s, pub)
	defer srv.Close()

	req, _ := http.NewRequest("POST", srv.URL+"/api/v1/vehicles/VIN1/lock", nil)
	req.Header.Set("Authorization", "Bearer "+token)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("POST lock: %v", err)
	}
	defer resp.Body.Close()

	// The response should be 202 Accepted, proving we didn't wait for
	// any MQTT round-trip.
	if resp.StatusCode != http.StatusAccepted {
		t.Errorf("status = %d, want %d (async pattern violated)", resp.StatusCode, http.StatusAccepted)
	}

	// The command should be in "accepted" state, not "success" — because
	// we haven't received a response from the vehicle yet.
	var cmdResp CommandAcceptedResponse
	json.NewDecoder(resp.Body).Decode(&cmdResp)

	if cmdResp.Status != "accepted" {
		t.Errorf("status = %q, want %q (async pattern violated)", cmdResp.Status, "accepted")
	}

	// The status endpoint should show the command as "accepted".
	statusReq, _ := http.NewRequest("GET", srv.URL+"/api/v1/vehicles/VIN1/status", nil)
	statusReq.Header.Set("Authorization", "Bearer "+token)

	statusResp, err := http.DefaultClient.Do(statusReq)
	if err != nil {
		t.Fatalf("GET status: %v", err)
	}
	defer statusResp.Body.Close()

	var status VehicleStatusResponse
	json.NewDecoder(statusResp.Body).Decode(&status)

	if status.LastCommand == nil {
		t.Fatal("expected last_command after lock")
	}
	if status.LastCommand.Status != "accepted" {
		t.Errorf("last_command.status = %q, want %q", status.LastCommand.Status, "accepted")
	}
}
