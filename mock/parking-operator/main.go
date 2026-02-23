// Package main provides a mock PARKING_OPERATOR HTTP service for integration
// testing of the PARKING_OPERATOR_ADAPTOR. This is a stub — implementation
// will be added in task group 2.
package main

import (
	"fmt"
	"net/http"
	"os"
)

func main() {
	port := os.Getenv("PORT")
	if port == "" {
		port = "8090"
	}
	mux := NewRouter()
	fmt.Printf("mock parking-operator listening on :%s\n", port)
	if err := http.ListenAndServe(":"+port, mux); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
}

// NewRouter creates the HTTP mux with all routes registered.
// Stub implementation — returns 501 for all endpoints.
func NewRouter() *http.ServeMux {
	mux := http.NewServeMux()
	// Routes will be registered by handler.go in task group 2.
	return mux
}
