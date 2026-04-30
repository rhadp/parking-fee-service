package databroker_test

import (
	"testing"

	kuksa "github.com/rhadp/parking-fee-service/gen/kuksa"
)

// --- Signal registry: all 8 expected VSS signals ---

type signalSpec struct {
	Path     string
	DataType kuksa.DataType
}

// standardSignals are the 5 built-in VSS v5.1 signals.
var standardSignals = []signalSpec{
	{"Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", kuksa.DataType_DATA_TYPE_BOOLEAN},
	{"Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", kuksa.DataType_DATA_TYPE_BOOLEAN},
	{"Vehicle.CurrentLocation.Latitude", kuksa.DataType_DATA_TYPE_DOUBLE},
	{"Vehicle.CurrentLocation.Longitude", kuksa.DataType_DATA_TYPE_DOUBLE},
	{"Vehicle.Speed", kuksa.DataType_DATA_TYPE_FLOAT},
}

// customSignals are the 3 custom signals defined in the VSS overlay.
var customSignals = []signalSpec{
	{"Vehicle.Parking.SessionActive", kuksa.DataType_DATA_TYPE_BOOLEAN},
	{"Vehicle.Command.Door.Lock", kuksa.DataType_DATA_TYPE_STRING},
	{"Vehicle.Command.Door.Response", kuksa.DataType_DATA_TYPE_STRING},
}

// allSignals is the complete set of 8 expected signals.
var allSignals = append(append([]signalSpec{}, standardSignals...), customSignals...)

// --- Connectivity tests ---

// TestTCPConnectivity verifies that a gRPC client can connect to the DATA_BROKER
// via TCP on host port 55556 and perform a metadata query.
// Test Spec: TS-02-1
// Requirement: 02-REQ-2.1, 02-REQ-2.2
func TestTCPConnectivity(t *testing.T) {
	skipIfTCPUnreachable(t)
	_, client := dialTCP(t)

	// Verify connectivity by querying metadata for a known signal.
	md := listMetadataOrFail(t, client, "Vehicle.Speed")
	if len(md) == 0 {
		t.Error("ListMetadata for Vehicle.Speed returned no entries via TCP")
	}
}

// TestUDSConnectivity verifies that a gRPC client can connect to the DATA_BROKER
// via Unix Domain Socket and perform a metadata query.
// Test Spec: TS-02-2
// Requirement: 02-REQ-3.1, 02-REQ-3.2
func TestUDSConnectivity(t *testing.T) {
	skipIfUDSUnreachable(t)
	_, client := dialUDS(t)

	// Verify connectivity by querying metadata for a known signal.
	md := listMetadataOrFail(t, client, "Vehicle.Speed")
	if len(md) == 0 {
		t.Error("ListMetadata for Vehicle.Speed returned no entries via UDS")
	}
}

// --- Standard VSS signal metadata tests ---

// TestStandardVSSSignalMetadata verifies that all 5 standard VSS v5.1 signals
// are present in the DATA_BROKER metadata with correct data types.
// Test Spec: TS-02-4
// Requirement: 02-REQ-5.1, 02-REQ-5.2
func TestStandardVSSSignalMetadata(t *testing.T) {
	skipIfTCPUnreachable(t)
	_, client := dialTCP(t)

	for _, sig := range standardSignals {
		t.Run(sig.Path, func(t *testing.T) {
			md := listMetadataOrFail(t, client, sig.Path)
			if len(md) == 0 {
				t.Fatalf("missing standard signal: %s", sig.Path)
			}
			if md[0].DataType != sig.DataType {
				t.Errorf("signal %s: expected data type %v, got %v",
					sig.Path, sig.DataType, md[0].DataType)
			}
		})
	}
}

// --- Custom VSS signal metadata tests ---

// TestCustomVSSSignalMetadata verifies that all 3 custom VSS signals from the
// overlay are present in the DATA_BROKER metadata with correct data types.
// Test Spec: TS-02-5
// Requirement: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3, 02-REQ-6.4
func TestCustomVSSSignalMetadata(t *testing.T) {
	skipIfTCPUnreachable(t)
	_, client := dialTCP(t)

	for _, sig := range customSignals {
		t.Run(sig.Path, func(t *testing.T) {
			md := listMetadataOrFail(t, client, sig.Path)
			if len(md) == 0 {
				t.Fatalf("missing custom signal: %s", sig.Path)
			}
			if md[0].DataType != sig.DataType {
				t.Errorf("signal %s: expected data type %v, got %v",
					sig.Path, sig.DataType, md[0].DataType)
			}
		})
	}
}

// --- Signal set/get tests ---

