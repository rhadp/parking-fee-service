package main

import (
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
)

func main() {
	// Load configuration
	cfg := LoadConfig()

	// Create token store with demo tokens
	tokens := map[string]string{
		"companion-token-vehicle-1": "VIN12345",
		"companion-token-vehicle-2": "VIN67890",
	}
	tokenStore := NewTokenStore(tokens)

	// Build known VINs map
	knownVINs := make(map[string]bool)
	for _, vin := range cfg.KnownVINs {
		knownVINs[vin] = true
	}

	// Create stores
	commandStore := NewCommandStore()
	telemetryStore := NewTelemetryStore()

	// Connect NATS client
	natsClient, err := NewNATSClient(cfg.NATSURL)
	if err != nil {
		log.Printf("WARNING: failed to connect to NATS at %s: %v", cfg.NATSURL, err)
		log.Println("Running without NATS - command publishing will return 503")
		natsClient = nil
	}

	// Subscribe to command responses and telemetry for all known VINs
	if natsClient != nil {
		for _, vin := range cfg.KnownVINs {
			// Subscribe to command responses
			if err := natsClient.SubscribeCommandResponses(vin, func(resp NATSCommandResponse) {
				commandStore.UpdateCommandStatus(resp.CommandID, resp.Status, resp.Reason)
				log.Printf("received command response for %s: command_id=%s status=%s", vin, resp.CommandID, resp.Status)
			}); err != nil {
				log.Printf("WARNING: failed to subscribe to command responses for %s: %v", vin, err)
			}

			// Subscribe to telemetry
			if err := natsClient.SubscribeTelemetry(vin, func(data TelemetryData) {
				telemetryStore.StoreTelemetry(vin, data)
				log.Printf("received telemetry for %s", vin)
			}); err != nil {
				log.Printf("WARNING: failed to subscribe to telemetry for %s: %v", vin, err)
			}
		}
	}

	// Set up HTTP routes
	mux := http.NewServeMux()

	// Health endpoint (no auth)
	mux.HandleFunc("GET /health", HandleHealth)

	// Command submission (auth required)
	mux.Handle("POST /vehicles/{vin}/commands",
		AuthMiddleware(tokenStore, knownVINs)(HandleCommandSubmit(commandStore, natsClient, knownVINs)))

	// Command status query (auth required)
	mux.Handle("GET /vehicles/{vin}/commands/{command_id}",
		AuthMiddleware(tokenStore, knownVINs)(HandleCommandStatus(commandStore, knownVINs)))

	// Default 404 handler for undefined routes
	mux.HandleFunc("/", NotFoundHandler())

	// Wrap with recovery middleware
	handler := recoveryMiddleware(mux)

	// Start HTTP server
	addr := ":" + cfg.HTTPPort
	log.Printf("cloud-gateway starting on %s", addr)
	log.Printf("known VINs: %v", cfg.KnownVINs)

	// Graceful shutdown
	go func() {
		sigCh := make(chan os.Signal, 1)
		signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)
		sig := <-sigCh
		log.Printf("received signal %v, shutting down...", sig)
		if natsClient != nil {
			natsClient.Close()
		}
		os.Exit(0)
	}()

	if err := http.ListenAndServe(addr, handler); err != nil {
		log.Fatalf("server failed: %v", err)
	}
}

// recoveryMiddleware catches panics in HTTP handlers and returns 500.
func recoveryMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		defer func() {
			if rec := recover(); rec != nil {
				log.Printf("panic recovered: %v", rec)
				writeError(w, http.StatusInternalServerError, "internal server error")
			}
		}()
		next.ServeHTTP(w, r)
	})
}
