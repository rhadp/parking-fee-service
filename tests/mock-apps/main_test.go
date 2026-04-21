package mockappstests

import (
	"os"
	"testing"
)

// sharedBinDir is a temp directory that persists for the entire test run.
// It is set up by TestMain and cleaned up after all tests finish.
var sharedBinDir string

func TestMain(m *testing.M) {
	// Create a shared temp dir that outlives individual test cases.
	dir, err := os.MkdirTemp("", "mock-apps-test-*")
	if err != nil {
		panic("create shared binary dir: " + err.Error())
	}
	sharedBinDir = dir

	code := m.Run()

	os.RemoveAll(dir) //nolint
	os.Exit(code)
}
