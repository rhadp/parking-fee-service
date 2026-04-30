package databroker_test

import (
	"context"
	"net"
	"os"
	"path/filepath"
	"testing"
	"time"

	kuksa "github.com/rhadp/parking-fee-service/gen/kuksa"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/protobuf/types/known/timestamppb"
)

const (
	// tcpTarget is the host address for the databroker TCP listener.
	// Compose maps container port 55555 to host port 55556.
	tcpTarget = "localhost:55556"
)

// repoRoot returns the absolute path to the repository root.
// tests/databroker/ is two levels deep from the repo root.
func repoRoot(t *testing.T) string {
	t.Helper()
	root, err := filepath.Abs("../..")
	if err != nil {
		t.Fatalf("failed to resolve repo root: %v", err)
	}
	return root
}

// effectiveUDSSocket returns the path to the UDS socket on the host.
// The compose volume mount maps the container /tmp directory to a host
// directory. Checks both /tmp/kuksa/kuksa-databroker.sock (host bind mount)
// and /tmp/kuksa-databroker.sock (in-container path, reachable if running
// tests inside the container network).
func effectiveUDSSocket() string {
	paths := []string{
		"/tmp/kuksa/kuksa-databroker.sock",
		"/tmp/kuksa-databroker.sock",
	}
	for _, p := range paths {
		if _, err := os.Stat(p); err == nil {
			return p
		}
	}
	// Default to the expected host path per compose volume mount.
	return "/tmp/kuksa/kuksa-databroker.sock"
}

// udsTarget returns the gRPC target string for UDS connection.
func udsTarget() string {
	return "unix://" + effectiveUDSSocket()
}

// skipIfTCPUnreachable skips the test if the databroker TCP port is not reachable.
func skipIfTCPUnreachable(t *testing.T) {
	t.Helper()
	conn, err := net.DialTimeout("tcp", tcpTarget, 2*time.Second)
	if err != nil {
		t.Skipf("databroker TCP port %s not reachable, skipping: %v", tcpTarget, err)
	}
	conn.Close()
}

// skipIfUDSUnreachable skips the test if the databroker UDS socket is not reachable.
func skipIfUDSUnreachable(t *testing.T) {
	t.Helper()
	sock := effectiveUDSSocket()
	if _, err := os.Stat(sock); os.IsNotExist(err) {
		t.Skipf("UDS socket %s does not exist, skipping", sock)
	}
	conn, err := net.DialTimeout("unix", sock, 2*time.Second)
	if err != nil {
		t.Skipf("UDS socket %s not reachable, skipping: %v", sock, err)
	}
	conn.Close()
}

