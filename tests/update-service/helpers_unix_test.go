//go:build !windows

package updateservice

import "syscall"

// sigTERM returns syscall.SIGTERM on Unix platforms.
func sigTERM() syscall.Signal {
	return syscall.SIGTERM
}
