package databroker_test

import (
	"testing"
)

// TestTCPConnectivity verifies that a gRPC client can establish a connection
// to the DATA_BROKER via TCP on host port 55556.
//
// Test Spec: TS-02-1
// Requirements: 02-REQ-2.1, 02-REQ-2.2
func TestTCPConnectivity(t *testing.T) {
	skipIfTCPUnreachable(t)

	client := newTCPClient(t)

	// Verify connectivity by performing a Get request for a known signal.
	entry, err := getSignalValue(t, client, "Vehicle.Speed")
	if err != nil {
		t.Fatalf("TCP connectivity check failed: Get returned error: %v", err)
	}
	if entry == nil {
		t.Fatal("TCP connectivity check failed: Get returned nil entry")
	}
	if entry.Path != "Vehicle.Speed" {
		t.Errorf("expected path Vehicle.Speed, got %q", entry.Path)
	}
}

// TestUDSConnectivity verifies that a gRPC client can establish a connection
// to the DATA_BROKER via UDS.
//
// Test Spec: TS-02-2
// Requirements: 02-REQ-3.1, 02-REQ-3.2
func TestUDSConnectivity(t *testing.T) {
	skipIfTCPUnreachable(t)
	skipIfUDSUnreachable(t)

	client := newUDSClient(t)

	entry, err := getSignalValue(t, client, "Vehicle.Speed")
	if err != nil {
		t.Fatalf("UDS connectivity check failed: Get returned error: %v", err)
	}
	if entry == nil {
		t.Fatal("UDS connectivity check failed: Get returned nil entry")
	}
	if entry.Path != "Vehicle.Speed" {
		t.Errorf("expected path Vehicle.Speed, got %q", entry.Path)
	}
}

// TestStandardVSSSignalMetadata verifies that all 5 standard VSS v5.1
// signals are present in the DATA_BROKER with correct types.
//
// Since the v1 gRPC API does not expose metadata types directly, type
// correctness is validated via a set/get roundtrip with the expected type.
//
// Test Spec: TS-02-4
// Requirements: 02-REQ-5.1, 02-REQ-5.2
func TestStandardVSSSignalMetadata(t *testing.T) {
	skipIfTCPUnreachable(t)

	client := newTCPClient(t)

	for _, sig := range standardSignals() {
		t.Run(sig.Path, func(t *testing.T) {
			entry, err := getSignalValue(t, client, sig.Path)
			if err != nil {
				t.Fatalf("signal %s not accessible: %v", sig.Path, err)
			}
			if entry == nil {
				t.Fatalf("signal %s returned nil entry", sig.Path)
			}
			if entry.Path != sig.Path {
				t.Errorf("expected path %q, got %q", sig.Path, entry.Path)
			}

			// Validate data type via write-read roundtrip with expected type.
			testVal := firstTestValue(sig.DataType)
			setSignalByType(t, client, sig, testVal)
			readback, err := getSignalValue(t, client, sig.Path)
			if err != nil {
				t.Fatalf("get after set failed for %s: %v", sig.Path, err)
			}
			assertDatapointValue(t, readback.Value, sig.DataType, testVal)
		})
	}
}

// TestCustomVSSSignalMetadata verifies that all 3 custom VSS signals from
// the overlay are present in the DATA_BROKER with correct types.
//
// Since the v1 gRPC API does not expose metadata types directly, type
// correctness is validated via a set/get roundtrip with the expected type.
//
// Test Spec: TS-02-5
// Requirements: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3, 02-REQ-6.4
func TestCustomVSSSignalMetadata(t *testing.T) {
	skipIfTCPUnreachable(t)

	client := newTCPClient(t)

	for _, sig := range customSignals() {
		t.Run(sig.Path, func(t *testing.T) {
			entry, err := getSignalValue(t, client, sig.Path)
			if err != nil {
				t.Fatalf("custom signal %s not accessible: %v", sig.Path, err)
			}
			if entry == nil {
				t.Fatalf("custom signal %s returned nil entry", sig.Path)
			}
			if entry.Path != sig.Path {
				t.Errorf("expected path %q, got %q", sig.Path, entry.Path)
			}

			// Validate data type via write-read roundtrip with expected type.
			testVal := firstTestValue(sig.DataType)
			setSignalByType(t, client, sig, testVal)
			readback, err := getSignalValue(t, client, sig.Path)
			if err != nil {
				t.Fatalf("get after set failed for %s: %v", sig.Path, err)
			}
			assertDatapointValue(t, readback.Value, sig.DataType, testVal)
		})
	}
}

