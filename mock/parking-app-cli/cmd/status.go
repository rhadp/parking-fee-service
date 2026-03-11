package cmd

import (
	"fmt"
	"os"
)

// RunStatus executes the status subcommand.
// It calls UPDATE_SERVICE.GetAdapterStatus via gRPC.
func RunStatus(args []string, serviceAddr string) error {
	// TODO: implement
	fmt.Fprintln(os.Stderr, "status: not yet implemented")
	return fmt.Errorf("not implemented")
}
