package cmd

import (
	"fmt"
	"os"
)

// RunLookup executes the lookup subcommand.
// It queries PARKING_FEE_SERVICE for operators near the given lat/lon.
func RunLookup(args []string, serviceURL string) error {
	// TODO: implement
	fmt.Fprintln(os.Stderr, "lookup: not yet implemented")
	return fmt.Errorf("not implemented")
}
