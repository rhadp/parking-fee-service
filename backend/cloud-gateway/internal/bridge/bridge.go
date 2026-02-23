// Package bridge provides the REST-to-MQTT bridge logic for the CLOUD_GATEWAY
// service. The Bridge orchestrates command publishing, response handling, and
// telemetry caching.
//
// The Bridge connects three subsystems:
//   - REST handlers: submit commands via SendCommand and read telemetry via the cache
//   - MQTT client: publishes commands and receives responses/telemetry
//   - Command Tracker: correlates MQTT responses with pending REST requests
package bridge

import (
	"encoding/json"
	"log"
)

// MQTTPublisher is the interface used by the bridge to publish messages to MQTT.
// It is satisfied by *mqtt.Client, and can be mocked for unit testing.
type MQTTPublisher interface {
	Publish(topic string, payload []byte) error
}

// TelemetryStore is the interface used by the bridge to cache telemetry data.
// It is satisfied by *api.TelemetryCache.
type TelemetryStore interface {
	Update(vin string, data TelemetryData)
}

// TelemetryData mirrors api.TelemetryData for the bridge package. This avoids
// a circular import between bridge and api. The main.go wiring layer converts
// between the two types (or both packages use the same type via an interface).
type TelemetryData struct {
	VIN       string `json:"vin"`
	Locked    bool   `json:"locked"`
	Timestamp int64  `json:"timestamp"`
}

// Command represents a vehicle command received via REST.
type Command struct {
	CommandID string   `json:"command_id"`
	Type      string   `json:"type"` // "lock" or "unlock"
	Doors     []string `json:"doors"`
}

// MQTTCommand is the JSON payload published to the MQTT command topic.
// The REST field "type" is mapped to the MQTT field "action" per design.md.
type MQTTCommand struct {
	CommandID string   `json:"command_id"`
	Action    string   `json:"action"` // mapped from Command.Type
	Doors     []string `json:"doors"`
	Source    string   `json:"source"` // always "companion_app"
}

// MQTTResponse is the JSON payload expected on the MQTT command_responses topic.
type MQTTResponse struct {
	CommandID string `json:"command_id"`
	Status    string `json:"status"` // "success" or "failed"
	Reason    string `json:"reason"`
	Timestamp int64  `json:"timestamp"`
}

// Bridge connects the REST API to MQTT via the command tracker and telemetry
// cache. It provides methods for publishing commands, handling responses, and
// processing telemetry updates.
type Bridge struct {
	tracker   *Tracker
	publisher MQTTPublisher
}

// NewBridge creates a new Bridge with the given tracker and MQTT publisher.
func NewBridge(tracker *Tracker, publisher MQTTPublisher) *Bridge {
	return &Bridge{
		tracker:   tracker,
		publisher: publisher,
	}
}

// Tracker returns the bridge's command tracker.
func (b *Bridge) Tracker() *Tracker {
	return b.tracker
}

// SendCommand publishes a command to the MQTT topic for the given VIN and
// registers it as pending in the tracker. Returns the response channel and
// any publish error.
//
// The MQTT message maps REST "type" to MQTT "action" and sets "source" to
// "companion_app" (per 03-REQ-2.2, 03-REQ-3.3).
func (b *Bridge) SendCommand(vin string, cmd Command) (<-chan CommandResponse, error) {
	mqttCmd := MQTTCommand{
		CommandID: cmd.CommandID,
		Action:    cmd.Type, // REST "type" -> MQTT "action"
		Doors:     cmd.Doors,
		Source:    "companion_app",
	}

	payload, err := json.Marshal(mqttCmd)
	if err != nil {
		return nil, err
	}

	topic := "vehicles/" + vin + "/commands"
	if err := b.publisher.Publish(topic, payload); err != nil {
		return nil, err
	}

	ch := b.tracker.Register(cmd.CommandID)
	return ch, nil
}

// HandleResponse processes an MQTT message from vehicles/+/command_responses.
// It parses the response JSON and resolves the matching pending command in the
// tracker. Invalid payloads or unknown command_ids are logged and discarded
// (per 03-REQ-3.E1).
func (b *Bridge) HandleResponse(payload []byte) {
	var resp MQTTResponse
	if err := json.Unmarshal(payload, &resp); err != nil {
		log.Printf("WARN: invalid command response payload: %v", err)
		return
	}

	if resp.CommandID == "" {
		log.Printf("WARN: command response missing command_id")
		return
	}

	b.tracker.Resolve(resp.CommandID, CommandResponse{
		CommandID: resp.CommandID,
		Status:    resp.Status,
		Reason:    resp.Reason,
		Timestamp: resp.Timestamp,
	})
}

// HandleTelemetry processes an MQTT message from vehicles/+/telemetry.
// It extracts the VIN from the topic, parses the telemetry JSON, and updates
// the telemetry store (per 03-REQ-2.4).
func HandleTelemetry(store TelemetryStore, vin string, payload []byte) {
	var data struct {
		VIN       string `json:"vin"`
		Locked    bool   `json:"locked"`
		Timestamp int64  `json:"timestamp"`
	}

	if err := json.Unmarshal(payload, &data); err != nil {
		log.Printf("WARN: invalid telemetry payload for %s: %v", vin, err)
		return
	}

	store.Update(vin, TelemetryData{
		VIN:       vin,
		Locked:    data.Locked,
		Timestamp: data.Timestamp,
	})
}
