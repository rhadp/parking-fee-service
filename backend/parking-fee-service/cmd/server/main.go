// Package main provides the entry point for the parking-fee-service.
//
// The parking-fee-service is a Go backend service that handles parking
// operations including fee calculation, session management, and payment
// processing for the SDV Parking Demo System.
//
// Communication:
// - HTTPS/REST for PARKING_OPERATOR_ADAPTOR and UPDATE_SERVICE
package main

import (
	"database/sql"
	"log/slog"
	"net/http"
	"os"

	"github.com/gorilla/mux"
	_ "modernc.org/sqlite"
)

func main() {
	// Initialize structured logger
	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{
		Level: slog.LevelInfo,
	}))
	slog.SetDefault(logger)

	logger.Info("parking-fee-service: starting",
		"status", "stub implementation - service not yet fully implemented",
	)

	// Initialize router (gorilla/mux)
	router := mux.NewRouter()

	// Placeholder health endpoint
	router.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte(`{"status":"healthy","service":"parking-fee-service"}`))
	}).Methods("GET")

	// Test SQLite driver is available
	db, err := sql.Open("sqlite", ":memory:")
	if err != nil {
		logger.Error("failed to open SQLite", "error", err)
		os.Exit(1)
	}
	defer db.Close()

	if err := db.Ping(); err != nil {
		logger.Error("failed to ping SQLite", "error", err)
		os.Exit(1)
	}
	logger.Info("SQLite driver initialized successfully")

	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}

	logger.Info("starting HTTP server", "port", port)

	if err := http.ListenAndServe(":"+port, router); err != nil {
		logger.Error("server failed", "error", err)
		os.Exit(1)
	}
}
