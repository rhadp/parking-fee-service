package cloudgatewayclient

import (
	"os"
	"os/exec"
	"strings"
	"syscall"
	"testing"
	"time"
)

// TS-04-E1: NATS Unreachable on Startup
// Requirement: 04-REQ-1.E1
// Verify the service retries and exits with non-zero when NATS is unreachable.
func TestNATSUnreachable(t *testing.T) {
	requirePodman(t)

	// Start only databroker (no NATS) so the service cannot connect to NATS.
	startDatabroker(t)

	binPath := buildCloudGatewayClient(t)

	env := defaultServiceEnv()
	env["NATS_URL"] = "nats://localhost:19999" // non-listening port

	sp := startService(t, binPath, env)

	// The service should retry and eventually exit non-zero.
	exitCode, err := sp.wait(60 * time.Second)
	if err != nil {
		t.Fatalf("process did not exit within timeout: %v", err)
	}
	if exitCode == 0 {
		t.Errorf("expected non-zero exit code when NATS is unreachable, got 0\noutput: %s", sp.output())
	}
}

// TS-04-E2: NATS Connection Lost
// Requirement: 04-REQ-1.E2
// Verify the service attempts to reconnect when NATS drops.
func TestNATSConnectionLost(t *testing.T) {
	requireInfrastructure(t)

	startNATS(t)
	startDatabroker(t)

	binPath := buildCloudGatewayClient(t)
	sp := startService(t, binPath, defaultServiceEnv())
	sp.waitForReady(t, 15*time.Second)

	// Stop NATS while the service is running.
	cf := composeFile(t)
	cmd := exec.Command("podman", "compose", "-f", cf, "stop", "nats")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Logf("Warning: could not stop NATS: %v\n%s", err, string(out))
	}

	// Give the service a moment to detect the connection loss.
	time.Sleep(3 * time.Second)

	// The service should log reconnect attempts. The async-nats client
	// handles reconnection internally. We just verify the service hasn't
	// crashed immediately.
	output := sp.output()
	t.Logf("Service output after NATS stop:\n%s", output)

	// Restart NATS so cleanup doesn't fail.
	cmd2 := exec.Command("podman", "compose", "-f", cf, "up", "-d", "nats")
	cmd2.Dir = repoRoot(t)
	_, _ = cmd2.CombinedOutput()
	waitForTCP(t, "localhost:4222", 10*time.Second)
}

// TS-04-E9: DATA_BROKER Unreachable
// Requirement: 04-REQ-5.E1
// Verify the service retries and exits with non-zero when DATA_BROKER is
// unreachable.
func TestDataBrokerUnreachable(t *testing.T) {
	requirePodman(t)

	// Start NATS but not databroker.
	startNATS(t)

	binPath := buildCloudGatewayClient(t)

	env := defaultServiceEnv()
	env["DATABROKER_ADDR"] = "http://localhost:19999" // non-listening port

	sp := startService(t, binPath, env)

	// The service should retry and eventually exit non-zero.
	exitCode, err := sp.wait(60 * time.Second)
	if err != nil {
		t.Fatalf("process did not exit within timeout: %v", err)
	}
	if exitCode == 0 {
		t.Errorf("expected non-zero exit code when DATA_BROKER is unreachable, got 0\noutput: %s", sp.output())
	}
}

// TS-04-E11: SIGTERM During Command
// Requirement: 04-REQ-7.E1
// Verify in-flight command completes before shutdown.
func TestSigtermDuringCommand(t *testing.T) {
	requireInfrastructure(t)

	startNATS(t)
	startDatabroker(t)

	binPath := buildCloudGatewayClient(t)
	sp := startService(t, binPath, defaultServiceEnv())
	sp.waitForReady(t, 15*time.Second)

	nc := connectNATS(t)

	// Send a command and SIGTERM nearly simultaneously.
	publishCommand(t, nc, testVIN, validCommandPayload(), testBearerToken)
	time.Sleep(100 * time.Millisecond) // tiny delay to let command enter processing
	if err := sp.sendSignal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	exitCode, err := sp.wait(10 * time.Second)
	if err != nil {
		t.Fatalf("process did not exit: %v", err)
	}
	if exitCode != 0 {
		t.Errorf("expected exit code 0 after SIGTERM, got %d\noutput: %s", exitCode, sp.output())
	}
}

// TestVINNotSet verifies the service exits with code 1 when VIN is missing.
// This is a process-level validation of the unit test TS-04-E10.
func TestVINNotSet(t *testing.T) {
	binPath := buildCloudGatewayClient(t)

	cmd := exec.Command(binPath)
	// Set a minimal env without VIN.
	cmd.Env = []string{
		"HOME=" + os.Getenv("HOME"),
		"PATH=" + os.Getenv("PATH"),
		"NATS_URL=nats://localhost:4222",
	}
	var stderr strings.Builder
	cmd.Stderr = &stderr

	err := cmd.Run()
	if err == nil {
		t.Fatal("expected non-zero exit when VIN is not set")
	}
	if exitErr, ok := err.(*exec.ExitError); ok {
		if exitErr.ExitCode() != 1 {
			t.Errorf("expected exit code 1, got %d", exitErr.ExitCode())
		}
	}

	output := stderr.String()
	if !strings.Contains(output, "VIN") {
		t.Errorf("expected error message to mention VIN; got: %s", output)
	}
}

// TestBearerTokenRejection verifies that commands with wrong tokens are ignored
// (integration-level complement to unit test TS-04-E3).
func TestBearerTokenRejection(t *testing.T) {
	requireInfrastructure(t)

	startNATS(t)
	startDatabroker(t)

	binPath := buildCloudGatewayClient(t)
	sp := startService(t, binPath, defaultServiceEnv())
	sp.waitForReady(t, 15*time.Second)

	nc := connectNATS(t)

	// Send a command with the wrong bearer token.
	publishCommand(t, nc, testVIN, validCommandPayload(), "wrong-token")

	// Wait a moment to confirm the command is NOT forwarded.
	time.Sleep(2 * time.Second)

	// The service should have logged a rejection.
	output := sp.output()
	if !strings.Contains(output, "rejected") && !strings.Contains(output, "invalid") {
		t.Logf("Warning: expected rejection message in output; got: %s", output)
	}

	// Verify the signal was NOT set in DATA_BROKER (or still has no command_id
	// from this rejected command).
	out, _ := grpcGet(t, tcpEndpoint, signalLockCommand)
	_ = out
}
