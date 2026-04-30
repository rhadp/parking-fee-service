// Package handler provides HTTP request handlers for the cloud-gateway
// REST API.
package handler

import (
	"encoding/json"
	"net/http"
	"strings"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// Commander is an interface for publishing commands to NATS.
// This allows handler tests to use a mock instead of a real NATS connection.
type Commander interface {
	PublishCommand(vin string, cmd model.Command, token string) error
}

// NewSubmitCommandHandler returns an HTTP handler for POST /vehicles/{vin}/commands.
// It parses the command, validates it, publishes to NATS, and starts a timeout.
func NewSubmitCommandHandler(nc Commander, s *store.Store, timeout time.Duration) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Parse JSON body
		var cmd model.Command
		if err := json.NewDecoder(r.Body).Decode(&cmd); err != nil {
			writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid command payload"})
			return
		}

		// Validate required fields
		if cmd.CommandID == "" || cmd.Type == "" || cmd.Doors == nil || len(cmd.Doors) == 0 {
			writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid command payload"})
			return
		}

		// Validate command type
		if cmd.Type != "lock" && cmd.Type != "unlock" {
			writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid command type"})
			return
		}

		// Extract VIN from URL path
		vin := r.PathValue("vin")

		// Extract token from Authorization header
		token := extractToken(r)

		// Publish to NATS
		if err := nc.PublishCommand(vin, cmd, token); err != nil {
			writeJSON(w, http.StatusInternalServerError, map[string]string{"error": "failed to publish command"})
			return
		}

		// Start timeout timer
		s.StartTimeout(cmd.CommandID, timeout)

		// Return 202 with command echoed back
		writeJSON(w, http.StatusAccepted, cmd)
	}
}

// NewGetCommandStatusHandler returns an HTTP handler for GET /vehicles/{vin}/commands/{command_id}.
// It looks up the command response in the store.
func NewGetCommandStatusHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		commandID := r.PathValue("command_id")

		resp, found := s.GetResponse(commandID)
		if !found {
			writeJSON(w, http.StatusNotFound, map[string]string{"error": "command not found"})
			return
		}

		writeJSON(w, http.StatusOK, resp)
	}
}

// HealthHandler returns an HTTP handler for GET /health.
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	}
}

// writeJSON writes a JSON response with the given status code and value.
func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(v)
}

// extractToken extracts the bearer token from the Authorization header.
func extractToken(r *http.Request) string {
	authHeader := r.Header.Get("Authorization")
	if strings.HasPrefix(authHeader, "Bearer ") {
		return strings.TrimPrefix(authHeader, "Bearer ")
	}
	return ""
}
