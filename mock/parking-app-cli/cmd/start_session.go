package cmd

import (
	"fmt"
	"os"
)

// RunStartSession executes the start-session subcommand.
// It calls PARKING_OPERATOR_ADAPTOR.StartSession via gRPC.
func RunStartSession(args []string, adaptorAddr string) error {
	// TODO: implement
	fmt.Fprintln(os.Stderr, "start-session: not yet implemented")
	return fmt.Errorf("not implemented")
}
