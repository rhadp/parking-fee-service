//go:build !windows

package lockingservice

import "syscall"

// sigTERM returns the SIGTERM signal on Unix platforms.
func sigTERM() syscall.Signal {
	return syscall.SIGTERM
}
