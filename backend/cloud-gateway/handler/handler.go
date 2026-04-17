// Package handler provides HTTP handlers for the CLOUD_GATEWAY REST API.
package handler

import (
	"encoding/json"
	"net/http"
	"time"

	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/model"
	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/store"
)

// NATSPublisher is the interface for publishing commands to NATS.
// Using an interface here lets handler tests inject a mock without requiring
// a real NATS server.
type NATSPublisher interface {
	PublishCommand(vin string, cmd model.Command, token string) error
}

// writeJSON writes a JSON-encoded value with the given HTTP status code.
// It sets Content-Type: application/json on the response.
func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	if err := json.NewEncoder(w).Encode(v); err != nil {
		// Best-effort: headers already sent, nothing more we can do.
		_ = err
	}
}

// writeError writes a JSON error response: {"error":"<message>"}.
func writeError(w http.ResponseWriter, status int, msg string) {
	writeJSON(w, status, map[string]string{"error": msg})
}

// NewSubmitCommandHandler returns an HTTP handler for POST /vehicles/{vin}/commands.
// It parses the request body, validates the command, publishes to NATS, starts a
// timeout timer, and responds with HTTP 202 echoing the command back.
func NewSubmitCommandHandler(nc NATSPublisher, s *store.Store, timeout time.Duration) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Decode the command from the request body.
		var cmd model.Command
		if err := json.NewDecoder(r.Body).Decode(&cmd); err != nil {
			writeError(w, http.StatusBadRequest, "invalid command payload")
			return
		}

		// Validate required fields: command_id, type, doors.
		if cmd.CommandID == "" || cmd.Type == "" || cmd.Doors == nil {
			writeError(w, http.StatusBadRequest, "invalid command payload")
			return
		}

		// Validate the command type.
		if cmd.Type != "lock" && cmd.Type != "unlock" {
			writeError(w, http.StatusBadRequest, "invalid command type")
			return
		}

		// Extract the VIN from the URL path (populated by Go 1.22 ServeMux).
		vin := r.PathValue("vin")

		// Extract the bearer token from the request context (set by auth middleware).
		token, _ := r.Context().Value(auth.TokenKey).(string)

		// Publish the command to NATS.
		if err := nc.PublishCommand(vin, cmd, token); err != nil {
			writeError(w, http.StatusInternalServerError, "failed to publish command")
			return
		}

		// Start the timeout timer for this command.
		s.StartTimeout(cmd.CommandID, timeout)

		// Respond with HTTP 202 and echo the command back.
		writeJSON(w, http.StatusAccepted, cmd)
	}
}

// NewGetCommandStatusHandler returns an HTTP handler for GET /vehicles/{vin}/commands/{command_id}.
// It looks up the command response in the store and returns it as JSON.
func NewGetCommandStatusHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		commandID := r.PathValue("command_id")

		resp, found := s.GetResponse(commandID)
		if !found {
			writeError(w, http.StatusNotFound, "command not found")
			return
		}

		writeJSON(w, http.StatusOK, resp)
	}
}

// HealthHandler returns an HTTP handler for GET /health.
// It responds with HTTP 200 and {"status":"ok"}.
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	}
}
