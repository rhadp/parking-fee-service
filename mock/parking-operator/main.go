// Package main provides a mock PARKING_OPERATOR HTTP service for integration
// testing of the PARKING_OPERATOR_ADAPTOR. It exposes REST endpoints for
// parking session management and zone rate queries.
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
// Each call creates a fresh Handler with its own in-memory store,
// enabling isolated test instances via httptest.NewServer(NewRouter()).
func NewRouter() *http.ServeMux {
	h := NewHandler()
	mux := http.NewServeMux()

	mux.HandleFunc("/parking/start", h.HandleStartSession)
	mux.HandleFunc("/parking/stop", h.HandleStopSession)
	mux.HandleFunc("/parking/", h.HandleSessionStatus) // matches /parking/{id}/status
	mux.HandleFunc("/rate/", h.HandleRate)              // matches /rate/{zone_id}
	mux.HandleFunc("/health", h.HandleHealth)

	return mux
}
