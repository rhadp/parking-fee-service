// Package main implements the cloud-gateway skeleton.
//
// This service provides a REST API for vehicle remote operations (lock,
// unlock, status). In this skeleton, all endpoints return HTTP 501
// (Not Implemented).
package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"
)

func main() {
	listenAddr := flag.String("listen-addr", envOrDefault("LISTEN_ADDR", ":8081"), "Address to listen on")
	flag.Parse()

	mux := newServeMux()

	srv := &http.Server{
		Addr:    *listenAddr,
		Handler: mux,
	}

	// Channel to listen for OS signals.
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)

	// Start server in a goroutine.
	go func() {
		log.Printf("cloud-gateway starting on %s", *listenAddr)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("cloud-gateway failed to start: %v", err)
		}
	}()

	// Wait for signal.
	sig := <-sigCh
	log.Printf("cloud-gateway received signal %v, shutting down", sig)

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	if err := srv.Shutdown(ctx); err != nil {
		log.Fatalf("cloud-gateway shutdown error: %v", err)
	}

	log.Println("cloud-gateway stopped")
}

// newServeMux creates the HTTP mux with all stub routes.
func newServeMux() *http.ServeMux {
	mux := http.NewServeMux()

	mux.HandleFunc("GET /healthz", handleHealthz)
	mux.HandleFunc("POST /api/v1/vehicles/{vin}/lock", stubHandler("POST /api/v1/vehicles/{vin}/lock"))
	mux.HandleFunc("POST /api/v1/vehicles/{vin}/unlock", stubHandler("POST /api/v1/vehicles/{vin}/unlock"))
	mux.HandleFunc("GET /api/v1/vehicles/{vin}/status", stubHandler("GET /api/v1/vehicles/{vin}/status"))

	return mux
}

// handleHealthz returns a 200 OK for health checks.
func handleHealthz(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	fmt.Fprintln(w, `{"status":"ok"}`)
}

// stubHandler returns an HTTP 501 handler for unimplemented endpoints.
func stubHandler(route string) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusNotImplemented)
		fmt.Fprintf(w, `{"error":"not implemented","route":%q}`+"\n", route)
	}
}

// envOrDefault returns the value of the given environment variable, or the
// default value if the variable is not set.
func envOrDefault(key, defaultVal string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return defaultVal
}
