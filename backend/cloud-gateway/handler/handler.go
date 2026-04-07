package handler

import (
	"encoding/json"
	"net/http"
	"strings"
	"time"

	"parking-fee-service/backend/cloud-gateway/model"
	"parking-fee-service/backend/cloud-gateway/store"
)

// NATSPublisher is the interface used by the handler to publish commands to NATS.
type NATSPublisher interface {
	PublishCommand(vin string, cmd model.Command, token string) error
}

// NewSubmitCommandHandler returns an HTTP handler for POST /vehicles/{vin}/commands.
func NewSubmitCommandHandler(pub NATSPublisher, s *store.Store, timeout time.Duration) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")

		// Parse request body.
		var cmd model.Command
		if err := json.NewDecoder(r.Body).Decode(&cmd); err != nil {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": "invalid command payload"})
			return
		}

		// Validate required fields.
		if cmd.CommandID == "" || cmd.Type == "" || len(cmd.Doors) == 0 {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": "invalid command payload"})
			return
		}

		// Validate command type.
		if cmd.Type != "lock" && cmd.Type != "unlock" {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(map[string]string{"error": "invalid command type"})
			return
		}

		// Extract VIN from URL path.
		vin := r.PathValue("vin")

		// Extract bearer token from Authorization header.
		token := strings.TrimPrefix(r.Header.Get("Authorization"), "Bearer ")

		// Publish command to NATS.
		if err := pub.PublishCommand(vin, cmd, token); err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(map[string]string{"error": "failed to publish command"})
			return
		}

		// Start timeout timer.
		s.StartTimeout(cmd.CommandID, timeout)

		// Return 202 with the command echoed back.
		w.WriteHeader(http.StatusAccepted)
		json.NewEncoder(w).Encode(cmd)
	}
}

// NewGetCommandStatusHandler returns an HTTP handler for GET /vehicles/{vin}/commands/{command_id}.
func NewGetCommandStatusHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")

		commandID := r.PathValue("command_id")

		resp, found := s.GetResponse(commandID)
		if !found {
			w.WriteHeader(http.StatusNotFound)
			json.NewEncoder(w).Encode(map[string]string{"error": "command not found"})
			return
		}

		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(resp)
	}
}

// HealthHandler returns an HTTP handler for GET /health.
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(map[string]string{"status": "ok"})
	}
}
