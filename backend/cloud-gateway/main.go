// Package main implements the CLOUD_GATEWAY service.
//
// CLOUD_GATEWAY provides a REST API for vehicle remote operations (lock,
// unlock, status) and vehicle pairing. It maintains an in-memory vehicle
// state store and will connect to MQTT (Mosquitto) in a future task group.
package main

import (
	"context"
	"flag"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/api"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/state"
)

func main() {
	listenAddr := flag.String("listen-addr", envOrDefault("LISTEN_ADDR", ":8081"), "REST API listen address")
	_ = flag.String("mqtt-addr", envOrDefault("MQTT_ADDR", "localhost:1883"), "MQTT broker address (used in task group 3)")
	flag.Parse()

	store := state.NewStore()

	mux := newServeMux(store)

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

// newServeMux creates the HTTP mux with all REST API routes.
// The MQTT publisher is nil for now (no-op); it will be wired up in
// task group 3.
func newServeMux(store *state.Store) *http.ServeMux {
	mux := http.NewServeMux()

	h := api.NewHandlers(store, nil)
	h.RegisterRoutes(mux)

	return mux
}

// envOrDefault returns the value of the given environment variable, or the
// default value if the variable is not set.
func envOrDefault(key, defaultVal string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return defaultVal
}
