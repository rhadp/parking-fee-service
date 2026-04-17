//go:build !windows

package parkingoperatoradaptor

import "syscall"

// sigTERM returns syscall.SIGTERM on Unix platforms.
func sigTERM() syscall.Signal {
	return syscall.SIGTERM
}
