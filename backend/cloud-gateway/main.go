package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/internal/api"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/internal/bridge"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/internal/config"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/internal/mqtt"
)

func main() {
	cfg := config.Load()

	// Initialize MQTT client (not connected yet)
	mqttClient := mqtt.NewClient(cfg.MQTTBrokerURL, cfg.MQTTClientID)

	// Initialize command tracker, telemetry cache, and bridge
	tracker := bridge.NewTracker(cfg.CommandTimeout)
	cache := api.NewTelemetryCache()
	b := bridge.NewBridge(tracker, mqttClient)

	// Register MQTT subscription handlers (will be applied when connected)
	// vehicles/+/command_responses → bridge resolves pending commands
	mqttClient.Subscribe(mqtt.WildcardResponseTopic(), func(topic string, payload []byte) {
		b.HandleResponse(payload)
	})
	// vehicles/+/telemetry → cache updates for status endpoint
	mqttClient.Subscribe(mqtt.WildcardTelemetryTopic(), func(topic string, payload []byte) {
		handleTelemetry(cache, topic, payload)
	})

	// Connect MQTT in background — REST API starts immediately (degraded mode)
	// per design decision D1 and requirement 03-REQ-2.E1.
	log.Printf("MQTT connecting to %s (background)", cfg.MQTTBrokerURL)
	go func() {
		if err := mqttClient.Connect(); err != nil {
			log.Printf("MQTT connect error (non-fatal): %v", err)
		}
	}()

	// Create HTTP router with all dependencies
	router := api.NewRouter(cfg.AuthToken, tracker, mqttClient, cache)

	addr := ":" + cfg.Port
	server := &http.Server{
		Addr:    addr,
		Handler: router,
	}

	// Graceful shutdown on SIGINT/SIGTERM
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

	go func() {
		sig := <-sigChan
		log.Printf("received signal %v, shutting down...", sig)

		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()

		if err := server.Shutdown(ctx); err != nil {
			log.Printf("HTTP server shutdown error: %v", err)
		}
		mqttClient.Disconnect()
	}()

	fmt.Printf("cloud-gateway listening on %s\n", addr)
	if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		log.Fatalf("server error: %v", err)
	}

	log.Println("cloud-gateway stopped")
}

// handleTelemetry processes MQTT messages from vehicles/+/telemetry.
// It extracts the VIN from the topic and updates the telemetry cache.
func handleTelemetry(cache *api.TelemetryCache, topic string, payload []byte) {
	vin, ok := mqtt.ExtractVINFromTopic(topic)
	if !ok {
		log.Printf("WARN: cannot extract VIN from telemetry topic %s", topic)
		return
	}

	var data struct {
		VIN       string `json:"vin"`
		Locked    bool   `json:"locked"`
		Timestamp int64  `json:"timestamp"`
	}

	if err := parseJSON(payload, &data); err != nil {
		log.Printf("WARN: invalid telemetry on %s: %v", topic, err)
		return
	}

	cache.Update(vin, api.TelemetryData{
		VIN:       vin,
		Locked:    data.Locked,
		Timestamp: data.Timestamp,
	})
}

// parseJSON unmarshals JSON payload into the target struct.
func parseJSON(payload []byte, target interface{}) error {
	return json.Unmarshal(payload, target)
}
