package handler

import (
	"encoding/json"
	"net/http"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// CommandPublisher defines the interface for publishing commands to NATS.
type CommandPublisher interface {
	PublishCommand(vin string, cmd model.Command, token string) error
}

// NewSubmitCommandHandler returns a handler for POST /vehicles/{vin}/commands.
// It parses the JSON body, validates required fields and command type,
// publishes the command via the publisher, starts a timeout timer, and
// returns HTTP 202 with the command echoed back.
func NewSubmitCommandHandler(publisher CommandPublisher, s *store.Store, timeout time.Duration) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")

		// Parse JSON body into Command.
		var cmd model.Command
		if err := json.NewDecoder(r.Body).Decode(&cmd); err != nil {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": "invalid command payload"})
			return
		}

		// Validate required fields: command_id, type, doors.
		if cmd.CommandID == "" || cmd.Type == "" || cmd.Doors == nil {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": "invalid command payload"})
			return
		}

		// Validate command type is "lock" or "unlock".
		if cmd.Type != "lock" && cmd.Type != "unlock" {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": "invalid command type"})
			return
		}

		// Extract token from request context (set by auth middleware).
		token, _ := r.Context().Value(auth.TokenContextKey).(string)

		// Extract VIN from URL path via ServeMux pattern matching.
		vin := r.PathValue("vin")

		// Publish command to NATS.
		if err := publisher.PublishCommand(vin, cmd, token); err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": "failed to publish command"})
			return
		}

		// Start timeout timer for this command.
		s.StartTimeout(cmd.CommandID, timeout)

		// Return HTTP 202 with command JSON echoed back.
		w.WriteHeader(http.StatusAccepted)
		json.NewEncoder(w).Encode(cmd)
	}
}

// NewGetCommandStatusHandler returns a handler for GET /vehicles/{vin}/commands/{command_id}.
// It looks up the command response in the store and returns it, or HTTP 404
// if not found.
func NewGetCommandStatusHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")

		// Extract command_id from URL path via ServeMux pattern matching.
		commandID := r.PathValue("command_id")

		resp, ok := s.GetResponse(commandID)
		if !ok {
			w.WriteHeader(http.StatusNotFound)
			json.NewEncoder(w).Encode(map[string]string{"error": "command not found"})
			return
		}

		json.NewEncoder(w).Encode(resp)
	}
}

// HealthHandler returns a handler for GET /health that responds with
// {"status":"ok"} and HTTP 200.
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]string{"status": "ok"})
	}
}
