package main

import (
	"fmt"
	"log"
	"net/http"
	"os"
)

func main() {
	configPath := os.Getenv("CONFIG_PATH")

	cfg, err := LoadConfig(configPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "failed to load configuration: %v\n", err)
		os.Exit(1)
	}

	store := NewStore(cfg)
	router := NewRouter(store)

	port := cfg.Settings.Port
	if port == 0 {
		port = 8080
	}

	addr := fmt.Sprintf(":%d", port)
	log.Printf("parking-fee-service starting on %s", addr)

	if err := http.ListenAndServe(addr, router); err != nil {
		fmt.Fprintf(os.Stderr, "server error: %v\n", err)
		os.Exit(1)
	}
}
