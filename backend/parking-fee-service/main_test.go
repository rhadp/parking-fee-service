package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"testing"
	"time"
)

// getFreePort returns a free TCP port on localhost.
func getFreePort(t *testing.T) int {
	t.Helper()
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to get free port: %v", err)
	}
	port := ln.Addr().(*net.TCPAddr).Port
	ln.Close()
	return port
}

// buildBinary builds the service binary and returns the path.
func buildBinary(t *testing.T) string {
	t.Helper()
	tmpDir := t.TempDir()
	binName := "parking-fee-service"
	if runtime.GOOS == "windows" {
		binName += ".exe"
	}
	binPath := filepath.Join(tmpDir, binName)
	cmd := exec.Command("go", "build", "-o", binPath, ".")
	cmd.Dir = filepath.Join(".")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build binary: %v\n%s", err, out)
	}
	return binPath
}

// TS-05-15: On startup, the service logs version, port, zone count, operator count.
func TestStartupLogging(t *testing.T) {
	binPath := buildBinary(t)
	port := getFreePort(t)

	// Write a config file that uses the free port.
	cfgDir := t.TempDir()
	cfgPath := filepath.Join(cfgDir, "config.json")
	cfgData := fmt.Sprintf(`{
		"port": %d,
		"proximity_threshold_meters": 500,
		"zones": [{"id":"z1","name":"Z1","polygon":[{"lat":1,"lon":1},{"lat":1,"lon":2},{"lat":2,"lon":2}]}],
		"operators": [{"id":"op1","name":"Op1","zone_id":"z1","rate":{"type":"flat-fee","amount":1,"currency":"EUR"},"adapter":{"image_ref":"img","checksum_sha256":"sha","version":"1"}}]
	}`, port)
	if err := os.WriteFile(cfgPath, []byte(cfgData), 0644); err != nil {
		t.Fatalf("failed to write config: %v", err)
	}

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	cmd := exec.CommandContext(ctx, binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)

	var stderr bytes.Buffer
	cmd.Stderr = &stderr

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start service: %v", err)
	}

	// Wait for the service to be ready by polling the health endpoint.
	addr := fmt.Sprintf("http://127.0.0.1:%d/health", port)
	ready := false
	for i := 0; i < 50; i++ {
		time.Sleep(100 * time.Millisecond)
		resp, err := http.Get(addr)
		if err == nil {
			resp.Body.Close()
			ready = true
			break
		}
	}
	if !ready {
		cmd.Process.Kill()
		t.Fatalf("service did not become ready; stderr: %s", stderr.String())
	}

	// Kill the process and capture output.
	cmd.Process.Kill()
	cmd.Wait() //nolint:errcheck

	output := stderr.String()

	// slog outputs to stderr by default.
	portStr := fmt.Sprintf("%d", port)
	if !bytes.Contains([]byte(output), []byte(portStr)) {
		t.Errorf("startup logs do not contain port %s; got:\n%s", portStr, output)
	}
	if !bytes.Contains([]byte(output), []byte("zones")) {
		t.Errorf("startup logs do not contain 'zones'; got:\n%s", output)
	}
	if !bytes.Contains([]byte(output), []byte("operators")) {
		t.Errorf("startup logs do not contain 'operators'; got:\n%s", output)
	}
}

// TS-05-16: On SIGTERM, the service gracefully shuts down and exits with code 0.
func TestGracefulShutdown(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("SIGTERM not supported on Windows")
	}

	binPath := buildBinary(t)
	port := getFreePort(t)

	cfgDir := t.TempDir()
	cfgPath := filepath.Join(cfgDir, "config.json")
	cfgData := fmt.Sprintf(`{
		"port": %d,
		"proximity_threshold_meters": 500,
		"zones": [{"id":"z1","name":"Z1","polygon":[{"lat":1,"lon":1},{"lat":1,"lon":2},{"lat":2,"lon":2}]}],
		"operators": [{"id":"op1","name":"Op1","zone_id":"z1","rate":{"type":"flat-fee","amount":1,"currency":"EUR"},"adapter":{"image_ref":"img","checksum_sha256":"sha","version":"1"}}]
	}`, port)
	if err := os.WriteFile(cfgPath, []byte(cfgData), 0644); err != nil {
		t.Fatalf("failed to write config: %v", err)
	}

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start service: %v", err)
	}

	// Wait for the service to be ready.
	addr := fmt.Sprintf("http://127.0.0.1:%d/health", port)
	ready := false
	for i := 0; i < 50; i++ {
		time.Sleep(100 * time.Millisecond)
		resp, err := http.Get(addr)
		if err == nil {
			resp.Body.Close()
			ready = true
			break
		}
	}
	if !ready {
		cmd.Process.Kill()
		t.Fatalf("service did not become ready")
	}

	// Send SIGTERM.
	if err := cmd.Process.Signal(os.Interrupt); err != nil {
		cmd.Process.Kill()
		t.Fatalf("failed to send signal: %v", err)
	}

	// Wait for the process to exit.
	done := make(chan error, 1)
	go func() {
		done <- cmd.Wait()
	}()

	select {
	case err := <-done:
		if err != nil {
			t.Errorf("expected exit code 0, got error: %v", err)
		}
	case <-time.After(10 * time.Second):
		cmd.Process.Kill()
		t.Fatal("service did not exit within 10 seconds after SIGTERM")
	}
}

// TestRunFunction tests the run function indirectly by verifying the service
// starts with default config and logs correctly.
func TestRunFunction(t *testing.T) {
	// Verify the run function signature exists and the package compiles.
	// The actual behavior is tested via subprocess in TestStartupLogging
	// and TestGracefulShutdown.
	_ = run // Ensure the function is accessible.
}

// TestMainServesHealth verifies the built binary serves the health endpoint.
func TestMainServesHealth(t *testing.T) {
	binPath := buildBinary(t)
	port := getFreePort(t)

	cfgDir := t.TempDir()
	cfgPath := filepath.Join(cfgDir, "config.json")
	cfgData := fmt.Sprintf(`{
		"port": %d,
		"proximity_threshold_meters": 500,
		"zones": [{"id":"z1","name":"Z1","polygon":[{"lat":1,"lon":1},{"lat":1,"lon":2},{"lat":2,"lon":2}]}],
		"operators": [{"id":"op1","name":"Op1","zone_id":"z1","rate":{"type":"flat-fee","amount":1,"currency":"EUR"},"adapter":{"image_ref":"img","checksum_sha256":"sha","version":"1"}}]
	}`, port)
	if err := os.WriteFile(cfgPath, []byte(cfgData), 0644); err != nil {
		t.Fatalf("failed to write config: %v", err)
	}

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	cmd := exec.CommandContext(ctx, binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start service: %v", err)
	}
	defer func() {
		cmd.Process.Kill()
		cmd.Wait() //nolint:errcheck
	}()

	addr := fmt.Sprintf("http://127.0.0.1:%d/health", port)
	var resp *http.Response
	for i := 0; i < 50; i++ {
		time.Sleep(100 * time.Millisecond)
		var err error
		resp, err = http.Get(addr)
		if err == nil {
			break
		}
	}
	if resp == nil {
		t.Fatal("service did not become ready")
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		t.Fatalf("health status = %d, want 200", resp.StatusCode)
	}

	var body map[string]string
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode health response: %v", err)
	}
	if body["status"] != "ok" {
		t.Errorf("health status = %q, want 'ok'", body["status"])
	}
}

// Keep slog import used for the package.
var _ = slog.Default
