//go:build !windows

package updateservice_test

import (
	"syscall"
	"testing"
)

// sendSIGTERM sends SIGTERM to the service process.
func sendSIGTERM(t *testing.T, sp *serviceProcess) {
	t.Helper()
	if sp.cmd.Process == nil {
		t.Fatal("process not started")
	}
	if err := sp.cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("send SIGTERM: %v", err)
	}
}
