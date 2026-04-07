package databroker_test

import (
	"context"
	"testing"

	kuksapb "parking-fee-service/tests/databroker/kuksa"
)

// ---------------------------------------------------------------------------
// TS-02-SMOKE-1: Databroker health check
// Requirement: 02-REQ-1.1, 02-REQ-2.1
// ---------------------------------------------------------------------------

// TestSmokeHealthCheck is a quick smoke test verifying that the DATA_BROKER
// container is running and accepts TCP connections on the expected port.
func TestSmokeHealthCheck(t *testing.T) {
	conn := dialTCP(t)
	client := valClient(conn)

	ctx, cancel := context.WithTimeout(context.Background(), connectTimeout)
	defer cancel()

	_, err := client.GetValue(ctx, &kuksapb.GetValueRequest{
		SignalId: signalID("Vehicle.Speed"),
	})
	if err != nil {
		t.Fatalf("TS-02-SMOKE-1: databroker health check failed — TCP GetValue(Vehicle.Speed) error: %v", err)
	}
	t.Log("TS-02-SMOKE-1: databroker health check passed — TCP connectivity OK")
}

// ---------------------------------------------------------------------------
// TS-02-SMOKE-2: Full signal inventory check
// Requirement: 02-REQ-5.1, 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3
// ---------------------------------------------------------------------------

// TestSmokeSignalInventory is a quick smoke test verifying that all 8 expected
// VSS signals (5 standard + 3 custom overlay) are present in the DATA_BROKER.
func TestSmokeSignalInventory(t *testing.T) {
	conn := dialTCP(t)
	client := valClient(conn)

	expectedSignals := []string{
		"Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
		"Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
		"Vehicle.CurrentLocation.Latitude",
		"Vehicle.CurrentLocation.Longitude",
		"Vehicle.Speed",
		"Vehicle.Parking.SessionActive",
		"Vehicle.Command.Door.Lock",
		"Vehicle.Command.Door.Response",
	}

	missing := 0
	for _, sig := range expectedSignals {
		ctx, cancel := context.WithTimeout(context.Background(), opTimeout)
		_, err := client.GetValue(ctx, &kuksapb.GetValueRequest{
			SignalId: signalID(sig),
		})
		cancel()

		if err != nil {
			t.Errorf("TS-02-SMOKE-2: signal %q not available: %v", sig, err)
			missing++
		}
	}

	if missing > 0 {
		t.Fatalf("TS-02-SMOKE-2: %d/%d signals missing from DATA_BROKER inventory", missing, len(expectedSignals))
	}
	t.Logf("TS-02-SMOKE-2: all %d signals present in DATA_BROKER", len(expectedSignals))
}
