package handler

import (
	"encoding/json"
	"net/http"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// NATSPublisher is the interface for publishing commands to NATS.
type NATSPublisher interface {
	PublishCommand(vin string, cmd model.Command, token string) error
}

// writeJSON encodes v as JSON and writes it to w with the given status code.
// Always sets Content-Type: application/json.
func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

// writeError writes a JSON error response with the given status code and message.
func writeError(w http.ResponseWriter, status int, message string) {
	writeJSON(w, status, map[string]string{"error": message})
}

// NewSubmitCommandHandler returns an HTTP handler for POST /vehicles/{vin}/commands.
// It validates the command body, publishes it to NATS, starts a timeout timer,
// and returns HTTP 202 with the command echoed back (06-REQ-1.1).
func NewSubmitCommandHandler(nc NATSPublisher, s *store.Store, timeout time.Duration) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Parse command body.
		var cmd model.Command
		if err := json.NewDecoder(r.Body).Decode(&cmd); err != nil {
			writeError(w, http.StatusBadRequest, "invalid command payload")
			return
		}

		// Validate required fields (06-REQ-1.E1).
		if cmd.CommandID == "" || cmd.Type == "" || cmd.Doors == nil {
			writeError(w, http.StatusBadRequest, "invalid command payload")
			return
		}

		// Validate command type (06-REQ-1.E2).
		if cmd.Type != "lock" && cmd.Type != "unlock" {
			writeError(w, http.StatusBadRequest, "invalid command type")
			return
		}

		// Extract VIN from URL path and bearer token from request context.
		vin := r.PathValue("vin")
		token, _ := auth.TokenFromContext(r.Context())

		// Publish command to NATS.
		if err := nc.PublishCommand(vin, cmd, token); err != nil {
			writeError(w, http.StatusInternalServerError, "failed to publish command")
			return
		}

		// Start timeout timer (06-REQ-1.3).
		s.StartTimeout(cmd.CommandID, timeout)

		// Return 202 with command echoed back (06-REQ-1.1).
		writeJSON(w, http.StatusAccepted, cmd)
	}
}

// NewGetCommandStatusHandler returns an HTTP handler for GET /vehicles/{vin}/commands/{command_id}.
// It looks up the command response in the store and returns it (06-REQ-2.1).
func NewGetCommandStatusHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		commandID := r.PathValue("command_id")
		resp, ok := s.GetResponse(commandID)
		if !ok {
			writeError(w, http.StatusNotFound, "command not found")
			return
		}
		writeJSON(w, http.StatusOK, resp)
	}
}

// HealthHandler returns an HTTP handler for GET /health (06-REQ-4.1).
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	}
}