// TestSignalSetGetViaTCP verifies that signals can be set and retrieved via
// the TCP gRPC interface for multiple data types.
//
// Test Spec: TS-02-6
// Requirements: 02-REQ-8.1, 02-REQ-8.2
func TestSignalSetGetViaTCP(t *testing.T) {
	skipIfTCPUnreachable(t)

	client := newTCPClient(t)

	t.Run("float/Vehicle.Speed", func(t *testing.T) {
		setSignalFloat(t, client, "Vehicle.Speed", 50.0)
		entry, err := getSignalValue(t, client, "Vehicle.Speed")
		if err != nil {
			t.Fatalf("Get failed: %v", err)
		}
		assertDatapointValue(t, entry.Value, "float", float32(50.0))
	})

	t.Run("bool/Vehicle.Parking.SessionActive", func(t *testing.T) {
		setSignalBool(t, client, "Vehicle.Parking.SessionActive", true)
		entry, err := getSignalValue(t, client, "Vehicle.Parking.SessionActive")
		if err != nil {
			t.Fatalf("Get failed: %v", err)
		}
		assertDatapointValue(t, entry.Value, "bool", true)
	})

	t.Run("string/Vehicle.Command.Door.Lock", func(t *testing.T) {
		json := `{"command_id":"abc","action":"lock"}`
		setSignalString(t, client, "Vehicle.Command.Door.Lock", json)
		entry, err := getSignalValue(t, client, "Vehicle.Command.Door.Lock")
		if err != nil {
			t.Fatalf("Get failed: %v", err)
		}
		assertDatapointValue(t, entry.Value, "string", json)
	})

	t.Run("double/Vehicle.CurrentLocation.Latitude", func(t *testing.T) {
		setSignalDouble(t, client, "Vehicle.CurrentLocation.Latitude", 48.1351)
		entry, err := getSignalValue(t, client, "Vehicle.CurrentLocation.Latitude")
		if err != nil {
			t.Fatalf("Get failed: %v", err)
		}
		assertDatapointValue(t, entry.Value, "double", 48.1351)
	})

	t.Run("bool/Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", func(t *testing.T) {
		setSignalBool(t, client, "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
		entry, err := getSignalValue(t, client, "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
		if err != nil {
			t.Fatalf("Get failed: %v", err)
		}
		assertDatapointValue(t, entry.Value, "bool", true)
	})
}

// TestSignalSetGetViaUDS verifies that signals can be set and retrieved via
// the UDS gRPC interface.
//
// Test Spec: TS-02-7
// Requirements: 02-REQ-9.1
func TestSignalSetGetViaUDS(t *testing.T) {
	skipIfTCPUnreachable(t)
	skipIfUDSUnreachable(t)

	client := newUDSClient(t)

	t.Run("bool/Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", func(t *testing.T) {
		setSignalBool(t, client, "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
		entry, err := getSignalValue(t, client, "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
		if err != nil {
			t.Fatalf("Get failed: %v", err)
		}
		assertDatapointValue(t, entry.Value, "bool", true)
	})

	t.Run("double/Vehicle.CurrentLocation.Latitude", func(t *testing.T) {
		setSignalDouble(t, client, "Vehicle.CurrentLocation.Latitude", 48.1351)
		entry, err := getSignalValue(t, client, "Vehicle.CurrentLocation.Latitude")
		if err != nil {
			t.Fatalf("Get failed: %v", err)
		}
		assertDatapointValue(t, entry.Value, "double", 48.1351)
	})
}

// TestCrossTransportTCPWriteUDSRead verifies that a signal written via TCP
// is readable via UDS.
//
// Test Spec: TS-02-8
// Requirements: 02-REQ-4.1, 02-REQ-9.2
func TestCrossTransportTCPWriteUDSRead(t *testing.T) {
	skipIfTCPUnreachable(t)
	skipIfUDSUnreachable(t)

	tcpClient := newTCPClient(t)
	udsClient := newUDSClient(t)

	setSignalFloat(t, tcpClient, "Vehicle.Speed", 75.5)

	entry, err := getSignalValue(t, udsClient, "Vehicle.Speed")
	if err != nil {
		t.Fatalf("UDS Get after TCP Set failed: %v", err)
	}
	assertDatapointValue(t, entry.Value, "float", float32(75.5))
}

// TestCrossTransportUDSWriteTCPRead verifies that a signal written via UDS
// is readable via TCP.
//
// Test Spec: TS-02-9
// Requirements: 02-REQ-4.1, 02-REQ-9.2
func TestCrossTransportUDSWriteTCPRead(t *testing.T) {
	skipIfTCPUnreachable(t)
	skipIfUDSUnreachable(t)

	tcpClient := newTCPClient(t)
	udsClient := newUDSClient(t)

	setSignalBool(t, udsClient, "Vehicle.Parking.SessionActive", true)

	entry, err := getSignalValue(t, tcpClient, "Vehicle.Parking.SessionActive")
	if err != nil {
		t.Fatalf("TCP Get after UDS Set failed: %v", err)
	}
	assertDatapointValue(t, entry.Value, "bool", true)
}

// TestPermissiveMode verifies that the DATA_BROKER accepts requests without
// any authorization token (permissive mode).
//
// Test Spec: TS-02-12
// Requirements: 02-REQ-7.1
func TestPermissiveMode(t *testing.T) {
	skipIfTCPUnreachable(t)

	// Connect without any credentials (no TLS, no token).
	client := newTCPClient(t)

	// A set operation should succeed without authorization.
	setSignalFloat(t, client, "Vehicle.Speed", 10.0)

	entry, err := getSignalValue(t, client, "Vehicle.Speed")
	if err != nil {
		t.Fatalf("Get without auth token failed: %v", err)
	}
	assertDatapointValue(t, entry.Value, "float", float32(10.0))
}
