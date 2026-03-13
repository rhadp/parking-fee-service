package cloudgatewayclient

import (
	"fmt"
	"strings"
	"syscall"
	"testing"
	"time"
)

// TS-04-2: NATS Command Subscription
// Requirement: 04-REQ-1.2
// Verify the service subscribes to vehicles.{VIN}.commands and forwards valid
// commands to DATA_BROKER (visible via the Vehicle.Command.Door.Lock signal).
func TestNATSCommandSubscription(t *testing.T) {
	requireInfrastructure(t)

	startNATS(t)
	startDatabroker(t)

	binPath := buildCloudGatewayClient(t)
	sp := startService(t, binPath, defaultServiceEnv())
	sp.waitForReady(t, 15*time.Second)

	nc := connectNATS(t)
	publishCommand(t, nc, testVIN, validCommandPayload(), testBearerToken)

	// Wait for the command to propagate to DATA_BROKER.
	time.Sleep(2 * time.Second)

	out, err := grpcGet(t, tcpEndpoint, signalLockCommand)
	if err != nil {
		t.Fatalf("grpcGet(%s) failed: %v\n%s", signalLockCommand, err, out)
	}
	if !strings.Contains(out, "test-cmd-001") {
		t.Errorf("Vehicle.Command.Door.Lock should contain command_id; got: %s", out)
	}
}

// TS-04-7: Response Subscription
// Requirement: 04-REQ-3.1
// Verify the service subscribes to Vehicle.Command.Door.Response and relays
// it to vehicles.{VIN}.command_responses on NATS.
func TestResponseRelay(t *testing.T) {
	requireInfrastructure(t)

	startNATS(t)
	startDatabroker(t)

	binPath := buildCloudGatewayClient(t)
	sp := startService(t, binPath, defaultServiceEnv())
	sp.waitForReady(t, 15*time.Second)

	nc := connectNATS(t)

	// Subscribe to command_responses before triggering the response signal.
	sub, err := nc.SubscribeSync(fmt.Sprintf("vehicles.%s.command_responses", testVIN))
	if err != nil {
		t.Fatalf("NATS subscribe failed: %v", err)
	}
	defer sub.Unsubscribe()
	nc.Flush()

	// Set the response signal in DATA_BROKER.
	responseJSON := `{"command_id":"test-cmd-001","status":"success","timestamp":1700000001}`
	if out, err := grpcSetString(t, tcpEndpoint, signalResponse, responseJSON); err != nil {
		t.Fatalf("grpcSetString(%s) failed: %v\n%s", signalResponse, err, out)
	}

	// Wait for the relayed response on NATS.
	msg, err := sub.NextMsg(5 * time.Second)
	if err != nil {
		t.Fatalf("did not receive command_responses message: %v", err)
	}
	if !strings.Contains(string(msg.Data), "test-cmd-001") {
		t.Errorf("expected response to contain command_id; got: %s", string(msg.Data))
	}
}

// TS-04-9: Telemetry Signal Subscription
// Requirement: 04-REQ-4.1
// Verify the service subscribes to telemetry signals and publishes aggregated
// telemetry to NATS.
func TestTelemetrySubscription(t *testing.T) {
	requireInfrastructure(t)

	startNATS(t)
	startDatabroker(t)

	binPath := buildCloudGatewayClient(t)
	sp := startService(t, binPath, defaultServiceEnv())
	sp.waitForReady(t, 15*time.Second)

	nc := connectNATS(t)

	// Subscribe to telemetry before triggering signal change.
	sub, err := nc.SubscribeSync(fmt.Sprintf("vehicles.%s.telemetry", testVIN))
	if err != nil {
		t.Fatalf("NATS subscribe failed: %v", err)
	}
	defer sub.Unsubscribe()
	nc.Flush()

	// Set IsLocked in DATA_BROKER.
	if out, err := grpcSetBool(t, tcpEndpoint, signalIsLocked, true); err != nil {
		t.Fatalf("grpcSetBool(%s) failed: %v\n%s", signalIsLocked, err, out)
	}

	// Wait for telemetry on NATS.
	msg, err := sub.NextMsg(5 * time.Second)
	if err != nil {
		t.Fatalf("did not receive telemetry message: %v", err)
	}
	parsed := parseJSON(t, msg.Data)
	if parsed["vin"] != testVIN {
		t.Errorf("telemetry vin mismatch: got %v, want %s", parsed["vin"], testVIN)
	}
	if parsed["is_locked"] != true {
		t.Errorf("telemetry is_locked should be true; got %v", parsed["is_locked"])
	}
}

