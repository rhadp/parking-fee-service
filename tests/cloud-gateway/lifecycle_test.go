package cloudgateway

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
	"testing"
	"time"
)

// ── TS-06-24: Startup Logging ─────────────────────────────────────────────────

// TestStartupLogging verifies that the service logs its port, NATS URL, and
// token count at startup.
// Requirement: 06-REQ-9.2
func TestStartupLogging(t *testing.T) {
	ensureNATS(t)

	cfg := defaultTestConfig()
	gp := startGateway(t, cfg)

	logs := gp.logs()

	if !gp.logContains(fmt.Sprintf("%d", cfg.Port)) {
		t.Errorf("expected port %d in startup logs; logs:\n%s", cfg.Port, logs)
	}
	if !gp.logContains("nats://") {
		t.Errorf("expected NATS URL in startup logs; logs:\n%s", logs)
	}
	if !gp.logContains("token") {
		t.Errorf("expected token count mention in startup logs; logs:\n%s", logs)
	}
}

// ── TS-06-25: Graceful Shutdown ───────────────────────────────────────────────

// TestGracefulShutdown verifies that SIGTERM causes graceful shutdown with
// exit code 0.
// Requirement: 06-REQ-9.3
func TestGracefulShutdown(t *testing.T) {
	ensureNATS(t)

	cfg := defaultTestConfig()
	gp := startGateway(t, cfg)

	gp.sendSIGTERM()

	code, ok := gp.waitForExit(10 * time.Second)
	if !ok {
		t.Fatal("cloud-gateway did not exit within timeout after SIGTERM")
	}
	if code != 0 {
		t.Errorf("expected exit code 0 after SIGTERM, got %d; logs:\n%s", code, gp.logs())
	}
}

// ── TS-06-E7: Invalid NATS Response JSON ─────────────────────────────────────

// TestInvalidNATSResponseJSON verifies that invalid JSON in a NATS response is
// logged and discarded without crashing the service.
// Requirement: 06-REQ-3.E1
func TestInvalidNATSResponseJSON(t *testing.T) {
	ensureNATS(t)

	cfg := defaultTestConfig()
	gp := startGateway(t, cfg)

	nc := connectNATS(t)

	// Publish invalid JSON to the command_responses subject.
	if err := nc.Publish(
		fmt.Sprintf("vehicles.%s.command_responses", testVIN),
		[]byte("{invalid json"),
	); err != nil {
		t.Fatalf("publish: %v", err)
	}
	_ = nc.Flush()

	// Wait briefly for the message to be processed.
	time.Sleep(500 * time.Millisecond)

	// Verify the service is still running (health endpoint responds).
	if !gp.waitForHealth(3 * time.Second) {
		t.Errorf("service stopped after invalid NATS response JSON; logs:\n%s", gp.logs())
	}
}

// ── TS-06-E8: Unknown Command ID in NATS Response ────────────────────────────

// TestUnknownCommandIDInNATS verifies that a NATS response with an unknown
// command_id is logged and discarded without crashing the service.
// Requirement: 06-REQ-3.E2
func TestUnknownCommandIDInNATS(t *testing.T) {
	ensureNATS(t)

	cfg := defaultTestConfig()
	gp := startGateway(t, cfg)

	nc := connectNATS(t)

	// Publish a response for a command that was never submitted.
	payload := `{"command_id":"nonexistent-cmd-999","status":"success"}`
	if err := nc.Publish(
		fmt.Sprintf("vehicles.%s.command_responses", testVIN),
		[]byte(payload),
	); err != nil {
		t.Fatalf("publish: %v", err)
	}
	_ = nc.Flush()

	time.Sleep(500 * time.Millisecond)

	// Verify the service is still running.
	if !gp.waitForHealth(3 * time.Second) {
		t.Errorf("service stopped after unknown command_id in NATS response; logs:\n%s", gp.logs())
	}
}

// ── TS-06-E9: Invalid Telemetry JSON ─────────────────────────────────────────

// TestInvalidTelemetryJSON verifies that invalid JSON in a NATS telemetry
// message is logged and discarded without crashing the service.
// Requirement: 06-REQ-5.E1
func TestInvalidTelemetryJSON(t *testing.T) {
	ensureNATS(t)

	cfg := defaultTestConfig()
	gp := startGateway(t, cfg)

	nc := connectNATS(t)

	// Publish invalid telemetry JSON.
	if err := nc.Publish(
		fmt.Sprintf("vehicles.%s.telemetry", testVIN),
		[]byte("not valid json at all"),
	); err != nil {
		t.Fatalf("publish: %v", err)
	}
	_ = nc.Flush()

	time.Sleep(500 * time.Millisecond)

	// Verify the service is still running.
	if !gp.waitForHealth(3 * time.Second) {
		t.Errorf("service stopped after invalid telemetry JSON; logs:\n%s", gp.logs())
	}
}

// ── TS-06-E12: NATS Unreachable at Startup ───────────────────────────────────

// TestNATSUnreachable verifies that when NATS is unreachable the cloud-gateway
// exits non-zero after retries.
// Requirement: 06-REQ-8.E1
//
// Strategy: build and start the binary with a NATS URL pointing to a non-
// listening port, then wait for it to exit with a non-zero code.
func TestNATSUnreachable(t *testing.T) {
	bin := ensureBinary(t)

	// Config pointing at a non-listening NATS port.
	cfg := gatewayConfig{
		Port:    nextPort(),
		NatsURL: "nats://localhost:19999",
		Tokens:  map[string]string{testToken: testVIN},
	}
	configPath := createConfig(t, cfg)

	buf := &safeBuffer{}
	cmd := exec.Command(bin)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+configPath)
	cmd.Stdout = buf
	cmd.Stderr = buf

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start cloud-gateway: %v", err)
	}
	t.Cleanup(func() {
		if cmd.ProcessState == nil || !cmd.ProcessState.Exited() {
			_ = cmd.Process.Kill()
			_ = cmd.Wait()
		}
	})

	// Wait for the process to exit. With 5 retries the delays are
	// 1s+2s+4s+8s+16s ≈ 31s, so allow up to 60 seconds.
	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()

	select {
	case err := <-done:
		if err == nil {
			t.Error("expected non-zero exit when NATS is unreachable, got exit code 0")
		}
		// Non-zero exit is the expected outcome. Optionally verify retry logs.
		output := buf.String()
		if !strings.Contains(output, "retry") &&
			!strings.Contains(output, "attempt") &&
			!strings.Contains(output, "failed to connect") {
			t.Logf("no retry/fail log found (acceptable); output:\n%s", output)
		}
	case <-time.After(70 * time.Second):
		_ = cmd.Process.Kill()
		t.Fatalf("cloud-gateway did not exit within timeout when NATS unreachable; output:\n%s", buf.String())
	}
}
