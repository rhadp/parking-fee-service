// Package databroker_test contains integration tests for the DATA_BROKER
// component (Eclipse Kuksa Databroker). Tests require a running databroker
// container accessible at localhost:55556 (TCP) and /tmp/kuksa-databroker.sock
// (UDS).
//
// Start the databroker before running tests:
//
//	cd deployments && podman compose up -d kuksa-databroker
package databroker_test

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"testing"
	"time"

	kuksapb "parking-fee-service/tests/databroker/kuksa"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/metadata"
)

// Endpoints for the DATA_BROKER container.
const (
	tcpAddr = "localhost:55556"
	udsAddr = "unix:///tmp/kuksa-databroker.sock"

	// connectTimeout is the maximum time to wait when establishing a gRPC
	// connection. We use a short timeout so tests fail fast when the
	// databroker is not running.
	connectTimeout = 5 * time.Second

	// opTimeout is the deadline applied to individual gRPC unary RPCs.
	opTimeout = 5 * time.Second

	// subTimeout is the deadline applied to subscription stream receives.
	subTimeout = 10 * time.Second
)

// signalDef describes an expected VSS signal: its path and Go-level type tag.
type signalDef struct {
	path     string
	typeName string // "bool" | "float" | "double" | "string"
}

// Standard VSS v5.1 signals that must be present in the Kuksa built-in tree.
// Requirement: 02-REQ-5.1
var standardSignals = []signalDef{
	{path: "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", typeName: "bool"},
	{path: "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", typeName: "bool"},
	{path: "Vehicle.CurrentLocation.Latitude", typeName: "double"},
	{path: "Vehicle.CurrentLocation.Longitude", typeName: "double"},
	{path: "Vehicle.Speed", typeName: "float"},
}

// Custom VSS signals defined in the overlay file.
// Requirement: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3
var customSignals = []signalDef{
	{path: "Vehicle.Parking.SessionActive", typeName: "bool"},
	{path: "Vehicle.Command.Door.Lock", typeName: "string"},
	{path: "Vehicle.Command.Door.Response", typeName: "string"},
}

// allSignals is the full registry of 8 expected VSS signals (5 standard + 3 custom).
// Requirement: 02-REQ-5.1, 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3
var allSignals = append(standardSignals, customSignals...)

// signalID constructs a SignalID from a path string.
func signalID(path string) *kuksapb.SignalID {
	return &kuksapb.SignalID{Signal: &kuksapb.SignalID_Path{Path: path}}
}

