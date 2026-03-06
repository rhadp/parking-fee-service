package main

import (
	"flag"
	"fmt"
	"net/http"
	"os"
)

func main() {
	portFlag := flag.String("port", "", "Port to listen on (default: 9090)")
	flag.Parse()

	port := *portFlag
	if port == "" {
		port = os.Getenv("PORT")
	}
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
