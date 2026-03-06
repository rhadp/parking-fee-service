package main

import (
	"fmt"
	"net/http"
	"os"
)

func main() {
	port := os.Getenv("PORT")
	if port == "" {
		port = "9090"
	}

	store := NewSessionStore()
	h := NewHandler(store)

	mux := http.NewServeMux()
	mux.HandleFunc("POST /parking/start", h.HandleStartParking)
	mux.HandleFunc("POST /parking/stop", h.HandleStopParking)
	mux.HandleFunc("GET /parking/status", h.HandleParkingStatus)

	addr := ":" + port
	fmt.Printf("parking-operator listening on %s\n", addr)
	if err := http.ListenAndServe(addr, mux); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}
