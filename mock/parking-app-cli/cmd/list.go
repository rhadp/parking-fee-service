package cmd

import (
	"fmt"
	"os"
)

// RunList executes the list subcommand.
// It calls UPDATE_SERVICE.ListAdapters via gRPC.
func RunList(args []string, serviceAddr string) error {
	// TODO: implement
	fmt.Fprintln(os.Stderr, "list: not yet implemented")
	return fmt.Errorf("not implemented")
}
