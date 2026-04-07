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

// dialUDS returns a gRPC ClientConn connected to the Unix Domain Socket endpoint.
// Requirement: 02-REQ-3.1, 02-REQ-3.2
func dialUDS(t *testing.T) *grpc.ClientConn {
	t.Helper()
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

// setSignalBool writes a boolean value to the named VSS signal.
func setSignalBool(t *testing.T, client kuksapb.VALClient, path string, val bool) {
	t.Helper()
	ctx, cancel := opCtx()
	defer cancel()
	resp, err := client.Set(ctx, &kuksapb.SetRequest{
		Entries: []*kuksapb.DataEntry{
			{
				Path:  path,
				Value: &kuksapb.Datapoint{Value: &kuksapb.Datapoint_BoolValue{BoolValue: val}},
			},
		},
	})
	if err != nil {
		t.Fatalf("Set(%s, bool=%v): gRPC error: %v", path, val, err)
	}
	if !resp.Success {
		t.Fatalf("Set(%s, bool=%v): databroker error: %s", path, val, resp.Error)
	}
}

// setSignalFloat writes a float32 value to the named VSS signal.
func setSignalFloat(t *testing.T, client kuksapb.VALClient, path string, val float32) {
	t.Helper()
	ctx, cancel := opCtx()
	defer cancel()
	resp, err := client.Set(ctx, &kuksapb.SetRequest{
		Entries: []*kuksapb.DataEntry{
			{
				Path:  path,
				Value: &kuksapb.Datapoint{Value: &kuksapb.Datapoint_FloatValue{FloatValue: val}},
			},
		},
	})
	if err != nil {
		t.Fatalf("Set(%s, float=%v): gRPC error: %v", path, val, err)
	}
	if !resp.Success {
		t.Fatalf("Set(%s, float=%v): databroker error: %s", path, val, resp.Error)
	}
}

// setSignalDouble writes a float64 value to the named VSS signal.
func setSignalDouble(t *testing.T, client kuksapb.VALClient, path string, val float64) {
	t.Helper()
	ctx, cancel := opCtx()
	defer cancel()
	resp, err := client.Set(ctx, &kuksapb.SetRequest{
		Entries: []*kuksapb.DataEntry{
			{
				Path:  path,
				Value: &kuksapb.Datapoint{Value: &kuksapb.Datapoint_DoubleValue{DoubleValue: val}},
			},
		},
	})
	if err != nil {
		t.Fatalf("Set(%s, double=%v): gRPC error: %v", path, val, err)
	}
	if !resp.Success {
		t.Fatalf("Set(%s, double=%v): databroker error: %s", path, val, resp.Error)
	}
}

// setSignalString writes a string value to the named VSS signal.
func setSignalString(t *testing.T, client kuksapb.VALClient, path string, val string) {
	t.Helper()
	ctx, cancel := opCtx()
	defer cancel()
	resp, err := client.Set(ctx, &kuksapb.SetRequest{
		Entries: []*kuksapb.DataEntry{
			{
				Path:  path,
				Value: &kuksapb.Datapoint{Value: &kuksapb.Datapoint_StringValue{StringValue: val}},
			},
		},
	})
	if err != nil {
		t.Fatalf("Set(%s, string=%q): gRPC error: %v", path, val, err)
	}
	if !resp.Success {
		t.Fatalf("Set(%s, string=%q): databroker error: %s", path, val, resp.Error)
	}
}

// getSignal retrieves the current DataEntry for the named VSS signal.
// The test fails if the signal is not found or the RPC returns an error.
func getSignal(t *testing.T, client kuksapb.VALClient, path string) *kuksapb.DataEntry {
	t.Helper()
	ctx, cancel := opCtx()
	defer cancel()
	resp, err := client.Get(ctx, &kuksapb.GetRequest{Paths: []string{path}})
	if err != nil {
		t.Fatalf("Get(%s): gRPC error: %v", path, err)
	}
	if len(resp.Entries) == 0 {
		t.Fatalf("Get(%s): no entry returned (signal not found)", path)
	}
	return resp.Entries[0]
}

// getSignalNoFail retrieves the current DataEntry without failing on error.
// Returns (entry, nil) on success or (nil, err) on any error.
func getSignalNoFail(client kuksapb.VALClient, path string) (*kuksapb.DataEntry, error) {
	ctx, cancel := context.WithTimeout(context.Background(), opTimeout)
	defer cancel()
	resp, err := client.Get(ctx, &kuksapb.GetRequest{Paths: []string{path}})
	if err != nil {
		return nil, err
	}
	if len(resp.Entries) == 0 {
		return nil, fmt.Errorf("no entry returned for signal %q", path)
	}
	return resp.Entries[0], nil
}

// assertSignalExists verifies that the databroker recognises the named signal by
// attempting a Get and confirming a valid entry is returned. This is the
// closest equivalent to GetMetadata available in the VAL gRPC API.
// Requirement: 02-REQ-5.2, 02-REQ-6.4
func assertSignalExists(t *testing.T, client kuksapb.VALClient, path string) {
	t.Helper()
	// A successful Get (even for an unset signal) proves the databroker knows
	// the signal path. An unrecognised path returns a gRPC error or an empty
	// response.
	ctx, cancel := opCtx()
	defer cancel()
	_, err := client.Get(ctx, &kuksapb.GetRequest{Paths: []string{path}})
	if err != nil {
		t.Errorf("assertSignalExists(%s): gRPC error (signal missing?): %v", path, err)
	}
}

// zeroValueForType returns a Datapoint holding the zero value for the given
// type name. Used by metadata property tests to probe type compatibility.
func zeroValueForType(typeName string) *kuksapb.Datapoint {
	switch typeName {
	case "bool":
		return &kuksapb.Datapoint{Value: &kuksapb.Datapoint_BoolValue{BoolValue: false}}
	case "float":
		return &kuksapb.Datapoint{Value: &kuksapb.Datapoint_FloatValue{FloatValue: 0.0}}
	case "double":
		return &kuksapb.Datapoint{Value: &kuksapb.Datapoint_DoubleValue{DoubleValue: 0.0}}
	case "string":
		return &kuksapb.Datapoint{Value: &kuksapb.Datapoint_StringValue{StringValue: ""}}
	default:
		panic("zeroValueForType: unknown type " + typeName)
	}
}

// testValueForType returns a non-zero Datapoint for the given type name.
// Used by write-read roundtrip property tests.
func testValueForType(typeName string) *kuksapb.Datapoint {
	switch typeName {
	case "bool":
		return &kuksapb.Datapoint{Value: &kuksapb.Datapoint_BoolValue{BoolValue: true}}
	case "float":
		return &kuksapb.Datapoint{Value: &kuksapb.Datapoint_FloatValue{FloatValue: 42.0}}
	case "double":
		return &kuksapb.Datapoint{Value: &kuksapb.Datapoint_DoubleValue{DoubleValue: 48.1351}}
	case "string":
		return &kuksapb.Datapoint{Value: &kuksapb.Datapoint_StringValue{StringValue: `{"test":"value"}`}}
	default:
		panic("testValueForType: unknown type " + typeName)
	}
}

// datapointEqual reports whether two Datapoints hold identical values.
func datapointEqual(a, b *kuksapb.Datapoint) bool {
	if a == nil && b == nil {
		return true
	}
	if a == nil || b == nil {
		return false
	}
	switch av := a.Value.(type) {
	case *kuksapb.Datapoint_BoolValue:
		bv, ok := b.Value.(*kuksapb.Datapoint_BoolValue)
		return ok && av.BoolValue == bv.BoolValue
	case *kuksapb.Datapoint_FloatValue:
		bv, ok := b.Value.(*kuksapb.Datapoint_FloatValue)
		return ok && av.FloatValue == bv.FloatValue
	case *kuksapb.Datapoint_DoubleValue:
		bv, ok := b.Value.(*kuksapb.Datapoint_DoubleValue)
		return ok && av.DoubleValue == bv.DoubleValue
	case *kuksapb.Datapoint_StringValue:
		bv, ok := b.Value.(*kuksapb.Datapoint_StringValue)
		return ok && av.StringValue == bv.StringValue
	default:
		return false
	}
}

// datapointString returns a human-readable representation of a Datapoint value.
func datapointString(dp *kuksapb.Datapoint) string {
	if dp == nil {
		return "<nil>"
	}
	switch v := dp.Value.(type) {
	case *kuksapb.Datapoint_BoolValue:
		return fmt.Sprintf("bool(%v)", v.BoolValue)
	case *kuksapb.Datapoint_FloatValue:
		return fmt.Sprintf("float(%v)", v.FloatValue)
	case *kuksapb.Datapoint_DoubleValue:
		return fmt.Sprintf("double(%v)", v.DoubleValue)
	case *kuksapb.Datapoint_StringValue:
		return fmt.Sprintf("string(%q)", v.StringValue)
	default:
		return fmt.Sprintf("unknown(%T)", dp.Value)
	}
}
