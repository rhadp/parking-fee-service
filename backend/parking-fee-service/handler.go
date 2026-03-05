package main

import "net/http"

// writeJSON writes a JSON response with the given status code.
func writeJSON(w http.ResponseWriter, status int, data any) {
	// Stub: not yet implemented
}

// writeError writes a JSON error response.
func writeError(w http.ResponseWriter, status int, message string) {
	// Stub: not yet implemented
}

// recoveryMiddleware recovers from panics and returns HTTP 500 with a JSON error body.
func recoveryMiddleware(next http.Handler) http.Handler {
	// Stub: not yet implemented
	return next
}

// HandleHealth handles GET /health requests.
func HandleHealth(w http.ResponseWriter, r *http.Request) {
	// Stub: not yet implemented
}

// HandleOperatorLookup returns a handler for GET /operators?lat=...&lon=...
func HandleOperatorLookup(store *Store) http.HandlerFunc {
	// Stub: not yet implemented
	return func(w http.ResponseWriter, r *http.Request) {}
}

// HandleAdapterMetadata returns a handler for GET /operators/{id}/adapter
func HandleAdapterMetadata(store *Store) http.HandlerFunc {
	// Stub: not yet implemented
	return func(w http.ResponseWriter, r *http.Request) {}
}

// NewRouter creates the HTTP router with all routes registered.
func NewRouter(store *Store) http.Handler {
	mux := http.NewServeMux()
	// Stub: routes not yet registered
	return recoveryMiddleware(mux)
}