// TS-04-14: Self-Registration on Startup
// Requirement: 04-REQ-6.2
// Verify the service publishes a registration message on startup.
func TestSelfRegistration(t *testing.T) {
	requireInfrastructure(t)

	startNATS(t)
	startDatabroker(t)

	nc := connectNATS(t)

	// Subscribe to status BEFORE starting the service.
	sub, err := nc.SubscribeSync(fmt.Sprintf("vehicles.%s.status", testVIN))
	if err != nil {
		t.Fatalf("NATS subscribe failed: %v", err)
	}
	defer sub.Unsubscribe()
	nc.Flush()

	binPath := buildCloudGatewayClient(t)
	sp := startService(t, binPath, defaultServiceEnv())
	sp.waitForReady(t, 15*time.Second)

	msg, err := sub.NextMsg(5 * time.Second)
	if err != nil {
		t.Fatalf("did not receive registration message: %v", err)
	}
	parsed := parseJSON(t, msg.Data)
	if parsed["vin"] != testVIN {
		t.Errorf("registration vin mismatch: got %v, want %s", parsed["vin"], testVIN)
	}
	if parsed["status"] != "online" {
		t.Errorf("registration status should be 'online'; got %v", parsed["status"])
	}
	ts, ok := parsed["timestamp"]
	if !ok || ts == nil {
		t.Error("registration message missing timestamp")
	}
	if tsFloat, ok := ts.(float64); ok && tsFloat <= 0 {
		t.Error("registration timestamp should be positive")
	}
}

// TS-04-15: Graceful Shutdown
// Requirement: 04-REQ-7.1
// Verify clean shutdown on SIGTERM.
func TestGracefulShutdown(t *testing.T) {
	requireInfrastructure(t)

	startNATS(t)
	startDatabroker(t)

	binPath := buildCloudGatewayClient(t)
	sp := startService(t, binPath, defaultServiceEnv())
	sp.waitForReady(t, 15*time.Second)

	if err := sp.sendSignal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	exitCode, err := sp.wait(5 * time.Second)
	if err != nil {
		t.Fatalf("process did not exit: %v", err)
	}
	if exitCode != 0 {
		t.Errorf("expected exit code 0, got %d\noutput: %s", exitCode, sp.output())
	}

	output := sp.output()
	if !strings.Contains(output, "shutdown") {
		t.Errorf("expected shutdown message in output; got: %s", output)
	}
}

// TS-04-16: Startup Logging
// Requirement: 04-REQ-7.2
// Verify startup log output contains VIN, NATS URL, and ready.
func TestStartupLogging(t *testing.T) {
	requireInfrastructure(t)

	startNATS(t)
	startDatabroker(t)

	binPath := buildCloudGatewayClient(t)
	sp := startService(t, binPath, defaultServiceEnv())
	sp.waitForReady(t, 15*time.Second)

	output := sp.output()
	if !strings.Contains(output, testVIN) {
		t.Errorf("startup output should contain VIN %q; got: %s", testVIN, output)
	}
	if !strings.Contains(output, "nats://localhost:4222") {
		t.Errorf("startup output should contain NATS URL; got: %s", output)
	}
	if !strings.Contains(output, "ready") {
		t.Errorf("startup output should contain 'ready'; got: %s", output)
	}
}

// TS-04-17: DATA_BROKER gRPC Operations
// Requirement: 04-REQ-5.2
// Verify the service can set and subscribe to DATA_BROKER signals.
func TestDataBrokerOperations(t *testing.T) {
	requireInfrastructure(t)

	startNATS(t)
	startDatabroker(t)

	binPath := buildCloudGatewayClient(t)
	sp := startService(t, binPath, defaultServiceEnv())
	sp.waitForReady(t, 15*time.Second)

	nc := connectNATS(t)

	// Send a valid command to trigger a DATA_BROKER set operation.
	publishCommand(t, nc, testVIN, validCommandPayload(), testBearerToken)

	// Allow time for the command to be processed.
	time.Sleep(2 * time.Second)

	// Verify the command was forwarded to DATA_BROKER.
	out, err := grpcGet(t, tcpEndpoint, signalLockCommand)
	if err != nil {
		t.Fatalf("grpcGet(%s) failed: %v\n%s", signalLockCommand, err, out)
	}
	if !strings.Contains(out, "test-cmd-001") {
		t.Errorf("Vehicle.Command.Door.Lock should contain command JSON; got: %s", out)
	}
}