// dialTCP establishes a gRPC connection to the databroker via TCP.
func dialTCP(t *testing.T) (*grpc.ClientConn, kuksa.VALClient) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	conn, err := grpc.DialContext(ctx, tcpTarget, //nolint:staticcheck // DialContext is fine for tests
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(), //nolint:staticcheck // WithBlock is fine for tests
	)
	if err != nil {
		t.Fatalf("failed to dial TCP %s: %v", tcpTarget, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn, kuksa.NewVALClient(conn)
}

// dialUDS establishes a gRPC connection to the databroker via UDS.
func dialUDS(t *testing.T) (*grpc.ClientConn, kuksa.VALClient) {
	t.Helper()
	target := udsTarget()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	conn, err := grpc.DialContext(ctx, target, //nolint:staticcheck // DialContext is fine for tests
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(), //nolint:staticcheck // WithBlock is fine for tests
	)
	if err != nil {
		t.Fatalf("failed to dial UDS %s: %v", target, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn, kuksa.NewVALClient(conn)
}

// publishValue sets a signal value using the PublishValue RPC.
func publishValue(t *testing.T, client kuksa.VALClient, path string, value *kuksa.Value) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	_, err := client.PublishValue(ctx, &kuksa.PublishValueRequest{
		SignalId:  &kuksa.SignalID{Signal: &kuksa.SignalID_Path{Path: path}},
		DataPoint: &kuksa.Datapoint{Timestamp: timestamppb.Now(), Value: value},
	})
	if err != nil {
		t.Fatalf("failed to publish %s: %v", path, err)
	}
}

// getValueOrFail gets a signal value using the GetValue RPC; fails the test on error.
func getValueOrFail(t *testing.T, client kuksa.VALClient, path string) *kuksa.Datapoint {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	resp, err := client.GetValue(ctx, &kuksa.GetValueRequest{
		SignalId: &kuksa.SignalID{Signal: &kuksa.SignalID_Path{Path: path}},
	})
	if err != nil {
		t.Fatalf("failed to get %s: %v", path, err)
	}
	return resp.GetDataPoint()
}

// listMetadataOrFail queries metadata for a signal path; fails the test on error.
func listMetadataOrFail(t *testing.T, client kuksa.VALClient, root string) []*kuksa.Metadata {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	resp, err := client.ListMetadata(ctx, &kuksa.ListMetadataRequest{
		Root:   root,
		Filter: "",
	})
	if err != nil {
		t.Fatalf("failed to list metadata for %s: %v", root, err)
	}
	return resp.GetMetadata()
}

// boolValue creates a Value with a bool typed value.
func boolValue(v bool) *kuksa.Value {
	return &kuksa.Value{TypedValue: &kuksa.Value_Bool{Bool: v}}
}

// floatValue creates a Value with a float32 typed value.
func floatValue(v float32) *kuksa.Value {
	return &kuksa.Value{TypedValue: &kuksa.Value_Float{Float: v}}
}

// doubleValue creates a Value with a float64 typed value.
func doubleValue(v float64) *kuksa.Value {
	return &kuksa.Value{TypedValue: &kuksa.Value_Double{Double: v}}
}

// stringValue creates a Value with a string typed value.
func stringValue(v string) *kuksa.Value {
	return &kuksa.Value{TypedValue: &kuksa.Value_String_{String_: v}}
}

// testValueForType returns a representative test value for a given VSS data type.
// Used by metadata tests to validate type correctness via set/get roundtrip.
func testValueForType(dt kuksa.DataType) *kuksa.Value {
	switch dt {
	case kuksa.DataType_DATA_TYPE_BOOLEAN:
		return boolValue(true)
	case kuksa.DataType_DATA_TYPE_FLOAT:
		return floatValue(42.0)
	case kuksa.DataType_DATA_TYPE_DOUBLE:
		return doubleValue(48.1351)
	case kuksa.DataType_DATA_TYPE_STRING:
		return stringValue("test-value")
	default:
		return nil
	}
}

// assertValueMatchesType verifies that a retrieved datapoint holds a value of
// the expected VSS data type. This is a type-level check (the value has the
// correct oneof variant), not a value-equality check.
func assertValueMatchesType(t *testing.T, path string, dt kuksa.DataType, dp *kuksa.Datapoint) {
	t.Helper()
	if dp == nil || dp.GetValue() == nil {
		t.Errorf("signal %s: datapoint value is nil after set/get roundtrip", path)
		return
	}
	val := dp.GetValue()
	switch dt {
	case kuksa.DataType_DATA_TYPE_BOOLEAN:
		if val.GetTypedValue() == nil {
			t.Errorf("signal %s: expected bool value, got nil typed value", path)
		} else if _, ok := val.GetTypedValue().(*kuksa.Value_Bool); !ok {
			t.Errorf("signal %s: expected bool typed value, got %T", path, val.GetTypedValue())
		}
	case kuksa.DataType_DATA_TYPE_FLOAT:
		if val.GetTypedValue() == nil {
			t.Errorf("signal %s: expected float value, got nil typed value", path)
		} else if _, ok := val.GetTypedValue().(*kuksa.Value_Float); !ok {
			t.Errorf("signal %s: expected float typed value, got %T", path, val.GetTypedValue())
		}
	case kuksa.DataType_DATA_TYPE_DOUBLE:
		if val.GetTypedValue() == nil {
			t.Errorf("signal %s: expected double value, got nil typed value", path)
		} else if _, ok := val.GetTypedValue().(*kuksa.Value_Double); !ok {
			t.Errorf("signal %s: expected double typed value, got %T", path, val.GetTypedValue())
		}
	case kuksa.DataType_DATA_TYPE_STRING:
		if val.GetTypedValue() == nil {
			t.Errorf("signal %s: expected string value, got nil typed value", path)
		} else if _, ok := val.GetTypedValue().(*kuksa.Value_String_); !ok {
			t.Errorf("signal %s: expected string typed value, got %T", path, val.GetTypedValue())
		}
	}
}

// drainStream reads and discards one initial notification from a subscription
// stream using a goroutine so the caller is not blocked. If no notification
// arrives within timeout the call returns gracefully. Any leaked goroutine is
// cleaned up when the stream's parent context is cancelled at test cleanup.
func drainStream(stream kuksa.VAL_SubscribeClient, timeout time.Duration) {
	ch := make(chan struct{}, 1)
	go func() {
		_, _ = stream.Recv() // discard initial current-value notification
		ch <- struct{}{}
	}()
	select {
	case <-ch:
	case <-time.After(timeout):
	}
}