// dialTCP returns a gRPC ClientConn connected to the TCP endpoint.
// The test fails immediately if the endpoint is unreachable.
// Requirement: 02-REQ-2.1, 02-REQ-2.2
func dialTCP(t *testing.T) *grpc.ClientConn {
	t.Helper()
	conn, err := grpc.NewClient(tcpAddr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		t.Fatalf("dialTCP: failed to create gRPC client for %s: %v", tcpAddr, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn
}

// dialTCPWithToken returns a gRPC ClientConn that attaches the given
// Authorization header to every outgoing call. Used to test permissive mode.
// Requirement: 02-REQ-7.E1
func dialTCPWithToken(t *testing.T, token string) *grpc.ClientConn {
	t.Helper()
	md := metadata.Pairs("authorization", "Bearer "+token)
	conn, err := grpc.NewClient(tcpAddr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithUnaryInterceptor(func(
			ctx context.Context, method string, req, reply any,
			cc *grpc.ClientConn, invoker grpc.UnaryInvoker, opts ...grpc.CallOption,
		) error {
			ctx = metadata.NewOutgoingContext(ctx, md)
			return invoker(ctx, method, req, reply, cc, opts...)
		}),
	)
	if err != nil {
		t.Fatalf("dialTCPWithToken: failed to create gRPC client for %s: %v", tcpAddr, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn
}

// udsSocketPath is the host path for the UDS socket. On macOS with podman
// machine, the socket is inside the VM and not accessible from the host.
// UDS tests skip gracefully when the socket is unavailable.
const udsSocketPath = "/tmp/kuksa-databroker.sock"

// dialUDS returns a gRPC ClientConn connected to the Unix Domain Socket endpoint.
// Skips the test if the UDS socket is not accessible (e.g., running on macOS
// with podman machine where the named volume is inside the VM).
// Requirement: 02-REQ-3.1, 02-REQ-3.2
func dialUDS(t *testing.T) *grpc.ClientConn {
	t.Helper()
	// Check if the socket file exists on the host.
	if _, err := os.Stat(udsSocketPath); err != nil {
		t.Skipf("UDS socket not accessible at %s (expected when running on macOS with podman machine); "+
			"UDS tests require Linux or a bind-mounted socket volume: %v", udsSocketPath, err)
	}
	conn, err := grpc.NewClient(udsAddr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		t.Fatalf("dialUDS: failed to create gRPC client for %s: %v", udsAddr, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn
}

// valClient wraps a connection in a VAL gRPC client.
func valClient(conn *grpc.ClientConn) kuksapb.VALClient {
	return kuksapb.NewVALClient(conn)
}

// findRepoRoot walks up from the test working directory until it finds .git.
func findRepoRoot(t *testing.T) string {
	t.Helper()
	dir, err := os.Getwd()
	if err != nil {
		t.Fatalf("findRepoRoot: failed to get working directory: %v", err)
	}
	for {
		if _, err := os.Stat(filepath.Join(dir, ".git")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("findRepoRoot: .git not found; are tests run from within the repo?")
		}
		dir = parent
	}
}

// opCtx returns a context with the standard operation timeout.
func opCtx() (context.Context, context.CancelFunc) {
	return context.WithTimeout(context.Background(), opTimeout)
}

// subCtx returns a context with the subscription timeout.
func subCtx() (context.Context, context.CancelFunc) {
	return context.WithTimeout(context.Background(), subTimeout)
}

// publishSignal writes a value to the named VSS signal using PublishValue.
func publishSignal(t *testing.T, client kuksapb.VALClient, path string, val *kuksapb.Value) {
	t.Helper()
	ctx, cancel := opCtx()
	defer cancel()
	_, err := client.PublishValue(ctx, &kuksapb.PublishValueRequest{
		SignalId:  signalID(path),
		DataPoint: &kuksapb.Datapoint{Value: val},
	})
	if err != nil {
		t.Fatalf("PublishValue(%s): %v", path, err)
	}
}

// setSignalBool writes a boolean value to the named VSS signal.
func setSignalBool(t *testing.T, client kuksapb.VALClient, path string, val bool) {
	t.Helper()
	publishSignal(t, client, path, &kuksapb.Value{TypedValue: &kuksapb.Value_Bool{Bool: val}})
}

// setSignalFloat writes a float32 value to the named VSS signal.
func setSignalFloat(t *testing.T, client kuksapb.VALClient, path string, val float32) {
	t.Helper()
	publishSignal(t, client, path, &kuksapb.Value{TypedValue: &kuksapb.Value_Float{Float: val}})
}

// setSignalDouble writes a float64 value to the named VSS signal.
func setSignalDouble(t *testing.T, client kuksapb.VALClient, path string, val float64) {
	t.Helper()
	publishSignal(t, client, path, &kuksapb.Value{TypedValue: &kuksapb.Value_Double{Double: val}})
}

// setSignalString writes a string value to the named VSS signal.
func setSignalString(t *testing.T, client kuksapb.VALClient, path string, val string) {
	t.Helper()
	publishSignal(t, client, path, &kuksapb.Value{TypedValue: &kuksapb.Value_String_{String_: val}})
}

// getSignalValue retrieves the current Value for the named VSS signal.
// The test fails if the signal is not found or the RPC returns an error.
func getSignalValue(t *testing.T, client kuksapb.VALClient, path string) *kuksapb.Value {
	t.Helper()
	ctx, cancel := opCtx()
	defer cancel()
	resp, err := client.GetValue(ctx, &kuksapb.GetValueRequest{
		SignalId: signalID(path),
	})
	if err != nil {
		t.Fatalf("GetValue(%s): %v", path, err)
	}
	if resp.DataPoint == nil || resp.DataPoint.Value == nil {
		t.Fatalf("GetValue(%s): no value returned", path)
	}
	return resp.DataPoint.Value
}

// getSignalValueNoFail retrieves the current Value without failing on error.
func getSignalValueNoFail(client kuksapb.VALClient, path string) (*kuksapb.Value, error) {
	ctx, cancel := context.WithTimeout(context.Background(), opTimeout)
	defer cancel()
	resp, err := client.GetValue(ctx, &kuksapb.GetValueRequest{
		SignalId: signalID(path),
	})
	if err != nil {
		return nil, err
	}
	if resp.DataPoint == nil || resp.DataPoint.Value == nil {
		return nil, fmt.Errorf("no value returned for signal %q", path)
	}
	return resp.DataPoint.Value, nil
}

// assertSignalExists verifies that the databroker recognises the named signal by
// attempting a GetValue and confirming no error is returned.
// Requirement: 02-REQ-5.2, 02-REQ-6.4
func assertSignalExists(t *testing.T, client kuksapb.VALClient, path string) {
	t.Helper()
	ctx, cancel := opCtx()
	defer cancel()
	_, err := client.GetValue(ctx, &kuksapb.GetValueRequest{
		SignalId: signalID(path),
	})
	if err != nil {
		t.Errorf("assertSignalExists(%s): gRPC error (signal missing?): %v", path, err)
	}
}

// zeroValueForType returns a Value holding the zero value for the given type name.
func zeroValueForType(typeName string) *kuksapb.Value {
	switch typeName {
	case "bool":
		return &kuksapb.Value{TypedValue: &kuksapb.Value_Bool{Bool: false}}
	case "float":
		return &kuksapb.Value{TypedValue: &kuksapb.Value_Float{Float: 0.0}}
	case "double":
		return &kuksapb.Value{TypedValue: &kuksapb.Value_Double{Double: 0.0}}
	case "string":
		return &kuksapb.Value{TypedValue: &kuksapb.Value_String_{String_: ""}}
	default:
		panic("zeroValueForType: unknown type " + typeName)
	}
}

// testValueForType returns a non-zero Value for the given type name.
func testValueForType(typeName string) *kuksapb.Value {
	switch typeName {
	case "bool":
		return &kuksapb.Value{TypedValue: &kuksapb.Value_Bool{Bool: true}}
	case "float":
		return &kuksapb.Value{TypedValue: &kuksapb.Value_Float{Float: 42.0}}
	case "double":
		return &kuksapb.Value{TypedValue: &kuksapb.Value_Double{Double: 48.1351}}
	case "string":
		return &kuksapb.Value{TypedValue: &kuksapb.Value_String_{String_: `{"test":"value"}`}}
	default:
		panic("testValueForType: unknown type " + typeName)
	}
}

// valueEqual reports whether two Values hold identical values.
func valueEqual(a, b *kuksapb.Value) bool {
	if a == nil && b == nil {
		return true
	}
	if a == nil || b == nil {
		return false
	}
	switch av := a.TypedValue.(type) {
	case *kuksapb.Value_Bool:
		bv, ok := b.TypedValue.(*kuksapb.Value_Bool)
		return ok && av.Bool == bv.Bool
	case *kuksapb.Value_Float:
		bv, ok := b.TypedValue.(*kuksapb.Value_Float)
		return ok && av.Float == bv.Float
	case *kuksapb.Value_Double:
		bv, ok := b.TypedValue.(*kuksapb.Value_Double)
		return ok && av.Double == bv.Double
	case *kuksapb.Value_String_:
		bv, ok := b.TypedValue.(*kuksapb.Value_String_)
		return ok && av.String_ == bv.String_
	default:
		return false
	}
}

// valueString returns a human-readable representation of a Value.
func valueString(v *kuksapb.Value) string {
	if v == nil {
		return "<nil>"
	}
	switch tv := v.TypedValue.(type) {
	case *kuksapb.Value_Bool:
		return fmt.Sprintf("bool(%v)", tv.Bool)
	case *kuksapb.Value_Float:
		return fmt.Sprintf("float(%v)", tv.Float)
	case *kuksapb.Value_Double:
		return fmt.Sprintf("double(%v)", tv.Double)
	case *kuksapb.Value_String_:
		return fmt.Sprintf("string(%q)", tv.String_)
	default:
		return fmt.Sprintf("unknown(%T)", v.TypedValue)
	}
}
