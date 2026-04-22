package mockapps_test

import (
	"os"
	"os/exec"
	"path/filepath"
	"testing"
)

// repoRoot returns the absolute path to the repository root directory.
func repoRoot(t *testing.T) string {
	t.Helper()
	dir, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get working directory: %v", err)
	}
	for {
		if _, err := os.Stat(filepath.Join(dir, "Makefile")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("could not find repository root (no Makefile found)")
		}
		dir = parent
	}
}

// buildBinary compiles a Go binary from mock/<name> and returns the path
// to the compiled executable. The binary is placed in a test-managed temp
// directory and cleaned up automatically.
func buildBinary(t *testing.T, name string) string {
	t.Helper()
	root := repoRoot(t)
	srcDir := filepath.Join(root, "mock", name)

	tmpDir := t.TempDir()
	binary := filepath.Join(tmpDir, name)

	cmd := exec.Command("go", "build", "-o", binary, ".")
	cmd.Dir = srcDir
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build %s: %v\n%s", name, err, output)
	}
	return binary
}

// runBinary executes a compiled binary with the given arguments and returns
// stdout, stderr, and exit code. It does NOT fail the test on non-zero exit.
func runBinary(t *testing.T, binary string, args ...string) (stdout, stderr string, exitCode int) {
	t.Helper()
	return runBinaryWithEnv(t, binary, nil, args...)
}

// runBinaryWithEnv executes a compiled binary with extra environment variables.
// env entries are in "KEY=VALUE" format. Returns stdout, stderr, and exit code.
func runBinaryWithEnv(t *testing.T, binary string, env []string, args ...string) (stdout, stderr string, exitCode int) {
	t.Helper()

	cmd := exec.Command(binary, args...)
	if len(env) > 0 {
		cmd.Env = append(os.Environ(), env...)
	}

	var outBuf, errBuf []byte
	outPipe, _ := cmd.StdoutPipe()
	errPipe, _ := cmd.StderrPipe()

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start %s: %v", binary, err)
	}

	outBuf, _ = readAll(outPipe)
	errBuf, _ = readAll(errPipe)

	err := cmd.Wait()
	exitCode = cmd.ProcessState.ExitCode()
	_ = err

	return string(outBuf), string(errBuf), exitCode
}

func readAll(r interface{ Read([]byte) (int, error) }) ([]byte, error) {
	var buf []byte
	tmp := make([]byte, 4096)
	for {
		n, err := r.Read(tmp)
		if n > 0 {
			buf = append(buf, tmp[:n]...)
		}
		if err != nil {
			break
		}
	}
	return buf, nil
}
