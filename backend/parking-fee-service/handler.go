package main

import (
	"net/http"
)

// writeJSON writes a JSON response with the given status code.
func writeJSON(w http.ResponseWriter, status int, data any) {
	// TODO: implement
}

// writeError writes a JSON error response.
func writeError(w http.ResponseWriter, status int, message string) {
	// TODO: implement
}

// recoveryMiddleware recovers from panics and returns HTTP 500 with a JSON error body.
func recoveryMiddleware(next http.Handler) http.Handler {
	// TODO: implement
	return next
}

// HandleHealth handles GET /health requests.
func HandleHealth(w http.ResponseWriter, r *http.Request) {
	// TODO: implement
}

// HandleOperatorLookup returns a handler for GET /operators?lat=...&lon=... requests.
func HandleOperatorLookup(store *Store) http.HandlerFunc {
	// TODO: implement
	return func(w http.ResponseWriter, r *http.Request) {}
}

// HandleAdapterMetadata returns a handler for GET /operators/{id}/adapter requests.
func HandleAdapterMetadata(store *Store) http.HandlerFunc {
	// TODO: implement
	return func(w http.ResponseWriter, r *http.Request) {}
}

// NewRouter creates a configured HTTP handler with all routes and middleware.
func NewRouter(store *Store) http.Handler {
	// TODO: implement
	return http.NewServeMux()
}
