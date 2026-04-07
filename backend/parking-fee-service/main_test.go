package main

import (
	"bufio"
	"context"
	"fmt"
	"net"
	"net/http"
	"os"
	"os/exec"
	"strings"
	"testing"
	"time"
)

// getFreePort returns a free TCP port on localhost.
func getFreePort(t *testing.T) int {
	t.Helper()
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to get free port: %v", err)
	}
	port := l.Addr().(*net.TCPAddr).Port
	l.Close()
	return port
}

// buildBinary builds the service binary and returns its path.
func buildBinary(t *testing.T) string {
	t.Helper()
	bin := t.TempDir() + "/parking-fee-service"
	cmd := exec.Command("go", "build", "-o", bin, ".")
	cmd.Dir = "."
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build binary: %v\n%s", err, out)
	}
	return bin
}

// TS-05-15: Startup logging includes version, port, zone count, operator count.
func TestStartupLogging(t *testing.T) {
	bin := buildBinary(t)
	port := getFreePort(t)

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	cmd := exec.CommandContext(ctx, bin)
	cmd.Env = append(os.Environ(),
		"CONFIG_PATH=/nonexistent/config.json",
	)
	// Override port via a temp config file
	cfgFile := t.TempDir() + "/config.json"
	cfgJSON := fmt.Sprintf(`{"port":%d,"proximity_threshold_meters":500,"zones":[],"operators":[]}`, port)
	if err := os.WriteFile(cfgFile, []byte(cfgJSON), 0644); err != nil {
		t.Fatalf("failed to write config: %v", err)
	}
	cmd.Env = append(os.Environ(), fmt.Sprintf("CONFIG_PATH=%s", cfgFile))

	stderr, err := cmd.StderrPipe()
	if err != nil {
		t.Fatalf("failed to get stderr: %v", err)
	}

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start binary: %v", err)
	}

	// Read log lines until we see the ready message or timeout.
	scanner := bufio.NewScanner(stderr)
	var logLines []string
	readyCh := make(chan struct{})

	go func() {
		for scanner.Scan() {
			line := scanner.Text()
			logLines = append(logLines, line)
			if strings.Contains(line, "ready") {
				close(readyCh)
				return
			}
		}
	}()

	select {
	case <-readyCh:
		// Success — check log lines.
	case <-time.After(5 * time.Second):
		cmd.Process.Kill()
		t.Fatalf("timed out waiting for ready log; got: %v", logLines)
	}

	allLogs := strings.Join(logLines, "\n")

	if !strings.Contains(allLogs, fmt.Sprintf("%d", port)) {
		t.Errorf("startup logs should contain port %d, got:\n%s", port, allLogs)
	}
	if !strings.Contains(allLogs, "zones") {
		t.Errorf("startup logs should mention zones, got:\n%s", allLogs)
	}
	if !strings.Contains(allLogs, "operators") {
		t.Errorf("startup logs should mention operators, got:\n%s", allLogs)
	}

	// Clean up.
	cmd.Process.Kill()
	cmd.Wait()
}

// TS-05-16: Graceful shutdown on SIGTERM exits with code 0.
func TestGracefulShutdown(t *testing.T) {
	bin := buildBinary(t)
	port := getFreePort(t)

	cfgFile := t.TempDir() + "/config.json"
	cfgJSON := fmt.Sprintf(`{"port":%d,"proximity_threshold_meters":500,"zones":[],"operators":[]}`, port)
	if err := os.WriteFile(cfgFile, []byte(cfgJSON), 0644); err != nil {
		t.Fatalf("failed to write config: %v", err)
	}

	cmd := exec.Command(bin)
	cmd.Env = append(os.Environ(), fmt.Sprintf("CONFIG_PATH=%s", cfgFile))

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start binary: %v", err)
	}

	// Wait for the server to be ready by polling the health endpoint.
	addr := fmt.Sprintf("http://127.0.0.1:%d/health", port)
	deadline := time.Now().Add(5 * time.Second)
	for time.Now().Before(deadline) {
		resp, err := http.Get(addr)
		if err == nil {
			resp.Body.Close()
			if resp.StatusCode == 200 {
				break
			}
		}
		time.Sleep(50 * time.Millisecond)
	}

	// Send SIGTERM.
	if err := cmd.Process.Signal(os.Interrupt); err != nil {
		t.Fatalf("failed to send signal: %v", err)
	}

	// Wait for exit.
	doneCh := make(chan error, 1)
	go func() {
		doneCh <- cmd.Wait()
	}()

	select {
	case err := <-doneCh:
		if err != nil {
			t.Errorf("expected exit code 0, got error: %v", err)
		}
	case <-time.After(10 * time.Second):
		cmd.Process.Kill()
		t.Fatal("timed out waiting for graceful shutdown")
	}
}
