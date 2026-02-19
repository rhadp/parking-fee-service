// Package main implements the PARKING_FEE_SERVICE.
//
// This service provides a REST API for parking zone discovery and adapter
// metadata retrieval. On startup it loads hardcoded demo zone data with
// realistic Munich geofence polygons and serves zone lookup, zone details,
// and adapter metadata endpoints.
package main

import (
	"context"
	"flag"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/api"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/zones"
)

func main() {
	listenAddr := flag.String("listen-addr", envOrDefault("LISTEN_ADDR", ":8080"), "REST listen address")
	flag.Parse()

	// Load seed zone data into the in-memory store.
	store := zones.LoadSeedData()

	mux := newServeMux(store)

	srv := &http.Server{
		Addr:    *listenAddr,
		Handler: api.LoggingMiddleware(mux),
	}

	// Channel to listen for OS signals.
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)

	// Start server in a goroutine.
	go func() {
		slog.Info("parking-fee-service starting", "addr", *listenAddr)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			slog.Error("parking-fee-service failed to start", "error", err)
			os.Exit(1)
		}
	}()

	// Wait for signal.
	sig := <-sigCh
	slog.Info("parking-fee-service received signal, shutting down", "signal", sig)

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	if err := srv.Shutdown(ctx); err != nil {
		slog.Error("parking-fee-service shutdown error", "error", err)
		os.Exit(1)
	}

	slog.Info("parking-fee-service stopped")
}

// newServeMux creates the HTTP mux with all routes registered via the api
// package handler.
func newServeMux(store *zones.Store) *http.ServeMux {
	mux := http.NewServeMux()
	h := api.NewHandler(store)
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
