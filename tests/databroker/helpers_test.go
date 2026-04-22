package databroker_test

import (
	"context"
	"net"
	"os"
	"path/filepath"
	"testing"
	"time"

	pb "github.com/rhadp/parking-fee-service/gen/kuksa/val/v1"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// tcpTarget returns the host:port for the DATA_BROKER TCP listener.
const tcpTarget = "localhost:55556"

// udsSocketPaths lists possible host-side UDS socket paths.
// The container creates the socket at /tmp/kuksa-databroker.sock inside the
// container. Depending on volume mount configuration, the host-side path may
// be /tmp/kuksa/kuksa-databroker.sock (bind mount of /tmp/kuksa at /tmp) or
// /tmp/kuksa-databroker.sock (direct mount).
var udsSocketPaths = []string{
	"/tmp/kuksa/kuksa-databroker.sock",
	"/tmp/kuksa-databroker.sock",
}

// repoRoot returns the absolute path to the repository root directory.
func repoRoot(t *testing.T) string {
	t.Helper()
	dir, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get working directory: %v", err)
	}
	for {
		if _, err := os.Stat(filepath.Join(dir, "Makefile")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("could not find repository root (no Makefile found)")
		}
		dir = parent
	}
}

// effectiveUDSSocket returns the first existing UDS socket path from the
// candidate list, or an empty string if none exist.
func effectiveUDSSocket() string {
	for _, p := range udsSocketPaths {
		if _, err := os.Stat(p); err == nil {
			return p
		}
	}
	return ""
}

// skipIfTCPUnreachable skips the test if the DATA_BROKER TCP port is not
// reachable within 2 seconds.
func skipIfTCPUnreachable(t *testing.T) {
	t.Helper()
	conn, err := net.DialTimeout("tcp", tcpTarget, 2*time.Second)
	if err != nil {
		t.Skipf("DATA_BROKER TCP port %s not reachable: %v (container not running?)", tcpTarget, err)
	}
	conn.Close()
}

// skipIfUDSUnreachable skips the test if no UDS socket file exists.
func skipIfUDSUnreachable(t *testing.T) string {
	t.Helper()
	sock := effectiveUDSSocket()
	if sock == "" {
		t.Skipf("DATA_BROKER UDS socket not found at any of %v (container not running?)", udsSocketPaths)
	}
	return sock
}

// connectTCP creates a gRPC client connection to the DATA_BROKER via TCP.
// The connection is closed automatically when the test completes.
func connectTCP(t *testing.T) *grpc.ClientConn {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	conn, err := grpc.DialContext(ctx, tcpTarget,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		t.Fatalf("failed to connect to DATA_BROKER via TCP at %s: %v", tcpTarget, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn
}

// connectUDS creates a gRPC client connection to the DATA_BROKER via UDS.
// The connection is closed automatically when the test completes.
func connectUDS(t *testing.T, sockPath string) *grpc.ClientConn {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	conn, err := grpc.DialContext(ctx, "unix://"+sockPath,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		t.Fatalf("failed to connect to DATA_BROKER via UDS at %s: %v", sockPath, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn
}

// newVALClient creates a VAL gRPC client from a connection.
func newVALClient(conn *grpc.ClientConn) pb.VALClient {
	return pb.NewVALClient(conn)
}

// signalSpec describes an expected VSS signal with its path and data type.
type signalSpec struct {
	Path     string
	DataType pb.DataType
}

// standardSignals are the 5 standard VSS signals expected in the DATA_BROKER.
var standardSignals = []signalSpec{
	{"Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", pb.DataType_DATA_TYPE_BOOLEAN},
	{"Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", pb.DataType_DATA_TYPE_BOOLEAN},
	{"Vehicle.CurrentLocation.Latitude", pb.DataType_DATA_TYPE_DOUBLE},
	{"Vehicle.CurrentLocation.Longitude", pb.DataType_DATA_TYPE_DOUBLE},
	{"Vehicle.Speed", pb.DataType_DATA_TYPE_FLOAT},
}

// customSignals are the 3 custom VSS signals from the overlay.
var customSignals = []signalSpec{
	{"Vehicle.Parking.SessionActive", pb.DataType_DATA_TYPE_BOOLEAN},
	{"Vehicle.Command.Door.Lock", pb.DataType_DATA_TYPE_STRING},
	{"Vehicle.Command.Door.Response", pb.DataType_DATA_TYPE_STRING},
}

// allSignals is the combined list of all 8 expected VSS signals.
var allSignals = append(append([]signalSpec{}, standardSignals...), customSignals...)

// getMetadata sends a Get request with VIEW_METADATA for a signal and returns
// the DataEntry from the response. Returns an error if the request fails or
// the signal is not found.
func getMetadata(t *testing.T, client pb.VALClient, path string) *pb.DataEntry {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	resp, err := client.Get(ctx, &pb.GetRequest{
		Entries: []*pb.EntryRequest{
			{
				Path:   path,
				View:   pb.View_VIEW_METADATA,
				Fields: []pb.Field{pb.Field_FIELD_METADATA},
			},
		},
	})
	if err != nil {
		t.Fatalf("GetMetadata(%q) failed: %v", path, err)
	}
	if len(resp.Entries) == 0 {
		t.Fatalf("GetMetadata(%q) returned no entries", path)
	}
	return resp.Entries[0]
}

// setValue sets a signal value via a VAL client using FIELD_VALUE.
func setValue(t *testing.T, client pb.VALClient, path string, dp *pb.Datapoint) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	_, err := client.Set(ctx, &pb.SetRequest{
		Updates: []*pb.EntryUpdate{
			{
				Entry: &pb.DataEntry{
					Path:  path,
					Value: dp,
				},
				Fields: []pb.Field{pb.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		t.Fatalf("Set(%q) failed: %v", path, err)
	}
}

// getValue gets the current value of a signal via a VAL client.
func getValue(t *testing.T, client pb.VALClient, path string) *pb.DataEntry {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	resp, err := client.Get(ctx, &pb.GetRequest{
		Entries: []*pb.EntryRequest{
			{
				Path:   path,
				View:   pb.View_VIEW_CURRENT_VALUE,
				Fields: []pb.Field{pb.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		t.Fatalf("Get(%q) failed: %v", path, err)
	}
	if len(resp.Entries) == 0 {
		t.Fatalf("Get(%q) returned no entries", path)
	}
	return resp.Entries[0]
}
