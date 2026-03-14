// Package handler provides HTTP request handlers for the cloud-gateway service.
package handler

import (
	"encoding/json"
	"net/http"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// NATSPublisher abstracts NATS publishing for testability.
type NATSPublisher interface {
	PublishCommand(vin string, cmd model.Command, bearerToken string) error
}

// writeJSON writes a JSON response with the given status code and body.
func writeJSON(w http.ResponseWriter, status int, body any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(body)
}

// writeError writes a JSON error response.
func writeError(w http.ResponseWriter, status int, message string) {
	writeJSON(w, status, map[string]string{"error": message})
}

// authenticate validates the Authorization header and checks VIN authorization.
// Returns (token, true) on success, or writes an error response and returns ("", false).
func authenticate(w http.ResponseWriter, r *http.Request, a *auth.Authenticator, vin string) (string, bool) {
	header := r.Header.Get("Authorization")
	token, err := auth.ValidateToken(header)
	if err != nil {
		writeError(w, http.StatusUnauthorized, "unauthorized")
		return "", false
	}
	if !a.AuthorizeVIN(token, vin) {
		writeError(w, http.StatusForbidden, "forbidden")
		return "", false
	}
	return token, true
}

// NewCommandHandler returns an http.HandlerFunc for command submission.
// POST /vehicles/{vin}/commands
func NewCommandHandler(s *store.Store, pub NATSPublisher, a *auth.Authenticator) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vin := r.PathValue("vin")

		// Authenticate
		token, ok := authenticate(w, r, a, vin)
		if !ok {
			return
		}

		// Parse and validate command payload
		var raw json.RawMessage
		if err := json.NewDecoder(r.Body).Decode(&raw); err != nil {
			writeError(w, http.StatusBadRequest, "invalid command payload")
			return
		}

		cmd, err := model.ParseCommand([]byte(raw))
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid command payload")
			return
		}

		// Store the command as pending
		s.Add(model.CommandStatus{
			CommandID: cmd.CommandID,
			Status:    "pending",
			VIN:       vin,
			CreatedAt: time.Now(),
		})

		// Publish to NATS
		if err := pub.PublishCommand(vin, *cmd, token); err != nil {
			writeError(w, http.StatusInternalServerError, "failed to publish command")
			return
		}

		// Return 202 Accepted
		writeJSON(w, http.StatusAccepted, map[string]string{
			"command_id": cmd.CommandID,
			"status":     "pending",
		})
	}
}

// NewStatusHandler returns an http.HandlerFunc for command status queries.
// GET /vehicles/{vin}/commands/{command_id}
func NewStatusHandler(s *store.Store, a *auth.Authenticator) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vin := r.PathValue("vin")
		commandID := r.PathValue("command_id")

		// Authenticate
		if _, ok := authenticate(w, r, a, vin); !ok {
			return
		}

		// Look up the command
		cs, found := s.Get(commandID)
		if !found {
			writeError(w, http.StatusNotFound, "command not found")
			return
		}

		writeJSON(w, http.StatusOK, cs)
	}
}

// HealthHandler returns an http.HandlerFunc for health checks.
// GET /health
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	}
}
