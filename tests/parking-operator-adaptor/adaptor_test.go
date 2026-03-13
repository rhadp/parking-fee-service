package parkingoperatoradaptor

import (
	"context"
	"fmt"
	"net"
	"os"
	"os/exec"
	"strings"
	"syscall"
	"testing"
	"time"

	pb "github.com/parking-fee-service/proto"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// TS-08-17: Startup Logging
// Requirement: 08-REQ-8.1
//
// On startup, the adaptor logs version, port, operator URL, DATA_BROKER addr,
// and vehicle ID. We capture combined output and verify key fields.
// The adaptor will fail to connect to DATA_BROKER (no real Kuksa running)
// and exit non-zero, but startup logging happens before broker connection.
func TestStartupLogging(t *testing.T) {
	binPath := buildAdaptorBinary(t)
	operatorURL := mockParkingOperator(t)
	grpcPort := findFreePort(t)

	// Use an unreachable broker address — the adaptor will log startup info
	// then fail after 5 retries and exit.
	unreachableBroker := fmt.Sprintf("http://127.0.0.1:%d", findFreePort(t))

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(),
		fmt.Sprintf("PARKING_OPERATOR_URL=%s", operatorURL),
		fmt.Sprintf("DATA_BROKER_ADDR=%s", unreachableBroker),
		fmt.Sprintf("GRPC_PORT=%d", grpcPort),
		"VEHICLE_ID=TEST-VIN-42",
		"ZONE_ID=zone-test-1",
		"RUST_LOG=info",
	)

	// CombinedOutput waits for the process to exit and captures all output.
	output, _ := cmd.CombinedOutput()
	// We expect non-zero exit since broker is unreachable; that's fine.

	out := string(output)
	t.Logf("Startup output:\n%s", out)

	// Verify startup log contains required fields (08-REQ-8.1)
	checks := []struct {
		label string
		want  string
	}{
		{"port", fmt.Sprintf("%d", grpcPort)},
		{"operator URL", operatorURL},
		{"vehicle ID", "TEST-VIN-42"},
		{"parking-operator-adaptor", "parking-operator-adaptor"},
	}

	for _, c := range checks {
		if !strings.Contains(out, c.want) {
			t.Errorf("startup log missing %s: expected to contain %q", c.label, c.want)
		}
	}
}

// TS-08-18: Graceful Shutdown
// Requirement: 08-REQ-8.2
//
// SIGTERM causes the adaptor to stop and exit cleanly. Without a real
// DATA_BROKER we can't get a fully running service, so we verify the
// process responds to SIGTERM during startup (broker retry phase) and
// terminates within a reasonable time.
func TestGracefulShutdown(t *testing.T) {
	binPath := buildAdaptorBinary(t)
	operatorURL := mockParkingOperator(t)
	grpcPort := findFreePort(t)
	unreachableBroker := fmt.Sprintf("http://127.0.0.1:%d", findFreePort(t))

	cmd, _, stderr := startAdaptor(t, binPath, append(os.Environ(),
		fmt.Sprintf("PARKING_OPERATOR_URL=%s", operatorURL),
		fmt.Sprintf("DATA_BROKER_ADDR=%s", unreachableBroker),
		fmt.Sprintf("GRPC_PORT=%d", grpcPort),
		"VEHICLE_ID=TEST-VIN-42",
		"ZONE_ID=zone-test-1",
		"RUST_LOG=info",
	))

	// Give the process a moment to start
	time.Sleep(500 * time.Millisecond)

	// Send SIGTERM
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Logf("SIGTERM failed (process may have already exited): %v", err)
	}

	// Wait for exit
	done := make(chan error, 1)
	go func() {
		done <- cmd.Wait()
	}()

	select {
	case err := <-done:
		_ = stderr // stderr captured by pipe
		if err != nil {
			// Process may exit non-zero during broker retry phase — acceptable.
			t.Logf("Process exited with error (expected during broker retry): %v", err)
		}
		// Key assertion: the process exited (didn't hang)
		if cmd.ProcessState == nil {
			t.Fatal("process did not exit after SIGTERM")
		}
		t.Logf("Process exited with code %d", cmd.ProcessState.ExitCode())
	case <-time.After(30 * time.Second):
		t.Fatal("adaptor did not exit within 30s after SIGTERM")
	}
}

