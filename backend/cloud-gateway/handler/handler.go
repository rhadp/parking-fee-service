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

// writeJSON writes a JSON response with the given status code and value.
func writeJSON(w http.ResponseWriter, statusCode int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(statusCode)
	json.NewEncoder(w).Encode(v) //nolint:errcheck
}

// writeJSONError writes a JSON error response with the given status code and message.
func writeJSONError(w http.ResponseWriter, statusCode int, message string) {
	writeJSON(w, statusCode, map[string]string{"error": message})
}

// NewSubmitCommandHandler returns a handler that accepts command submissions,
// publishes them via the CommandPublisher, and starts a timeout timer.
func NewSubmitCommandHandler(pub CommandPublisher, s *store.Store, timeout time.Duration) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Parse request body.
		var cmd model.Command
		if err := json.NewDecoder(r.Body).Decode(&cmd); err != nil {
			writeJSONError(w, http.StatusBadRequest, "invalid command payload")
			return
		}

		// Validate required fields.
		if cmd.CommandID == "" || cmd.Type == "" || cmd.Doors == nil {
			writeJSONError(w, http.StatusBadRequest, "invalid command payload")
			return
		}

		// Validate command type.
		if cmd.Type != "lock" && cmd.Type != "unlock" {
			writeJSONError(w, http.StatusBadRequest, "invalid command type")
			return
		}

		// Extract token from context (set by auth middleware).
		token, _ := r.Context().Value(auth.TokenContextKey).(string)
		vin := r.PathValue("vin")

		// Publish command to NATS.
		if err := pub.PublishCommand(vin, cmd, token); err != nil {
			writeJSONError(w, http.StatusInternalServerError, "failed to publish command")
			return
		}

		// Start timeout timer.
		s.StartTimeout(cmd.CommandID, timeout)

		// Return 202 Accepted with command echoed back.
		writeJSON(w, http.StatusAccepted, cmd)
	}
}

// NewGetCommandStatusHandler returns a handler that looks up and returns
// the status of a previously submitted command.
func NewGetCommandStatusHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		commandID := r.PathValue("command_id")

		resp, found := s.GetResponse(commandID)
		if !found {
			writeJSONError(w, http.StatusNotFound, "command not found")
			return
		}

		writeJSON(w, http.StatusOK, resp)
	}
}

// HealthHandler returns a handler that responds with a health check status.
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	}
}
