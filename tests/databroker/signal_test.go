package databroker_test

import (
	"testing"

	pb "github.com/rhadp/parking-fee-service/gen/kuksa/val/v1"
)

// TestTCPConnectivity verifies that a gRPC client can connect to the
// DATA_BROKER via TCP on host port 55556 and perform a basic metadata query.
// TS-02-1 | Requirement: 02-REQ-2.1, 02-REQ-2.2
func TestTCPConnectivity(t *testing.T) {
	skipIfTCPUnreachable(t)
	conn := connectTCP(t)
	client := newVALClient(conn)

	// Verify a simple metadata query succeeds.
	entry := getMetadata(t, client, "Vehicle.Speed")
	if entry.Path == "" {
		t.Error("expected non-empty path in metadata response")
	}
}

// TestUDSConnectivity verifies that a gRPC client can connect to the
// DATA_BROKER via Unix Domain Socket and perform a basic metadata query.
// TS-02-2 | Requirement: 02-REQ-3.1, 02-REQ-3.2
func TestUDSConnectivity(t *testing.T) {
	sockPath := skipIfUDSUnreachable(t)
	conn := connectUDS(t, sockPath)
	client := newVALClient(conn)

	// Verify a simple metadata query succeeds.
	entry := getMetadata(t, client, "Vehicle.Speed")
	if entry.Path == "" {
		t.Error("expected non-empty path in metadata response")
	}
}

// TestStandardVSSSignalMetadata verifies that all 5 standard VSS v5.1 signals
// are present in the DATA_BROKER metadata with correct data types.
// TS-02-4 | Requirement: 02-REQ-5.1, 02-REQ-5.2
func TestStandardVSSSignalMetadata(t *testing.T) {
	skipIfTCPUnreachable(t)
	conn := connectTCP(t)
	client := newVALClient(conn)

	for _, sig := range standardSignals {
		t.Run(sig.Path, func(t *testing.T) {
			entry := getMetadata(t, client, sig.Path)
			if entry.Metadata == nil {
				t.Fatalf("no metadata returned for %s", sig.Path)
			}
			if entry.Metadata.DataType != sig.DataType {
				t.Errorf("expected data type %v for %s, got %v",
					sig.DataType, sig.Path, entry.Metadata.DataType)
			}
		})
	}
}

// TestCustomVSSSignalMetadata verifies that all 3 custom VSS signals from the
// overlay are present in the DATA_BROKER with correct data types.
// TS-02-5 | Requirement: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3, 02-REQ-6.4
func TestCustomVSSSignalMetadata(t *testing.T) {
	skipIfTCPUnreachable(t)
	conn := connectTCP(t)
	client := newVALClient(conn)

	for _, sig := range customSignals {
		t.Run(sig.Path, func(t *testing.T) {
			entry := getMetadata(t, client, sig.Path)
			if entry.Metadata == nil {
				t.Fatalf("no metadata returned for %s", sig.Path)
			}
			if entry.Metadata.DataType != sig.DataType {
				t.Errorf("expected data type %v for %s, got %v",
					sig.DataType, sig.Path, entry.Metadata.DataType)
			}
		})
	}
}

// TestSignalSetGetViaTCP verifies that signals can be set and retrieved via
// the TCP gRPC interface for different data types.
// TS-02-6 | Requirement: 02-REQ-8.1, 02-REQ-8.2
func TestSignalSetGetViaTCP(t *testing.T) {
	skipIfTCPUnreachable(t)
	conn := connectTCP(t)
	client := newVALClient(conn)

	t.Run("float/Vehicle.Speed", func(t *testing.T) {
		setValue(t, client, "Vehicle.Speed", &pb.Datapoint{
			Value: &pb.Datapoint_FloatValue{FloatValue: 50.0},
		})
		entry := getValue(t, client, "Vehicle.Speed")
		if entry.Value == nil {
			t.Fatal("expected non-nil value")
		}
		got := entry.Value.GetFloatValue()
		if got != 50.0 {
			t.Errorf("expected 50.0, got %v", got)
		}
	})

	t.Run("bool/Vehicle.Parking.SessionActive", func(t *testing.T) {
		setValue(t, client, "Vehicle.Parking.SessionActive", &pb.Datapoint{
			Value: &pb.Datapoint_BoolValue{BoolValue: true},
		})
		entry := getValue(t, client, "Vehicle.Parking.SessionActive")
		if entry.Value == nil {
			t.Fatal("expected non-nil value")
		}
		if !entry.Value.GetBoolValue() {
			t.Error("expected true, got false")
		}
	})

	t.Run("string/Vehicle.Command.Door.Lock", func(t *testing.T) {
		jsonPayload := `{"command_id":"abc","action":"lock"}`
		setValue(t, client, "Vehicle.Command.Door.Lock", &pb.Datapoint{
			Value: &pb.Datapoint_StringValue{StringValue: jsonPayload},
		})
		entry := getValue(t, client, "Vehicle.Command.Door.Lock")
		if entry.Value == nil {
			t.Fatal("expected non-nil value")
		}
		if entry.Value.GetStringValue() != jsonPayload {
			t.Errorf("expected %q, got %q", jsonPayload, entry.Value.GetStringValue())
		}
	})
}

