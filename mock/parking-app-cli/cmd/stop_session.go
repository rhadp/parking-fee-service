package cmd

import (
	"fmt"
	"os"
)

// RunStopSession executes the stop-session subcommand.
// It calls PARKING_OPERATOR_ADAPTOR.StopSession via gRPC.
func RunStopSession(args []string, adaptorAddr string) error {
	// TODO: implement
	fmt.Fprintln(os.Stderr, "stop-session: not yet implemented")
	return fmt.Errorf("not implemented")
}
