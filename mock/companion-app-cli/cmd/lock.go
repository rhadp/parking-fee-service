package cmd

import (
	"fmt"
	"os"
)

// RunLock executes the lock subcommand.
// Sends a lock command to CLOUD_GATEWAY for the specified VIN.
func RunLock(args []string, gatewayURL string, bearerToken string) error {
	// TODO: implement
	fmt.Fprintln(os.Stderr, "lock: not yet implemented")
	return fmt.Errorf("not implemented")
}
