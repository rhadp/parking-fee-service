package databroker_test

import (
	"context"
	"fmt"
	"net"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/gen/kuksa"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

const (
	// tcpTarget is the host address for the DATA_BROKER TCP listener.
	tcpTarget = "localhost:55556"

	// connectTimeout is the maximum time to wait for a gRPC connection.
	connectTimeout = 5 * time.Second

	// rpcTimeout is the maximum time to wait for a single gRPC call.
	rpcTimeout = 5 * time.Second
)

// signalDef describes a VSS signal with its path and expected data type.
type signalDef struct {
	Path     string
	DataType string // "bool", "float", "double", "string"
}

// standardSignals returns the 5 standard VSS v5.1 signals.
func standardSignals() []signalDef {
	return []signalDef{
		{Path: "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", DataType: "bool"},
		{Path: "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", DataType: "bool"},
		{Path: "Vehicle.CurrentLocation.Latitude", DataType: "double"},
		{Path: "Vehicle.CurrentLocation.Longitude", DataType: "double"},
		{Path: "Vehicle.Speed", DataType: "float"},
	}
}

// customSignals returns the 3 custom VSS signals from the overlay.
func customSignals() []signalDef {
	return []signalDef{
		{Path: "Vehicle.Parking.SessionActive", DataType: "bool"},
		{Path: "Vehicle.Command.Door.Lock", DataType: "string"},
		{Path: "Vehicle.Command.Door.Response", DataType: "string"},
	}
}

// allSignals returns all 8 expected VSS signals (5 standard + 3 custom).
func allSignals() []signalDef {
	return append(standardSignals(), customSignals()...)
}

// repoRoot returns the absolute path to the repository root.
func repoRoot(t *testing.T) string {
	t.Helper()

	dir, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get working directory: %v", err)
	}

	// Walk up from tests/databroker/ to the repo root (two levels up).
	root := filepath.Join(dir, "..", "..")
	root, err = filepath.Abs(root)
	if err != nil {
		t.Fatalf("failed to resolve repo root: %v", err)
	}

	if _, err := os.Stat(filepath.Join(root, ".gitignore")); os.IsNotExist(err) {
		t.Fatalf("repo root %s does not contain .gitignore; test must run from tests/databroker/", root)
	}

	return root
}

// effectiveUDSSocket returns the path to the UDS socket, checking common
// mount locations. Returns empty string if no socket is found.
func effectiveUDSSocket() string {
	// When compose binds /tmp/kuksa (host) to /tmp (container), the socket
	// appears at /tmp/kuksa/kuksa-databroker.sock on the host.
	candidates := []string{
		"/tmp/kuksa/kuksa-databroker.sock",
		"/tmp/kuksa-databroker.sock",
	}
	for _, path := range candidates {
		if info, err := os.Stat(path); err == nil && !info.IsDir() {
			return path
		}
	}
	return ""
}

// udsTarget returns the gRPC dial target for the UDS socket.
func udsTarget() string {
	sock := effectiveUDSSocket()
	if sock == "" {
		return ""
	}
	return "unix://" + sock
}

// skipIfTCPUnreachable skips the test if the DATA_BROKER TCP port is not
// reachable. This allows live tests to be skipped in environments where the
// container is not running.
func skipIfTCPUnreachable(t *testing.T) {
	t.Helper()
	conn, err := net.DialTimeout("tcp", tcpTarget, 2*time.Second)
	if err != nil {
		t.Skipf("DATA_BROKER TCP port not reachable at %s: %v", tcpTarget, err)
	}
	conn.Close()
}

// skipIfUDSUnreachable skips the test if the DATA_BROKER UDS socket is not
// accessible.
func skipIfUDSUnreachable(t *testing.T) {
	t.Helper()
	sock := effectiveUDSSocket()
	if sock == "" {
		t.Skip("DATA_BROKER UDS socket not found at any known path")
	}
}

