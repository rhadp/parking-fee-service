package main

import (
	"flag"
	"fmt"
	"log"
	"net/http"
	"os"
)

func main() {
	defaultPort := "9090"
	if envPort := os.Getenv("PORT"); envPort != "" {
		defaultPort = envPort
	}

	port := flag.String("port", defaultPort, "HTTP server port")
	flag.Parse()

	store := NewSessionStore()

	mux := http.NewServeMux()
	mux.HandleFunc("/parking/start", HandleStartParking(store))
	mux.HandleFunc("/parking/stop", HandleStopParking(store))
	mux.HandleFunc("/parking/status", HandleParkingStatus(store))

	addr := fmt.Sprintf(":%s", *port)
	log.Printf("parking-operator listening on %s", addr)
	if err := http.ListenAndServe(addr, mux); err != nil {
		log.Fatalf("server error: %v", err)
	}
}
