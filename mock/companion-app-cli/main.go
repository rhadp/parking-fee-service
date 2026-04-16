// companion-app-cli simulates the COMPANION_APP sending lock/unlock commands
// to CLOUD_GATEWAY. Stub — full implementation in task group 4.
package main

import (
	"fmt"
	"os"
)

func main() {
	// Stub: always exits 0 without processing args or making HTTP calls.
	fmt.Fprintln(os.Stderr, "stub: not implemented")
	os.Exit(0)
}
