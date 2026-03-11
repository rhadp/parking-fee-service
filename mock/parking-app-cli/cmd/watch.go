package cmd

import (
	"fmt"
	"os"
)

// RunWatch executes the watch subcommand.
// It streams adapter state changes from UPDATE_SERVICE.
func RunWatch(args []string, serviceAddr string) error {
	// TODO: implement
	fmt.Fprintln(os.Stderr, "watch: not yet implemented")
	return fmt.Errorf("not implemented")
}
