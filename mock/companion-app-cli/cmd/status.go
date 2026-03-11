package cmd

import (
	"fmt"
	"os"
)

// RunStatus executes the status subcommand.
// Queries vehicle status from CLOUD_GATEWAY for the specified VIN.
func RunStatus(args []string, gatewayURL string, bearerToken string) error {
	// TODO: implement
	fmt.Fprintln(os.Stderr, "status: not yet implemented")
	return fmt.Errorf("not implemented")
}
