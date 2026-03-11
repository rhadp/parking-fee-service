package main

import (
	"encoding/json"
	"fmt"
	"net/http"
)

// HandleHealth returns the health status of the service.
func HandleHealth(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
}

// HandleCommandSubmit returns an http.HandlerFunc that handles command submission.
func HandleCommandSubmit(commandStore *CommandStore, natsClient *NATSClient, knownVINs map[string]bool) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vin := r.PathValue("vin")

		// Parse request body
		var cmd CommandRequest
		if err := json.NewDecoder(r.Body).Decode(&cmd); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		// Validate required fields
		if cmd.CommandID == "" {
			writeError(w, http.StatusBadRequest, "missing required field: command_id")
			return
		}
		if cmd.Type == "" {
			writeError(w, http.StatusBadRequest, "missing required field: type")
			return
		}
		if len(cmd.Doors) == 0 {
			writeError(w, http.StatusBadRequest, "missing required field: doors")
			return
		}

		// Validate command type
		if cmd.Type != "lock" && cmd.Type != "unlock" {
			writeError(w, http.StatusBadRequest, fmt.Sprintf("invalid command type: %q", cmd.Type))
			return
		}

		// Build NATS command
		natsCmd := NATSCommand{
			CommandID: cmd.CommandID,
			Action:    cmd.Type,
			Doors:     cmd.Doors,
			Source:    "companion_app",
		}

		// Publish to NATS
		if natsClient == nil || !natsClient.IsConnected() {
			writeError(w, http.StatusServiceUnavailable, "messaging service unavailable")
			return
		}
		if err := natsClient.PublishCommand(vin, natsCmd); err != nil {
			writeError(w, http.StatusServiceUnavailable, "messaging service unavailable")
			return
		}

		// Store command as pending
		commandStore.StoreCommand(cmd.CommandID, "pending")

		// Return 202 Accepted
		writeJSON(w, http.StatusAccepted, CommandStatus{
			CommandID: cmd.CommandID,
			Status:    "pending",
		})
	}
}

// HandleCommandStatus returns an http.HandlerFunc that handles command status queries.
func HandleCommandStatus(commandStore *CommandStore, knownVINs map[string]bool) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		commandID := r.PathValue("command_id")

		status, found := commandStore.GetCommandStatus(commandID)
		if !found {
			writeError(w, http.StatusNotFound, "command not found")
			return
		}

		writeJSON(w, http.StatusOK, status)
	}
}

// writeJSON writes a JSON response with the given status code.
func writeJSON(w http.ResponseWriter, status int, data interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(data)
}

// writeError writes a JSON error response with the given status code and message.
func writeError(w http.ResponseWriter, status int, message string) {
	writeJSON(w, status, ErrorResponse{Error: message})
}

// NewRouter creates the HTTP router with all routes configured.
func NewRouter(tokenStore *TokenStore, commandStore *CommandStore, natsClient *NATSClient, knownVINs map[string]bool) http.Handler {
	mux := http.NewServeMux()

	authMw := AuthMiddleware(tokenStore, knownVINs)

	// Health endpoint - no auth required
	mux.HandleFunc("GET /health", HandleHealth)

	// Vehicle command endpoints - auth required
	mux.Handle("POST /vehicles/{vin}/commands",
		authMw(HandleCommandSubmit(commandStore, natsClient, knownVINs)))

	mux.Handle("GET /vehicles/{vin}/commands/{command_id}",
		authMw(HandleCommandStatus(commandStore, knownVINs)))

	// Wrap with recovery and default JSON 404 handler
	return &rootHandler{mux: mux}
}

// rootHandler wraps the mux to provide JSON 404 for undefined routes and panic recovery.
type rootHandler struct {
	mux *http.ServeMux
}

func (rh *rootHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	// Panic recovery
	defer func() {
		if rec := recover(); rec != nil {
			writeError(w, http.StatusInternalServerError, "internal server error")
		}
	}()

	// Check if route matches by trying to find a handler
	_, pattern := rh.mux.Handler(r)
	if pattern == "" {
		writeError(w, http.StatusNotFound, "not found")
		return
	}

	rh.mux.ServeHTTP(w, r)
}