// TestSignalSetGetViaUDS verifies that signals can be set and retrieved via
// the UDS gRPC interface.
// TS-02-7 | Requirement: 02-REQ-9.1
func TestSignalSetGetViaUDS(t *testing.T) {
	sockPath := skipIfUDSUnreachable(t)
	conn := connectUDS(t, sockPath)
	client := newVALClient(conn)

	t.Run("bool/Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", func(t *testing.T) {
		setValue(t, client, "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", &pb.Datapoint{
			Value: &pb.Datapoint_BoolValue{BoolValue: true},
		})
		entry := getValue(t, client, "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
		if entry.Value == nil {
			t.Fatal("expected non-nil value")
		}
		if !entry.Value.GetBoolValue() {
			t.Error("expected true, got false")
		}
	})

	t.Run("double/Vehicle.CurrentLocation.Latitude", func(t *testing.T) {
		setValue(t, client, "Vehicle.CurrentLocation.Latitude", &pb.Datapoint{
			Value: &pb.Datapoint_DoubleValue{DoubleValue: 48.1351},
		})
		entry := getValue(t, client, "Vehicle.CurrentLocation.Latitude")
		if entry.Value == nil {
			t.Fatal("expected non-nil value")
		}
		if entry.Value.GetDoubleValue() != 48.1351 {
			t.Errorf("expected 48.1351, got %v", entry.Value.GetDoubleValue())
		}
	})
}

// TestCrossTransportTCPWriteUDSRead verifies that a signal written via TCP
// is readable via UDS.
// TS-02-8 | Requirement: 02-REQ-4.1, 02-REQ-9.2
func TestCrossTransportTCPWriteUDSRead(t *testing.T) {
	skipIfTCPUnreachable(t)
	sockPath := skipIfUDSUnreachable(t)

	tcpConn := connectTCP(t)
	udsConn := connectUDS(t, sockPath)
	tcpClient := newVALClient(tcpConn)
	udsClient := newVALClient(udsConn)

	// Write via TCP.
	setValue(t, tcpClient, "Vehicle.Speed", &pb.Datapoint{
		Value: &pb.Datapoint_FloatValue{FloatValue: 75.5},
	})

	// Read via UDS.
	entry := getValue(t, udsClient, "Vehicle.Speed")
	if entry.Value == nil {
		t.Fatal("expected non-nil value")
	}
	if entry.Value.GetFloatValue() != 75.5 {
		t.Errorf("expected 75.5 via UDS, got %v", entry.Value.GetFloatValue())
	}
}

// TestCrossTransportUDSWriteTCPRead verifies that a signal written via UDS
// is readable via TCP.
// TS-02-9 | Requirement: 02-REQ-4.1, 02-REQ-9.2
func TestCrossTransportUDSWriteTCPRead(t *testing.T) {
	skipIfTCPUnreachable(t)
	sockPath := skipIfUDSUnreachable(t)

	tcpConn := connectTCP(t)
	udsConn := connectUDS(t, sockPath)
	tcpClient := newVALClient(tcpConn)
	udsClient := newVALClient(udsConn)

	// Write via UDS.
	setValue(t, udsClient, "Vehicle.Parking.SessionActive", &pb.Datapoint{
		Value: &pb.Datapoint_BoolValue{BoolValue: true},
	})

	// Read via TCP.
	entry := getValue(t, tcpClient, "Vehicle.Parking.SessionActive")
	if entry.Value == nil {
		t.Fatal("expected non-nil value")
	}
	if !entry.Value.GetBoolValue() {
		t.Error("expected true via TCP, got false")
	}
}

// TestPermissiveMode verifies that the DATA_BROKER accepts requests without
// any authorization token (permissive mode).
// TS-02-12 | Requirement: 02-REQ-7.1
func TestPermissiveMode(t *testing.T) {
	skipIfTCPUnreachable(t)
	conn := connectTCP(t)
	client := newVALClient(conn)

	// A request with no credentials should succeed.
	setValue(t, client, "Vehicle.Speed", &pb.Datapoint{
		Value: &pb.Datapoint_FloatValue{FloatValue: 10.0},
	})
	entry := getValue(t, client, "Vehicle.Speed")
	if entry.Value == nil {
		t.Fatal("expected non-nil value in permissive mode")
	}
}
