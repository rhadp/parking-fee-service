package cmd

import (
	"fmt"
	"os"
)

// RunRemove executes the remove subcommand.
// It calls UPDATE_SERVICE.RemoveAdapter via gRPC.
func RunRemove(args []string, serviceAddr string) error {
	// TODO: implement
	fmt.Fprintln(os.Stderr, "remove: not yet implemented")
	return fmt.Errorf("not implemented")
}
