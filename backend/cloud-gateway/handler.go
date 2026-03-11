package main

import "net/http"

// HandleHealth returns the health status of the service.
func HandleHealth(w http.ResponseWriter, r *http.Request) {
	// Stub - to be implemented
}

// HandleCommandSubmit returns an http.HandlerFunc that handles command submission.
func HandleCommandSubmit(commandStore *CommandStore, natsClient *NATSClient, knownVINs map[string]bool) http.HandlerFunc {
	// Stub - to be implemented
	return func(w http.ResponseWriter, r *http.Request) {}
}

// HandleCommandStatus returns an http.HandlerFunc that handles command status queries.
func HandleCommandStatus(commandStore *CommandStore, knownVINs map[string]bool) http.HandlerFunc {
	// Stub - to be implemented
	return func(w http.ResponseWriter, r *http.Request) {}
}

// writeJSON writes a JSON response with the given status code.
func writeJSON(w http.ResponseWriter, status int, data interface{}) {
	// Stub - to be implemented
}

// writeError writes a JSON error response with the given status code and message.
func writeError(w http.ResponseWriter, status int, message string) {
	// Stub - to be implemented
}

// NewRouter creates the HTTP router with all routes configured.
func NewRouter(tokenStore *TokenStore, commandStore *CommandStore, natsClient *NATSClient, knownVINs map[string]bool) http.Handler {
	// Stub - to be implemented
	return http.NewServeMux()
}
