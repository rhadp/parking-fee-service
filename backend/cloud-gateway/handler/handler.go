package handler

import (
	"encoding/json"
	"net/http"
	"time"

	"parking-fee-service/backend/cloud-gateway/auth"
	"parking-fee-service/backend/cloud-gateway/model"
	"parking-fee-service/backend/cloud-gateway/store"
)

// NATSPublisher is the interface used by command handler to publish to NATS.
type NATSPublisher interface {
	PublishCommand(vin string, cmd model.Command, token string) error
}

// writeJSON writes v as JSON with the given status code and Content-Type: application/json.
func writeJSON(w http.ResponseWriter, status int, v interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

// writeError writes a JSON error response with Content-Type: application/json.
func writeError(w http.ResponseWriter, status int, msg string) {
	writeJSON(w, status, map[string]string{"error": msg})
}

// NewSubmitCommandHandler returns an HTTP handler for POST /vehicles/{vin}/commands.
// It validates the command payload, publishes it to NATS, starts a timeout timer,
// and returns HTTP 202 with the command echoed back.
// Requirements: 06-REQ-1.1, 06-REQ-1.E1, 06-REQ-1.E2, 06-REQ-7.1, 06-REQ-7.2
func NewSubmitCommandHandler(nc NATSPublisher, s *store.Store, timeout time.Duration) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Parse JSON body into Command.
		var cmd model.Command
		if err := json.NewDecoder(r.Body).Decode(&cmd); err != nil {
			writeError(w, http.StatusBadRequest, "invalid command payload")
			return
		}

		// Validate required fields: command_id, type, and doors must all be present.
		if cmd.CommandID == "" || cmd.Type == "" || cmd.Doors == nil {
			writeError(w, http.StatusBadRequest, "invalid command payload")
			return
		}

		// Validate command type is "lock" or "unlock".
		if cmd.Type != "lock" && cmd.Type != "unlock" {
			writeError(w, http.StatusBadRequest, "invalid command type")
			return
		}

		// Extract VIN from URL path (Go 1.22 ServeMux pattern parameter).
		vin := r.PathValue("vin")

		// Extract bearer token from context (set by auth middleware).
		token, _ := r.Context().Value(auth.TokenContextKey).(string)

		// Publish command to NATS.
		if err := nc.PublishCommand(vin, cmd, token); err != nil {
			writeError(w, http.StatusInternalServerError, "failed to publish command")
			return
		}

		// Start timeout timer for this command.
		s.StartTimeout(cmd.CommandID, timeout)

		// Return HTTP 202 Accepted with the command echoed back.
		writeJSON(w, http.StatusAccepted, cmd)
	}
}

// NewGetCommandStatusHandler returns an HTTP handler for GET /vehicles/{vin}/commands/{command_id}.
// It looks up the command in the store and returns the response, or HTTP 404 if not found.
// Requirements: 06-REQ-2.1, 06-REQ-2.E1, 06-REQ-7.1, 06-REQ-7.2
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
// Requirements: 06-REQ-4.1, 06-REQ-7.1
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	}
}
