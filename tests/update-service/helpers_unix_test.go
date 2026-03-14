//go:build !windows

package updateservice

import "syscall"

// sigTERM returns the SIGTERM signal on Unix platforms.
func sigTERM() syscall.Signal {
	return syscall.SIGTERM
}
