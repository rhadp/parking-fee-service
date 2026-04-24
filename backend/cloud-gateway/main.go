package main

import (
	"context"
	"fmt"
	"log/slog"
	"net"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/config"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/handler"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/natsclient"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

const version = "0.1.0"

func main() {
	// Resolve config file path from environment or default.
	configPath := os.Getenv("CONFIG_PATH")
	if configPath == "" {
		configPath = "config.json"
	}

	cfg, err := config.LoadConfig(configPath)
	if err != nil {
		slog.Error("failed to load configuration", "error", err)
		os.Exit(1)
	}

	// Connect to NATS with exponential backoff retry (up to 5 attempts).
	nc, err := natsclient.Connect(cfg.NatsURL, 5)
	if err != nil {
		slog.Error("failed to connect to NATS", "error", err)
		os.Exit(1)
	}

	// Create in-memory command response store.
	s := store.NewStore()

	// Subscribe to NATS subjects for command responses and telemetry.
	if err := nc.SubscribeResponses(s); err != nil {
		slog.Error("failed to subscribe to command responses", "error", err)
		os.Exit(1)
	}
	if err := nc.SubscribeTelemetry(); err != nil {
		slog.Error("failed to subscribe to telemetry", "error", err)
		os.Exit(1)
	}

	// Compute command timeout duration from config.
	timeout := time.Duration(cfg.CommandTimeoutSeconds) * time.Second
	if timeout == 0 {
		timeout = 30 * time.Second
	}

	// Register HTTP routes using Go 1.22 ServeMux patterns.
	mux := http.NewServeMux()
	authMW := auth.Middleware(cfg)

	mux.Handle("POST /vehicles/{vin}/commands",
		authMW(handler.NewSubmitCommandHandler(nc, s, timeout)))
	mux.Handle("GET /vehicles/{vin}/commands/{command_id}",
		authMW(handler.NewGetCommandStatusHandler(s)))
	mux.HandleFunc("GET /health", handler.HealthHandler())

	addr := fmt.Sprintf(":%d", cfg.Port)
	srv := &http.Server{
		Addr:    addr,
		Handler: mux,
	}

	// Bind the listener before logging the ready message to avoid a race
	// where clients try to connect before the socket is actually bound.
	ln, err := net.Listen("tcp", addr)
	if err != nil {
		slog.Error("failed to bind address", "addr", addr, "error", err)
		os.Exit(1)
	}

	slog.Info("cloud-gateway ready",
		"version", version,
		"port", cfg.Port,
		"nats_url", cfg.NatsURL,
		"tokens", len(cfg.Tokens),
	)

	// Handle shutdown signals.
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGTERM, syscall.SIGINT)

	// Serve in a goroutine so the main goroutine can wait for signals.
	go func() {
		if err := srv.Serve(ln); err != nil && err != http.ErrServerClosed {
			slog.Error("server error", "error", err)
			os.Exit(1)
		}
	}()

	// Block until a shutdown signal is received.
	sig := <-sigCh
	slog.Info("shutting down", "signal", sig)

	// Drain NATS connection for graceful shutdown.
	if err := nc.Drain(); err != nil {
		slog.Error("NATS drain error", "error", err)
	}

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	if err := srv.Shutdown(ctx); err != nil {
		slog.Error("shutdown error", "error", err)
		os.Exit(1)
	}

	slog.Info("shutdown complete")
}
