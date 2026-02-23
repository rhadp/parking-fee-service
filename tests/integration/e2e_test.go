package integration

import (
	"testing"
)

// ===========================================================================
// TS-04-39: Integration test lock-to-session
// Requirement: 04-REQ-10.1
// ===========================================================================

func TestE2E_LockToSession(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	t.Skip("requires DATA_BROKER + PARKING_OPERATOR_ADAPTOR + mock operator running — will be enabled in task group 7")

	// Publish Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true to DATA_BROKER.
	// Wait 3s for adaptor to process.
	// Assert: mock PARKING_OPERATOR received POST /parking/start.
	// Assert: Vehicle.Parking.SessionActive is true in DATA_BROKER.
	t.Fatal("lock-to-session e2e test not yet implemented")
}

// ===========================================================================
// TS-04-40: Integration test CLI-to-UpdateService lifecycle
// Requirement: 04-REQ-10.2
// ===========================================================================

func TestE2E_CLIToUpdateService(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	t.Skip("requires UPDATE_SERVICE running — will be enabled in task group 7")

	// Run install via CLI. Assert exit code 0 and response contains adapter info.
	// Run list via CLI. Assert the installed adapter appears.
	binary := cliBinary(t)
	_ = binary
	t.Fatal("CLI-to-UpdateService e2e test not yet implemented")
}

// ===========================================================================
// TS-04-41: Integration test adaptor-to-operator communication
// Requirement: 04-REQ-10.3
// ===========================================================================

func TestE2E_AdaptorToOperator(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	t.Skip("requires PARKING_OPERATOR_ADAPTOR + mock operator running — will be enabled in task group 7")

	// Call StartSession via gRPC. Assert session_id is non-empty.
	// Call StopSession via gRPC. Assert total_fee >= 0.
	// Call GetRate via gRPC. Assert rate_per_hour == 2.50.
	t.Fatal("adaptor-to-operator e2e test not yet implemented")
}
