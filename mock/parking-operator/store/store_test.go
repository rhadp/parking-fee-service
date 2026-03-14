package store_test

import (
	"math"
	"testing"

	"github.com/rhadp/parking-fee-service/mock/parking-operator/store"
)

// newTestStore returns a fresh store pre-seeded with one active session.
func newTestStore(t *testing.T) (*store.Store, string) {
	t.Helper()
	s := store.NewStore()
	sid := "sess-001"
	s.Start(sid, store.StartRequest{
		VehicleID: "VIN001",
		ZoneID:    "zone-1",
		Timestamp: 1700000000,
	})
	return s, sid
}

// TS-09-5: PARKING_OPERATOR Start Session
// Requirement: 09-REQ-2.2
func TestStartSession(t *testing.T) {
	s := store.NewStore()
	resp := s.Start("sid-test", store.StartRequest{
		VehicleID: "VIN001",
		ZoneID:    "zone-1",
		Timestamp: 1700000000,
	})

	if resp.SessionID != "sid-test" {
		t.Errorf("SessionID = %q, want %q", resp.SessionID, "sid-test")
	}
	if resp.Status != "active" {
		t.Errorf("Status = %q, want %q", resp.Status, "active")
	}
	if resp.Rate.RateType != "per_hour" {
		t.Errorf("Rate.RateType = %q, want %q", resp.Rate.RateType, "per_hour")
	}
	if resp.Rate.Amount != 2.50 {
		t.Errorf("Rate.Amount = %v, want 2.50", resp.Rate.Amount)
	}
	if resp.Rate.Currency != "EUR" {
		t.Errorf("Rate.Currency = %q, want %q", resp.Rate.Currency, "EUR")
	}
}

// TS-09-6: PARKING_OPERATOR Stop Session — duration and total_amount
// Requirement: 09-REQ-2.3
func TestStopSession(t *testing.T) {
	s, sid := newTestStore(t)

	resp, err := s.Stop(store.StopRequest{
		SessionID: sid,
		Timestamp: 1700003600, // start + 3600 s
	})
	if err != nil {
		t.Fatalf("Stop returned unexpected error: %v", err)
	}

	if resp.Status != "stopped" {
		t.Errorf("Status = %q, want %q", resp.Status, "stopped")
	}
	if resp.DurationSeconds != 3600 {
		t.Errorf("DurationSeconds = %d, want 3600", resp.DurationSeconds)
	}
	// rate 2.50/hr × 1 hr = 2.50
	if math.Abs(resp.TotalAmount-2.50) > 0.001 {
		t.Errorf("TotalAmount = %v, want 2.50", resp.TotalAmount)
	}
	if resp.Currency != "EUR" {
		t.Errorf("Currency = %q, want %q", resp.Currency, "EUR")
	}
}

// TS-09-7: PARKING_OPERATOR Get Status — returns session info
// Requirement: 09-REQ-2.4
func TestGetStatus(t *testing.T) {
	s, sid := newTestStore(t)

	sess, err := s.GetStatus(sid)
	if err != nil {
		t.Fatalf("GetStatus returned unexpected error: %v", err)
	}
	if sess.SessionID != sid {
		t.Errorf("SessionID = %q, want %q", sess.SessionID, sid)
	}
	if sess.Status != "active" {
		t.Errorf("Status = %q, want %q", sess.Status, "active")
	}
}

// TS-09-E3: PARKING_OPERATOR Stop Unknown Session → error
// Requirement: 09-REQ-2.E1
func TestStopUnknownSession(t *testing.T) {
	s := store.NewStore()

	_, err := s.Stop(store.StopRequest{
		SessionID: "nonexistent",
		Timestamp: 1700000000,
	})
	if err == nil {
		t.Error("Stop with unknown session_id returned nil error, want error")
	}
}

// TS-09-E4: PARKING_OPERATOR Status Unknown Session → error
// Requirement: 09-REQ-2.E2
func TestStatusUnknownSession(t *testing.T) {
	s := store.NewStore()

	_, err := s.GetStatus("nonexistent")
	if err == nil {
		t.Error("GetStatus with unknown session_id returned nil error, want error")
	}
}

// TS-09-P2: Session Lifecycle Property
// Property 2 — for any start/stop sequence, duration and total_amount are correctly computed.
func TestPropertySessionLifecycle(t *testing.T) {
	cases := []struct {
		startTime int64
		duration  int64
	}{
		{1700000000, 1},
		{1700000000, 3600},
		{1700000000, 7200},
		{1700000000, 86400},
		{1600000000, 1800},
		{1700000000, 900},
	}

	for _, tc := range cases {
		s := store.NewStore()
		s.Start("prop-sess", store.StartRequest{
			VehicleID: "VIN-PROP",
			ZoneID:    "zone-prop",
			Timestamp: tc.startTime,
		})

		resp, err := s.Stop(store.StopRequest{
			SessionID: "prop-sess",
			Timestamp: tc.startTime + tc.duration,
		})
		if err != nil {
			t.Errorf("start=%d duration=%d: Stop error: %v", tc.startTime, tc.duration, err)
			continue
		}

		if resp.DurationSeconds != tc.duration {
			t.Errorf("start=%d duration=%d: DurationSeconds = %d, want %d",
				tc.startTime, tc.duration, resp.DurationSeconds, tc.duration)
		}

		expectedAmount := 2.50 * (float64(tc.duration) / 3600.0)
		if math.Abs(resp.TotalAmount-expectedAmount) > 0.01 {
			t.Errorf("start=%d duration=%d: TotalAmount = %v, want ~%v",
				tc.startTime, tc.duration, resp.TotalAmount, expectedAmount)
		}
	}
}