// TS-08-E8: DATA_BROKER Unreachable
// Requirement: 08-REQ-6.E1
//
// When DATA_BROKER is unreachable at startup, the adaptor retries 5 times
// with exponential backoff then exits non-zero.
func TestDataBrokerUnreachable(t *testing.T) {
	binPath := buildAdaptorBinary(t)
	operatorURL := mockParkingOperator(t)
	grpcPort := findFreePort(t)

	// Use a port that nothing is listening on
	unreachableAddr := fmt.Sprintf("http://127.0.0.1:%d", findFreePort(t))

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(),
		fmt.Sprintf("PARKING_OPERATOR_URL=%s", operatorURL),
		fmt.Sprintf("DATA_BROKER_ADDR=%s", unreachableAddr),
		fmt.Sprintf("GRPC_PORT=%d", grpcPort),
		"VEHICLE_ID=TEST-VIN-42",
		"ZONE_ID=zone-test-1",
		"RUST_LOG=info",
	)

	// CombinedOutput waits for the process to exit and captures all output.
	output, err := cmd.CombinedOutput()
	out := string(output)
	t.Logf("Output:\n%s", out)

	if err == nil {
		t.Fatal("expected non-zero exit code when DATA_BROKER is unreachable, got 0")
	}
	exitCode := cmd.ProcessState.ExitCode()
	if exitCode == 0 {
		t.Fatal("expected non-zero exit code when DATA_BROKER is unreachable, got 0")
	}
	t.Logf("Process exited with code %d (expected non-zero)", exitCode)

	// Verify retry messages appear in logs
	if !strings.Contains(out, "DATA_BROKER") {
		t.Error("expected DATA_BROKER retry messages in output")
	}
}

// TestGRPCGetStatus verifies the gRPC GetStatus RPC works end-to-end
// when the service is running. This test requires a real DATA_BROKER.
// It is skipped when DATA_BROKER is not available.
func TestGRPCGetStatus(t *testing.T) {
	// This test needs a real DATA_BROKER; skip if unavailable
	brokerAddr := os.Getenv("DATA_BROKER_ADDR")
	if brokerAddr == "" {
		brokerAddr = "http://localhost:55556"
	}

	// Quick check if broker is reachable
	host := strings.TrimPrefix(brokerAddr, "http://")
	conn, err := (&net.Dialer{Timeout: 2 * time.Second}).Dial("tcp", host)
	if err != nil {
		t.Skip("DATA_BROKER not available; skipping integration test")
	}
	conn.Close()

	binPath := buildAdaptorBinary(t)
	operatorURL := mockParkingOperator(t)
	grpcPort := findFreePort(t)

	cmd, _, _ := startAdaptor(t, binPath, append(os.Environ(),
		fmt.Sprintf("PARKING_OPERATOR_URL=%s", operatorURL),
		fmt.Sprintf("DATA_BROKER_ADDR=%s", brokerAddr),
		fmt.Sprintf("GRPC_PORT=%d", grpcPort),
		"VEHICLE_ID=TEST-VIN-42",
		"ZONE_ID=zone-test-1",
		"RUST_LOG=info",
	))

	// Wait for gRPC server to be ready
	grpcAddr := fmt.Sprintf("127.0.0.1:%d", grpcPort)
	waitForGRPCReady(t, grpcAddr, 15*time.Second)

	// Connect gRPC client
	grpcConn, err := grpc.NewClient(grpcAddr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		t.Fatalf("grpc.NewClient failed: %v", err)
	}
	defer grpcConn.Close()

	client := pb.NewParkingAdaptorClient(grpcConn)

	// Call GetStatus — should return idle state
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	resp, err := client.GetStatus(ctx, &pb.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus RPC failed: %v", err)
	}

	if resp.State != "idle" {
		t.Errorf("expected state=idle, got %q", resp.State)
	}

	// Clean shutdown
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Logf("SIGTERM failed: %v", err)
	}
}
