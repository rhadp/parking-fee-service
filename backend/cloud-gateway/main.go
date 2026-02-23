package main

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/internal/api"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/internal/bridge"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/internal/config"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/internal/mqtt"
)

func main() {
	cfg := config.Load()

	// Initialize MQTT client (not connected yet)
	mqttClient := mqtt.NewClient(cfg.MQTTBrokerURL, cfg.MQTTClientID)

	// Initialize command tracker and telemetry cache
	tracker := bridge.NewTracker(cfg.CommandTimeout)
	cache := api.NewTelemetryCache()

	// Register MQTT subscription handlers (will be applied when connected)
	mqttClient.Subscribe(mqtt.WildcardResponseTopic(), func(topic string, payload []byte) {
		handleCommandResponse(tracker, topic, payload)
	})
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
	fmt.Printf("cloud-gateway listening on %s\n", addr)
	if err := http.ListenAndServe(addr, router); err != nil {
		log.Fatalf("server error: %v", err)
	}
}

// handleCommandResponse processes MQTT messages from vehicles/+/command_responses.
// It parses the response JSON and resolves the matching pending command.
func handleCommandResponse(tracker *bridge.Tracker, topic string, payload []byte) {
	var resp struct {
		CommandID string `json:"command_id"`
		Status    string `json:"status"`
		Reason    string `json:"reason"`
		Timestamp int64  `json:"timestamp"`
	}

	if err := parseJSON(payload, &resp); err != nil {
		log.Printf("WARN: invalid command response on %s: %v", topic, err)
		return
	}

	if resp.CommandID == "" {
		log.Printf("WARN: command response on %s missing command_id", topic)
		return
	}

	tracker.Resolve(resp.CommandID, bridge.CommandResponse{
		CommandID: resp.CommandID,
		Status:    resp.Status,
		Reason:    resp.Reason,
		Timestamp: resp.Timestamp,
	})
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
