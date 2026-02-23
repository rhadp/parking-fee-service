package main

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
)

func main() {
	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}

	mux := http.NewServeMux()
	mux.HandleFunc("GET /health", handleHealth)
	mux.HandleFunc("GET /operators", handleOperators)
	mux.HandleFunc("GET /operators/{id}/adapter", handleOperatorAdapter)

	addr := ":" + port
	fmt.Printf("parking-fee-service listening on %s\n", addr)
	if err := http.ListenAndServe(addr, mux); err != nil {
		log.Fatalf("server error: %v", err)
	}
}

func handleHealth(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]string{"status": "ok"})
}

func handleOperators(w http.ResponseWriter, r *http.Request) {
	http.Error(w, "Not Implemented", http.StatusNotImplemented)
}

func handleOperatorAdapter(w http.ResponseWriter, r *http.Request) {
	http.Error(w, "Not Implemented", http.StatusNotImplemented)
}