// TestSignalSetGetViaTCP verifies that signals can be set and retrieved via TCP.
// Tests multiple signal types: bool, float, string.
// Test Spec: TS-02-6
// Requirement: 02-REQ-8.1, 02-REQ-8.2
func TestSignalSetGetViaTCP(t *testing.T) {
	skipIfTCPUnreachable(t)
	_, client := dialTCP(t)

	t.Run("bool/Vehicle.Parking.SessionActive", func(t *testing.T) {
		publishValue(t, client, "Vehicle.Parking.SessionActive", boolValue(true))
		dp := getValueOrFail(t, client, "Vehicle.Parking.SessionActive")
		if dp.GetValue().GetBool() != true {
			t.Errorf("expected true, got %v", dp.GetValue().GetBool())
		}
	})

	t.Run("float/Vehicle.Speed", func(t *testing.T) {
		publishValue(t, client, "Vehicle.Speed", floatValue(50.0))
		dp := getValueOrFail(t, client, "Vehicle.Speed")
		if dp.GetValue().GetFloat() != 50.0 {
			t.Errorf("expected 50.0, got %v", dp.GetValue().GetFloat())
		}
	})

	t.Run("string/Vehicle.Command.Door.Lock", func(t *testing.T) {
		json := `{"command_id":"abc","action":"lock"}`
		publishValue(t, client, "Vehicle.Command.Door.Lock", stringValue(json))
		dp := getValueOrFail(t, client, "Vehicle.Command.Door.Lock")
		if dp.GetValue().GetString_() != json {
			t.Errorf("expected %q, got %q", json, dp.GetValue().GetString_())
		}
	})

	t.Run("double/Vehicle.CurrentLocation.Latitude", func(t *testing.T) {
		publishValue(t, client, "Vehicle.CurrentLocation.Latitude", doubleValue(48.1351))
		dp := getValueOrFail(t, client, "Vehicle.CurrentLocation.Latitude")
		if dp.GetValue().GetDouble() != 48.1351 {
			t.Errorf("expected 48.1351, got %v", dp.GetValue().GetDouble())
		}
	})
}

// TestSignalSetGetViaUDS verifies that signals can be set and retrieved via UDS.
// Test Spec: TS-02-7
// Requirement: 02-REQ-9.1
func TestSignalSetGetViaUDS(t *testing.T) {
	skipIfUDSUnreachable(t)
	_, client := dialUDS(t)

	t.Run("bool/Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", func(t *testing.T) {
		publishValue(t, client, "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", boolValue(true))
		dp := getValueOrFail(t, client, "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
		if dp.GetValue().GetBool() != true {
			t.Errorf("expected true, got %v", dp.GetValue().GetBool())
		}
	})

	t.Run("double/Vehicle.CurrentLocation.Latitude", func(t *testing.T) {
		publishValue(t, client, "Vehicle.CurrentLocation.Latitude", doubleValue(48.1351))
		dp := getValueOrFail(t, client, "Vehicle.CurrentLocation.Latitude")
		if dp.GetValue().GetDouble() != 48.1351 {
			t.Errorf("expected 48.1351, got %v", dp.GetValue().GetDouble())
		}
	})
}

// --- Cross-transport consistency tests ---

// TestCrossTransportTCPWriteUDSRead verifies that a signal written via TCP
// is readable via UDS.
// Test Spec: TS-02-8
// Requirement: 02-REQ-4.1, 02-REQ-9.2
func TestCrossTransportTCPWriteUDSRead(t *testing.T) {
	skipIfTCPUnreachable(t)
	skipIfUDSUnreachable(t)
	_, tcpClient := dialTCP(t)
	_, udsClient := dialUDS(t)

	publishValue(t, tcpClient, "Vehicle.Speed", floatValue(75.5))
	dp := getValueOrFail(t, udsClient, "Vehicle.Speed")
	if dp.GetValue().GetFloat() != 75.5 {
		t.Errorf("cross-transport (TCP->UDS): expected 75.5, got %v", dp.GetValue().GetFloat())
	}
}

// TestCrossTransportUDSWriteTCPRead verifies that a signal written via UDS
// is readable via TCP.
// Test Spec: TS-02-9
// Requirement: 02-REQ-4.1, 02-REQ-9.2
func TestCrossTransportUDSWriteTCPRead(t *testing.T) {
	skipIfTCPUnreachable(t)
	skipIfUDSUnreachable(t)
	_, tcpClient := dialTCP(t)
	_, udsClient := dialUDS(t)

	publishValue(t, udsClient, "Vehicle.Parking.SessionActive", boolValue(true))
	dp := getValueOrFail(t, tcpClient, "Vehicle.Parking.SessionActive")
	if dp.GetValue().GetBool() != true {
		t.Errorf("cross-transport (UDS->TCP): expected true, got %v", dp.GetValue().GetBool())
	}
}

// --- Permissive mode test ---

// TestPermissiveMode verifies that the DATA_BROKER accepts requests without
// authorization tokens (permissive mode).
// Test Spec: TS-02-12
// Requirement: 02-REQ-7.1
func TestPermissiveMode(t *testing.T) {
	skipIfTCPUnreachable(t)
	_, client := dialTCP(t)

	// No credentials or tokens attached; request should succeed.
	publishValue(t, client, "Vehicle.Speed", floatValue(10.0))
	dp := getValueOrFail(t, client, "Vehicle.Speed")
	if dp.GetValue().GetFloat() != 10.0 {
		t.Errorf("expected 10.0, got %v", dp.GetValue().GetFloat())
	}
}
