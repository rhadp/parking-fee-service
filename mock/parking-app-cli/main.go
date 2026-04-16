// parking-app-cli simulates the PARKING_APP on AAOS IVI. It queries
// PARKING_FEE_SERVICE (REST), manages adapters via UPDATE_SERVICE (gRPC),
// and overrides sessions via PARKING_OPERATOR_ADAPTOR (gRPC).
// Stub — full implementation in task group 4.
package main

import (
	"fmt"
	"os"
)

func main() {
	// Stub: always exits 0 without processing args or making network calls.
	fmt.Fprintln(os.Stderr, "stub: not implemented")
	os.Exit(0)
}
