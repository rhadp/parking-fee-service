package main

import (
	"log"
	"net/http"
)

func main() {
	cfg := LoadConfig()

	// Demo tokens
	tokenStore := NewTokenStore(map[string]string{
		"companion-token-vehicle-1": "VIN12345",
		"companion-token-vehicle-2": "VIN67890",
	})

	// Build known VINs set
	knownVINs := make(map[string]bool)
	for _, vin := range cfg.KnownVINs {
		knownVINs[vin] = true
	}

	// Create stores
	commandStore := NewCommandStore()
	telemetryStore := NewTelemetryStore()

	// Connect to NATS
	natsClient, err := NewNATSClient(cfg.NATSURL)
	if err != nil {
		log.Fatalf("failed to connect to NATS: %v", err)
	}
	defer natsClient.Close()

	// Subscribe to command responses and telemetry for all known VINs
	for _, vin := range cfg.KnownVINs {
		v := vin
		if err := natsClient.SubscribeCommandResponses(v, func(resp NATSCommandResponse) {
			commandStore.UpdateCommandStatus(resp.CommandID, resp.Status, resp.Reason)
		}); err != nil {
			log.Fatalf("failed to subscribe to command responses for %s: %v", v, err)
		}

		if err := natsClient.SubscribeTelemetry(v, func(data TelemetryData) {
			telemetryStore.StoreTelemetry(data.VIN, data)
		}); err != nil {
			log.Fatalf("failed to subscribe to telemetry for %s: %v", v, err)
		}

		log.Printf("subscribed to responses and telemetry for VIN %s", v)
	}

	// Set up HTTP router
	router := NewRouter(tokenStore, commandStore, natsClient, knownVINs)

	addr := ":" + cfg.HTTPPort
	log.Printf("cloud-gateway starting on %s", addr)
	if err := http.ListenAndServe(addr, router); err != nil {
		log.Fatalf("HTTP server error: %v", err)
	}
}
