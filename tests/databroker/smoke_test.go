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

// TestSmokeHealthCheck is a quick smoke test that verifies the DATA_BROKER
// container starts and accepts TCP connections.
// TS-02-SMOKE-1 | Requirement: 02-REQ-1.1, 02-REQ-2.1
func TestSmokeHealthCheck(t *testing.T) {
	// Check if the databroker is already running.
	conn, err := net.DialTimeout("tcp", tcpTarget, 2*time.Second)
	if err != nil {
		// Databroker not running — try to start it only if compose.yml is
		// properly configured (TG2 applied).
		skipIfComposeNotConfigured(t)
		skipIfPodmanNotRunning(t)

		root := repoRoot(t)
		ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
		defer cancel()

		cmd := exec.CommandContext(ctx, "podman", "compose", "up", "-d", "kuksa-databroker")
		cmd.Dir = filepath.Join(root, "deployments")
		output, err := cmd.CombinedOutput()
		if err != nil {
			t.Skipf("failed to start databroker: %v\n%s", err, output)
		}

		// Ensure cleanup on test completion (uses composeDown for socket cleanup).
		t.Cleanup(func() {
			composeDown(t, filepath.Join(root, "deployments"))
		})

		// Wait for the TCP port to become available (up to 10 seconds).
		deadline := time.Now().Add(10 * time.Second)
		for time.Now().Before(deadline) {
			conn, err = net.DialTimeout("tcp", tcpTarget, 1*time.Second)
			if err == nil {
				conn.Close()
				break
			}
			time.Sleep(500 * time.Millisecond)
		}
		if err != nil {
			t.Fatalf("databroker TCP port not available within 10s after compose up: %v", err)
		}
	} else {
		conn.Close()
	}

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

	// Send a metadata query via the v1 VAL API. The kuksa-databroker 0.5.0
	// serves kuksa.val.v2, so v1 Get calls may return empty entries without
	// error (see errata 02_data_broker_spec_contradictions.md §7). The key
	// assertion is that the gRPC channel is functional (no transport error).
	client := pb.NewVALClient(grpcConn)
	_, err = client.Get(ctx, &pb.GetRequest{
		Entries: []*pb.EntryRequest{
			{
				Path:   "Vehicle.Speed",
				View:   pb.View_VIEW_METADATA,
				Fields: []pb.Field{pb.Field_FIELD_METADATA},
			},
		},
	})
	if err != nil {
		t.Fatalf("gRPC request failed (transport-level): %v", err)
	}
	// gRPC channel is functional — health check passed.
}

// TestSmokeFullSignalInventory is a quick smoke test that verifies all 8
// expected VSS signals are present in the DATA_BROKER.
// TS-02-SMOKE-2 | Requirement: 02-REQ-5.1, 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3
func TestSmokeFullSignalInventory(t *testing.T) {
	skipIfTCPUnreachable(t)
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

	missing := []string{}
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
				t.Errorf("signal %s not found: %v", path, err)
				missing = append(missing, path)
				return
			}
			if len(resp.Entries) == 0 {
				t.Errorf("signal %s returned no entries", path)
				missing = append(missing, path)
			}
		})
	}

	if len(missing) > 0 {
		t.Errorf("missing signals: %v", missing)
	}
}
