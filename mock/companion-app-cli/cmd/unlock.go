package cmd

import (
	"fmt"
	"os"
)

// RunUnlock executes the unlock subcommand.
// Sends an unlock command to CLOUD_GATEWAY for the specified VIN.
func RunUnlock(args []string, gatewayURL string, bearerToken string) error {
	// TODO: implement
	fmt.Fprintln(os.Stderr, "unlock: not yet implemented")
	return fmt.Errorf("not implemented")
}