// dialTCP creates a gRPC client connection to the DATA_BROKER via TCP.
func dialTCP(t *testing.T) *grpc.ClientConn {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), connectTimeout)
	defer cancel()

	conn, err := grpc.DialContext(ctx, tcpTarget,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		t.Fatalf("failed to dial TCP %s: %v", tcpTarget, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn
}

// dialUDS creates a gRPC client connection to the DATA_BROKER via UDS.
func dialUDS(t *testing.T) *grpc.ClientConn {
	t.Helper()
	target := udsTarget()
	if target == "" {
		t.Fatal("UDS socket not found")
	}

	ctx, cancel := context.WithTimeout(context.Background(), connectTimeout)
	defer cancel()

	conn, err := grpc.DialContext(ctx, target,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		t.Fatalf("failed to dial UDS %s: %v", target, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn
}

// newTCPClient creates a kuksa VAL gRPC client over TCP.
func newTCPClient(t *testing.T) kuksa.VALClient {
	t.Helper()
	return kuksa.NewVALClient(dialTCP(t))
}

// newUDSClient creates a kuksa VAL gRPC client over UDS.
func newUDSClient(t *testing.T) kuksa.VALClient {
	t.Helper()
	return kuksa.NewVALClient(dialUDS(t))
}

// getSignalValue performs a Get RPC for the given signal path and returns
// the DataEntry. Returns an error if the signal is not found.
func getSignalValue(t *testing.T, client kuksa.VALClient, path string) (*kuksa.DataEntry, error) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel()

	resp, err := client.Get(ctx, &kuksa.GetRequest{
		Entries: []*kuksa.EntryRequest{
			{
				Path:   path,
				View:   kuksa.View_VIEW_CURRENT_VALUE,
				Fields: []kuksa.Field{kuksa.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		return nil, err
	}
	if len(resp.Entries) == 0 {
		return nil, fmt.Errorf("no entries returned for %s", path)
	}
	return resp.Entries[0], nil
}

// setSignalBool sets a boolean signal value.
func setSignalBool(t *testing.T, client kuksa.VALClient, path string, val bool) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel()

	_, err := client.Set(ctx, &kuksa.SetRequest{
		Updates: []*kuksa.EntryUpdate{
			{
				Entry: &kuksa.DataEntry{
					Path: path,
					Value: &kuksa.Datapoint{
						Value: &kuksa.Datapoint_BoolValue{BoolValue: val},
					},
				},
				Fields: []kuksa.Field{kuksa.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		t.Fatalf("failed to set %s to %v: %v", path, val, err)
	}
}

// setSignalFloat sets a float signal value.
func setSignalFloat(t *testing.T, client kuksa.VALClient, path string, val float32) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel()

	_, err := client.Set(ctx, &kuksa.SetRequest{
		Updates: []*kuksa.EntryUpdate{
			{
				Entry: &kuksa.DataEntry{
					Path: path,
					Value: &kuksa.Datapoint{
						Value: &kuksa.Datapoint_FloatValue{FloatValue: val},
					},
				},
				Fields: []kuksa.Field{kuksa.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		t.Fatalf("failed to set %s to %v: %v", path, val, err)
	}
}

// setSignalDouble sets a double signal value.
func setSignalDouble(t *testing.T, client kuksa.VALClient, path string, val float64) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel()

	_, err := client.Set(ctx, &kuksa.SetRequest{
		Updates: []*kuksa.EntryUpdate{
			{
				Entry: &kuksa.DataEntry{
					Path: path,
					Value: &kuksa.Datapoint{
						Value: &kuksa.Datapoint_DoubleValue{DoubleValue: val},
					},
				},
				Fields: []kuksa.Field{kuksa.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		t.Fatalf("failed to set %s to %v: %v", path, val, err)
	}
}

// setSignalString sets a string signal value.
func setSignalString(t *testing.T, client kuksa.VALClient, path string, val string) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel()

	_, err := client.Set(ctx, &kuksa.SetRequest{
		Updates: []*kuksa.EntryUpdate{
			{
				Entry: &kuksa.DataEntry{
					Path: path,
					Value: &kuksa.Datapoint{
						Value: &kuksa.Datapoint_StringValue{StringValue: val},
					},
				},
				Fields: []kuksa.Field{kuksa.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		t.Fatalf("failed to set %s to %q: %v", path, val, err)
	}
}

// setSignalByType sets a signal value using the appropriate type-specific
// setter based on the signalDef's DataType.
func setSignalByType(t *testing.T, client kuksa.VALClient, sig signalDef, val interface{}) {
	t.Helper()
	switch sig.DataType {
	case "bool":
		setSignalBool(t, client, sig.Path, val.(bool))
	case "float":
		setSignalFloat(t, client, sig.Path, val.(float32))
	case "double":
		setSignalDouble(t, client, sig.Path, val.(float64))
	case "string":
		setSignalString(t, client, sig.Path, val.(string))
	default:
		t.Fatalf("unsupported data type %q for signal %s", sig.DataType, sig.Path)
	}
}

// testValuesForType returns a slice of test values for the given data type.
func testValuesForType(dataType string) []interface{} {
	switch dataType {
	case "bool":
		return []interface{}{true, false}
	case "float":
		return []interface{}{float32(0.0), float32(50.0), float32(999.9)}
	case "double":
		return []interface{}{float64(48.1351), float64(-122.4194)}
	case "string":
		return []interface{}{`{"command_id":"x"}`, `{}`}
	default:
		return nil
	}
}

// assertDatapointValue checks that a Datapoint contains the expected value.
func assertDatapointValue(t *testing.T, dp *kuksa.Datapoint, dataType string, expected interface{}) {
	t.Helper()
	if dp == nil {
		t.Fatal("datapoint is nil")
	}
	switch dataType {
	case "bool":
		got := dp.GetBoolValue()
		want := expected.(bool)
		if got != want {
			t.Errorf("bool value mismatch: got %v, want %v", got, want)
		}
	case "float":
		got := dp.GetFloatValue()
		want := expected.(float32)
		if got != want {
			t.Errorf("float value mismatch: got %v, want %v", got, want)
		}
	case "double":
		got := dp.GetDoubleValue()
		want := expected.(float64)
		if got != want {
			t.Errorf("double value mismatch: got %v, want %v", got, want)
		}
	case "string":
		got := dp.GetStringValue()
		want := expected.(string)
		if got != want {
			t.Errorf("string value mismatch: got %q, want %q", got, want)
		}
	default:
		t.Fatalf("unsupported data type %q", dataType)
	}
}
