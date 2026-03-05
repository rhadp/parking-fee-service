package main

import "net/http"

// HandleHealth returns the health status of the service.
func HandleHealth(w http.ResponseWriter, r *http.Request) {
	// Stub: not yet implemented
}

// HandleCommandSubmit returns an http.HandlerFunc that processes command submissions.
func HandleCommandSubmit(commandStore *CommandStore, natsClient *NATSClient, knownVINs map[string]bool) http.HandlerFunc {
	// Stub: not yet implemented
	return func(w http.ResponseWriter, r *http.Request) {}
}

// HandleCommandStatus returns an http.HandlerFunc that queries command status.
func HandleCommandStatus(commandStore *CommandStore, knownVINs map[string]bool) http.HandlerFunc {
	// Stub: not yet implemented
	return func(w http.ResponseWriter, r *http.Request) {}
}

// writeJSON writes a JSON response with the given status code.
func writeJSON(w http.ResponseWriter, status int, data interface{}) {
	// Stub: not yet implemented
}

// writeError writes a JSON error response with the given status code and message.
func writeError(w http.ResponseWriter, status int, message string) {
	// Stub: not yet implemented
}

// NotFoundHandler returns a handler for undefined routes that returns JSON 404.
func NotFoundHandler() http.HandlerFunc {
	// Stub: not yet implemented
	return func(w http.ResponseWriter, r *http.Request) {}
}
