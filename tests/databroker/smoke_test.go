package databroker_test

import (
	"context"
	"net"
	"os/exec"
	"path/filepath"
	"testing"
	"time"

	pb "github.com/rhadp/parking-fee-service/gen/kuksa/val/v1"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// ensureDatabrokerRunning starts the DATA_BROKER container if it is not
// already reachable on the TCP port. It waits up to 10 seconds for the
// port to become available and registers a t.Cleanup to tear down the
// container when the test completes. If the container is already running,
// this is a no-op and no cleanup is registered.
func ensureDatabrokerRunning(t *testing.T) {
	t.Helper()

	conn, err := net.DialTimeout("tcp", tcpTarget, 2*time.Second)
	if err == nil {
		// Already running — nothing to do.
		conn.Close()
		return
	}

	// Databroker not running — try to start it only if compose.yml is
	// properly configured (TG2 applied) and podman is available.
	skipIfComposeNotConfigured(t)
	skipIfPodmanNotRunning(t)

	root := repoRoot(t)
	deployDir := filepath.Join(root, "deployments")

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	cmd := exec.CommandContext(ctx, "podman", "compose", "up", "-d", "kuksa-databroker")
	cmd.Dir = deployDir
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Skipf("failed to start databroker: %v\n%s", err, output)
	}

	// Ensure cleanup on test completion (uses composeDown for socket cleanup).
	t.Cleanup(func() {
		composeDown(t, deployDir)
	})

	// Wait for the TCP port to become available (up to 10 seconds).
	deadline := time.Now().Add(10 * time.Second)
	for time.Now().Before(deadline) {
		conn, err = net.DialTimeout("tcp", tcpTarget, 1*time.Second)
		if err == nil {
			conn.Close()
			return
		}
		time.Sleep(500 * time.Millisecond)
	}
	t.Fatalf("databroker TCP port not available within 10s after compose up: %v", err)
}

// TestSmokeHealthCheck is a quick smoke test that verifies the DATA_BROKER
// container starts and accepts TCP connections.
// TS-02-SMOKE-1 | Requirement: 02-REQ-1.1, 02-REQ-2.1
func TestSmokeHealthCheck(t *testing.T) {
	ensureDatabrokerRunning(t)

	// Verify gRPC connectivity: establish a channel to the DATA_BROKER.
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	grpcConn, err := grpc.DialContext(ctx, tcpTarget,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		t.Fatalf("gRPC connection to DATA_BROKER failed: %v", err)
	}
	defer grpcConn.Close()

	// Send a metadata query via the v1 VAL API. kuksa-databroker 0.5.0
	// serves both v1 and v2 APIs simultaneously (see errata §6 in
	// docs/errata/02_data_broker_spec_contradictions.md). The v1 API
	// (kuksa.val.v1.VAL) should return populated entries.
	client := pb.NewVALClient(grpcConn)
	resp, err := client.Get(ctx, &pb.GetRequest{
		Entries: []*pb.EntryRequest{
			{
				Path:   "Vehicle.Speed",
				View:   pb.View_VIEW_METADATA,
				Fields: []pb.Field{pb.Field_FIELD_METADATA},
			},
		},
	})
	if err != nil {
		t.Fatalf("gRPC metadata request failed: %v", err)
	}

	// Verify that the response contains at least one entry. An empty
	// response indicates a v1/v2 API compatibility issue (see errata §6).
	if len(resp.Entries) == 0 {
		t.Fatal("gRPC metadata request returned 0 entries for Vehicle.Speed; " +
			"the DATA_BROKER may not be serving the kuksa.val.v1 API correctly " +
			"(see docs/errata/02_data_broker_spec_contradictions.md §6)")
	}

	t.Logf("health check passed: Vehicle.Speed metadata entry path=%q", resp.Entries[0].Path)
}

// TestSmokeFullSignalInventory is a quick smoke test that verifies all 8
// expected VSS signals are present in the DATA_BROKER.
// TS-02-SMOKE-2 | Requirement: 02-REQ-5.1, 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3
func TestSmokeFullSignalInventory(t *testing.T) {
	ensureDatabrokerRunning(t)
	conn := connectTCP(t)
	client := newVALClient(conn)

	signalPaths := []string{
		"Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
		"Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
		"Vehicle.CurrentLocation.Latitude",
		"Vehicle.CurrentLocation.Longitude",
		"Vehicle.Speed",
		"Vehicle.Parking.SessionActive",
		"Vehicle.Command.Door.Lock",
		"Vehicle.Command.Door.Response",
	}

	foundCount := 0
	for _, path := range signalPaths {
		t.Run(path, func(t *testing.T) {
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
				t.Errorf("signal %s: metadata query failed: %v", path, err)
				return
			}
			if len(resp.Entries) == 0 {
				t.Errorf("signal %s: returned no entries (missing from VSS tree)", path)
				return
			}
			foundCount++
		})
	}

	if foundCount != len(signalPaths) {
		t.Errorf("signal inventory incomplete: found %d of %d expected signals",
			foundCount, len(signalPaths))
	}
}
