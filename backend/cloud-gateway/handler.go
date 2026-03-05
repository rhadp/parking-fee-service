package main

import (
	"encoding/json"
	"net/http"
)

// HandleHealth returns the health status of the service.
func HandleHealth(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
}

// HandleCommandSubmit returns an http.HandlerFunc that processes command submissions.
func HandleCommandSubmit(commandStore *CommandStore, natsClient *NATSClient, knownVINs map[string]bool) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vin := r.PathValue("vin")

		// Check if VIN is known
		if !knownVINs[vin] {
			writeError(w, http.StatusNotFound, "unknown vehicle")
			return
		}

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
			writeError(w, http.StatusBadRequest, "invalid command type: must be 'lock' or 'unlock'")
			return
		}

		// Store command as pending
		commandStore.StoreCommand(cmd.CommandID, "pending")

		// Publish to NATS
		if natsClient != nil {
			natsCmd := NATSCommand{
				CommandID: cmd.CommandID,
				Action:    cmd.Type,
				Doors:     cmd.Doors,
				Source:    "companion_app",
			}
			if err := natsClient.PublishCommand(vin, natsCmd); err != nil {
				writeError(w, http.StatusServiceUnavailable, "messaging service unavailable")
				return
			}
		}

		// Return 202 Accepted
		writeJSON(w, http.StatusAccepted, CommandStatus{
			CommandID: cmd.CommandID,
			Status:    "pending",
		})
	}
}

// HandleCommandStatus returns an http.HandlerFunc that queries command status.
func HandleCommandStatus(commandStore *CommandStore, knownVINs map[string]bool) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vin := r.PathValue("vin")

		// Check if VIN is known
		if !knownVINs[vin] {
			writeError(w, http.StatusNotFound, "unknown vehicle")
			return
		}

		commandID := r.PathValue("command_id")
		status, ok := commandStore.GetCommandStatus(commandID)
		if !ok {
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

// NotFoundHandler returns a handler for undefined routes that returns JSON 404.
func NotFoundHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeError(w, http.StatusNotFound, "not found")
	}
}
