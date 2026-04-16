// parking-app-cli simulates the PARKING_APP on AAOS IVI. It queries
// PARKING_FEE_SERVICE (REST), manages adapters via UPDATE_SERVICE (gRPC),
// and overrides sessions via PARKING_OPERATOR_ADAPTOR (gRPC).
// Stub — full implementation in a future spec.
package main

import "fmt"

func main() {
	fmt.Println("parking-app-cli v0.1.0")
}
