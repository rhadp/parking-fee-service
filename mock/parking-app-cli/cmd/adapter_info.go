package cmd

import (
	"fmt"
	"os"
)

// RunAdapterInfo executes the adapter-info subcommand.
// It queries PARKING_FEE_SERVICE for adapter metadata.
func RunAdapterInfo(args []string, serviceURL string) error {
	// TODO: implement
	fmt.Fprintln(os.Stderr, "adapter-info: not yet implemented")
	return fmt.Errorf("not implemented")
}
