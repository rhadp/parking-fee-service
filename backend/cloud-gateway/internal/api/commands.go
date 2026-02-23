package api

import (
	"encoding/json"
	"io"
	"log"
	"net/http"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/internal/bridge"
)

// CommandRequest is the JSON body expected by POST /vehicles/{vin}/commands.
type CommandRequest struct {
	CommandID string   `json:"command_id"`
	Type      string   `json:"type"`
	Doors     []string `json:"doors"`
}

// MQTTCommand is the JSON payload published to the MQTT command topic.
// The REST field "type" is mapped to the MQTT field "action" per design.md.
type MQTTCommand struct {
	CommandID string   `json:"command_id"`
	Action    string   `json:"action"`
	Doors     []string `json:"doors"`
	Source    string   `json:"source"`
}

// CommandPublisher is the interface used by the command handler to publish
// commands to MQTT. This allows the handler to be tested without a real
// MQTT client.
type CommandPublisher interface {
	// Publish sends a message to the specified MQTT topic.
	Publish(topic string, payload []byte) error
}

// CommandHandler handles POST /vehicles/{vin}/commands requests. It validates
// the request body, registers a pending command in the tracker, publishes the
// command to MQTT, and waits for a response (or timeout).
type CommandHandler struct {
	tracker   *bridge.Tracker
	publisher CommandPublisher
}

// NewCommandHandler creates a handler with the given tracker and MQTT publisher.
func NewCommandHandler(tracker *bridge.Tracker, publisher CommandPublisher) *CommandHandler {
	return &CommandHandler{
		tracker:   tracker,
		publisher: publisher,
	}
}

// ServeHTTP processes POST /vehicles/{vin}/commands requests.
func (h *CommandHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	vin := r.PathValue("vin")
	if vin == "" {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "missing VIN in URL path"})
		return
	}

	// Read and parse the request body
	body, err := io.ReadAll(r.Body)
	if err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "failed to read request body"})
		return
	}

	var cmd CommandRequest
	if err := json.Unmarshal(body, &cmd); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid JSON: " + err.Error()})
		return
	}

	// Validate required fields
	if cmd.CommandID == "" {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "command_id is required"})
		return
	}
	if cmd.Type == "" {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "type is required"})
		return
	}
	if cmd.Type != "lock" && cmd.Type != "unlock" {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "type must be 'lock' or 'unlock'"})
		return
	}
	if cmd.Doors == nil || len(cmd.Doors) == 0 {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "doors is required"})
		return
	}

	// Map REST "type" to MQTT "action"
	mqttCmd := MQTTCommand{
		CommandID: cmd.CommandID,
		Action:    cmd.Type,
		Doors:     cmd.Doors,
		Source:    "companion_app",
	}

	mqttPayload, err := json.Marshal(mqttCmd)
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": "internal error"})
		return
	}

	// Publish to MQTT
	topic := "vehicles/" + vin + "/commands"
	if err := h.publisher.Publish(topic, mqttPayload); err != nil {
		// MQTT publish failed (broker unreachable). Per design decision D1,
		// return 202 Accepted immediately in degraded mode (03-REQ-2.E1).
		log.Printf("MQTT publish failed (degraded mode): %v", err)
		writeJSON(w, http.StatusAccepted, map[string]interface{}{
			"command_id": cmd.CommandID,
			"status":     "pending",
		})
		return
	}

	// Register pending command and block until response or timeout
	// (design decision D1: synchronous wait when MQTT is available).
	ch := h.tracker.Register(cmd.CommandID)
	resp := <-ch

	switch resp.Status {
	case "timeout":
		writeJSON(w, http.StatusGatewayTimeout, map[string]interface{}{
			"command_id": resp.CommandID,
			"status":     "timeout",
		})
	default:
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"command_id": resp.CommandID,
			"status":     resp.Status,
		})
	}
}
