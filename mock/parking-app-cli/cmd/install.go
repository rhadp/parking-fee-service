package cmd

import (
	"fmt"
	"os"
)

// RunInstall executes the install subcommand.
// It calls UPDATE_SERVICE.InstallAdapter via gRPC.
func RunInstall(args []string, serviceAddr string) error {
	// TODO: implement
	fmt.Fprintln(os.Stderr, "install: not yet implemented")
	return fmt.Errorf("not implemented")
}
